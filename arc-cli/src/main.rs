//! `arc` command-line interface.

use arc_core::linear::LinearSolverKind;
use arc_core::model::Network;
use arc_core::parser::MatpowerParser;
use arc_core::regression::RegressionHarness;
use arc_core::solver::{ACPowerFlow, ACPowerFlowOptions, DCPowerFlow};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::PathBuf;
use std::process;

/// arc — open-source deterministic power flow kernel for grid interconnection studies
#[derive(Parser, Debug)]
#[command(
    name = "arc",
    author = "arc contributors",
    version = "0.1.0",
    about = "Deterministic power flow kernel for transmission network simulation"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Solves steady-state power flow on a given case file (.m or .json)
    Solve(SolveArgs),
    /// Alias for solve
    Run(SolveArgs),
    /// Runs automated numerical regression test harness across all benchmark networks
    Test,
    /// Displays build and environment metadata
    Info,
}

/// Arguments for `arc solve` and `arc run`.
#[derive(Parser, Debug, Clone)]
pub struct SolveArgs {
    /// Path to MATPOWER (.m) or arc Grid JSON (.json) case file
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Power flow formulation mode
    #[arg(short, long, value_enum, default_value_t = SolveMode::Ac)]
    pub mode: SolveMode,

    /// Linear solver algorithm backend
    #[arg(short, long, value_enum, default_value_t = CliSolverKind::Sparse)]
    pub solver: CliSolverKind,

    /// Convergence tolerance for maximum mismatch in per-unit (AC mode)
    #[arg(short, long, default_value_t = 1e-8)]
    pub tolerance: f64,

    /// Maximum number of Newton-Raphson iterations (AC mode)
    #[arg(short = 'i', long, default_value_t = 30)]
    pub max_iterations: usize,

    /// Output reporting format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Default)]
pub enum SolveMode {
    /// Non-linear AC Newton-Raphson in polar coordinates
    #[default]
    Ac,
    /// Linear DC power flow (B * theta = P)
    Dc,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Default)]
pub enum CliSolverKind {
    /// Direct sparse LU with Markowitz threshold pivoting (O(N^1.2))
    #[default]
    Sparse,
    /// Dense Gaussian elimination with partial pivoting (O(N^3))
    Dense,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Default)]
pub enum OutputFormat {
    /// Aligned human-readable terminal table
    #[default]
    Table,
    /// Structured JSON output for piping into scripts or web dashboards
    Json,
}

impl From<CliSolverKind> for LinearSolverKind {
    fn from(kind: CliSolverKind) -> Self {
        match kind {
            CliSolverKind::Sparse => LinearSolverKind::Sparse,
            CliSolverKind::Dense => LinearSolverKind::Dense,
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Solve(args)) | Some(Commands::Run(args)) => {
            if let Err(e) = execute_solve(&args) {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
        Some(Commands::Test) => {
            println!(
                "Running arc numerical regression harness against pandapower 3.5.4 oracle...\n"
            );
            match RegressionHarness::run_all() {
                Ok(report) => {
                    report.print_table();
                    if report.all_passed() {
                        process::exit(0);
                    } else {
                        process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("Error running regression harness: {err}");
                    process::exit(1);
                }
            }
        }
        Some(Commands::Info) | None => {
            println!("arc v0.1.0 (pre-alpha / garage)");
            println!(
                "Open-source, deterministic power flow kernel for grid interconnection studies"
            );
            println!("Supported solvers: Linear DC, Polar Newton-Raphson AC");
            println!(
                "Supported linear solvers: Sparse (Markowitz LU), Dense (Gaussian Elimination)"
            );
            println!("Supported case formats: MATPOWER .m, arc Grid JSON");
            println!("\nUsage:");
            println!(
                "  arc solve <FILE> [--mode ac|dc] [--solver sparse|dense] [--format table|json]"
            );
            println!("  arc test");
            println!("  arc info");
        }
    }
}

fn execute_solve(args: &SolveArgs) -> Result<(), String> {
    let content = fs::read_to_string(&args.file)
        .map_err(|e| format!("Could not read file '{}': {e}", args.file.display()))?;

    let is_json = args.file.extension().is_some_and(|ext| ext == "json");

    let network: Network = if is_json {
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON file '{}': {e}", args.file.display()))?
    } else {
        MatpowerParser::parse(&content, true).map_err(|e| {
            format!(
                "Failed to parse MATPOWER file '{}': {e}",
                args.file.display()
            )
        })?
    };

    let solver_kind: LinearSolverKind = args.solver.into();

    match args.mode {
        SolveMode::Ac => {
            let options = ACPowerFlowOptions {
                max_iterations: args.max_iterations,
                tolerance: args.tolerance,
                solver_kind,
            };
            let result = ACPowerFlow::solve_with_options(&network, &options)
                .map_err(|e| format!("AC solve failed: {e}"))?;

            if !result.converged {
                return Err(format!(
                    "AC power flow did not converge within {} iterations (final mismatch: {:.2e} pu)",
                    result.iterations, result.final_mismatch_pu
                ));
            }

            match args.format {
                OutputFormat::Json => {
                    let json_str = serde_json::to_string_pretty(&result)
                        .map_err(|e| format!("Serialization error: {e}"))?;
                    println!("{json_str}");
                }
                OutputFormat::Table => {
                    print_network_summary(&network, args, solver_kind);
                    println!(
                        "Solving non-linear AC power flow (polar Newton-Raphson, {solver_kind})..."
                    );
                    println!(
                        "Converged:   {} in {} iterations (max mismatch: {:.2e} pu)",
                        result.converged, result.iterations, result.final_mismatch_pu
                    );
                    println!(
                        "Losses:      {:.3} MW, {:.3} MVar\n",
                        result.total_p_loss_mw, result.total_q_loss_mvar
                    );

                    println!("=== Bus Results ===");
                    println!(
                        "{:<6} {:<10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
                        "Bus",
                        "Type",
                        "Vm (pu)",
                        "Va (deg)",
                        "Pgen (MW)",
                        "Qgen (MV)",
                        "Pload (MW)",
                        "Qload (MV)"
                    );
                    println!("----------------------------------------------------------------------------------");
                    for (id, b) in &result.bus_results {
                        let bus = &network.buses[id];
                        println!(
                            "{:<6} {:<10} {:>10.4} {:>10.4} {:>10.2} {:>10.2} {:>10.2} {:>10.2}",
                            id,
                            format!("{:?}", bus.bus_type),
                            b.vm_pu,
                            b.va_deg,
                            b.p_gen_mw,
                            b.q_gen_mvar,
                            b.p_load_mw,
                            b.q_load_mvar
                        );
                    }

                    println!("\n=== Branch Flows & Losses ===");
                    println!(
                        "{:<8} {:>6} -> {:<6} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
                        "Branch",
                        "From",
                        "To",
                        "P_from (MW)",
                        "Q_from (MV)",
                        "P_to (MW)",
                        "Q_to (MV)",
                        "P_loss (MW)",
                        "Q_loss (MV)"
                    );
                    println!("------------------------------------------------------------------------------------------------------");
                    for (id, f) in &result.branch_flows {
                        println!(
                            "{:<8} {:>6} -> {:<6} {:>12.2} {:>12.2} {:>12.2} {:>12.2} {:>12.2} {:>12.2}",
                            id, f.from_bus, f.to_bus, f.p_from_mw, f.q_from_mw, f.p_to_mw, f.q_to_mw, f.p_loss_mw, f.q_loss_mvar
                        );
                    }
                }
            }
        }
        SolveMode::Dc => {
            let result = DCPowerFlow::solve_with_solver(&network, solver_kind)
                .map_err(|e| format!("DC solve failed: {e}"))?;

            match args.format {
                OutputFormat::Json => {
                    let json_str = serde_json::to_string_pretty(&result)
                        .map_err(|e| format!("Serialization error: {e}"))?;
                    println!("{json_str}");
                }
                OutputFormat::Table => {
                    print_network_summary(&network, args, solver_kind);
                    println!("Solving linear DC power flow (B * theta = P, {solver_kind})...");
                    println!("Solved in 1 iteration (lossless DC approximation)\n");

                    println!("=== Bus Results ===");
                    println!(
                        "{:<6} {:<10} {:>10} {:>10} {:>10} {:>10}",
                        "Bus", "Type", "Vm (pu)", "Va (deg)", "Pgen (MW)", "Pload (MW)"
                    );
                    println!("----------------------------------------------------------------");
                    for (id, b) in &result.bus_results {
                        let bus = &network.buses[id];
                        println!(
                            "{:<6} {:<10} {:>10.4} {:>10.4} {:>10.2} {:>10.2}",
                            id,
                            format!("{:?}", bus.bus_type),
                            b.vm_pu,
                            b.va_deg,
                            b.p_gen_mw,
                            b.p_load_mw
                        );
                    }

                    println!("\n=== Branch Flows ===");
                    println!(
                        "{:<8} {:>6} -> {:<6} {:>14} {:>14}",
                        "Branch", "From", "To", "P_from (MW)", "P_to (MW)"
                    );
                    println!("--------------------------------------------------------");
                    for (id, f) in &result.branch_flows {
                        println!(
                            "{:<8} {:>6} -> {:<6} {:>14.2} {:>14.2}",
                            id, f.from_bus, f.to_bus, f.p_from_mw, f.p_to_mw
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_network_summary(network: &Network, args: &SolveArgs, solver_kind: LinearSolverKind) {
    let n = network.bus_count();
    let m = network.branch_count();
    let nnz = n + 2 * m;
    let sparsity_pct = if n > 0 {
        (1.0 - (nnz as f64 / (n * n) as f64)) * 100.0
    } else {
        0.0
    };

    println!(
        "Loaded network '{}' from {}",
        args.file.file_name().unwrap_or_default().to_string_lossy(),
        args.file.display()
    );
    println!("  Buses:      {n}");
    println!("  Branches:   {m}");
    println!(
        "  Sparsity:   {sparsity_pct:.1}% ({nnz} non-zeros out of {})",
        n * n
    );
    println!("  Linear:     {solver_kind}");
    println!("  Generators: {}", network.generators.len());
    println!("  Loads:      {}", network.loads.len());
    println!("  Shunts:     {}", network.shunts.len());
    println!("  Base MVA:   {:.1} MVA", network.base_mva);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_argument_parsing() {
        let args = Cli::try_parse_from(["arc", "solve", "data/cases/case14.m"]).unwrap();
        match args.command {
            Some(Commands::Solve(solve_args)) => {
                assert_eq!(solve_args.file, PathBuf::from("data/cases/case14.m"));
                assert_eq!(solve_args.mode, SolveMode::Ac);
                assert_eq!(solve_args.solver, CliSolverKind::Sparse);
                assert_eq!(solve_args.format, OutputFormat::Table);
                assert_eq!(solve_args.tolerance, 1e-8);
                assert_eq!(solve_args.max_iterations, 30);
            }
            _ => panic!("Expected Solve command"),
        }
    }

    #[test]
    fn test_cli_run_alias_parsing() {
        let args = Cli::try_parse_from([
            "arc",
            "run",
            "data/cases/case14.m",
            "--mode",
            "dc",
            "--solver",
            "dense",
            "--format",
            "json",
        ])
        .unwrap();
        match args.command {
            Some(Commands::Run(solve_args)) => {
                assert_eq!(solve_args.mode, SolveMode::Dc);
                assert_eq!(solve_args.solver, CliSolverKind::Dense);
                assert_eq!(solve_args.format, OutputFormat::Json);
            }
            _ => panic!("Expected Run command"),
        }
    }
}

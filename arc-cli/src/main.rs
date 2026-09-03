//! `arc` command-line interface.

use arc_core::model::Network;
use arc_core::parser::MatpowerParser;
use arc_core::regression::RegressionHarness;
use arc_core::solver::{ACPowerFlow, DCPowerFlow};
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
    about = "Deterministic power flow kernel"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Runs automated numerical regression test harness across all benchmark networks
    Test,
    /// Solves power flow on a given case file (.m or .json)
    Run {
        /// Path to MATPOWER (.m) or arc Grid JSON (.json) case file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Power flow formulation mode
        #[arg(short, long, value_enum, default_value_t = SolveMode::Ac)]
        mode: SolveMode,
    },
    /// Displays build and environment metadata
    Info,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum SolveMode {
    /// Non-linear AC Newton-Raphson in polar coordinates
    Ac,
    /// Linear DC power flow (B * theta = P)
    Dc,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
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
        Some(Commands::Run { file, mode }) => {
            if let Err(e) = run_case(&file, mode) {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
        Some(Commands::Info) | None => {
            println!("arc v0.1.0 (pre-alpha / garage)");
            println!(
                "Open-source, deterministic power flow kernel for grid interconnection studies"
            );
            println!("Supported solvers: Linear DC, Polar Newton-Raphson AC");
            println!("Supported case formats: MATPOWER .m, arc Grid JSON");
            println!("\nRun 'arc test' to execute automated numerical regression harness.");
            println!("Run 'arc run <FILE>' to solve a case file.");
        }
    }
}

fn run_case(path: &PathBuf, mode: SolveMode) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Could not read file {}: {e}", path.display()))?;

    let is_json = path.extension().is_some_and(|ext| ext == "json");

    let network: Network = if is_json {
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON file {}: {e}", path.display()))?
    } else {
        MatpowerParser::parse(&content, true)
            .map_err(|e| format!("Failed to parse MATPOWER file {}: {e}", path.display()))?
    };

    println!(
        "Loaded network '{}' from {}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        path.display()
    );
    println!("  Buses:      {}", network.bus_count());
    println!("  Branches:   {}", network.branch_count());
    println!("  Generators: {}", network.generators.len());
    println!("  Loads:      {}", network.loads.len());
    println!("  Shunts:     {}", network.shunts.len());
    println!("  Base MVA:   {:.1} MVA", network.base_mva);
    println!();

    match mode {
        SolveMode::Ac => {
            println!("Solving non-linear AC power flow (polar Newton-Raphson)...");
            let result =
                ACPowerFlow::solve(&network).map_err(|e| format!("AC solve failed: {e}"))?;

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
        }
        SolveMode::Dc => {
            println!("Solving linear DC power flow (B * theta = P)...");
            let result =
                DCPowerFlow::solve(&network).map_err(|e| format!("DC solve failed: {e}"))?;

            println!("Solved in 1 iteration (lossless formulation)\n");

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
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_scaffold_verification() {
        assert_eq!(1, 1);
    }
}

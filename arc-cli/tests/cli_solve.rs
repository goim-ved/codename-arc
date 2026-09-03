//! End-to-end integration tests for the `arc` command-line executable.

use std::process::Command;

fn arc_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current test exe path");
    path.pop(); // Pop test binary name
    if path.ends_with("deps") {
        path.pop(); // Pop "deps" directory
    }
    path.push("arc.exe");
    if !path.exists() {
        // Fallback for non-Windows or direct target dir
        path.set_extension("");
    }
    path
}

#[test]
fn test_cli_solve_case14_ac_table() {
    let output = Command::new(arc_bin())
        .args(["solve", "data/cases/case14.m"])
        .current_dir("..")
        .output()
        .expect("Failed to execute arc CLI");

    assert!(output.status.success(), "Process must exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Loaded network 'case14.m'"));
    assert!(stdout.contains("Converged:   true in 4 iterations"));
    assert!(stdout.contains("=== Bus Results ==="));
    assert!(stdout.contains("=== Branch Flows & Losses ==="));
}

#[test]
fn test_cli_solve_case14_ac_json() {
    let output = Command::new(arc_bin())
        .args(["solve", "data/cases/case14.m", "--format", "json"])
        .current_dir("..")
        .output()
        .expect("Failed to execute arc CLI");

    assert!(output.status.success(), "Process must exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("CLI JSON output must be valid JSON");
    assert_eq!(parsed["converged"], true);
    assert_eq!(parsed["iterations"], 4);
    assert!(parsed["bus_results"]["1"]["vm_pu"].as_f64().unwrap() > 0.0);
}

#[test]
fn test_cli_solve_case14_dc() {
    let output = Command::new(arc_bin())
        .args(["solve", "data/cases/case14.m", "--mode", "dc"])
        .current_dir("..")
        .output()
        .expect("Failed to execute arc CLI");

    assert!(output.status.success(), "Process must exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("linear DC power flow"));
    assert!(stdout.contains("Solved in 1 iteration"));
    assert!(stdout.contains("=== Branch Flows ==="));
}

#[test]
fn test_cli_solve_json_case() {
    let output = Command::new(arc_bin())
        .args(["solve", "data/cases/case14.json"])
        .current_dir("..")
        .output()
        .expect("Failed to execute arc CLI");

    assert!(output.status.success(), "Process must exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Loaded network 'case14.json'"));
    assert!(stdout.contains("Converged:   true"));
}

#[test]
fn test_cli_solve_dense_solver() {
    let output = Command::new(arc_bin())
        .args(["solve", "data/cases/case14.m", "--solver", "dense"])
        .current_dir("..")
        .output()
        .expect("Failed to execute arc CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dense (Gaussian Elimination)"));
    assert!(stdout.contains("Converged:   true"));
}

#[test]
fn test_cli_solve_non_existent_file_fails() {
    let output = Command::new(arc_bin())
        .args(["solve", "non_existent_network.m"])
        .current_dir("..")
        .output()
        .expect("Failed to execute arc CLI");

    assert!(!output.status.success(), "Process must exit with code 1");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error: Could not read file"));
}

#[test]
fn test_cli_info() {
    let output = Command::new(arc_bin())
        .arg("info")
        .current_dir("..")
        .output()
        .expect("Failed to execute arc CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("arc v0.1.0"));
    assert!(stdout.contains("Usage:"));
}

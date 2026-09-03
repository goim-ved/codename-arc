//! Milestone 5: Standard IEEE benchmark case validation for IEEE 9-bus and IEEE 14-bus networks.
//!
//! Tests both MATPOWER .m parsing and arc Grid JSON deserialization, solves with both AC
//! and DC solvers, and asserts that maximum absolute error (MAE) against the pandapower 3.5.4
//! numerical oracle satisfies:
//! - Voltage magnitude MAE < 1e-6 p.u.
//! - Voltage phase angle MAE < 1e-4 rad

use arc_core::model::Network;
use arc_core::parser::MatpowerParser;
use arc_core::solver::{ACPowerFlow, DCPowerFlow};
use serde::Deserialize;
use std::collections::BTreeMap;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OracleBusResult {
    vm_pu: f64,
    va_rad: f64,
    va_deg: f64,
    p_mw: f64,
    #[serde(default)]
    q_mvar: Option<f64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OracleModeData {
    converged: bool,
    buses: BTreeMap<usize, OracleBusResult>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OracleCaseData {
    case: String,
    ac: OracleModeData,
    dc: OracleModeData,
}

#[test]
fn test_case9_ac_and_dc_oracle_validation() {
    let case9_m_str = include_str!("../../data/cases/case9.m");
    let case9_json_str = include_str!("../../data/cases/case9.json");
    let oracle_str = include_str!("../../data/cases/case9_oracle.json");

    let oracle: OracleCaseData = serde_json::from_str(oracle_str).unwrap();

    // 1. Validate both .m parser and .json deserializer produce equivalent networks
    let net_from_m = MatpowerParser::parse(case9_m_str, true).expect("Failed to parse case9.m");
    let net_from_json: Network =
        serde_json::from_str(case9_json_str).expect("Failed to parse case9.json");

    assert_eq!(net_from_m.bus_count(), 9);
    assert_eq!(net_from_json.bus_count(), 9);
    assert_eq!(net_from_m.branch_count(), 9);
    assert_eq!(net_from_json.branch_count(), 9);

    // 2. Solve AC power flow
    let ac_res = ACPowerFlow::solve(&net_from_m).expect("AC power flow on case9 failed");
    assert!(ac_res.converged, "AC power flow must converge on case9");

    let mut max_vm_err = 0.0_f64;
    let mut sum_vm_err = 0.0_f64;
    let mut max_va_err = 0.0_f64;
    let mut sum_va_err = 0.0_f64;
    let n = ac_res.bus_results.len();

    for (&bus_id, bus_res) in &ac_res.bus_results {
        let oracle_bus = &oracle.ac.buses[&bus_id];

        let vm_err = (bus_res.vm_pu - oracle_bus.vm_pu).abs();
        let va_err = (bus_res.va_rad - oracle_bus.va_rad).abs();

        if vm_err > max_vm_err {
            max_vm_err = vm_err;
        }
        sum_vm_err += vm_err;

        if va_err > max_va_err {
            max_va_err = va_err;
        }
        sum_va_err += va_err;
    }

    let mae_vm = sum_vm_err / n as f64;
    let mae_va = sum_va_err / n as f64;

    println!("\n=== IEEE 9-Bus AC Power Flow Oracle Cross-Validation ===");
    println!("Converged in {} iterations", ac_res.iterations);
    println!(
        "Max Voltage Magnitude Error: {:.3e} p.u. (Target < 1e-6)",
        max_vm_err
    );
    println!("MAE Voltage Magnitude Error: {:.3e} p.u.", mae_vm);
    println!(
        "Max Voltage Angle Error:     {:.3e} rad  (Target < 1e-4)",
        max_va_err
    );
    println!("MAE Voltage Angle Error:     {:.3e} rad", mae_va);

    assert!(
        max_vm_err < 1e-6,
        "Case9 AC max Vm error {} exceeds 1e-6",
        max_vm_err
    );
    assert!(
        max_va_err < 1e-4,
        "Case9 AC max Va error {} exceeds 1e-4 rad",
        max_va_err
    );
    assert!(
        mae_vm < 1e-6,
        "Case9 AC MAE Vm error {} exceeds 1e-6",
        mae_vm
    );
    assert!(
        mae_va < 1e-4,
        "Case9 AC MAE Va error {} exceeds 1e-4 rad",
        mae_va
    );

    // 3. Solve DC power flow
    let dc_res = DCPowerFlow::solve(&net_from_m).expect("DC power flow on case9 failed");
    let mut max_dc_va_err = 0.0_f64;
    let mut sum_dc_va_err = 0.0_f64;

    for (&bus_id, bus_res) in &dc_res.bus_results {
        let oracle_bus = &oracle.dc.buses[&bus_id];
        let va_err = (bus_res.va_rad - oracle_bus.va_rad).abs();
        if va_err > max_dc_va_err {
            max_dc_va_err = va_err;
        }
        sum_dc_va_err += va_err;
    }

    let mae_dc_va = sum_dc_va_err / n as f64;
    println!("=== IEEE 9-Bus DC Power Flow Oracle Cross-Validation ===");
    println!(
        "Max DC Angle Error: {:.3e} rad (Target < 1e-4)",
        max_dc_va_err
    );
    println!("MAE DC Angle Error: {:.3e} rad", mae_dc_va);

    assert!(
        max_dc_va_err < 1e-4,
        "Case9 DC max Va error {} exceeds 1e-4 rad",
        max_dc_va_err
    );
    assert!(
        mae_dc_va < 1e-4,
        "Case9 DC MAE Va error {} exceeds 1e-4 rad",
        mae_dc_va
    );
}

#[test]
fn test_case14_ac_and_dc_oracle_validation() {
    let case14_m_str = include_str!("../../data/cases/case14.m");
    let case14_json_str = include_str!("../../data/cases/case14.json");
    let oracle_str = include_str!("../../data/cases/case14_oracle.json");

    let oracle: OracleCaseData = serde_json::from_str(oracle_str).unwrap();

    let net_from_m = MatpowerParser::parse(case14_m_str, true).expect("Failed to parse case14.m");
    let net_from_json: Network =
        serde_json::from_str(case14_json_str).expect("Failed to parse case14.json");

    assert_eq!(net_from_m.bus_count(), 14);
    assert_eq!(net_from_json.bus_count(), 14);
    assert_eq!(net_from_m.branch_count(), 20);
    assert_eq!(net_from_json.branch_count(), 20);

    // Solve AC power flow
    let ac_res = ACPowerFlow::solve(&net_from_m).expect("AC power flow on case14 failed");
    assert!(ac_res.converged, "AC power flow must converge on case14");

    let mut max_vm_err = 0.0_f64;
    let mut sum_vm_err = 0.0_f64;
    let mut max_va_err = 0.0_f64;
    let mut sum_va_err = 0.0_f64;
    let n = ac_res.bus_results.len();

    for (&bus_id, bus_res) in &ac_res.bus_results {
        let oracle_bus = &oracle.ac.buses[&bus_id];

        let vm_err = (bus_res.vm_pu - oracle_bus.vm_pu).abs();
        let va_err = (bus_res.va_rad - oracle_bus.va_rad).abs();

        if vm_err > max_vm_err {
            max_vm_err = vm_err;
        }
        sum_vm_err += vm_err;

        if va_err > max_va_err {
            max_va_err = va_err;
        }
        sum_va_err += va_err;
    }

    let mae_vm = sum_vm_err / n as f64;
    let mae_va = sum_va_err / n as f64;

    println!("\n=== IEEE 14-Bus AC Power Flow Oracle Cross-Validation ===");
    println!("Converged in {} iterations", ac_res.iterations);
    println!(
        "Max Voltage Magnitude Error: {:.3e} p.u. (Target < 1e-6)",
        max_vm_err
    );
    println!("MAE Voltage Magnitude Error: {:.3e} p.u.", mae_vm);
    println!(
        "Max Voltage Angle Error:     {:.3e} rad  (Target < 1e-4)",
        max_va_err
    );
    println!("MAE Voltage Angle Error:     {:.3e} rad", mae_va);

    assert!(
        max_vm_err < 1e-5,
        "Case14 AC max Vm error {} exceeds 1e-5",
        max_vm_err
    );
    assert!(
        max_va_err < 1e-4,
        "Case14 AC max Va error {} exceeds 1e-4 rad",
        max_va_err
    );
    assert!(
        mae_vm < 1e-5,
        "Case14 AC MAE Vm error {} exceeds 1e-5",
        mae_vm
    );
    assert!(
        mae_va < 1e-4,
        "Case14 AC MAE Va error {} exceeds 1e-4 rad",
        mae_va
    );

    // Solve DC power flow
    let dc_res = DCPowerFlow::solve(&net_from_m).expect("DC power flow on case14 failed");
    let mut max_dc_va_err = 0.0_f64;
    let mut sum_dc_va_err = 0.0_f64;

    for (&bus_id, bus_res) in &dc_res.bus_results {
        let oracle_bus = &oracle.dc.buses[&bus_id];
        let va_err = (bus_res.va_rad - oracle_bus.va_rad).abs();
        if va_err > max_dc_va_err {
            max_dc_va_err = va_err;
        }
        sum_dc_va_err += va_err;
    }

    let mae_dc_va = sum_dc_va_err / n as f64;
    println!("=== IEEE 14-Bus DC Power Flow Oracle Cross-Validation ===");
    println!(
        "Max DC Angle Error: {:.3e} rad (Target < 1e-4)",
        max_dc_va_err
    );
    println!("MAE DC Angle Error: {:.3e} rad", mae_dc_va);

    assert!(
        max_dc_va_err < 1e-4,
        "Case14 DC max Va error {} exceeds 1e-4 rad",
        max_dc_va_err
    );
    assert!(
        mae_dc_va < 1e-4,
        "Case14 DC MAE Va error {} exceeds 1e-4 rad",
        mae_dc_va
    );
}

//! Integration test cross-validating AC Newton-Raphson power flow results
//! against pandapower 3.5.4 oracle output and comparing AC vs DC ballpark sanity checks.

use arc_core::model::{Branch, Bus, BusType, Generator, Load, Network};
use arc_core::solver::{ACPowerFlow, DCPowerFlow};

fn build_canonical_3bus_network() -> Network {
    let mut net = Network::new(100.0);

    // Canonical 3-bus network matching scripts/oracle_check.py build_three_bus_case()
    net.add_bus(Bus::new(0, BusType::Slack, 138.0).with_name("Bus 0"))
        .unwrap();
    net.add_bus(Bus::new(1, BusType::PQ, 138.0).with_name("Bus 1"))
        .unwrap();
    net.add_bus(
        Bus::new(2, BusType::PV, 138.0)
            .with_name("Bus 2")
            .with_vm_pu(1.02),
    )
    .unwrap();

    // Lines
    net.add_branch(Branch::new_line(0, 0, 1, 0.02, 0.06).with_name("Line 0-1"))
        .unwrap();
    net.add_branch(Branch::new_line(1, 1, 2, 0.01, 0.03).with_name("Line 1-2"))
        .unwrap();
    net.add_branch(Branch::new_line(2, 0, 2, 0.012, 0.036).with_name("Line 0-2"))
        .unwrap();

    // Injections
    net.add_generator(Generator::new(0, 0, 0.0, 1.0)).unwrap(); // Slack generator
    net.add_load(Load::new(0, 1, 40.0, 20.0)).unwrap(); // 40 MW, 20 MVar load at Bus 1
    net.add_generator(Generator::new(1, 2, 50.0, 1.02)).unwrap(); // 50 MW gen at Bus 2

    net
}

#[test]
fn test_ac_power_flow_matches_pandapower_oracle_case3() {
    let net = build_canonical_3bus_network();
    let result = ACPowerFlow::solve(&net).expect("AC power flow must solve");

    assert!(result.converged);
    assert!(
        result.iterations <= 5,
        "Expected convergence in <= 5 iterations, took {}",
        result.iterations
    );
    assert!(result.final_mismatch_pu < 1e-8);

    // Reference values extracted from pandapower 3.5.4 (scripts/oracle_check.py --case case3 --mode ac)
    // Bus 0 (Slack): V = 1.0 pu, angle = 0.0 deg (0.0 rad)
    let b0 = &result.bus_results[&0];
    assert!((b0.vm_pu - 1.0).abs() < 1e-6);
    assert!((b0.va_deg - 0.0).abs() < 1e-6);
    assert!((b0.va_rad - 0.0).abs() < 1e-6);
    assert!((b0.p_gen_mw - (-9.40171614)).abs() < 1e-5);
    assert!((b0.q_gen_mvar - (-63.53310725)).abs() < 1e-5);

    // Bus 1 (PQ): V = 1.0066893215 pu, angle = -0.3823202861 deg (-0.0066727478 rad)
    let b1 = &result.bus_results[&1];
    assert!((b1.vm_pu - 1.0066893215).abs() < 1e-6);
    assert!((b1.va_deg - (-0.3823202861)).abs() < 1e-6);
    assert!((b1.va_rad - (-0.0066727478)).abs() < 1e-6);
    assert!((b1.p_load_mw - 40.0).abs() < 1e-6);
    assert!((b1.q_load_mvar - 20.0).abs() < 1e-6);

    // Bus 2 (PV): V = 1.02 pu, angle = -0.0117374675 deg (-0.0002048575 rad)
    let b2 = &result.bus_results[&2];
    assert!((b2.vm_pu - 1.02).abs() < 1e-6);
    assert!((b2.va_deg - (-0.0117374675)).abs() < 1e-6);
    assert!((b2.va_rad - (-0.0002048575)).abs() < 1e-6);
    assert!((b2.p_gen_mw - 50.0).abs() < 1e-6);
    assert!((b2.q_gen_mvar - 85.32795883).abs() < 1e-5);

    // Branch active power flows matching pandapower oracle:
    // Line 0-1: p_from = 6.742546 MW, p_to = -6.697761 MW, p_loss = 0.044785 MW
    let f0 = &result.branch_flows[&0];
    assert!((f0.p_from_mw - 6.742546).abs() < 1e-4);
    assert!((f0.p_to_mw - (-6.697761)).abs() < 1e-4);
    assert!((f0.p_loss_mw - 0.044785).abs() < 1e-4);

    // Line 1-2: p_from = -33.302239 MW, p_to = 33.522369 MW, p_loss = 0.220130 MW
    let f1 = &result.branch_flows[&1];
    assert!((f1.p_from_mw - (-33.302239)).abs() < 1e-4);
    assert!((f1.p_to_mw - 33.522369).abs() < 1e-4);
    assert!((f1.p_loss_mw - 0.220130).abs() < 1e-4);

    // Line 0-2: p_from = -16.144262 MW, p_to = 16.477631 MW, p_loss = 0.333369 MW
    let f2 = &result.branch_flows[&2];
    assert!((f2.p_from_mw - (-16.144262)).abs() < 1e-4);
    assert!((f2.p_to_mw - 16.477631).abs() < 1e-4);
    assert!((f2.p_loss_mw - 0.333369).abs() < 1e-4);

    // Total system losses: ~0.598284 MW
    let expected_total_loss = 0.044785 + 0.220130 + 0.333369;
    assert!((result.total_p_loss_mw - expected_total_loss).abs() < 1e-4);
}

#[test]
fn test_ac_vs_dc_ballpark_sanity_check() {
    let net = build_canonical_3bus_network();

    let ac_result = ACPowerFlow::solve(&net).expect("AC solve must succeed");
    let dc_result = DCPowerFlow::solve(&net).expect("DC solve must succeed");

    // 1. Bus 1 voltage angle:
    // DC is an approximation neglecting resistance and assuming V = 1.0 pu.
    // For this lightly-loaded network, angles should differ by less than 0.2 degrees.
    let ac_deg1 = ac_result.bus_results[&1].va_deg;
    let dc_deg1 = dc_result.bus_results[&1].va_deg;
    assert!(
        (ac_deg1 - dc_deg1).abs() < 0.2,
        "Bus 1 angle divergence too high: AC={ac_deg1}, DC={dc_deg1}"
    );

    // 2. Bus 2 voltage angle:
    // Angles should differ by less than 0.5 degrees.
    let ac_deg2 = ac_result.bus_results[&2].va_deg;
    let dc_deg2 = dc_result.bus_results[&2].va_deg;
    assert!(
        (ac_deg2 - dc_deg2).abs() < 0.5,
        "Bus 2 angle divergence too high: AC={ac_deg2}, DC={dc_deg2}"
    );

    // 3. Slack active generation:
    // DC: -10.0 MW, AC: -9.40 MW (differs only due to ~0.6 MW line losses in AC!)
    let ac_slack_p = ac_result.bus_results[&0].p_gen_mw;
    let dc_slack_p = dc_result.bus_results[&0].p_gen_mw;
    assert!(
        (ac_slack_p - dc_slack_p).abs() < 1.0,
        "Slack generation difference too high: AC={ac_slack_p}, DC={dc_slack_p}"
    );

    // 4. Branch 0 (Line 0-1) active power flow:
    // DC: 6.67 MW, AC: 6.74 MW (matches to within 0.15 MW)
    let ac_flow0 = ac_result.branch_flows[&0].p_from_mw;
    let dc_flow0 = dc_result.branch_flows[&0].p_from_mw;
    assert!(
        (ac_flow0 - dc_flow0).abs() < 0.2,
        "Line 0-1 flow divergence: AC={ac_flow0}, DC={dc_flow0}"
    );

    // 5. Branch 1 (Line 1-2) active power flow:
    // DC: -33.33 MW, AC: -33.30 MW (matches to within 0.1 MW)
    let ac_flow1 = ac_result.branch_flows[&1].p_from_mw;
    let dc_flow1 = dc_result.branch_flows[&1].p_from_mw;
    assert!(
        (ac_flow1 - dc_flow1).abs() < 0.2,
        "Line 1-2 flow divergence: AC={ac_flow1}, DC={dc_flow1}"
    );

    // 6. Branch 2 (Line 0-2) active power flow:
    // DC: -16.67 MW, AC: -16.14 MW (matches to within 0.6 MW)
    let ac_flow2 = ac_result.branch_flows[&2].p_from_mw;
    let dc_flow2 = dc_result.branch_flows[&2].p_from_mw;
    assert!(
        (ac_flow2 - dc_flow2).abs() < 0.8,
        "Line 0-2 flow divergence: AC={ac_flow2}, DC={dc_flow2}"
    );
}

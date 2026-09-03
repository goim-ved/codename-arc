//! Integration test cross-validating DC power flow results against pandapower 3.5.4 oracle output.

use arc_core::model::{Branch, Bus, BusType, Generator, Load, Network};
use arc_core::solver::DCPowerFlow;

#[test]
fn test_dc_power_flow_matches_pandapower_oracle_case3() {
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
    net.add_load(Load::new(0, 1, 40.0, 20.0)).unwrap(); // 40 MW load at Bus 1
    net.add_generator(Generator::new(1, 2, 50.0, 1.02)).unwrap(); // 50 MW gen at Bus 2

    let result = DCPowerFlow::solve(&net).expect("DC power flow must solve");

    // --- Bus Cross-Validation against pandapower oracle ---
    // Bus 0 (Slack): V = 1.0 pu, angle = 0.0 deg (0.0 rad), slack gen absorbs 10 MW (-10 MW)
    let b0 = &result.bus_results[&0];
    assert!((b0.vm_pu - 1.0).abs() < 1e-6);
    assert!((b0.va_deg - 0.0).abs() < 1e-6);
    assert!((b0.va_rad - 0.0).abs() < 1e-6);
    assert!((b0.p_gen_mw - (-10.0)).abs() < 1e-6);

    // Bus 1 (PQ): V = 1.0 pu, angle = -0.229183 deg (-0.004 rad), load = 40 MW
    let b1 = &result.bus_results[&1];
    assert!((b1.vm_pu - 1.0).abs() < 1e-6);
    assert!((b1.va_deg - (-0.2291831181)).abs() < 1e-6);
    assert!((b1.va_rad - (-0.004)).abs() < 1e-6);
    assert!((b1.p_load_mw - 40.0).abs() < 1e-6);

    // Bus 2 (PV): V = 1.02 pu, angle = +0.343775 deg (+0.006 rad), gen = 50 MW
    let b2 = &result.bus_results[&2];
    assert!((b2.vm_pu - 1.02).abs() < 1e-6);
    assert!((b2.va_deg - 0.3437746771).abs() < 1e-6);
    assert!((b2.va_rad - 0.006).abs() < 1e-6);
    assert!((b2.p_gen_mw - 50.0).abs() < 1e-6);

    // --- Branch Flow Cross-Validation against pandapower oracle ---
    // Line 0-1 (Branch 0): p_from = 6.666667 MW, p_to = -6.666667 MW
    let f0 = &result.branch_flows[&0];
    assert!((f0.p_from_mw - 6.66666667).abs() < 1e-6);
    assert!((f0.p_to_mw - (-6.66666667)).abs() < 1e-6);

    // Line 1-2 (Branch 1): p_from = -33.333333 MW, p_to = 33.333333 MW
    let f1 = &result.branch_flows[&1];
    assert!((f1.p_from_mw - (-33.33333333)).abs() < 1e-6);
    assert!((f1.p_to_mw - 33.33333333).abs() < 1e-6);

    // Line 0-2 (Branch 2): p_from = -16.666667 MW, p_to = 16.666667 MW
    let f2 = &result.branch_flows[&2];
    assert!((f2.p_from_mw - (-16.66666667)).abs() < 1e-6);
    assert!((f2.p_to_mw - 16.66666667).abs() < 1e-6);
}

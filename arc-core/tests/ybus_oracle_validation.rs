//! Integration test validating Y-bus construction directly against pandapower oracle reference values.

use arc_core::model::{Branch, Bus, BusType, Network};
use arc_core::YBus;

#[test]
fn test_ybus_matches_pandapower_oracle_case3() {
    let mut net = Network::new(100.0);

    // Buses
    net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
    net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();
    net.add_bus(Bus::new(2, BusType::PV, 138.0)).unwrap();

    // Lines matching scripts/oracle_check.py build_three_bus_case()
    net.add_branch(Branch::new_line(0, 0, 1, 0.02, 0.06))
        .unwrap();
    net.add_branch(Branch::new_line(1, 1, 2, 0.01, 0.03))
        .unwrap();
    net.add_branch(Branch::new_line(2, 0, 2, 0.012, 0.036))
        .unwrap();

    let ybus = YBus::build(&net).expect("Ybus build should succeed");

    // Reference values extracted from pandapower 3.5.4 (scripts/oracle_check.py --case case3 --dump-ybus)
    // Row 0
    assert!((ybus.g_entry(0, 0) - 13.3333333333).abs() < 1e-9);
    assert!((ybus.b_entry(0, 0) - (-40.0)).abs() < 1e-9);

    assert!((ybus.g_entry(0, 1) - (-5.0)).abs() < 1e-9);
    assert!((ybus.b_entry(0, 1) - 15.0).abs() < 1e-9);

    assert!((ybus.g_entry(0, 2) - (-8.3333333333)).abs() < 1e-9);
    assert!((ybus.b_entry(0, 2) - 25.0).abs() < 1e-9);

    // Row 1
    assert!((ybus.g_entry(1, 0) - (-5.0)).abs() < 1e-9);
    assert!((ybus.b_entry(1, 0) - 15.0).abs() < 1e-9);

    assert!((ybus.g_entry(1, 1) - 15.0).abs() < 1e-9);
    assert!((ybus.b_entry(1, 1) - (-45.0)).abs() < 1e-9);

    assert!((ybus.g_entry(1, 2) - (-10.0)).abs() < 1e-9);
    assert!((ybus.b_entry(1, 2) - 30.0).abs() < 1e-9);

    // Row 2
    assert!((ybus.g_entry(2, 0) - (-8.3333333333)).abs() < 1e-9);
    assert!((ybus.b_entry(2, 0) - 25.0).abs() < 1e-9);

    assert!((ybus.g_entry(2, 1) - (-10.0)).abs() < 1e-9);
    assert!((ybus.b_entry(2, 1) - 30.0).abs() < 1e-9);

    assert!((ybus.g_entry(2, 2) - 18.3333333333).abs() < 1e-9);
    assert!((ybus.b_entry(2, 2) - (-55.0)).abs() < 1e-9);
}

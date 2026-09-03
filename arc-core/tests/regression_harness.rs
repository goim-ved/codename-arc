//! Integration test verifying that the automated regression harness passes all checks.

use arc_core::regression::RegressionHarness;

#[test]
fn test_automated_numerical_regression_harness() {
    let report = RegressionHarness::run_all().expect("Regression harness must execute cleanly");

    // Print summary table in test logs
    report.print_table();

    assert_eq!(
        report.results.len(),
        12,
        "Expected 12 benchmark evaluations (6 cases * 2 modes)"
    );
    assert!(
        report.all_passed(),
        "All cases must pass required tolerance thresholds"
    );
    assert!(report.assert_all_passed().is_ok());
}

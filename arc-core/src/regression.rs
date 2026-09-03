//! Automated numerical regression harness for `arc`.
//!
//! Executes AC and DC power flow across all benchmark networks (3-bus, 9-bus, 14-bus),
//! compares against stored pandapower 3.5.4 oracle outputs, checks tolerances,
//! and generates structured reports and formatted terminal tables.

use crate::parser::MatpowerParser;
use crate::solver::{ACPowerFlow, DCPowerFlow};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Raw bus result deserialized from stored oracle JSON.
#[derive(Debug, Deserialize)]
struct OracleBusResult {
    vm_pu: f64,
    va_rad: f64,
}

/// Mode section in oracle JSON.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OracleModeData {
    converged: bool,
    buses: BTreeMap<usize, OracleBusResult>,
}

/// Root structure of oracle JSON.
#[derive(Debug, Deserialize)]
struct OracleCaseData {
    ac: OracleModeData,
    dc: OracleModeData,
}

/// Result of running regression verification on an individual case and formulation mode.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseRegressionResult {
    /// Benchmark network identifier (e.g. "case3", "case9", "case14").
    pub case_name: String,
    /// Formulation mode ("AC" or "DC").
    pub mode: String,
    /// Number of electrical buses in network.
    pub buses: usize,
    /// Number of transmission branches in network.
    pub branches: usize,
    /// Matrix sparsity percentage ($1 - \text{nnz}/N^2$).
    pub sparsity_pct: f64,
    /// Number of iterations taken to solve (1 for DC).
    pub iterations: usize,
    /// Maximum voltage magnitude error in per-unit (None for DC).
    pub max_vm_err: Option<f64>,
    /// Mean absolute voltage magnitude error in per-unit (None for DC).
    pub mae_vm_err: Option<f64>,
    /// Maximum voltage phase angle error in radians.
    pub max_va_err: f64,
    /// Mean absolute voltage phase angle error in radians.
    pub mae_va_err: f64,
    /// Whether all metrics satisfied required tolerance thresholds.
    pub passed: bool,
    /// Explanation if the run failed.
    pub failure_reason: Option<String>,
}

/// Aggregated regression test report spanning multiple cases and modes.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionReport {
    /// Individual case results.
    pub results: Vec<CaseRegressionResult>,
}

impl RegressionReport {
    /// Returns true if all test cases passed their tolerance thresholds.
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.passed)
    }

    /// Asserts that all test cases passed, returning an error message listing failures if not.
    pub fn assert_all_passed(&self) -> Result<(), String> {
        let failures: Vec<&CaseRegressionResult> =
            self.results.iter().filter(|r| !r.passed).collect();
        if failures.is_empty() {
            Ok(())
        } else {
            let mut msg = format!("{} regression test(s) failed:\n", failures.len());
            for f in failures {
                msg.push_str(&format!(
                    "  - {} [{}]: {}\n",
                    f.case_name,
                    f.mode,
                    f.failure_reason.as_deref().unwrap_or("Tolerance exceeded")
                ));
            }
            Err(msg)
        }
    }

    /// Prints a clean, aligned summary table to standard output.
    pub fn print_table(&self) {
        println!("{}", self.format_table());
    }

    /// Formats the regression results as a clean ASCII table string.
    /// Formats the regression results as a clean ASCII table string.
    pub fn format_table(&self) -> String {
        let mut out = String::new();
        out.push_str("===================================================================================================================================\n");
        out.push_str(
            "Case       Mode   Buses  Branches  Sparsity   Iter  Max |Vm| Err (pu)  MAE |Vm| (pu)  Max |Va| Err (rad)  MAE |Va| (rad)  Status\n",
        );
        out.push_str("-----------------------------------------------------------------------------------------------------------------------------------\n");

        for r in &self.results {
            let vm_max_str = match r.max_vm_err {
                Some(val) => format!("{val:.2e}"),
                None => "    -   ".to_string(),
            };
            let vm_mae_str = match r.mae_vm_err {
                Some(val) => format!("{val:.2e}"),
                None => "    -   ".to_string(),
            };
            let va_max_str = format!("{:.2e}", r.max_va_err);
            let va_mae_str = format!("{:.2e}", r.mae_va_err);
            let status_str = if r.passed { "PASS" } else { "FAIL" };
            let sparsity_str = format!("{:>6.1}%", r.sparsity_pct);

            out.push_str(&format!(
                "{:<10} {:<6} {:>5} {:>9} {:>9} {:>6}  {:>17}  {:>13}  {:>18}  {:>14}  {:<6}\n",
                r.case_name,
                r.mode,
                r.buses,
                r.branches,
                sparsity_str,
                r.iterations,
                vm_max_str,
                vm_mae_str,
                va_max_str,
                va_mae_str,
                status_str,
            ));
        }

        out.push_str("===================================================================================================================================\n");
        let total = self.results.len();
        let passed_count = self.results.iter().filter(|r| r.passed).count();
        let failed_count = total - passed_count;

        if failed_count == 0 {
            out.push_str(&format!(
                "OVERALL STATUS: ALL {total} BENCHMARKS PASSED (0 failures)\n"
            ));
        } else {
            out.push_str(&format!(
                "OVERALL STATUS: {failed_count} OF {total} BENCHMARKS FAILED\n"
            ));
        }

        out
    }
}

/// Automated regression harness runner.
pub struct RegressionHarness;

impl RegressionHarness {
    /// Runs regression testing across all bundled benchmark cases (`case3`, `case9`, `case14`, `case30`, `case57`, `case118`).
    pub fn run_all() -> Result<RegressionReport, String> {
        let cases = [
            (
                "case3",
                include_str!("../../data/cases/case3.m"),
                include_str!("../../data/cases/case3_oracle.json"),
            ),
            (
                "case9",
                include_str!("../../data/cases/case9.m"),
                include_str!("../../data/cases/case9_oracle.json"),
            ),
            (
                "case14",
                include_str!("../../data/cases/case14.m"),
                include_str!("../../data/cases/case14_oracle.json"),
            ),
            (
                "case30",
                include_str!("../../data/cases/case30.m"),
                include_str!("../../data/cases/case30_oracle.json"),
            ),
            (
                "case57",
                include_str!("../../data/cases/case57.m"),
                include_str!("../../data/cases/case57_oracle.json"),
            ),
            (
                "case118",
                include_str!("../../data/cases/case118.m"),
                include_str!("../../data/cases/case118_oracle.json"),
            ),
        ];

        let mut results = Vec::new();
        for (case_name, m_content, oracle_json) in cases {
            let case_results = Self::run_case(case_name, m_content, oracle_json)?;
            results.extend(case_results);
        }

        Ok(RegressionReport { results })
    }

    /// Evaluates both AC and DC solvers for a given case against oracle reference output.
    pub fn run_case(
        case_name: &str,
        m_content: &str,
        oracle_json: &str,
    ) -> Result<Vec<CaseRegressionResult>, String> {
        let network = MatpowerParser::parse(m_content, true)
            .map_err(|e| format!("Failed to parse {case_name}: {e}"))?;

        let oracle: OracleCaseData = serde_json::from_str(oracle_json)
            .map_err(|e| format!("Failed to deserialize oracle JSON for {case_name}: {e}"))?;

        let n = network.bus_count();
        let m = network.branch_count();
        let nnz = n + 2 * m;
        let sparsity_pct = (1.0 - (nnz as f64 / (n * n) as f64)) * 100.0;

        let mut out = Vec::new();

        // 1. AC Evaluation
        let ac_res = ACPowerFlow::solve(&network)
            .map_err(|e| format!("AC solve failed on {case_name}: {e}"))?;

        let mut max_ac_vm_err = 0.0_f64;
        let mut sum_ac_vm_err = 0.0_f64;
        let mut max_ac_va_err = 0.0_f64;
        let mut sum_ac_va_err = 0.0_f64;

        for (&b_id, b_res) in &ac_res.bus_results {
            if let Some(oracle_bus) = oracle.ac.buses.get(&b_id) {
                let vm_err = (b_res.vm_pu - oracle_bus.vm_pu).abs();
                let va_err = (b_res.va_rad - oracle_bus.va_rad).abs();

                if vm_err > max_ac_vm_err {
                    max_ac_vm_err = vm_err;
                }
                sum_ac_vm_err += vm_err;

                if va_err > max_ac_va_err {
                    max_ac_va_err = va_err;
                }
                sum_ac_va_err += va_err;
            }
        }

        let mae_ac_vm = sum_ac_vm_err / n as f64;
        let mae_ac_va = sum_ac_va_err / n as f64;

        // Tolerances: Vm < 1e-5 pu, Va < 1e-4 rad
        let ac_passed = ac_res.converged && max_ac_vm_err < 1e-5 && max_ac_va_err < 1e-4;
        let ac_failure = if !ac_res.converged {
            Some("Solver did not converge".into())
        } else if max_ac_vm_err >= 1e-5 {
            Some(format!("Max Vm error {max_ac_vm_err:.2e} exceeded 1e-5"))
        } else if max_ac_va_err >= 1e-4 {
            Some(format!("Max Va error {max_ac_va_err:.2e} exceeded 1e-4"))
        } else {
            None
        };

        out.push(CaseRegressionResult {
            case_name: case_name.to_string(),
            mode: "AC".to_string(),
            buses: n,
            branches: m,
            sparsity_pct,
            iterations: ac_res.iterations,
            max_vm_err: Some(max_ac_vm_err),
            mae_vm_err: Some(mae_ac_vm),
            max_va_err: max_ac_va_err,
            mae_va_err: mae_ac_va,
            passed: ac_passed,
            failure_reason: ac_failure,
        });

        // 2. DC Evaluation
        let dc_res = DCPowerFlow::solve(&network)
            .map_err(|e| format!("DC solve failed on {case_name}: {e}"))?;

        let mut max_dc_va_err = 0.0_f64;
        let mut sum_dc_va_err = 0.0_f64;

        for (&b_id, b_res) in &dc_res.bus_results {
            if let Some(oracle_bus) = oracle.dc.buses.get(&b_id) {
                let va_err = (b_res.va_rad - oracle_bus.va_rad).abs();
                if va_err > max_dc_va_err {
                    max_dc_va_err = va_err;
                }
                sum_dc_va_err += va_err;
            }
        }

        let mae_dc_va = sum_dc_va_err / n as f64;
        let dc_passed = max_dc_va_err < 1e-4;
        let dc_failure = if !dc_passed {
            Some(format!("Max DC Va error {max_dc_va_err:.2e} exceeded 1e-4"))
        } else {
            None
        };

        out.push(CaseRegressionResult {
            case_name: case_name.to_string(),
            mode: "DC".to_string(),
            buses: n,
            branches: m,
            sparsity_pct,
            iterations: 1,
            max_vm_err: None,
            mae_vm_err: None,
            max_va_err: max_dc_va_err,
            mae_va_err: mae_dc_va,
            passed: dc_passed,
            failure_reason: dc_failure,
        });

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regression_harness_execution() {
        let report = RegressionHarness::run_all().expect("Regression harness must execute cleanly");
        assert_eq!(report.results.len(), 12); // 6 cases * 2 modes
        assert!(
            report.all_passed(),
            "All cases must pass regression thresholds"
        );
        assert!(report.assert_all_passed().is_ok());
    }
}

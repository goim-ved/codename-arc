//! Linear DC Power Flow Solver ($B\theta = P$).
//!
//! # Mathematical Derivation
//!
//! The DC power flow formulation is a non-iterative, linear approximation of AC power flow
//! built upon three foundational engineering assumptions:
//! 1. **Uniform Voltage Profile**: Voltage magnitudes are assumed to be near nominal:
//!    $$V_i \approx 1.0\text{ p.u.} \quad \forall i$$
//! 2. **Small Angle Differences**: Phase angle differences across connected branches are small:
//!    $$\sin(\theta_i - \theta_k) \approx \theta_i - \theta_k, \quad \cos(\theta_i - \theta_k) \approx 1$$
//! 3. **Lossless Transmission**: Series resistance and line charging susceptance are neglected:
//!    $$r_{ik} \ll x_{ik} \implies r_{ik} \approx 0, \quad b_{\text{shunt}} \approx 0$$
//!
//! ## Branch Flow Equation
//! For a branch connecting bus $i$ and bus $k$ with reactance $x_{ik}$, off-nominal tap ratio $a_{ik}$,
//! and phase shift $\phi_{ik}$:
//!
//! $$P_{ik} = \frac{\theta_i - \theta_k - \phi_{ik}}{a_{ik} x_{ik}} \quad [\text{p.u.}]$$
//!
//! ## Nodal Power Balance
//! Equating net power injection at bus $i$ to outgoing branch flows yields:
//!
//! $$P_i = \sum_{k \in \mathcal{N}(i)} \frac{\theta_i - \theta_k}{a_{ik} x_{ik}} - \sum_{k \in \mathcal{N}(i)} \frac{\phi_{ik}}{a_{ik} x_{ik}}$$
//!
//! In matrix notation:
//!
//! $$B_{\text{bus}} \boldsymbol{\theta} = \mathbf{P}_{\text{inj}} + \mathbf{P}_{\text{shift}}$$
//!
//! where the elements of the singular $N \times N$ DC susceptance matrix $B_{\text{bus}}$ are:
//! - Off-diagonal ($i \ne k$): $B_{ik} = -\sum_{m \in (i, k)} \frac{1}{a_m x_m}$
//! - Diagonal ($i = k$): $B_{ii} = \sum_{k \ne i} (-B_{ik})$
//!
//! ## Solution via Slack Reference Partition
//! Because $\sum_k B_{ik} = 0$, $B_{\text{bus}}$ has rank $N - 1$. Specifying the reference angle
//! at the Slack bus $\theta_{\text{slack}} = \theta_{\text{ref}}$ (typically $0.0$) eliminates its column
//! and row, leaving the non-slack reduced system:
//!
//! $$B_{\mathcal{NS}, \mathcal{NS}} \boldsymbol{\theta}_{\mathcal{NS}} = \mathbf{P}_{\text{eff}, \mathcal{NS}} - B_{\mathcal{NS}, \text{slack}} \theta_{\text{slack}}$$
//!
//! The reduced matrix $B_{\mathcal{NS}, \mathcal{NS}}$ is symmetric positive-definite for connected
//! grids and is solved directly in a single pass via Gaussian elimination with partial pivoting.

use crate::linear::{solve_dense_system, LinearSolverError};
use crate::model::{BusType, ModelError, Network};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Errors that can occur during power flow solution.
#[derive(Debug, Clone, PartialEq)]
pub enum SolverError {
    /// Network model validation failed.
    Model(ModelError),
    /// Dense linear system solver encountered singularity or dimension mismatch.
    Linear(LinearSolverError),
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(err) => write!(f, "Model error: {err}"),
            Self::Linear(err) => write!(f, "Linear solve error: {err}"),
        }
    }
}

impl std::error::Error for SolverError {}

impl From<ModelError> for SolverError {
    fn from(err: ModelError) -> Self {
        Self::Model(err)
    }
}

impl From<LinearSolverError> for SolverError {
    fn from(err: LinearSolverError) -> Self {
        Self::Linear(err)
    }
}

/// Solved steady-state state for an individual bus under DC power flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DCBusResult {
    /// Bus identifier.
    pub bus_id: usize,
    /// Voltage phase angle in radians.
    pub va_rad: f64,
    /// Voltage phase angle in degrees.
    pub va_deg: f64,
    /// Voltage magnitude in per-unit (1.0 for PQ, target for PV/Slack).
    pub vm_pu: f64,
    /// Net scheduled/solved active power injection in per-unit ($P_{\text{gen}} - P_{\text{load}}$).
    pub p_inj_pu: f64,
    /// Total active power generation at this bus in MW.
    pub p_gen_mw: f64,
    /// Total active power load at this bus in MW.
    pub p_load_mw: f64,
}

/// Solved active power transmission on an individual branch under DC power flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DCBranchFlow {
    /// Branch identifier.
    pub branch_id: usize,
    /// Sending / from-bus ID.
    pub from_bus: usize,
    /// Receiving / to-bus ID.
    pub to_bus: usize,
    /// Active power flow leaving from-bus in MW.
    pub p_from_mw: f64,
    /// Active power flow leaving to-bus in MW ($-P_{\text{from}}$ for lossless DC).
    pub p_to_mw: f64,
}

/// Complete output result of a DC power flow calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DCPowerFlowResult {
    /// Solved bus states, keyed by bus ID in ascending order.
    pub bus_results: BTreeMap<usize, DCBusResult>,
    /// Solved branch flows, keyed by branch ID in ascending order.
    pub branch_flows: BTreeMap<usize, DCBranchFlow>,
    /// Total network active power transmission losses in MW (identically 0.0 in DC approximation).
    pub p_loss_mw: f64,
}

/// Deterministic linear DC power flow solver.
pub struct DCPowerFlow;

impl DCPowerFlow {
    /// Solves linear DC power flow ($B\theta = P$) on the given network model.
    ///
    /// # Returns
    /// * `Ok(DCPowerFlowResult)` on successful solve.
    /// * `Err(SolverError)` if validation fails or $B$ is singular (e.g. disconnected network).
    pub fn solve(network: &Network) -> Result<DCPowerFlowResult, SolverError> {
        network.validate()?;

        let n = network.buses.len();
        let mut bus_ids = Vec::with_capacity(n);
        let mut bus_to_idx = BTreeMap::new();
        let mut slack_idx = None;
        let mut slack_id = 0;

        for (idx, (&id, bus)) in network.buses.iter().enumerate() {
            bus_ids.push(id);
            bus_to_idx.insert(id, idx);
            if bus.bus_type == BusType::Slack {
                slack_idx = Some(idx);
                slack_id = id;
            }
        }

        let s_idx = slack_idx.expect("Network validation guarantees exactly 1 slack bus");
        let slack_bus = &network.buses[&slack_id];
        let theta_slack = slack_bus.va_rad;

        // Step 1: Assemble full N x N B_bus matrix and phase shift vector
        let mut b_bus = vec![0.0; n * n];
        let mut p_shift = vec![0.0; n];

        for branch in network.branches.values() {
            if !branch.status {
                continue;
            }

            let &i = bus_to_idx.get(&branch.from_bus).unwrap();
            let &k = bus_to_idx.get(&branch.to_bus).unwrap();

            let x_eff = branch.tap_ratio * branch.x_pu;
            let b_series = 1.0 / x_eff;

            // Off-diagonal entries
            b_bus[i * n + k] -= b_series;
            b_bus[k * n + i] -= b_series;

            // Diagonal entries
            b_bus[i * n + i] += b_series;
            b_bus[k * n + k] += b_series;

            // Phase shifter contribution: P_ik = (theta_i - theta_k - phi) / x_eff
            if branch.shift_rad.abs() > 1e-12 {
                let shift_mw_pu = branch.shift_rad / x_eff;
                p_shift[i] -= shift_mw_pu;
                p_shift[k] += shift_mw_pu;
            }
        }

        // Step 2: Compute net scheduled power injection for all buses (in p.u.)
        let mut p_inj = vec![0.0; n];
        let mut p_load_mw_per_bus = vec![0.0; n];
        let mut p_gen_mw_per_bus = vec![0.0; n];

        for gen in network.generators.values() {
            if gen.status {
                let &idx = bus_to_idx.get(&gen.bus).unwrap();
                p_gen_mw_per_bus[idx] += gen.p_mw;
            }
        }

        for load in network.loads.values() {
            if load.status {
                let &idx = bus_to_idx.get(&load.bus).unwrap();
                p_load_mw_per_bus[idx] += load.p_mw;
            }
        }

        for i in 0..n {
            let net_mw = p_gen_mw_per_bus[i] - p_load_mw_per_bus[i];
            p_inj[i] = net_mw / network.base_mva + p_shift[i];
        }

        // Step 3: Form the reduced (N - 1) x (N - 1) system omitting the Slack bus
        let non_slack_indices: Vec<usize> = (0..n).filter(|&idx| idx != s_idx).collect();
        let red_dim = n - 1;

        let mut b_red = vec![0.0; red_dim * red_dim];
        let mut p_red = vec![0.0; red_dim];

        for (r_idx, &r) in non_slack_indices.iter().enumerate() {
            // RHS: P_red[r] = P_inj[r] - B[r, slack] * theta_slack
            p_red[r_idx] = p_inj[r] - b_bus[r * n + s_idx] * theta_slack;

            for (c_idx, &c) in non_slack_indices.iter().enumerate() {
                b_red[r_idx * red_dim + c_idx] = b_bus[r * n + c];
            }
        }

        // Step 4: Solve B_red * theta_red = P_red via deterministic Gaussian elimination
        let theta_red = solve_dense_system(&b_red, &p_red, red_dim)?;

        // Step 5: Assemble full theta vector of length N
        let mut theta = vec![0.0; n];
        theta[s_idx] = theta_slack;
        for (r_idx, &r) in non_slack_indices.iter().enumerate() {
            theta[r] = theta_red[r_idx];
        }

        // Step 6: Compute Slack bus net injection and generator power
        let mut p_slack_inj_pu = 0.0;
        for c in 0..n {
            p_slack_inj_pu += b_bus[s_idx * n + c] * theta[c];
        }
        p_slack_inj_pu -= p_shift[s_idx];

        // Generation at slack bus balances load plus net injection:
        // P_slack_inj = P_slack_gen - P_slack_load
        // P_slack_gen = P_slack_inj + P_slack_load
        let slack_gen_mw = p_slack_inj_pu * network.base_mva + p_load_mw_per_bus[s_idx];
        p_gen_mw_per_bus[s_idx] = slack_gen_mw;

        // Step 7: Build bus results
        let mut bus_results = BTreeMap::new();
        for (idx, &id) in bus_ids.iter().enumerate() {
            let bus = &network.buses[&id];
            let va_rad = theta[idx];
            let va_deg = va_rad.to_degrees();

            let vm_pu = match bus.bus_type {
                BusType::Slack | BusType::PV => bus.vm_pu,
                BusType::PQ => 1.0,
            };

            let p_inj_pu = if idx == s_idx {
                p_slack_inj_pu
            } else {
                p_inj[idx] - p_shift[idx]
            };

            bus_results.insert(
                id,
                DCBusResult {
                    bus_id: id,
                    va_rad,
                    va_deg,
                    vm_pu,
                    p_inj_pu,
                    p_gen_mw: p_gen_mw_per_bus[idx],
                    p_load_mw: p_load_mw_per_bus[idx],
                },
            );
        }

        // Step 8: Compute branch power flows
        let mut branch_flows = BTreeMap::new();
        for branch in network.branches.values() {
            if !branch.status {
                continue;
            }

            let &i = bus_to_idx.get(&branch.from_bus).unwrap();
            let &k = bus_to_idx.get(&branch.to_bus).unwrap();

            let x_eff = branch.tap_ratio * branch.x_pu;
            // P_from = (theta_i - theta_k - phi) / x_eff (in p.u.) * base_mva (in MW)
            let p_flow_pu = (theta[i] - theta[k] - branch.shift_rad) / x_eff;
            let p_from_mw = p_flow_pu * network.base_mva;
            let p_to_mw = -p_from_mw;

            branch_flows.insert(
                branch.id,
                DCBranchFlow {
                    branch_id: branch.id,
                    from_bus: branch.from_bus,
                    to_bus: branch.to_bus,
                    p_from_mw,
                    p_to_mw,
                },
            );
        }

        Ok(DCPowerFlowResult {
            bus_results,
            branch_flows,
            p_loss_mw: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Branch, Bus, Generator, Load};

    /// Hand-calculated 3-bus network verification:
    ///
    /// Canonical 3-bus network (matches pandapower case3):
    /// - Bus 0: Slack, V = 1.0, theta = 0.0 rad
    /// - Bus 1: Load P = 40 MW -> -0.40 p.u.
    /// - Bus 2: Gen P = 50 MW -> +0.50 p.u.
    ///
    /// Line reactances:
    /// - Line 0-1: x = 0.06 => 1/x = 50/3
    /// - Line 1-2: x = 0.03 => 1/x = 100/3
    /// - Line 0-2: x = 0.036 => 1/x = 250/9
    ///
    /// Reduced system for non-slack buses [1, 2]:
    ///   B_11 = 50/3 + 100/3 = 150/3 = 50.0
    ///   B_12 = B_21 = -100/3
    ///   B_22 = 100/3 + 250/9 = 550/9
    ///
    /// Injections:
    ///   P_1 = -0.40 p.u.
    ///   P_2 = +0.50 p.u.
    ///
    /// Analytical solution:
    ///   det(B_red) = 50 * (550/9) - (-100/3)^2 = 17500/9
    ///   theta_1 = -0.004 rad (-0.2291831181 deg)
    ///   theta_2 = +0.006 rad (+0.3437746771 deg)
    ///   P_slack = 10.0 MW
    ///   P_line_01 = (0 - (-0.004)) / 0.06 * 100 = 6.66666667 MW
    ///   P_line_12 = (-0.004 - 0.006) / 0.03 * 100 = -33.33333333 MW
    ///   P_line_02 = (0 - 0.006) / 0.036 * 100 = -16.66666667 MW
    #[test]
    fn test_canonical_3bus_dc_power_flow_hand_calculated() {
        let mut net = Network::new(100.0);

        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();
        net.add_bus(Bus::new(2, BusType::PV, 138.0).with_vm_pu(1.02))
            .unwrap();

        net.add_branch(Branch::new_line(0, 0, 1, 0.02, 0.06))
            .unwrap();
        net.add_branch(Branch::new_line(1, 1, 2, 0.01, 0.03))
            .unwrap();
        net.add_branch(Branch::new_line(2, 0, 2, 0.012, 0.036))
            .unwrap();

        // Injections
        net.add_generator(Generator::new(0, 0, 0.0, 1.0)).unwrap();
        net.add_load(Load::new(0, 1, 40.0, 20.0)).unwrap();
        net.add_generator(Generator::new(1, 2, 50.0, 1.02)).unwrap();

        let result = DCPowerFlow::solve(&net).expect("DC power flow should solve");

        // Bus 0 (Slack)
        let b0 = &result.bus_results[&0];
        assert_eq!(b0.va_rad, 0.0);
        assert_eq!(b0.va_deg, 0.0);
        assert!(
            (b0.p_gen_mw - (-10.0)).abs() < 1e-12,
            "Slack gen expected -10 MW, got {}",
            b0.p_gen_mw
        );
        assert!((b0.p_inj_pu - (-0.10)).abs() < 1e-12);

        // Bus 1 (Load)
        let b1 = &result.bus_results[&1];
        assert!(
            (b1.va_rad - (-0.004)).abs() < 1e-12,
            "theta_1 expected -0.004 rad, got {}",
            b1.va_rad
        );
        let expected_deg1 = (-0.004_f64).to_degrees();
        assert!((b1.va_deg - expected_deg1).abs() < 1e-12);
        assert!((b1.p_load_mw - 40.0).abs() < 1e-12);
        assert!((b1.p_inj_pu - (-0.40)).abs() < 1e-12);

        // Bus 2 (PV Gen)
        let b2 = &result.bus_results[&2];
        assert!(
            (b2.va_rad - 0.006).abs() < 1e-12,
            "theta_2 expected 0.006 rad, got {}",
            b2.va_rad
        );
        let expected_deg2 = (0.006_f64).to_degrees();
        assert!((b2.va_deg - expected_deg2).abs() < 1e-12);
        assert!((b2.p_gen_mw - 50.0).abs() < 1e-12);
        assert!((b2.p_inj_pu - 0.50).abs() < 1e-12);

        // Branch flows
        // Line 0-1: flow = 6.66666667 MW
        let f01 = &result.branch_flows[&0];
        let expected_f01 = (4.0 / 60.0) * 100.0;
        assert!((f01.p_from_mw - expected_f01).abs() < 1e-12);
        assert!((f01.p_to_mw - (-expected_f01)).abs() < 1e-12);

        // Line 1-2: flow = -33.33333333 MW
        let f12 = &result.branch_flows[&1];
        let expected_f12 = (-1.0 / 3.0) * 100.0;
        assert!((f12.p_from_mw - expected_f12).abs() < 1e-12);
        assert!((f12.p_to_mw - (-expected_f12)).abs() < 1e-12);

        // Line 0-2: flow = -16.66666667 MW
        let f02 = &result.branch_flows[&2];
        let expected_f02 = (-1.0 / 6.0) * 100.0;
        assert!((f02.p_from_mw - expected_f02).abs() < 1e-12);
        assert!((f02.p_to_mw - (-expected_f02)).abs() < 1e-12);

        // Total losses must be zero
        assert_eq!(result.p_loss_mw, 0.0);
    }
}

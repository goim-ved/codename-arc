//! Non-linear AC Power Flow Solver using the Newton-Raphson algorithm in polar coordinates.
//!
//! # Mathematical Formulation
//!
//! In polar coordinates, the complex voltage at bus $i$ is represented as:
//! $$V_i \angle \theta_i = V_i (\cos\theta_i + j\sin\theta_i)$$
//!
//! With the bus admittance matrix $Y_{\text{bus}} = G + jB$, the calculated complex power
//! injection at bus $i$ is:
//!
//! $$S_i = P_i + jQ_i = V_i I_i^* = V_i \sum_{k=1}^N V_k (\cos\theta_{ik} + j\sin\theta_{ik})(G_{ik} - jB_{ik})$$
//!
//! where $\theta_{ik} = \theta_i - \theta_k$.
//!
//! ## Calculated Active and Reactive Power
//! $$P_i(\mathbf{V}, \boldsymbol{\theta}) = V_i \sum_{k=1}^N V_k [G_{ik} \cos\theta_{ik} + B_{ik} \sin\theta_{ik}]$$
//! $$Q_i(\mathbf{V}, \boldsymbol{\theta}) = V_i \sum_{k=1}^N V_k [G_{ik} \sin\theta_{ik} - B_{ik} \cos\theta_{ik}]$$
//!
//! ## Power Mismatch Equations
//! $$\Delta P_i = P_{i, \text{scheduled}} - P_i(\mathbf{V}, \boldsymbol{\theta}) \quad \forall i \notin \text{Slack}$$
//! $$\Delta Q_i = Q_{i, \text{scheduled}} - Q_i(\mathbf{V}, \boldsymbol{\theta}) \quad \forall i \in \text{PQ}$$
//!
//! ## Jacobian Formulation
//! The linearized system at iteration $\nu$ is:
//!
//! $$\begin{bmatrix} \Delta \mathbf{P} \\ \Delta \mathbf{Q} \end{bmatrix} = \begin{bmatrix} H & N \\ M & L \end{bmatrix} \begin{bmatrix} \Delta \boldsymbol{\theta} \\ \Delta \mathbf{V} \end{bmatrix}$$
//!
//! The submatrices are defined analytically by:
//!
//! ### Submatrix $H = \frac{\partial P}{\partial \theta}$ ($(N-1) \times (N-1)$)
//! - For $k \ne i$:
//!   $$H_{ik} = \frac{\partial P_i}{\partial \theta_k} = V_i V_k [G_{ik} \sin\theta_{ik} - B_{ik} \cos\theta_{ik}]$$
//! - For $k = i$:
//!   $$H_{ii} = \frac{\partial P_i}{\partial \theta_i} = -Q_i(\mathbf{V}, \boldsymbol{\theta}) - B_{ii} V_i^2$$
//!
//! ### Submatrix $N = \frac{\partial P}{\partial V}$ ($(N-1) \times N_{\text{PQ}}$)
//! - For $k \ne i$:
//!   $$N_{ik} = \frac{\partial P_i}{\partial V_k} = V_i [G_{ik} \cos\theta_{ik} + B_{ik} \sin\theta_{ik}]$$
//! - For $k = i$:
//!   $$N_{ii} = \frac{\partial P_i}{\partial V_i} = \frac{P_i(\mathbf{V}, \boldsymbol{\theta})}{V_i} + G_{ii} V_i$$
//!
//! ### Submatrix $M = \frac{\partial Q}{\partial \theta}$ ($N_{\text{PQ}} \times (N-1)$)
//! - For $k \ne i$:
//!   $$M_{ik} = \frac{\partial Q_i}{\partial \theta_k} = -V_i V_k [G_{ik} \cos\theta_{ik} + B_{ik} \sin\theta_{ik}]$$
//! - For $k = i$:
//!   $$M_{ii} = \frac{\partial Q_i}{\partial \theta_i} = P_i(\mathbf{V}, \boldsymbol{\theta}) - G_{ii} V_i^2$$
//!
//! ### Submatrix $L = \frac{\partial Q}{\partial V}$ ($N_{\text{PQ}} \times N_{\text{PQ}}$)
//! - For $k \ne i$:
//!   $$L_{ik} = \frac{\partial Q_i}{\partial V_k} = V_i [G_{ik} \sin\theta_{ik} - B_{ik} \cos\theta_{ik}]$$
//! - For $k = i$:
//!   $$L_{ii} = \frac{\partial Q_i}{\partial V_i} = \frac{Q_i(\mathbf{V}, \boldsymbol{\theta})}{V_i} - B_{ii} V_i$$

use crate::admittance::YBus;
use crate::linear::solve_dense_system;
use crate::model::{BusType, Network};
use crate::solver::dc::SolverError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Solved steady-state state for an individual bus under AC power flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ACBusResult {
    /// Bus identifier.
    pub bus_id: usize,
    /// Solved voltage magnitude in per-unit.
    pub vm_pu: f64,
    /// Solved voltage phase angle in radians.
    pub va_rad: f64,
    /// Solved voltage phase angle in degrees.
    pub va_deg: f64,
    /// Net active power injection in per-unit ($P_{\text{gen}} - P_{\text{load}}$).
    pub p_inj_pu: f64,
    /// Net reactive power injection in per-unit ($Q_{\text{gen}} - Q_{\text{load}}$).
    pub q_inj_pu: f64,
    /// Total active power generation at this bus in MW.
    pub p_gen_mw: f64,
    /// Total reactive power generation at this bus in MVar.
    pub q_gen_mvar: f64,
    /// Total active power load at this bus in MW.
    pub p_load_mw: f64,
    /// Total reactive power load at this bus in MVar.
    pub q_load_mvar: f64,
}

/// Solved branch flows and losses under AC power flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ACBranchFlow {
    /// Branch identifier.
    pub branch_id: usize,
    /// Sending / from-bus ID.
    pub from_bus: usize,
    /// Receiving / to-bus ID.
    pub to_bus: usize,
    /// Active power flow leaving from-bus in MW.
    pub p_from_mw: f64,
    /// Reactive power flow leaving from-bus in MVar.
    pub q_from_mw: f64,
    /// Active power flow leaving to-bus in MW.
    pub p_to_mw: f64,
    /// Reactive power flow leaving to-bus in MVar.
    pub q_to_mw: f64,
    /// Active power losses on branch in MW ($P_{\text{from}} + P_{\text{to}}$).
    pub p_loss_mw: f64,
    /// Reactive power losses on branch in MVar ($Q_{\text{from}} + Q_{\text{to}}$).
    pub q_loss_mvar: f64,
}

/// Options controlling Newton-Raphson convergence.
#[derive(Debug, Clone, PartialEq)]
pub struct ACPowerFlowOptions {
    /// Maximum allowable iterations before reporting divergence.
    pub max_iterations: usize,
    /// Maximum absolute mismatch tolerance in per-unit ($||\Delta P, \Delta Q||_\infty$).
    pub tolerance: f64,
}

impl Default for ACPowerFlowOptions {
    fn default() -> Self {
        Self {
            max_iterations: 30,
            tolerance: 1e-8,
        }
    }
}

/// Complete output result of an AC Newton-Raphson power flow calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ACPowerFlowResult {
    /// Whether the solver converged to within specified tolerance.
    pub converged: bool,
    /// Number of Newton-Raphson iterations executed.
    pub iterations: usize,
    /// Final maximum mismatch magnitude in per-unit.
    pub final_mismatch_pu: f64,
    /// Solved bus states, keyed by bus ID in ascending order.
    pub bus_results: BTreeMap<usize, ACBusResult>,
    /// Solved branch flows, keyed by branch ID in ascending order.
    pub branch_flows: BTreeMap<usize, ACBranchFlow>,
    /// Total system active power transmission losses in MW.
    pub total_p_loss_mw: f64,
    /// Total system reactive power transmission losses in MVar.
    pub total_q_loss_mvar: f64,
}

/// Newton-Raphson AC power flow solver.
pub struct ACPowerFlow;

impl ACPowerFlow {
    /// Solves AC power flow on the given network using default convergence options ($10^{-8}$ p.u. tolerance).
    pub fn solve(network: &Network) -> Result<ACPowerFlowResult, SolverError> {
        Self::solve_with_options(network, &ACPowerFlowOptions::default())
    }

    /// Solves AC power flow with custom convergence parameters.
    pub fn solve_with_options(
        network: &Network,
        options: &ACPowerFlowOptions,
    ) -> Result<ACPowerFlowResult, SolverError> {
        network.validate()?;
        let ybus = YBus::build(network)?;
        let n = network.buses.len();

        let mut bus_ids = Vec::with_capacity(n);
        let mut bus_to_idx = BTreeMap::new();
        let mut slack_idx = None;
        let mut pv_indices = Vec::new();
        let mut pq_indices = Vec::new();
        let mut non_slack_indices = Vec::new();

        for (idx, (&id, bus)) in network.buses.iter().enumerate() {
            bus_ids.push(id);
            bus_to_idx.insert(id, idx);
            match bus.bus_type {
                BusType::Slack => slack_idx = Some(idx),
                BusType::PV => {
                    pv_indices.push(idx);
                    non_slack_indices.push(idx);
                }
                BusType::PQ => {
                    pq_indices.push(idx);
                    non_slack_indices.push(idx);
                }
            }
        }

        let s_idx = slack_idx.expect("Network must have 1 slack bus");

        // Scheduled injections in per-unit: P_sched, Q_sched
        let mut p_sched = vec![0.0; n];
        let mut q_sched = vec![0.0; n];
        let mut p_gen_mw = vec![0.0; n];
        let mut q_gen_mw = vec![0.0; n];
        let mut p_load_mw = vec![0.0; n];
        let mut q_load_mw = vec![0.0; n];

        for gen in network.generators.values() {
            if gen.status {
                let &idx = bus_to_idx.get(&gen.bus).unwrap();
                p_gen_mw[idx] += gen.p_mw;
                q_gen_mw[idx] += gen.q_mvar;
            }
        }

        for load in network.loads.values() {
            if load.status {
                let &idx = bus_to_idx.get(&load.bus).unwrap();
                p_load_mw[idx] += load.p_mw;
                q_load_mw[idx] += load.q_mvar;
            }
        }

        for i in 0..n {
            p_sched[i] = (p_gen_mw[i] - p_load_mw[i]) / network.base_mva;
            q_sched[i] = (q_gen_mw[i] - q_load_mw[i]) / network.base_mva;
        }

        // Initialize state vectors: vm (magnitude) and va (angle in radians)
        let mut vm = vec![1.0; n];
        let mut va = vec![0.0; n];

        for (idx, &id) in bus_ids.iter().enumerate() {
            let bus = &network.buses[&id];
            match bus.bus_type {
                BusType::Slack => {
                    vm[idx] = bus.vm_pu;
                    va[idx] = bus.va_rad;
                }
                BusType::PV => {
                    vm[idx] = bus.vm_pu;
                    va[idx] = bus.va_rad;
                }
                BusType::PQ => {
                    vm[idx] = 1.0;
                    va[idx] = bus.va_rad;
                }
            }
        }

        let num_non_slack = non_slack_indices.len();
        let num_pq = pq_indices.len();
        let sys_dim = num_non_slack + num_pq;

        let mut iterations = 0;
        let mut converged = false;
        let mut max_mismatch = 0.0;

        while iterations < options.max_iterations {
            // Step 1: Compute calculated P and Q injections
            let mut p_calc = vec![0.0; n];
            let mut q_calc = vec![0.0; n];

            for i in 0..n {
                let vi = vm[i];
                let theta_i = va[i];
                let mut p_i = 0.0;
                let mut q_i = 0.0;

                for k in 0..n {
                    let vk = vm[k];
                    let theta_k = va[k];
                    let theta_ik = theta_i - theta_k;
                    let g_ik = ybus.g_entry(i, k);
                    let b_ik = ybus.b_entry(i, k);

                    let cos_ik = theta_ik.cos();
                    let sin_ik = theta_ik.sin();

                    p_i += vk * (g_ik * cos_ik + b_ik * sin_ik);
                    q_i += vk * (g_ik * sin_ik - b_ik * cos_ik);
                }

                p_calc[i] = vi * p_i;
                q_calc[i] = vi * q_i;
            }

            // Step 2: Form mismatch vector
            let mut mismatch = Vec::with_capacity(sys_dim);
            max_mismatch = 0.0;

            for &i in &non_slack_indices {
                let dp = p_sched[i] - p_calc[i];
                if dp.abs() > max_mismatch {
                    max_mismatch = dp.abs();
                }
                mismatch.push(dp);
            }

            for &i in &pq_indices {
                let dq = q_sched[i] - q_calc[i];
                if dq.abs() > max_mismatch {
                    max_mismatch = dq.abs();
                }
                mismatch.push(dq);
            }

            // Step 3: Check convergence
            if max_mismatch < options.tolerance {
                converged = true;
                break;
            }

            // Step 4: Assemble dense Jacobian J = [ H  N ; M  L ]
            let mut j = vec![0.0; sys_dim * sys_dim];

            // Mapping from bus index to position in subvectors
            let mut non_slack_pos = BTreeMap::new();
            for (pos, &b_idx) in non_slack_indices.iter().enumerate() {
                non_slack_pos.insert(b_idx, pos);
            }
            let mut pq_pos = BTreeMap::new();
            for (pos, &b_idx) in pq_indices.iter().enumerate() {
                pq_pos.insert(b_idx, pos);
            }

            // Submatrix H: dP / dtheta (num_non_slack x num_non_slack)
            for (r_pos, &i) in non_slack_indices.iter().enumerate() {
                for (c_pos, &k) in non_slack_indices.iter().enumerate() {
                    let h_val = if i != k {
                        let theta_ik = va[i] - va[k];
                        let g_ik = ybus.g_entry(i, k);
                        let b_ik = ybus.b_entry(i, k);
                        vm[i] * vm[k] * (g_ik * theta_ik.sin() - b_ik * theta_ik.cos())
                    } else {
                        // -Q_i - B_ii * V_i^2
                        -q_calc[i] - ybus.b_entry(i, i) * vm[i] * vm[i]
                    };
                    j[r_pos * sys_dim + c_pos] = h_val;
                }
            }

            // Submatrix N: dP / dV (num_non_slack x num_pq)
            for (r_pos, &i) in non_slack_indices.iter().enumerate() {
                for (c_pos, &k) in pq_indices.iter().enumerate() {
                    let col_idx = num_non_slack + c_pos;
                    let n_val = if i != k {
                        let theta_ik = va[i] - va[k];
                        let g_ik = ybus.g_entry(i, k);
                        let b_ik = ybus.b_entry(i, k);
                        vm[i] * (g_ik * theta_ik.cos() + b_ik * theta_ik.sin())
                    } else {
                        // P_i / V_i + G_ii * V_i
                        p_calc[i] / vm[i] + ybus.g_entry(i, i) * vm[i]
                    };
                    j[r_pos * sys_dim + col_idx] = n_val;
                }
            }

            // Submatrix M: dQ / dtheta (num_pq x num_non_slack)
            for (r_pos, &i) in pq_indices.iter().enumerate() {
                let row_idx = num_non_slack + r_pos;
                for (c_pos, &k) in non_slack_indices.iter().enumerate() {
                    let m_val = if i != k {
                        let theta_ik = va[i] - va[k];
                        let g_ik = ybus.g_entry(i, k);
                        let b_ik = ybus.b_entry(i, k);
                        -vm[i] * vm[k] * (g_ik * theta_ik.cos() + b_ik * theta_ik.sin())
                    } else {
                        // P_i - G_ii * V_i^2
                        p_calc[i] - ybus.g_entry(i, i) * vm[i] * vm[i]
                    };
                    j[row_idx * sys_dim + c_pos] = m_val;
                }
            }

            // Submatrix L: dQ / dV (num_pq x num_pq)
            for (r_pos, &i) in pq_indices.iter().enumerate() {
                let row_idx = num_non_slack + r_pos;
                for (c_pos, &k) in pq_indices.iter().enumerate() {
                    let col_idx = num_non_slack + c_pos;
                    let l_val = if i != k {
                        let theta_ik = va[i] - va[k];
                        let g_ik = ybus.g_entry(i, k);
                        let b_ik = ybus.b_entry(i, k);
                        vm[i] * (g_ik * theta_ik.sin() - b_ik * theta_ik.cos())
                    } else {
                        // Q_i / V_i - B_ii * V_i
                        q_calc[i] / vm[i] - ybus.b_entry(i, i) * vm[i]
                    };
                    j[row_idx * sys_dim + col_idx] = l_val;
                }
            }

            // Step 5: Solve J * dx = mismatch via deterministic linear solver
            let dx = solve_dense_system(&j, &mismatch, sys_dim)?;

            // Step 6: Update state variables
            for (pos, &i) in non_slack_indices.iter().enumerate() {
                va[i] += dx[pos];
            }
            for (pos, &i) in pq_indices.iter().enumerate() {
                vm[i] += dx[num_non_slack + pos];
            }

            iterations += 1;
        }

        // Final recalculation of power injections with converged state
        let mut p_final = vec![0.0; n];
        let mut q_final = vec![0.0; n];

        for i in 0..n {
            let vi = vm[i];
            let theta_i = va[i];
            let mut p_i = 0.0;
            let mut q_i = 0.0;

            for k in 0..n {
                let vk = vm[k];
                let theta_ik = theta_i - va[k];
                let g_ik = ybus.g_entry(i, k);
                let b_ik = ybus.b_entry(i, k);

                p_i += vk * (g_ik * theta_ik.cos() + b_ik * theta_ik.sin());
                q_i += vk * (g_ik * theta_ik.sin() - b_ik * theta_ik.cos());
            }

            p_final[i] = vi * p_i;
            q_final[i] = vi * q_i;
        }

        // Slack bus generation: P_slack_gen = P_slack_calc * base_mva + P_slack_load
        p_gen_mw[s_idx] = p_final[s_idx] * network.base_mva + p_load_mw[s_idx];
        q_gen_mw[s_idx] = q_final[s_idx] * network.base_mva + q_load_mw[s_idx];

        // PV buses reactive generation: Q_pv_gen = Q_pv_calc * base_mva + Q_pv_load
        for &i in &pv_indices {
            q_gen_mw[i] = q_final[i] * network.base_mva + q_load_mw[i];
        }

        // Build bus results
        let mut bus_results = BTreeMap::new();
        for (idx, &id) in bus_ids.iter().enumerate() {
            bus_results.insert(
                id,
                ACBusResult {
                    bus_id: id,
                    vm_pu: vm[idx],
                    va_rad: va[idx],
                    va_deg: va[idx].to_degrees(),
                    p_inj_pu: p_final[idx],
                    q_inj_pu: q_final[idx],
                    p_gen_mw: p_gen_mw[idx],
                    q_gen_mvar: q_gen_mw[idx],
                    p_load_mw: p_load_mw[idx],
                    q_load_mvar: q_load_mw[idx],
                },
            );
        }

        // Branch flows and losses
        let mut branch_flows = BTreeMap::new();
        let mut total_p_loss_mw = 0.0;
        let mut total_q_loss_mvar = 0.0;

        for branch in network.branches.values() {
            if !branch.status {
                continue;
            }

            let &i = bus_to_idx.get(&branch.from_bus).unwrap();
            let &k = bus_to_idx.get(&branch.to_bus).unwrap();

            let (g_s, b_s) = branch.series_y_pu();
            let half_b = branch.b_pu / 2.0;

            let a = branch.tap_ratio;
            let phi = branch.shift_rad;

            // Voltage at from-bus and to-bus
            let vi = vm[i];
            let vk = vm[k];
            let theta_i = va[i];
            let theta_k = va[k];

            // Branch flow from i to k:
            // S_ik = V_i * I_ik*
            // I_ik = (V_i/t - V_k) * y_s * (1/t*) + (V_i/t) * (j b/2) * (1/t*)
            // For a regular line (a=1, phi=0):
            // P_ik = V_i^2 * g_s - V_i V_k [g_s cos(theta_ik) + b_s sin(theta_ik)]
            // Q_ik = -V_i^2 * (b_s + b/2) - V_i V_k [g_s sin(theta_ik) - b_s cos(theta_ik)]
            let theta_ik = theta_i - theta_k - phi;
            let cos_ik = theta_ik.cos();
            let sin_ik = theta_ik.sin();

            let p_from_pu =
                (vi * vi / (a * a)) * g_s - (vi * vk / a) * (g_s * cos_ik + b_s * sin_ik);
            let q_from_pu = -(vi * vi / (a * a)) * (b_s + half_b)
                - (vi * vk / a) * (g_s * sin_ik - b_s * cos_ik);

            let theta_ki = theta_k - theta_i + phi;
            let cos_ki = theta_ki.cos();
            let sin_ki = theta_ki.sin();

            let p_to_pu = vk * vk * g_s - (vi * vk / a) * (g_s * cos_ki + b_s * sin_ki);
            let q_to_pu = -vk * vk * (b_s + half_b) - (vi * vk / a) * (g_s * sin_ki - b_s * cos_ki);

            let p_from_mw = p_from_pu * network.base_mva;
            let q_from_mw = q_from_pu * network.base_mva;
            let p_to_mw = p_to_pu * network.base_mva;
            let q_to_mw = q_to_pu * network.base_mva;

            let p_loss = p_from_mw + p_to_mw;
            let q_loss = q_from_mw + q_to_mw;

            total_p_loss_mw += p_loss;
            total_q_loss_mvar += q_loss;

            branch_flows.insert(
                branch.id,
                ACBranchFlow {
                    branch_id: branch.id,
                    from_bus: branch.from_bus,
                    to_bus: branch.to_bus,
                    p_from_mw,
                    q_from_mw,
                    p_to_mw,
                    q_to_mw,
                    p_loss_mw: p_loss,
                    q_loss_mvar: q_loss,
                },
            );
        }

        Ok(ACPowerFlowResult {
            converged,
            iterations,
            final_mismatch_pu: max_mismatch,
            bus_results,
            branch_flows,
            total_p_loss_mw,
            total_q_loss_mvar,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Branch, Bus, Generator, Load};

    #[test]
    fn test_ac_solver_3bus_convergence() {
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

        net.add_generator(Generator::new(0, 0, 0.0, 1.0)).unwrap();
        net.add_load(Load::new(0, 1, 40.0, 20.0)).unwrap();
        net.add_generator(Generator::new(1, 2, 50.0, 1.02)).unwrap();

        let result = ACPowerFlow::solve(&net).expect("AC solve should succeed");

        assert!(result.converged, "AC power flow must converge");
        assert!(
            result.iterations <= 6,
            "Expected quadratic convergence in <= 6 iterations, took {}",
            result.iterations
        );
        assert!(result.final_mismatch_pu < 1e-8);

        // Check voltages
        let b0 = &result.bus_results[&0];
        assert_eq!(b0.vm_pu, 1.0);
        assert_eq!(b0.va_rad, 0.0);

        let b1 = &result.bus_results[&1];
        assert!((b1.vm_pu - 1.006689).abs() < 1e-4);
        assert!((b1.va_deg - (-0.3823)).abs() < 1e-3);

        let b2 = &result.bus_results[&2];
        assert_eq!(b2.vm_pu, 1.02);
        assert!((b2.va_deg - (-0.0117)).abs() < 1e-3);
    }
}

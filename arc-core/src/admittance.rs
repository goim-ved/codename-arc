//! Bus Admittance Matrix ($Y_{\text{bus}}$) builder.
//!
//! # Mathematical Formulation
//!
//! The bus admittance matrix relates complex nodal current injections $\mathbf{I}$
//! to complex bus voltages $\mathbf{V}$ via Ohm's law in matrix form:
//!
//! $$\mathbf{I} = Y_{\text{bus}} \mathbf{V}$$
//!
//! For an $N$-bus network, $Y_{\text{bus}} \in \mathbb{C}^{N \times N}$ is structured as:
//!
//! $$Y_{\text{bus}} = G + jB$$
//!
//! where $G$ is the conductance matrix and $B$ is the susceptance matrix.
//!
//! ## Branch Modeling (Lumped $\Pi$-equivalent)
//! For each in-service branch $m$ connecting bus $i$ to bus $k$:
//!
//! 1. **Series Admittance**:
//!    $$y_m = \frac{1}{r_m + j x_m} = g_m + j b_m$$
//!    where:
//!    $$g_m = \frac{r_m}{r_m^2 + x_m^2}, \quad b_m = \frac{-x_m}{r_m^2 + x_m^2}$$
//!
//! 2. **Transformer Tap & Phase Shift**:
//!    If the branch is a transformer with off-nominal turns ratio $a$ and phase shift $\phi$
//!    at sending bus $i$, the complex turns ratio is $t = a e^{j\phi} = a(\cos\phi + j\sin\phi)$.
//!
//! 3. **Admittance Matrix Contributions**:
//!    With total line charging susceptance $b_{\text{shunt}}$ ($b_{\text{shunt}} / 2$ per side):
//!    - Self-admittance at bus $i$ (diagonal):
//!      $$Y_{ii} \mathrel{+}= \frac{y_m + j \frac{b_{\text{shunt}}}{2}}{|t|^2} = \frac{y_m + j \frac{b_{\text{shunt}}}{2}}{a^2}$$
//!    - Self-admittance at bus $k$ (diagonal):
//!      $$Y_{kk} \mathrel{+}= y_m + j \frac{b_{\text{shunt}}}{2}$$
//!    - Mutual admittance from $i$ to $k$ (off-diagonal):
//!      $$Y_{ik} \mathrel{-}= \frac{y_m}{t^*} = \frac{y_m}{a(\cos\phi - j\sin\phi)}$$
//!    - Mutual admittance from $k$ to $i$ (off-diagonal):
//!      $$Y_{ki} \mathrel{-}= \frac{y_m}{t} = \frac{y_m}{a(\cos\phi + j\sin\phi)}$$
//!
//! For a standard transmission line without transformer taps ($a = 1.0, \phi = 0.0$):
//! $$Y_{ii} \mathrel{+}= y_m + j \frac{b_{\text{shunt}}}{2}, \quad Y_{kk} \mathrel{+}= y_m + j \frac{b_{\text{shunt}}}{2}, \quad Y_{ik} = Y_{ki} \mathrel{-}= y_m$$

use crate::model::{ModelError, Network};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bus admittance matrix ($Y_{\text{bus}} = G + jB$) for steady-state power flow.
///
/// Matrix entries are stored in deterministic row-major layout with separate
/// conductance ($G$) and susceptance ($B$) arrays to guarantee deterministic,
/// zero-overhead access during linear solves and Jacobian evaluations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YBus {
    /// Dimension of the square matrix ($N \times N$).
    pub n: usize,
    /// Ordered list of bus IDs corresponding to matrix rows/columns $0 \dots N-1$.
    pub bus_ids: Vec<usize>,
    /// Mapping from external bus ID to matrix row/column index.
    pub bus_index_map: BTreeMap<usize, usize>,
    /// Conductance matrix elements $G$ in row-major order ($N \times N$).
    pub g: Vec<f64>,
    /// Susceptance matrix elements $B$ in row-major order ($N \times N$).
    pub b: Vec<f64>,
}

impl YBus {
    /// Builds the bus admittance matrix from a validated power network model.
    ///
    /// # Errors
    /// Returns `ModelError` if the network fails validation or contains invalid bus references.
    pub fn build(network: &Network) -> Result<Self, ModelError> {
        network.validate()?;

        let n = network.buses.len();
        let mut bus_ids = Vec::with_capacity(n);
        let mut bus_index_map = BTreeMap::new();

        for (idx, &bus_id) in network.buses.keys().enumerate() {
            bus_ids.push(bus_id);
            bus_index_map.insert(bus_id, idx);
        }

        let matrix_size = n * n;
        let mut g = vec![0.0; matrix_size];
        let mut b = vec![0.0; matrix_size];

        // Process all active branches
        for branch in network.branches.values() {
            if !branch.status {
                continue;
            }

            let &i = bus_index_map
                .get(&branch.from_bus)
                .ok_or(ModelError::BusNotFound(branch.from_bus))?;
            let &k = bus_index_map
                .get(&branch.to_bus)
                .ok_or(ModelError::BusNotFound(branch.to_bus))?;

            // Series admittance y = 1 / (r + jx) = g_s + j b_s
            let (g_s, b_s) = branch.series_y_pu();
            let half_b_shunt = branch.b_pu / 2.0;

            let a = branch.tap_ratio;
            let phi = branch.shift_rad;
            let a_sq = a * a;

            // Diagonal addition at bus i: (y + j * b_shunt/2) / a^2
            let ii = i * n + i;
            g[ii] += g_s / a_sq;
            b[ii] += (b_s + half_b_shunt) / a_sq;

            // Diagonal addition at bus k: y + j * b_shunt/2
            let kk = k * n + k;
            g[kk] += g_s;
            b[kk] += b_s + half_b_shunt;

            // Mutual admittance Y_ik = -y / t*
            // t* = a * (cos phi - j sin phi)
            // 1 / t* = (cos phi + j sin phi) / a
            // Y_ik = -(g_s + j b_s) * (cos phi + j sin phi) / a
            //      = -[ (g_s cos phi - b_s sin phi) + j (g_s sin phi + b_s cos phi) ] / a
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();

            let g_ik = -(g_s * cos_phi - b_s * sin_phi) / a;
            let b_ik = -(g_s * sin_phi + b_s * cos_phi) / a;

            let ik = i * n + k;
            g[ik] += g_ik;
            b[ik] += b_ik;

            // Mutual admittance Y_ki = -y / t
            // t = a * (cos phi + j sin phi)
            // 1 / t = (cos phi - j sin phi) / a
            // Y_ki = -(g_s + j b_s) * (cos phi - j sin phi) / a
            //      = -[ (g_s cos phi + b_s sin phi) + j (-g_s sin phi + b_s cos phi) ] / a
            let g_ki = -(g_s * cos_phi + b_s * sin_phi) / a;
            let b_ki = -(-g_s * sin_phi + b_s * cos_phi) / a;

            let ki = k * n + i;
            g[ki] += g_ki;
            b[ki] += b_ki;
        }

        // Process bus shunts (capacitors and reactors)
        for shunt in network.shunts.values() {
            if !shunt.status {
                continue;
            }
            let &idx = bus_index_map
                .get(&shunt.bus)
                .ok_or(ModelError::BusNotFound(shunt.bus))?;
            let g_pu = shunt.g_pu(network.base_mva);
            let b_pu = shunt.b_pu(network.base_mva);
            let ii = idx * n + idx;
            g[ii] += g_pu;
            b[ii] += b_pu;
        }

        Ok(Self {
            n,
            bus_ids,
            bus_index_map,
            g,
            b,
        })
    }

    /// Conductance entry $G_{ik}$ at row $i$, column $k$ (0-indexed).
    #[inline]
    pub fn g_entry(&self, i: usize, k: usize) -> f64 {
        self.g[i * self.n + k]
    }

    /// Susceptance entry $B_{ik}$ at row $i$, column $k$ (0-indexed).
    #[inline]
    pub fn b_entry(&self, i: usize, k: usize) -> f64 {
        self.b[i * self.n + k]
    }

    /// Complex admittance entry $Y_{ik} = G_{ik} + j B_{ik}$ at row $i$, column $k$.
    #[inline]
    pub fn y_entry(&self, i: usize, k: usize) -> (f64, f64) {
        let idx = i * self.n + k;
        (self.g[idx], self.b[idx])
    }

    /// Returns the matrix row/column index for a given bus ID.
    #[inline]
    pub fn bus_index(&self, bus_id: usize) -> Option<usize> {
        self.bus_index_map.get(&bus_id).copied()
    }

    /// Returns the bus ID corresponding to a given matrix row/column index.
    #[inline]
    pub fn bus_id(&self, index: usize) -> Option<usize> {
        self.bus_ids.get(index).copied()
    }

    /// Checks whether the admittance matrix is symmetric within a numerical tolerance:
    /// $|Y_{ik} - Y_{ki}| \le \text{tol}$.
    ///
    /// Networks without phase-shifting transformers are strictly symmetric.
    pub fn is_symmetric(&self, tol: f64) -> bool {
        for i in 0..self.n {
            for k in (i + 1)..self.n {
                let diff_g = (self.g_entry(i, k) - self.g_entry(k, i)).abs();
                let diff_b = (self.b_entry(i, k) - self.b_entry(k, i)).abs();
                if diff_g > tol || diff_b > tol {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Branch, Bus, BusType, Network};

    /// Hand-calculated 2-bus network verification:
    ///
    /// Network specification:
    /// - Bus 0 (Slack, 138 kV)
    /// - Bus 1 (PQ, 138 kV)
    /// - Branch 0-1: R = 0.05 p.u., X = 0.15 p.u., B = 0.0 p.u.
    ///
    /// Step 1: Series impedance
    ///   Z = 0.05 + j 0.15
    ///   |Z|^2 = 0.05^2 + 0.15^2 = 0.0025 + 0.0225 = 0.0250
    ///
    /// Step 2: Series admittance
    ///   y_01 = 1 / Z = (0.05 - j 0.15) / 0.0250
    ///   g_01 = 0.05 / 0.0250 = 2.0 p.u.
    ///   b_01 = -0.15 / 0.0250 = -6.0 p.u.
    ///   y_01 = 2.0 - j 6.0 p.u.
    ///
    /// Step 3: Ybus elements
    ///   Y_00 = y_01 = 2.0 - j 6.0 p.u.
    ///   Y_11 = y_01 = 2.0 - j 6.0 p.u.
    ///   Y_01 = -y_01 = -2.0 + j 6.0 p.u.
    ///   Y_10 = -y_01 = -2.0 + j 6.0 p.u.
    #[test]
    fn test_hand_derived_2bus_ybus() {
        let mut net = Network::new(100.0);
        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();
        net.add_branch(Branch::new_line(0, 0, 1, 0.05, 0.15))
            .unwrap();

        let ybus = YBus::build(&net).expect("Ybus construction should succeed");

        assert_eq!(ybus.n, 2);
        assert!(ybus.is_symmetric(1e-12));

        // Diagonal Y_00 = 2.0 - j 6.0
        let (g00, b00) = ybus.y_entry(0, 0);
        assert!((g00 - 2.0).abs() < 1e-12, "g00 expected 2.0, got {g00}");
        assert!((b00 - (-6.0)).abs() < 1e-12, "b00 expected -6.0, got {b00}");

        // Diagonal Y_11 = 2.0 - j 6.0
        let (g11, b11) = ybus.y_entry(1, 1);
        assert!((g11 - 2.0).abs() < 1e-12, "g11 expected 2.0, got {g11}");
        assert!((b11 - (-6.0)).abs() < 1e-12, "b11 expected -6.0, got {b11}");

        // Off-diagonal Y_01 = -2.0 + j 6.0
        let (g01, b01) = ybus.y_entry(0, 1);
        assert!((g01 - (-2.0)).abs() < 1e-12, "g01 expected -2.0, got {g01}");
        assert!((b01 - 6.0).abs() < 1e-12, "b01 expected 6.0, got {b01}");

        // Off-diagonal Y_10 = -2.0 + j 6.0
        let (g10, b10) = ybus.y_entry(1, 0);
        assert!((g10 - (-2.0)).abs() < 1e-12, "g10 expected -2.0, got {g10}");
        assert!((b10 - 6.0).abs() < 1e-12, "b10 expected 6.0, got {b10}");
    }

    /// Hand-calculated 3-bus network verification:
    ///
    /// Canonical 3-bus network specification (matching pandapower oracle `case3`):
    /// - Bus 0: Slack, 138 kV
    /// - Bus 1: PQ, 138 kV
    /// - Bus 2: PV, 138 kV
    ///
    /// Branch impedances:
    /// - Line 0-1: r_01 = 0.02, x_01 = 0.06, b_01 = 0.0
    ///   |z_01|^2 = 0.02^2 + 0.06^2 = 0.0004 + 0.0036 = 0.0040
    ///   y_01 = (0.02 - j 0.06) / 0.0040 = 5.0 - j 15.0
    ///
    /// - Line 1-2: r_12 = 0.01, x_12 = 0.03, b_12 = 0.0
    ///   |z_12|^2 = 0.01^2 + 0.03^2 = 0.0001 + 0.0009 = 0.0010
    ///   y_12 = (0.01 - j 0.03) / 0.0010 = 10.0 - j 30.0
    ///
    /// - Line 0-2: r_02 = 0.012, x_02 = 0.036, b_02 = 0.0
    ///   |z_02|^2 = 0.012^2 + 0.036^2 = 0.000144 + 0.001296 = 0.001440
    ///   y_02 = (0.012 - j 0.036) / 0.001440 = 25/3 - j 25.0 = 8.333333333333... - j 25.0
    ///
    /// Complete Bus Admittance Matrix:
    ///   Y_00 = y_01 + y_02 = (5.0 + 25/3) - j (15.0 + 25.0) = 40/3 - j 40.0
    ///   Y_01 = -y_01 = -5.0 + j 15.0
    ///   Y_02 = -y_02 = -25/3 + j 25.0
    ///
    ///   Y_10 = -y_01 = -5.0 + j 15.0
    ///   Y_11 = y_01 + y_12 = (5.0 + 10.0) - j (15.0 + 30.0) = 15.0 - j 45.0
    ///   Y_12 = -y_12 = -10.0 + j 30.0
    ///
    ///   Y_20 = -y_02 = -25/3 + j 25.0
    ///   Y_21 = -y_12 = -10.0 + j 30.0
    ///   Y_22 = y_12 + y_02 = (10.0 + 25/3) - j (30.0 + 25.0) = 55/3 - j 55.0
    #[test]
    fn test_hand_derived_3bus_canonical_ybus() {
        let mut net = Network::new(100.0);
        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();
        net.add_bus(Bus::new(2, BusType::PV, 138.0)).unwrap();

        net.add_branch(Branch::new_line(0, 0, 1, 0.02, 0.06))
            .unwrap();
        net.add_branch(Branch::new_line(1, 1, 2, 0.01, 0.03))
            .unwrap();
        net.add_branch(Branch::new_line(2, 0, 2, 0.012, 0.036))
            .unwrap();

        let ybus = YBus::build(&net).expect("Ybus construction should succeed");

        assert_eq!(ybus.n, 3);
        assert!(
            ybus.is_symmetric(1e-12),
            "Transmission line Ybus must be strictly symmetric"
        );

        // Row 0:
        let (g00, b00) = ybus.y_entry(0, 0);
        let expected_g00 = 40.0 / 3.0; // 13.333333333333334
        let expected_b00 = -40.0;
        assert!((g00 - expected_g00).abs() < 1e-12);
        assert!((b00 - expected_b00).abs() < 1e-12);

        let (g01, b01) = ybus.y_entry(0, 1);
        assert!((g01 - (-5.0)).abs() < 1e-12);
        assert!((b01 - 15.0).abs() < 1e-12);

        let (g02, b02) = ybus.y_entry(0, 2);
        let expected_g02 = -25.0 / 3.0; // -8.333333333333334
        let expected_b02 = 25.0;
        assert!((g02 - expected_g02).abs() < 1e-12);
        assert!((b02 - expected_b02).abs() < 1e-12);

        // Row 1:
        let (g10, b10) = ybus.y_entry(1, 0);
        assert!((g10 - (-5.0)).abs() < 1e-12);
        assert!((b10 - 15.0).abs() < 1e-12);

        let (g11, b11) = ybus.y_entry(1, 1);
        assert!((g11 - 15.0).abs() < 1e-12);
        assert!((b11 - (-45.0)).abs() < 1e-12);

        let (g12, b12) = ybus.y_entry(1, 2);
        assert!((g12 - (-10.0)).abs() < 1e-12);
        assert!((b12 - 30.0).abs() < 1e-12);

        // Row 2:
        let (g20, b20) = ybus.y_entry(2, 0);
        assert!((g20 - expected_g02).abs() < 1e-12);
        assert!((b20 - expected_b02).abs() < 1e-12);

        let (g21, b21) = ybus.y_entry(2, 1);
        assert!((g21 - (-10.0)).abs() < 1e-12);
        assert!((b21 - 30.0).abs() < 1e-12);

        let (g22, b22) = ybus.y_entry(2, 2);
        let expected_g22 = 55.0 / 3.0; // 18.333333333333334
        let expected_b22 = -55.0;
        assert!((g22 - expected_g22).abs() < 1e-12);
        assert!((b22 - expected_b22).abs() < 1e-12);
    }

    /// Shunt line charging susceptance test:
    /// When line charging B_shunt is present, it adds B_shunt / 2 to the diagonal susceptance
    /// of each terminating bus without altering the mutual (off-diagonal) admittance.
    #[test]
    fn test_line_charging_shunt_susceptance() {
        let mut net = Network::new(100.0);
        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();

        // Line with R = 0.05, X = 0.15, B_shunt = 0.04 pu
        let branch = Branch::new_line(0, 0, 1, 0.05, 0.15).with_b_pu(0.04);
        net.add_branch(branch).unwrap();

        let ybus = YBus::build(&net).unwrap();

        // Series y = 2.0 - j 6.0
        // Diagonal B should be: -6.0 + 0.04 / 2 = -6.0 + 0.02 = -5.98
        let (_g00, b00) = ybus.y_entry(0, 0);
        let (_g11, b11) = ybus.y_entry(1, 1);
        assert!((b00 - (-5.98)).abs() < 1e-12);
        assert!((b11 - (-5.98)).abs() < 1e-12);

        // Off-diagonal B should still be -(-6.0) = +6.0
        let (_g01, b01) = ybus.y_entry(0, 1);
        assert!((b01 - 6.0).abs() < 1e-12);
    }

    /// Transformer off-nominal tap ratio and phase shifter test:
    /// A transformer with tap ratio a != 1.0 scales the diagonals and off-diagonals appropriately.
    #[test]
    fn test_transformer_tap_ratio() {
        let mut net = Network::new(100.0);
        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();

        // Transformer with R = 0.0, X = 0.10, tap = 1.05 (5% boost on from-bus 0)
        // y_series = 1 / (j 0.10) = -j 10.0 => g_s = 0, b_s = -10.0
        // Y_00 = y_series / a^2 = -j 10.0 / (1.05^2) = -j (10.0 / 1.1025)
        // Y_11 = y_series = -j 10.0
        // Y_01 = Y_10 = -y_series / a = +j 10.0 / 1.05
        let xfmr = Branch::new_transformer(0, 0, 1, 0.0, 0.10, 1.05, 0.0);
        net.add_branch(xfmr).unwrap();

        let ybus = YBus::build(&net).unwrap();

        let expected_b00 = -10.0 / (1.05 * 1.05);
        let expected_b11 = -10.0;
        let expected_b01 = 10.0 / 1.05;

        assert!((ybus.b_entry(0, 0) - expected_b00).abs() < 1e-12);
        assert!((ybus.b_entry(1, 1) - expected_b11).abs() < 1e-12);
        assert!((ybus.b_entry(0, 1) - expected_b01).abs() < 1e-12);
        assert!((ybus.b_entry(1, 0) - expected_b01).abs() < 1e-12);
    }

    /// Out-of-service branch test:
    /// Branches with status = false must not be included in the admittance matrix.
    #[test]
    fn test_out_of_service_branch_ignored() {
        let mut net = Network::new(100.0);
        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();

        let mut branch = Branch::new_line(0, 0, 1, 0.05, 0.15);
        branch.status = false;
        net.add_branch(branch).unwrap();

        let ybus = YBus::build(&net).unwrap();

        // Since the only branch is out of service, all entries should be 0.0
        assert_eq!(ybus.g_entry(0, 0), 0.0);
        assert_eq!(ybus.b_entry(0, 0), 0.0);
        assert_eq!(ybus.g_entry(0, 0), 0.0);
        assert_eq!(ybus.b_entry(0, 0), 0.0);
        assert_eq!(ybus.g_entry(1, 1), 0.0);
        assert_eq!(ybus.b_entry(1, 1), 0.0);
    }

    /// Bus shunt capacitor/reactor test:
    /// Injected susceptance B_shunt (MVar) and conductance G_shunt (MW) add directly to diagonal.
    #[test]
    fn test_bus_shunt_admittance() {
        use crate::model::Shunt;

        let mut net = Network::new(100.0);
        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();

        // Shunt at Bus 1: 5 MW, +19 MVar capacitor (B_pu = +0.19 pu, G_pu = +0.05 pu)
        let shunt = Shunt::new(0, 1, 5.0, 19.0);
        net.add_shunt(shunt).unwrap();

        let ybus = YBus::build(&net).unwrap();

        assert!((ybus.g_entry(1, 1) - 0.05).abs() < 1e-12);
        assert!((ybus.b_entry(1, 1) - 0.19).abs() < 1e-12);
        assert_eq!(ybus.g_entry(0, 0), 0.0);
        assert_eq!(ybus.b_entry(0, 0), 0.0);
    }
}

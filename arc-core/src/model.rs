//! Core power grid data models and per-unit normalization.
//!
//! # Per-Unit System Conventions
//!
//! All power flow calculations in `arc` operate in the normalized per-unit (p.u.) system.
//!
//! ## System-Wide Base Values
//! - **Base Apparent Power ($S_{\text{base}}$)**: By convention in standard North American
//!   interconnection studies, $S_{\text{base}} = 100.0\text{ MVA}$ across the entire network.
//!   While configurable on a per-network basis, $100.0\text{ MVA}$ is the explicit default.
//! - **Base Voltage ($V_{\text{base}}$)**: Specified per bus in line-to-line kilovolts ($\text{kV}$).
//!
//! ## Derived Base Relationships
//! For a bus with nominal line-to-line voltage $V_{\text{base}}$ (in $\text{kV}$) and system base
//! $S_{\text{base}}$ (in $\text{MVA}$):
//!
//! - **Base Impedance ($Z_{\text{base}}$)**:
//!   $$Z_{\text{base}} = \frac{(V_{\text{base, kV}})^2}{S_{\text{base, MVA}}} \quad [\Omega]$$
//!
//! - **Base Admittance ($Y_{\text{base}}$)**:
//!   $$Y_{\text{base}} = \frac{1}{Z_{\text{base}}} = \frac{S_{\text{base, MVA}}}{(V_{\text{base, kV}})^2} \quad [\text{S} = \Omega^{-1}]$$
//!
//! - **Base Current ($I_{\text{base}}$)**:
//!   $$I_{\text{base}} = \frac{S_{\text{base, MVA}}}{\sqrt{3} \cdot V_{\text{base, kV}}} \quad [\text{kA}]$$
//!
//! ## Normalization Formulas
//! - Active Power: $P_{\text{pu}} = \frac{P_{\text{MW}}}{S_{\text{base, MVA}}}$
//! - Reactive Power: $Q_{\text{pu}} = \frac{Q_{\text{MVar}}}{S_{\text{base, MVA}}}$
//! - Series Impedance: $R_{\text{pu}} = \frac{R_{\Omega}}{Z_{\text{base}}}$, $X_{\text{pu}} = \frac{X_{\Omega}}{Z_{\text{base}}}$
//! - Shunt Admittance: $B_{\text{pu}} = B_{\text{S}} \cdot Z_{\text{base}} = \frac{B_{\text{S}}}{Y_{\text{base}}}$

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Standard default base apparent power in MVA.
pub const DEFAULT_BASE_MVA: f64 = 100.0;

/// Type of grid bus defining the knowns and unknowns in steady-state power flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BusType {
    /// Slack / Swing bus: Reference bus where voltage magnitude ($V$) and voltage angle ($\theta = 0^\circ$)
    /// are fixed; active ($P$) and reactive ($Q$) power generation are solved.
    Slack,
    /// Voltage-controlled / Generator bus: Active power generation ($P$) and voltage magnitude ($V$)
    /// are fixed; reactive power ($Q$) and voltage angle ($\theta$) are solved.
    PV,
    /// Load / Demand bus: Net active ($P$) and reactive ($Q$) power injections are fixed;
    /// voltage magnitude ($V$) and voltage angle ($\theta$) are solved.
    PQ,
}

impl fmt::Display for BusType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Slack => write!(f, "Slack"),
            Self::PV => write!(f, "PV"),
            Self::PQ => write!(f, "PQ"),
        }
    }
}

/// Representation of an electrical substation or junction bus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bus {
    /// Unique bus identifier (e.g. 0, 1, 2...).
    pub id: usize,
    /// Human-readable label or station name.
    pub name: Option<String>,
    /// Power flow bus type (Slack, PV, or PQ).
    pub bus_type: BusType,
    /// Nominal line-to-line voltage rating in kilovolts (kV). Must be strictly positive.
    pub base_kv: f64,
    /// Voltage magnitude in per-unit (p.u.). Defaults to 1.0 p.u. (flat start).
    pub vm_pu: f64,
    /// Voltage phase angle in radians. Defaults to 0.0 rad.
    pub va_rad: f64,
    /// Minimum allowable voltage magnitude in per-unit (p.u.). Typically ~0.9 p.u.
    pub v_min_pu: f64,
    /// Maximum allowable voltage magnitude in per-unit (p.u.). Typically ~1.1 p.u.
    pub v_max_pu: f64,
}

impl Bus {
    /// Creates a new Bus with default flat-start voltage ($V = 1.0\text{ p.u.}, \theta = 0.0\text{ rad}$)
    /// and standard voltage limits ($[0.9, 1.1]\text{ p.u.}$).
    pub fn new(id: usize, bus_type: BusType, base_kv: f64) -> Self {
        Self {
            id,
            name: None,
            bus_type,
            base_kv,
            vm_pu: 1.0,
            va_rad: 0.0,
            v_min_pu: 0.9,
            v_max_pu: 1.1,
        }
    }

    /// Sets optional human-readable name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets initial voltage magnitude in per-unit.
    pub fn with_vm_pu(mut self, vm_pu: f64) -> Self {
        self.vm_pu = vm_pu;
        self
    }

    /// Sets initial voltage angle in degrees (converted internally to radians).
    pub fn with_va_deg(mut self, va_deg: f64) -> Self {
        self.va_rad = va_deg.to_radians();
        self
    }

    /// Sets initial voltage angle in radians.
    pub fn with_va_rad(mut self, va_rad: f64) -> Self {
        self.va_rad = va_rad;
        self
    }

    /// Sets operational voltage limits in per-unit.
    pub fn with_voltage_limits(mut self, v_min_pu: f64, v_max_pu: f64) -> Self {
        self.v_min_pu = v_min_pu;
        self.v_max_pu = v_max_pu;
        self
    }

    /// Returns the voltage angle in degrees.
    pub fn va_deg(&self) -> f64 {
        self.va_rad.to_degrees()
    }

    /// Computes the base impedance for this bus using the given system base MVA.
    ///
    /// $$Z_{\text{base}} = \frac{V_{\text{base, kV}}^2}{S_{\text{base, MVA}}}$$
    pub fn base_impedance_ohms(&self, base_mva: f64) -> f64 {
        (self.base_kv * self.base_kv) / base_mva
    }

    /// Computes the base current for this bus in kiloamperes (kA).
    ///
    /// $$I_{\text{base}} = \frac{S_{\text{base, MVA}}}{\sqrt{3} \cdot V_{\text{base, kV}}}$$
    pub fn base_current_ka(&self, base_mva: f64) -> f64 {
        base_mva / (3.0_f64.sqrt() * self.base_kv)
    }
}

/// Transmission line or transformer branch connecting two buses.
///
/// Modeled using the standard lumped $\Pi$-equivalent model:
/// - Series impedance: $Z_{\text{series}} = R_{\text{pu}} + j X_{\text{pu}}$
/// - Total shunt susceptance: $B_{\text{pu}}$ (half $B_{\text{pu}} / 2$ placed at each terminating bus)
/// - Off-nominal tap ratio $a$ and phase shift angle $\phi$ placed at `from_bus`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Branch {
    /// Unique branch identifier.
    pub id: usize,
    /// Human-readable label.
    pub name: Option<String>,
    /// Sending/from bus identifier.
    pub from_bus: usize,
    /// Receiving/to bus identifier.
    pub to_bus: usize,
    /// Series resistance in per-unit (p.u.).
    pub r_pu: f64,
    /// Series reactance in per-unit (p.u.). Must be non-zero.
    pub x_pu: f64,
    /// Total line charging shunt susceptance in per-unit (p.u.). Defaults to 0.0.
    pub b_pu: f64,
    /// Off-nominal transformer turns ratio $a$ (1.0 for regular transmission line).
    pub tap_ratio: f64,
    /// Phase shifter angle $\phi$ in radians (0.0 for regular transmission line).
    pub shift_rad: f64,
    /// Thermal continuous transmission rating in MVA.
    pub rating_mva: Option<f64>,
    /// Service status (`true` = active in service, `false` = disconnected).
    pub status: bool,
}

impl Branch {
    /// Creates a new transmission line branch with standard default parameters
    /// ($a = 1.0, \phi = 0.0\text{ rad}, B = 0.0\text{ p.u.}$, in-service).
    pub fn new_line(id: usize, from_bus: usize, to_bus: usize, r_pu: f64, x_pu: f64) -> Self {
        Self {
            id,
            name: None,
            from_bus,
            to_bus,
            r_pu,
            x_pu,
            b_pu: 0.0,
            tap_ratio: 1.0,
            shift_rad: 0.0,
            rating_mva: None,
            status: true,
        }
    }

    /// Creates a new transformer branch with off-nominal tap ratio and optional phase shift.
    pub fn new_transformer(
        id: usize,
        from_bus: usize,
        to_bus: usize,
        r_pu: f64,
        x_pu: f64,
        tap_ratio: f64,
        shift_rad: f64,
    ) -> Self {
        Self {
            id,
            name: None,
            from_bus,
            to_bus,
            r_pu,
            x_pu,
            b_pu: 0.0,
            tap_ratio,
            shift_rad,
            rating_mva: None,
            status: true,
        }
    }

    /// Sets human-readable name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets total shunt charging susceptance $B_{\text{pu}}$.
    pub fn with_b_pu(mut self, b_pu: f64) -> Self {
        self.b_pu = b_pu;
        self
    }

    /// Sets thermal rating in MVA.
    pub fn with_rating_mva(mut self, rating_mva: f64) -> Self {
        self.rating_mva = Some(rating_mva);
        self
    }

    /// Sets in-service status.
    pub fn with_status(mut self, status: bool) -> Self {
        self.status = status;
        self
    }

    /// Series impedance in complex per-unit: $Z = R + jX$.
    /// Returns $(R_{\text{pu}}, X_{\text{pu}})$.
    pub fn series_z_pu(&self) -> (f64, f64) {
        (self.r_pu, self.x_pu)
    }

    /// Series admittance in complex per-unit:
    ///
    /// $$Y = \frac{1}{R + jX} = \frac{R}{R^2 + X^2} - j \frac{X}{R^2 + X^2} = G + jB$$
    ///
    /// Returns $(G_{\text{series}}, B_{\text{series}})$.
    pub fn series_y_pu(&self) -> (f64, f64) {
        let denom = self.r_pu * self.r_pu + self.x_pu * self.x_pu;
        let g = self.r_pu / denom;
        let b = -self.x_pu / denom;
        (g, b)
    }
}

/// Synchronous or inverter-based generator injecting power at a bus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Generator {
    /// Unique generator identifier.
    pub id: usize,
    /// Human-readable label.
    pub name: Option<String>,
    /// Bus identifier where generator is connected.
    pub bus: usize,
    /// Scheduled active power output in megawatts (MW).
    pub p_mw: f64,
    /// Scheduled or solved reactive power output in megavars (MVar).
    pub q_mvar: f64,
    /// Voltage magnitude setpoint in per-unit (p.u.).
    pub vm_pu: f64,
    /// Minimum active power limit in MW.
    pub p_min_mw: f64,
    /// Maximum active power limit in MW.
    pub p_max_mw: f64,
    /// Minimum reactive power capability in MVar.
    pub q_min_mvar: f64,
    /// Maximum reactive power capability in MVar.
    pub q_max_mvar: f64,
    /// Service status (`true` = online, `false` = tripped/offline).
    pub status: bool,
}

impl Generator {
    /// Creates a new online Generator with nominal active output and voltage setpoint.
    pub fn new(id: usize, bus: usize, p_mw: f64, vm_pu: f64) -> Self {
        Self {
            id,
            name: None,
            bus,
            p_mw,
            q_mvar: 0.0,
            vm_pu,
            p_min_mw: 0.0,
            p_max_mw: p_mw * 1.5,
            q_min_mvar: -100.0,
            q_max_mvar: 100.0,
            status: true,
        }
    }

    /// Sets human-readable name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets reactive power limits in MVar.
    pub fn with_q_limits(mut self, q_min_mvar: f64, q_max_mvar: f64) -> Self {
        self.q_min_mvar = q_min_mvar;
        self.q_max_mvar = q_max_mvar;
        self
    }

    /// Sets active power limits in MW.
    pub fn with_p_limits(mut self, p_min_mw: f64, p_max_mw: f64) -> Self {
        self.p_min_mw = p_min_mw;
        self.p_max_mw = p_max_mw;
        self
    }

    /// Sets online status.
    pub fn with_status(mut self, status: bool) -> Self {
        self.status = status;
        self
    }

    /// Active power generation in per-unit: $P_{\text{pu}} = P_{\text{MW}} / S_{\text{base}}$.
    pub fn p_pu(&self, base_mva: f64) -> f64 {
        self.p_mw / base_mva
    }

    /// Reactive power generation in per-unit: $Q_{\text{pu}} = Q_{\text{MVar}} / S_{\text{base}}$.
    pub fn q_pu(&self, base_mva: f64) -> f64 {
        self.q_mvar / base_mva
    }
}

/// Constant power load consuming active and reactive power at a bus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Load {
    /// Unique load identifier.
    pub id: usize,
    /// Human-readable label.
    pub name: Option<String>,
    /// Bus identifier where load is connected.
    pub bus: usize,
    /// Active power demand in megawatts (MW).
    pub p_mw: f64,
    /// Reactive power demand in megavars (MVar).
    pub q_mvar: f64,
    /// Service status (`true` = connected, `false` = shed/disconnected).
    pub status: bool,
}

impl Load {
    /// Creates a new active Load with active (MW) and reactive (MVar) demand.
    pub fn new(id: usize, bus: usize, p_mw: f64, q_mvar: f64) -> Self {
        Self {
            id,
            name: None,
            bus,
            p_mw,
            q_mvar,
            status: true,
        }
    }

    /// Sets human-readable name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets connected status.
    pub fn with_status(mut self, status: bool) -> Self {
        self.status = status;
        self
    }

    /// Active power consumption in per-unit: $P_{\text{pu}} = P_{\text{MW}} / S_{\text{base}}$.
    pub fn p_pu(&self, base_mva: f64) -> f64 {
        self.p_mw / base_mva
    }

    /// Reactive power consumption in per-unit: $Q_{\text{pu}} = Q_{\text{MVar}} / S_{\text{base}}$.
    pub fn q_pu(&self, base_mva: f64) -> f64 {
        self.q_mvar / base_mva
    }
}

/// Errors that can occur when constructing or validating power network models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// Referenced bus ID was not found in network.
    BusNotFound(usize),
    /// Duplicate bus ID encountered.
    DuplicateBus(usize),
    /// Duplicate branch ID encountered.
    DuplicateBranch(usize),
    /// Branch connects a bus to itself (unsupported loop).
    SelfLoopBranch(usize),
    /// Base MVA is non-positive.
    InvalidBaseMva(String),
    /// Reactance ($X$) of a branch is zero or near-zero, which prevents admittance calculation.
    ZeroReactanceBranch(usize),
    /// Network does not contain exactly one Slack bus (required for standard power flow).
    InvalidSlackCount(usize),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BusNotFound(id) => write!(f, "Bus with id {id} not found in network"),
            Self::DuplicateBus(id) => write!(f, "Duplicate bus id {id} in network"),
            Self::DuplicateBranch(id) => write!(f, "Duplicate branch id {id} in network"),
            Self::SelfLoopBranch(id) => write!(f, "Branch id {id} connects a bus to itself"),
            Self::InvalidBaseMva(msg) => write!(f, "Invalid base MVA: {msg}"),
            Self::ZeroReactanceBranch(id) => {
                write!(f, "Branch id {id} has zero or near-zero series reactance")
            }
            Self::InvalidSlackCount(cnt) => {
                write!(f, "Network must have exactly 1 slack bus, found {cnt}")
            }
        }
    }
}

impl std::error::Error for ModelError {}

/// Comprehensive power system network model.
///
/// Contains all topological elements (buses, branches) and injections (generators, loads)
/// stored in deterministic ordered maps (`BTreeMap`) to ensure bit-for-bit identical iteration order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Network {
    /// System-wide base apparent power in MVA (defaults to 100.0 MVA).
    pub base_mva: f64,
    /// Buses keyed by unique bus id in strictly ascending order.
    pub buses: BTreeMap<usize, Bus>,
    /// Branches keyed by unique branch id in strictly ascending order.
    pub branches: BTreeMap<usize, Branch>,
    /// Generators keyed by unique generator id in strictly ascending order.
    pub generators: BTreeMap<usize, Generator>,
    /// Loads keyed by unique load id in strictly ascending order.
    pub loads: BTreeMap<usize, Load>,
}

impl Default for Network {
    fn default() -> Self {
        Self::new(DEFAULT_BASE_MVA)
    }
}

impl Network {
    /// Creates an empty network with the specified base MVA.
    pub fn new(base_mva: f64) -> Self {
        Self {
            base_mva,
            buses: BTreeMap::new(),
            branches: BTreeMap::new(),
            generators: BTreeMap::new(),
            loads: BTreeMap::new(),
        }
    }

    /// Adds a bus to the network.
    pub fn add_bus(&mut self, bus: Bus) -> Result<(), ModelError> {
        if self.buses.contains_key(&bus.id) {
            return Err(ModelError::DuplicateBus(bus.id));
        }
        self.buses.insert(bus.id, bus);
        Ok(())
    }

    /// Adds a branch (line or transformer) to the network.
    pub fn add_branch(&mut self, branch: Branch) -> Result<(), ModelError> {
        if self.branches.contains_key(&branch.id) {
            return Err(ModelError::DuplicateBranch(branch.id));
        }
        if branch.from_bus == branch.to_bus {
            return Err(ModelError::SelfLoopBranch(branch.id));
        }
        if branch.x_pu.abs() < 1e-12 {
            return Err(ModelError::ZeroReactanceBranch(branch.id));
        }
        self.branches.insert(branch.id, branch);
        Ok(())
    }

    /// Adds a generator to the network.
    pub fn add_generator(&mut self, gen: Generator) -> Result<(), ModelError> {
        self.generators.insert(gen.id, gen);
        Ok(())
    }

    /// Adds a load to the network.
    pub fn add_load(&mut self, load: Load) -> Result<(), ModelError> {
        self.loads.insert(load.id, load);
        Ok(())
    }

    /// Total number of buses in the network.
    pub fn bus_count(&self) -> usize {
        self.buses.len()
    }

    /// Total number of branches in the network.
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    /// Calculates net scheduled power injection $(P_{\text{inj, pu}}, Q_{\text{inj, pu}})$ at a specific bus.
    ///
    /// Defined as generation minus demand:
    /// $$P_{\text{inj}} = \sum P_{\text{gen}} - \sum P_{\text{load}}$$
    /// $$Q_{\text{inj}} = \sum Q_{\text{gen}} - \sum Q_{\text{load}}$$
    pub fn net_power_injection_pu(&self, bus_id: usize) -> Result<(f64, f64), ModelError> {
        if !self.buses.contains_key(&bus_id) {
            return Err(ModelError::BusNotFound(bus_id));
        }

        let mut p_inj_mw = 0.0;
        let mut q_inj_mvar = 0.0;

        for gen in self.generators.values() {
            if gen.status && gen.bus == bus_id {
                p_inj_mw += gen.p_mw;
                q_inj_mvar += gen.q_mvar;
            }
        }

        for load in self.loads.values() {
            if load.status && load.bus == bus_id {
                p_inj_mw -= load.p_mw;
                q_inj_mvar -= load.q_mvar;
            }
        }

        Ok((p_inj_mw / self.base_mva, q_inj_mvar / self.base_mva))
    }

    /// Validates network topology and consistency.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.base_mva <= 0.0 {
            return Err(ModelError::InvalidBaseMva(format!(
                "base_mva must be positive, found {}",
                self.base_mva
            )));
        }

        let mut slack_count = 0;
        for bus in self.buses.values() {
            if bus.bus_type == BusType::Slack {
                slack_count += 1;
            }
        }
        if slack_count != 1 {
            return Err(ModelError::InvalidSlackCount(slack_count));
        }

        for branch in self.branches.values() {
            if !self.buses.contains_key(&branch.from_bus) {
                return Err(ModelError::BusNotFound(branch.from_bus));
            }
            if !self.buses.contains_key(&branch.to_bus) {
                return Err(ModelError::BusNotFound(branch.to_bus));
            }
        }

        for gen in self.generators.values() {
            if !self.buses.contains_key(&gen.bus) {
                return Err(ModelError::BusNotFound(gen.bus));
            }
        }

        for load in self.loads.values() {
            if !self.buses.contains_key(&load.bus) {
                return Err(ModelError::BusNotFound(load.bus));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_per_unit_base_conversions() {
        // Given: S_base = 100 MVA, V_base = 138 kV
        // Hand calculation:
        // Z_base = (138)^2 / 100 = 19044 / 100 = 190.44 Ohm
        // I_base = 100 / (sqrt(3) * 138) = 100 / 239.02292777... = 0.4183699... kA = 418.37 A
        let bus = Bus::new(0, BusType::Slack, 138.0);
        let z_base = bus.base_impedance_ohms(100.0);
        assert!((z_base - 190.44).abs() < 1e-10);

        let i_base = bus.base_current_ka(100.0);
        let expected_i_base = 100.0 / (3.0_f64.sqrt() * 138.0);
        assert!((i_base - expected_i_base).abs() < 1e-12);
    }

    #[test]
    fn test_bus_angle_conversions() {
        let bus = Bus::new(1, BusType::PQ, 138.0).with_va_deg(-30.0);
        assert!((bus.va_deg() - (-30.0)).abs() < 1e-12);
        assert!((bus.va_rad - (-std::f64::consts::PI / 6.0)).abs() < 1e-12);
    }

    #[test]
    fn test_branch_series_admittance() {
        // Line with R = 0.02 pu, X = 0.06 pu
        // Denominator = 0.02^2 + 0.06^2 = 0.0004 + 0.0036 = 0.0040
        // G = 0.02 / 0.0040 = 5.0 pu
        // B = -0.06 / 0.0040 = -15.0 pu
        let branch = Branch::new_line(0, 0, 1, 0.02, 0.06);
        let (g, b) = branch.series_y_pu();
        assert!((g - 5.0).abs() < 1e-12);
        assert!((b - (-15.0)).abs() < 1e-12);
    }

    #[test]
    fn test_generator_and_load_per_unit_power() {
        let gen = Generator::new(0, 0, 50.0, 1.02);
        assert_eq!(gen.p_pu(100.0), 0.50);

        let load = Load::new(0, 1, 40.0, 20.0);
        assert_eq!(load.p_pu(100.0), 0.40);
        assert_eq!(load.q_pu(100.0), 0.20);
    }

    #[test]
    fn test_net_power_injection() {
        let mut net = Network::new(100.0);
        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0)).unwrap();

        // Add 50 MW gen and 10 MW load at Bus 1
        net.add_generator(Generator::new(0, 1, 50.0, 1.0)).unwrap();
        net.add_load(Load::new(0, 1, 10.0, 5.0)).unwrap();

        let (p_inj, q_inj) = net.net_power_injection_pu(1).unwrap();
        // Net P = 50 - 10 = 40 MW -> 0.40 pu
        // Net Q = 0 - 5 = -5 MVar -> -0.05 pu
        assert!((p_inj - 0.40).abs() < 1e-12);
        assert!((q_inj - (-0.05)).abs() < 1e-12);
    }

    #[test]
    fn test_canonical_3bus_network_construction() {
        let mut net = Network::new(100.0);

        // Buses
        net.add_bus(Bus::new(0, BusType::Slack, 138.0).with_name("Bus 0 - Slack"))
            .unwrap();
        net.add_bus(Bus::new(1, BusType::PQ, 138.0).with_name("Bus 1 - Load"))
            .unwrap();
        net.add_bus(
            Bus::new(2, BusType::PV, 138.0)
                .with_name("Bus 2 - Gen")
                .with_vm_pu(1.02),
        )
        .unwrap();

        // Branches
        net.add_branch(Branch::new_line(0, 0, 1, 0.02, 0.06).with_name("Line 0-1"))
            .unwrap();
        net.add_branch(Branch::new_line(1, 1, 2, 0.01, 0.03).with_name("Line 1-2"))
            .unwrap();
        net.add_branch(Branch::new_line(2, 0, 2, 0.012, 0.036).with_name("Line 0-2"))
            .unwrap();

        // Injections
        // Slack generator at Bus 0
        net.add_generator(Generator::new(0, 0, 0.0, 1.0)).unwrap();
        // Load at Bus 1: 40 MW, 20 MVar
        net.add_load(Load::new(0, 1, 40.0, 20.0)).unwrap();
        // Generator at Bus 2: 50 MW, V = 1.02 pu
        net.add_generator(Generator::new(1, 2, 50.0, 1.02)).unwrap();

        // Validation
        assert!(net.validate().is_ok());
        assert_eq!(net.bus_count(), 3);
        assert_eq!(net.branch_count(), 3);

        // Check net injections:
        // Bus 0: 0 MW gen (slack unallocated yet), 0 load -> 0.0 pu
        let (p0, q0) = net.net_power_injection_pu(0).unwrap();
        assert_eq!(p0, 0.0);
        assert_eq!(q0, 0.0);

        // Bus 1: -40 MW load -> -0.40 pu, -20 MVar load -> -0.20 pu
        let (p1, q1) = net.net_power_injection_pu(1).unwrap();
        assert!((p1 - (-0.40)).abs() < 1e-12);
        assert!((q1 - (-0.20)).abs() < 1e-12);

        // Bus 2: 50 MW gen -> +0.50 pu, 0 load -> 0.0 pu
        let (p2, q2) = net.net_power_injection_pu(2).unwrap();
        assert!((p2 - 0.50).abs() < 1e-12);
        assert_eq!(q2, 0.0);
    }

    #[test]
    fn test_offline_generator_and_load_ignored_in_injections() {
        let mut net = Network::new(100.0);
        net.add_bus(Bus::new(0, BusType::Slack, 138.0)).unwrap();

        let online_gen = Generator::new(0, 0, 30.0, 1.0);
        let mut offline_gen = Generator::new(1, 0, 50.0, 1.0);
        offline_gen.status = false;

        let online_load = Load::new(0, 0, 10.0, 5.0);
        let mut offline_load = Load::new(1, 0, 20.0, 10.0);
        offline_load.status = false;

        net.add_generator(online_gen).unwrap();
        net.add_generator(offline_gen).unwrap();
        net.add_load(online_load).unwrap();
        net.add_load(offline_load).unwrap();

        let (p_inj, q_inj) = net.net_power_injection_pu(0).unwrap();
        // Only online should count: P = 30 - 10 = 20 MW -> 0.20 pu, Q = 0 - 5 = -5 MVar -> -0.05 pu
        assert!((p_inj - 0.20).abs() < 1e-12);
        assert!((q_inj - (-0.05)).abs() < 1e-12);
    }

    #[test]
    fn test_network_validation_errors() {
        let mut net = Network::new(100.0);
        // Missing slack bus validation
        net.add_bus(Bus::new(0, BusType::PQ, 138.0)).unwrap();
        assert_eq!(net.validate(), Err(ModelError::InvalidSlackCount(0)));

        // Duplicate bus error
        assert_eq!(
            net.add_bus(Bus::new(0, BusType::Slack, 138.0)),
            Err(ModelError::DuplicateBus(0))
        );

        // Self-loop branch error
        assert_eq!(
            net.add_branch(Branch::new_line(0, 0, 0, 0.01, 0.05)),
            Err(ModelError::SelfLoopBranch(0))
        );

        // Zero reactance branch error
        assert_eq!(
            net.add_branch(Branch::new_line(0, 0, 1, 0.01, 0.0)),
            Err(ModelError::ZeroReactanceBranch(0))
        );
    }
}

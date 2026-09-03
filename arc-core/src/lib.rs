//! `arc-core`
//!
//! An open-source, deterministic power flow kernel for grid interconnection studies.
//! Pre-alpha / garage version (v0.1).

#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod admittance;
pub mod linear;
pub mod model;
pub mod parser;
pub mod regression;
pub mod solver;

pub use admittance::YBus;
pub use linear::{solve_dense_system, LinearSolverError};
pub use model::{
    Branch, Bus, BusType, Generator, Load, ModelError, Network, Shunt, DEFAULT_BASE_MVA,
};
pub use parser::{MatpowerParser, ParseError};
pub use regression::{CaseRegressionResult, RegressionHarness, RegressionReport};
pub use solver::{
    ACBranchFlow, ACBusResult, ACPowerFlow, ACPowerFlowOptions, ACPowerFlowResult, DCBranchFlow,
    DCBusResult, DCPowerFlow, DCPowerFlowResult, SolverError,
};

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_verification() {
        assert_eq!(2 + 2, 4);
    }
}

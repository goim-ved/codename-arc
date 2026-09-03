//! `arc-core`
//!
//! An open-source, deterministic power flow kernel for grid interconnection studies.
//! Pre-alpha / garage version (v0.1).

#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod admittance;
pub mod linear;
pub mod model;

pub use admittance::YBus;
pub use linear::{solve_dense_system, LinearSolverError};
pub use model::{Branch, Bus, BusType, Generator, Load, ModelError, Network, DEFAULT_BASE_MVA};

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_verification() {
        assert_eq!(2 + 2, 4);
    }
}

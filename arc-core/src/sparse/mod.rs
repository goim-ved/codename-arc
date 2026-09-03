//! Sparse linear algebra module for `arc`.

pub mod csr;
pub mod solver;

pub use csr::{CsrMatrix, SparseError, TripletList};
pub use solver::SparseLuSolver;

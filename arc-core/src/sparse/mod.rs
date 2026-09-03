//! Sparse linear algebra module for `arc`.

pub mod csr;

pub use csr::{CsrMatrix, SparseError, TripletList};

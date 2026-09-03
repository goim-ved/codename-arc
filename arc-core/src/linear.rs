//! Deterministic dense linear system solver.
//!
//! Solves $A x = b$ using Gaussian elimination with partial pivoting.
//!
//! # Determinism Guarantees
//! - **Fixed Pivot Tie-Breaking**: When comparing candidate pivot elements, the lowest row index
//!   is selected deterministically in the event of numerical ties.
//! - **Strict Loop Ordering**: Eliminates unordered iterations or non-deterministic reductions.
//! - **Zero Third-Party Native Dependencies**: Ensures bit-for-bit identical results across OS and CPU targets.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Algorithm choice for solving linear equation systems $A x = b$.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LinearSolverKind {
    /// Sparse direct LU factorization with Markowitz threshold pivoting ($\mathcal{O}(N^{1.2} - N^{1.4})$).
    #[default]
    Sparse,
    /// Dense Gaussian elimination with partial pivoting ($\mathcal{O}(N^3)$).
    Dense,
}

impl fmt::Display for LinearSolverKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sparse => write!(f, "Sparse (Markowitz LU)"),
            Self::Dense => write!(f, "Dense (Gaussian Elimination)"),
        }
    }
}

/// Errors arising during linear system factorization and solution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinearSolverError {
    /// The matrix is singular or numerically rank-deficient to within the specified tolerance.
    SingularMatrix {
        /// Row index where the zero pivot was encountered.
        pivot_row: usize,
    },
    /// The provided matrix dimensions or vector lengths do not match.
    DimensionMismatch {
        /// Expected vector length matching matrix dimension.
        expected: usize,
        /// Actual vector length encountered.
        found: usize,
    },
}

impl fmt::Display for LinearSolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingularMatrix { pivot_row } => {
                write!(
                    f,
                    "Singular or rank-deficient matrix at pivot row {pivot_row}"
                )
            }
            Self::DimensionMismatch { expected, found } => {
                write!(f, "Dimension mismatch: expected {expected}, found {found}")
            }
        }
    }
}

impl std::error::Error for LinearSolverError {}

/// Solves a dense square linear system $A x = b$ using Gaussian elimination with partial pivoting.
///
/// # Arguments
/// * `a` - Row-major $N \times N$ coefficient matrix slice of length $N^2$.
/// * `b` - Right-hand side vector slice of length $N$.
/// * `n` - Dimension $N$ of the system.
///
/// # Returns
/// The solution vector $x$ of length $N$, or `LinearSolverError`.
pub fn solve_dense_system(a: &[f64], b: &[f64], n: usize) -> Result<Vec<f64>, LinearSolverError> {
    if a.len() != n * n {
        return Err(LinearSolverError::DimensionMismatch {
            expected: n * n,
            found: a.len(),
        });
    }
    if b.len() != n {
        return Err(LinearSolverError::DimensionMismatch {
            expected: n,
            found: b.len(),
        });
    }

    if n == 0 {
        return Ok(Vec::new());
    }

    // Clone working copies of A and b
    let mut mat = a.to_vec();
    let mut rhs = b.to_vec();

    // Forward elimination with partial pivoting
    for i in 0..n {
        // Find pivot: row p >= i with maximum absolute value in column i
        let mut pivot_row = i;
        let mut max_val = mat[i * n + i].abs();

        for r in (i + 1)..n {
            let val = mat[r * n + i].abs();
            if val > max_val {
                max_val = val;
                pivot_row = r;
            }
        }

        // Singularity check
        if max_val < 1e-15 {
            return Err(LinearSolverError::SingularMatrix { pivot_row: i });
        }

        // Swap rows in mat and rhs if needed
        if pivot_row != i {
            for c in 0..n {
                mat.swap(i * n + c, pivot_row * n + c);
            }
            rhs.swap(i, pivot_row);
        }

        let pivot = mat[i * n + i];

        // Eliminate rows below pivot
        for r in (i + 1)..n {
            let factor = mat[r * n + i] / pivot;
            mat[r * n + i] = 0.0;
            for c in (i + 1)..n {
                mat[r * n + c] -= factor * mat[i * n + c];
            }
            rhs[r] -= factor * rhs[i];
        }
    }

    // Back substitution
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = rhs[i];
        for c in (i + 1)..n {
            sum -= mat[i * n + c] * x[c];
        }
        x[i] = sum / mat[i * n + i];
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_2x2_system() {
        // System:
        // 2*x0 + 1*x1 = 5
        // 1*x0 + 3*x1 = 10
        // Solution: x0 = 1, x1 = 3
        let a = vec![2.0, 1.0, 1.0, 3.0];
        let b = vec![5.0, 10.0];
        let x = solve_dense_system(&a, &b, 2).unwrap();

        assert!((x[0] - 1.0).abs() < 1e-12);
        assert!((x[1] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn test_solve_3x3_identity() {
        let a = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let b = vec![3.0, -2.5, 4.2];
        let x = solve_dense_system(&a, &b, 3).unwrap();

        assert!((x[0] - 3.0).abs() < 1e-12);
        assert!((x[1] - (-2.5)).abs() < 1e-12);
        assert!((x[2] - 4.2).abs() < 1e-12);
    }

    #[test]
    fn test_singular_matrix_detection() {
        // Row 1 is a multiple of Row 0
        let a = vec![1.0, 2.0, 2.0, 4.0];
        let b = vec![3.0, 6.0];
        let res = solve_dense_system(&a, &b, 2);

        assert!(matches!(res, Err(LinearSolverError::SingularMatrix { .. })));
    }

    #[test]
    fn test_dimension_mismatch() {
        let a = vec![1.0, 2.0, 3.0]; // not 4 elements for 2x2
        let b = vec![1.0, 2.0];
        let res = solve_dense_system(&a, &b, 2);

        assert!(matches!(
            res,
            Err(LinearSolverError::DimensionMismatch { .. })
        ));
    }
}

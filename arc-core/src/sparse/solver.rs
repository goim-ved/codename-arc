//! Sparse linear equation solver ($A x = b$) using Markowitz threshold pivoting.
//!
//! Implements direct sparse Gaussian elimination with dynamic Markowitz ordering
//! to minimize fill-in while enforcing numerical stability via threshold partial pivoting.
//! Tailored for sparse asymmetric circuit and power system matrices.

use crate::sparse::csr::{CsrMatrix, SparseError};
use std::collections::BTreeMap;

/// Sparse linear solver configuration and executor.
#[derive(Debug, Clone)]
pub struct SparseLuSolver {
    /// Markowitz threshold pivoting parameter $u \in (0, 1]$.
    /// A candidate pivot $a_{ij}$ must satisfy $|a_{ij}| \ge u \cdot \max_k |a_{kj}|$
    /// Default is 0.1 (KLU / SuperLU standard).
    pub threshold: f64,
    /// Absolute zero pivot tolerance.
    pub zero_tolerance: f64,
}

impl Default for SparseLuSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseLuSolver {
    /// Creates a new sparse LU solver with standard default parameters ($u = 0.1$).
    pub fn new() -> Self {
        Self {
            threshold: 0.1,
            zero_tolerance: 1e-15,
        }
    }

    /// Sets the Markowitz threshold parameter $u \in (0, 1]$.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        assert!(
            threshold > 0.0 && threshold <= 1.0,
            "Threshold parameter must be in (0, 1]"
        );
        self.threshold = threshold;
        self
    }

    /// Solves $A x = b$ for a square sparse matrix $A$ in CSR format and dense RHS vector $b$.
    pub fn solve(&self, a: &CsrMatrix, b: &[f64]) -> Result<Vec<f64>, SparseError> {
        let n = a.nrows;
        if a.ncols != n {
            return Err(SparseError::DimensionMismatch {
                expected: n,
                found: a.ncols,
            });
        }
        if b.len() != n {
            return Err(SparseError::DimensionMismatch {
                expected: n,
                found: b.len(),
            });
        }

        if n == 0 {
            return Ok(Vec::new());
        }

        // Represent active matrix as a dynamic sparse row map: row -> BTreeMap<col, val>
        let mut rows: Vec<BTreeMap<usize, f64>> = Vec::with_capacity(n);
        for r in 0..n {
            let start = a.row_ptrs[r];
            let end = a.row_ptrs[r + 1];
            let mut row_map = BTreeMap::new();
            for idx in start..end {
                let c = a.col_indices[idx];
                let v = a.values[idx];
                if v.abs() > self.zero_tolerance {
                    row_map.insert(c, v);
                }
            }
            rows.push(row_map);
        }

        let mut rhs = b.to_vec();

        // Permutation arrays and inverse column mapping for O(1) position lookup
        let mut row_perm: Vec<usize> = (0..n).collect();
        let mut col_perm: Vec<usize> = (0..n).collect();
        let mut col_inv_perm: Vec<usize> = (0..n).collect();

        // Elimination stage: for step k = 0 .. n-1
        for k in 0..n {
            // Find best pivot in the active submatrix (rows k..n, cols k..n)
            // using Markowitz threshold pivoting
            let (best_i, best_j) =
                self.select_pivot(&rows, &row_perm, &col_perm, &col_inv_perm, k, n)?;

            // Swap row k and best_i in row_perm
            row_perm.swap(k, best_i);

            // Swap col k and best_j in col_perm and update col_inv_perm
            let c_k = col_perm[k];
            let c_best = col_perm[best_j];
            col_perm.swap(k, best_j);
            col_inv_perm[c_k] = best_j;
            col_inv_perm[c_best] = k;

            let pivot_row_idx = row_perm[k];
            let pivot_col = col_perm[k];

            let pivot_val = *rows[pivot_row_idx].get(&pivot_col).ok_or_else(|| {
                SparseError::SingularMatrix(format!("Zero pivot encountered at step {k}"))
            })?;

            if pivot_val.abs() < self.zero_tolerance {
                return Err(SparseError::SingularMatrix(format!(
                    "Near-zero pivot |{pivot_val:.2e}| < {:.2e} at step {k}",
                    self.zero_tolerance
                )));
            }

            // Normalize pivot row
            let inv_pivot = 1.0 / pivot_val;
            rhs[pivot_row_idx] *= inv_pivot;
            for val in rows[pivot_row_idx].values_mut() {
                *val *= inv_pivot;
            }

            // Copy pivot row non-zeros for fast elimination of other active rows
            let pivot_entries: Vec<(usize, f64)> =
                rows[pivot_row_idx].iter().map(|(&c, &v)| (c, v)).collect();

            // Eliminate all other active rows i (k+1 .. n) that have a non-zero in pivot_col
            for &target_row_idx in row_perm.iter().take(n).skip(k + 1) {
                if let Some(&multiplier) = rows[target_row_idx].get(&pivot_col) {
                    if multiplier.abs() <= self.zero_tolerance {
                        rows[target_row_idx].remove(&pivot_col);
                        continue;
                    }

                    // Subtract multiplier * pivot_row from target_row
                    rhs[target_row_idx] -= multiplier * rhs[pivot_row_idx];

                    for &(pc, pv) in &pivot_entries {
                        let new_val = match rows[target_row_idx].get_mut(&pc) {
                            Some(target_val) => {
                                *target_val -= multiplier * pv;
                                *target_val
                            }
                            None => {
                                let val = -multiplier * pv;
                                rows[target_row_idx].insert(pc, val);
                                val
                            }
                        };
                        if new_val.abs() < self.zero_tolerance {
                            rows[target_row_idx].remove(&pc);
                        }
                    }
                }
            }
        }

        // Back-substitution stage
        let mut x = vec![0.0; n];
        for k in (0..n).rev() {
            let row_idx = row_perm[k];
            let col = col_perm[k];

            let mut sum = rhs[row_idx];
            for (&c, &v) in &rows[row_idx] {
                if c != col {
                    sum -= v * x[c];
                }
            }
            x[col] = sum;
        }

        Ok(x)
    }

    /// Selects the optimal pivot $(i, j)$ in active submatrix $k \dots n-1$
    /// minimizing Markowitz product $(r_i - 1)(c_j - 1)$ subject to threshold criterion.
    fn select_pivot(
        &self,
        rows: &[BTreeMap<usize, f64>],
        row_perm: &[usize],
        _col_perm: &[usize],
        col_inv_perm: &[usize],
        k: usize,
        n: usize,
    ) -> Result<(usize, usize), SparseError> {
        let mut col_max = vec![0.0; n];
        let mut col_counts = vec![0usize; n];

        for &r_idx in row_perm.iter().take(n).skip(k) {
            for (&c, &val) in &rows[r_idx] {
                if col_inv_perm[c] >= k {
                    let abs_val = val.abs();
                    if abs_val > col_max[c] {
                        col_max[c] = abs_val;
                    }
                    col_counts[c] += 1;
                }
            }
        }

        let mut best_markowitz = usize::MAX;
        let mut best_val_abs = 0.0_f64;
        let mut best_i = k;
        let mut best_j = k;
        let mut found = false;

        // Search for eligible pivot minimizing Markowitz count
        for (i, &r_idx) in row_perm.iter().enumerate().take(n).skip(k) {
            let active_row_count = rows[r_idx]
                .keys()
                .filter(|&&c| col_inv_perm[c] >= k)
                .count();

            if active_row_count == 0 {
                continue;
            }

            for (&c, &val) in &rows[r_idx] {
                if col_inv_perm[c] < k {
                    continue;
                }

                let abs_val = val.abs();
                let max_in_col = col_max[c];

                // Threshold criterion: |a_ij| >= u * max_k |a_kj|
                if abs_val >= self.threshold * max_in_col && abs_val > self.zero_tolerance {
                    let col_count = col_counts[c];
                    let markowitz = (active_row_count - 1) * (col_count - 1);

                    if markowitz < best_markowitz
                        || (markowitz == best_markowitz && abs_val > best_val_abs)
                    {
                        best_markowitz = markowitz;
                        best_val_abs = abs_val;
                        best_i = i;
                        best_j = col_inv_perm[c];
                        found = true;

                        // Zero fill-in shortcut: if markowitz == 0, this pivot produces 0 fill-in!
                        if best_markowitz == 0 {
                            return Ok((best_i, best_j));
                        }
                    }
                }
            }
        }

        if !found {
            return Err(SparseError::SingularMatrix(format!(
                "No non-zero pivot found in active submatrix at step {k}"
            )));
        }

        Ok((best_i, best_j))
    }
}

impl CsrMatrix {
    /// Solves $A x = b$ using direct sparse LU with Markowitz threshold pivoting.
    pub fn solve(&self, b: &[f64]) -> Result<Vec<f64>, SparseError> {
        SparseLuSolver::new().solve(self, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_lu_solve_2x2() {
        let dense = vec![vec![2.0, 1.0], vec![1.0, 3.0]];
        let csr = CsrMatrix::from_dense(&dense);
        let b = vec![5.0, 10.0];

        let x = csr.solve(&b).unwrap();
        // 2*1 + 1*3 = 5
        // 1*1 + 3*3 = 10
        assert!((x[0] - 1.0).abs() < 1e-12);
        assert!((x[1] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn test_sparse_lu_solve_3x3_with_zeros() {
        let dense = vec![
            vec![1.0, 0.0, 2.0],
            vec![0.0, 3.0, 0.0],
            vec![4.0, 0.0, 5.0],
        ];
        let csr = CsrMatrix::from_dense(&dense);
        let b = vec![5.0, 6.0, 14.0];

        let x = csr.solve(&b).unwrap();
        assert!((x[0] - 1.0).abs() < 1e-12);
        assert!((x[1] - 2.0).abs() < 1e-12);
        assert!((x[2] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn test_sparse_lu_matches_dense_solver() {
        use crate::linear::solve_dense_system;

        let dense = vec![
            vec![10.0, -2.0, -1.0, 0.0],
            vec![-2.0, 8.0, 0.0, -3.0],
            vec![-1.0, 0.0, 6.0, -1.0],
            vec![0.0, -3.0, -1.0, 7.0],
        ];
        let csr = CsrMatrix::from_dense(&dense);
        let b = vec![1.5, -2.0, 3.2, 0.5];

        let dense_flat: Vec<f64> = dense.iter().flatten().copied().collect();
        let x_dense = solve_dense_system(&dense_flat, &b, 4).unwrap();
        let x_sparse = csr.solve(&b).unwrap();

        for i in 0..4 {
            assert!(
                (x_dense[i] - x_sparse[i]).abs() < 1e-12,
                "Mismatch at {i}: dense={}, sparse={}",
                x_dense[i],
                x_sparse[i]
            );
        }
    }

    #[test]
    fn test_singular_sparse_matrix_detection() {
        let dense = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        let csr = CsrMatrix::from_dense(&dense);
        let b = vec![1.0, 2.0];
        assert!(csr.solve(&b).is_err());
    }
}

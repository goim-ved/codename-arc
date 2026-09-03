//! Compressed Sparse Row (CSR) matrix representation and operations.
//!
//! Provides coordinate (COO / Triplet) accumulation and conversion to sorted
//! Compressed Sparse Row (CSR) format with automatic summing of duplicate entries.

use std::fmt;

/// Errors arising during sparse matrix operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SparseError {
    /// Dimension mismatch between matrix and vector or between matrices.
    DimensionMismatch {
        /// Expected dimension.
        expected: usize,
        /// Actual dimension found.
        found: usize,
    },
    /// Matrix index out of bounds.
    IndexOutOfBounds {
        /// Row index.
        row: usize,
        /// Column index.
        col: usize,
        /// Matrix shape `(nrows, ncols)`.
        shape: (usize, usize),
    },
    /// Singular matrix encountered during linear solve.
    SingularMatrix(String),
}

impl fmt::Display for SparseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { expected, found } => {
                write!(f, "Dimension mismatch: expected {expected}, found {found}")
            }
            Self::IndexOutOfBounds { row, col, shape } => {
                write!(
                    f,
                    "Index ({row}, {col}) out of bounds for shape {}x{}",
                    shape.0, shape.1
                )
            }
            Self::SingularMatrix(msg) => write!(f, "Singular sparse matrix: {msg}"),
        }
    }
}

impl std::error::Error for SparseError {}

/// Coordinate list (COO / Triplet) for accumulating sparse entries in arbitrary order.
#[derive(Debug, Clone, Default)]
pub struct TripletList {
    triplets: Vec<(usize, usize, f64)>,
}

impl TripletList {
    /// Creates an empty triplet list.
    pub fn new() -> Self {
        Self {
            triplets: Vec::new(),
        }
    }

    /// Creates an empty triplet list with reserved capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            triplets: Vec::with_capacity(capacity),
        }
    }

    /// Adds a coordinate entry `(row, col, value)`. Duplicate coordinates are permitted
    /// and will be summed together upon conversion to CSR.
    pub fn add(&mut self, row: usize, col: usize, val: f64) {
        if val.abs() > 1e-18 {
            self.triplets.push((row, col, val));
        }
    }

    /// Number of accumulated triplets.
    pub fn len(&self) -> usize {
        self.triplets.len()
    }

    /// Returns true if the triplet list is empty.
    pub fn is_empty(&self) -> bool {
        self.triplets.is_empty()
    }

    /// Clears all accumulated triplets.
    pub fn clear(&mut self) {
        self.triplets.clear();
    }

    /// Returns a slice of the raw triplets.
    pub fn as_slice(&self) -> &[(usize, usize, f64)] {
        &self.triplets
    }
}

/// Compressed Sparse Row (CSR) matrix.
#[derive(Debug, Clone, PartialEq)]
pub struct CsrMatrix {
    /// Number of rows.
    pub nrows: usize,
    /// Number of columns.
    pub ncols: usize,
    /// Row pointer offsets of length `nrows + 1`.
    pub row_ptrs: Vec<usize>,
    /// Column indices of length `nnz`.
    pub col_indices: Vec<usize>,
    /// Non-zero values of length `nnz`.
    pub values: Vec<f64>,
}

impl CsrMatrix {
    /// Constructs a CSR matrix from a list of triplets.
    ///
    /// Duplicate entries with identical `(row, col)` indices are automatically summed together.
    /// Entries within each row are sorted in ascending order of column index.
    pub fn from_triplets(
        nrows: usize,
        ncols: usize,
        triplets: &[(usize, usize, f64)],
    ) -> Result<Self, SparseError> {
        // Validate bounds
        for &(r, c, _) in triplets {
            if r >= nrows || c >= ncols {
                return Err(SparseError::IndexOutOfBounds {
                    row: r,
                    col: c,
                    shape: (nrows, ncols),
                });
            }
        }

        // Group by row using temporary row-buckets
        let mut row_entries: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nrows];
        for &(r, c, v) in triplets {
            row_entries[r].push((c, v));
        }

        let mut row_ptrs = Vec::with_capacity(nrows + 1);
        let mut col_indices = Vec::new();
        let mut values = Vec::new();

        row_ptrs.push(0);

        for row in &mut row_entries {
            if row.is_empty() {
                row_ptrs.push(col_indices.len());
                continue;
            }

            // Sort row by column index
            row.sort_by_key(|&(c, _)| c);

            // Sum duplicates
            let mut curr_col = row[0].0;
            let mut curr_val = row[0].1;

            for &(c, v) in &row[1..] {
                if c == curr_col {
                    curr_val += v;
                } else {
                    if curr_val.abs() > 1e-18 {
                        col_indices.push(curr_col);
                        values.push(curr_val);
                    }
                    curr_col = c;
                    curr_val = v;
                }
            }

            // Push last accumulated entry in this row
            if curr_val.abs() > 1e-18 {
                col_indices.push(curr_col);
                values.push(curr_val);
            }

            row_ptrs.push(col_indices.len());
        }

        Ok(Self {
            nrows,
            ncols,
            row_ptrs,
            col_indices,
            values,
        })
    }

    /// Constructs a CSR matrix from a dense 2D slice.
    pub fn from_dense(dense: &[Vec<f64>]) -> Self {
        let nrows = dense.len();
        let ncols = if nrows > 0 { dense[0].len() } else { 0 };

        let mut row_ptrs = Vec::with_capacity(nrows + 1);
        let mut col_indices = Vec::new();
        let mut values = Vec::new();

        row_ptrs.push(0);

        for row in dense {
            for (c, &val) in row.iter().enumerate() {
                if val.abs() > 1e-18 {
                    col_indices.push(c);
                    values.push(val);
                }
            }
            row_ptrs.push(col_indices.len());
        }

        Self {
            nrows,
            ncols,
            row_ptrs,
            col_indices,
            values,
        }
    }

    /// Converts the CSR matrix to a dense 2D vector.
    pub fn to_dense(&self) -> Vec<Vec<f64>> {
        let mut dense = vec![vec![0.0; self.ncols]; self.nrows];
        for (r, row) in dense.iter_mut().enumerate().take(self.nrows) {
            let start = self.row_ptrs[r];
            let end = self.row_ptrs[r + 1];
            for idx in start..end {
                row[self.col_indices[idx]] = self.values[idx];
            }
        }
        dense
    }

    /// Retrieves an entry at `(row, col)`. Returns 0.0 if the entry is structural zero.
    pub fn get(&self, row: usize, col: usize) -> f64 {
        if row >= self.nrows || col >= self.ncols {
            return 0.0;
        }
        let start = self.row_ptrs[row];
        let end = self.row_ptrs[row + 1];
        let cols = &self.col_indices[start..end];

        match cols.binary_search(&col) {
            Ok(idx) => self.values[start + idx],
            Err(_) => 0.0,
        }
    }

    /// Multiplies the sparse matrix with a dense vector: $y = A x$.
    pub fn matvec(&self, x: &[f64]) -> Result<Vec<f64>, SparseError> {
        if x.len() != self.ncols {
            return Err(SparseError::DimensionMismatch {
                expected: self.ncols,
                found: x.len(),
            });
        }

        let mut y = vec![0.0; self.nrows];
        for (r, y_val) in y.iter_mut().enumerate().take(self.nrows) {
            let start = self.row_ptrs[r];
            let end = self.row_ptrs[r + 1];
            let mut sum = 0.0;
            for idx in start..end {
                sum += self.values[idx] * x[self.col_indices[idx]];
            }
            *y_val = sum;
        }

        Ok(y)
    }

    /// Total number of stored non-zero elements.
    pub fn non_zeros(&self) -> usize {
        self.values.len()
    }

    /// Density of non-zeros: $\text{nnz} / (m \cdot n)$.
    pub fn density(&self) -> f64 {
        let total = (self.nrows * self.ncols) as f64;
        if total > 0.0 {
            self.non_zeros() as f64 / total
        } else {
            0.0
        }
    }

    /// Sparsity percentage: $1.0 - \text{density}$.
    pub fn sparsity(&self) -> f64 {
        1.0 - self.density()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triplet_to_csr_and_duplicate_summing() {
        let mut trips = TripletList::new();
        // Row 0
        trips.add(0, 0, 1.0);
        trips.add(0, 2, 4.0);
        // Duplicate entry at (0, 0): should sum to 1.0 + 2.5 = 3.5
        trips.add(0, 0, 2.5);
        // Row 1
        trips.add(1, 1, 5.0);
        // Row 2
        trips.add(2, 0, 0.5);
        trips.add(2, 2, 3.0);

        let csr = CsrMatrix::from_triplets(3, 3, trips.as_slice()).unwrap();
        assert_eq!(csr.nrows, 3);
        assert_eq!(csr.ncols, 3);
        assert_eq!(csr.non_zeros(), 5);

        // Check values
        assert_eq!(csr.get(0, 0), 3.5);
        assert_eq!(csr.get(0, 1), 0.0);
        assert_eq!(csr.get(0, 2), 4.0);
        assert_eq!(csr.get(1, 1), 5.0);
        assert_eq!(csr.get(2, 0), 0.5);
        assert_eq!(csr.get(2, 2), 3.0);

        // Dense conversion
        let dense = csr.to_dense();
        assert_eq!(dense[0][0], 3.5);
        assert_eq!(dense[0][2], 4.0);
        assert_eq!(dense[1][1], 5.0);
        assert_eq!(dense[2][0], 0.5);
        assert_eq!(dense[2][2], 3.0);
    }

    #[test]
    fn test_matvec_multiplication() {
        let dense = vec![
            vec![2.0, 0.0, 1.0],
            vec![0.0, 3.0, 0.0],
            vec![4.0, 0.0, 5.0],
        ];
        let csr = CsrMatrix::from_dense(&dense);
        let x = vec![1.0, 2.0, 3.0];
        let y = csr.matvec(&x).unwrap();

        // 2*1 + 0*2 + 1*3 = 5
        // 0*1 + 3*2 + 0*3 = 6
        // 4*1 + 0*2 + 5*3 = 19
        assert_eq!(y, vec![5.0, 6.0, 19.0]);
    }

    #[test]
    fn test_sparsity_metrics() {
        let dense = vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 1.0],
        ];
        let csr = CsrMatrix::from_dense(&dense);
        assert_eq!(csr.non_zeros(), 4);
        assert_eq!(csr.density(), 4.0 / 16.0);
        assert_eq!(csr.sparsity(), 0.75);
    }
}

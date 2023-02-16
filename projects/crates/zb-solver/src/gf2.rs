//! GF(2) bitset linear solver for fixed-skeleton CRC / byte equations.

use crate::cancel::CancelToken;
use zb_types::{ErrorCategory, ErrorCode, ZbError, ZbResult};

/// Dense GF(2) augmented matrix: each row packs `cols` variable bits + 1 RHS bit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitMatrix {
    /// Equation count.
    pub rows: usize,
    /// Variable count.
    pub cols: usize,
    /// Row-major packed words. Each row uses [`Self::words_per_row`] words.
    data: Vec<u64>,
}

impl BitMatrix {
    /// Words needed for `cols` variables plus one RHS bit.
    pub fn words_per_row(cols: usize) -> usize {
        (cols + 1).div_ceil(64)
    }

    /// Zero matrix.
    pub fn zeros(rows: usize, cols: usize) -> Self {
        let wpr = Self::words_per_row(cols);
        Self { rows, cols, data: vec![0; rows.saturating_mul(wpr)] }
    }

    fn wpr(&self) -> usize {
        Self::words_per_row(self.cols)
    }

    fn bit_pos(col: usize) -> (usize, u64) {
        (col / 64, 1u64 << (col % 64))
    }

    /// Set bit `col` in `row` (use `col == cols` for RHS).
    pub fn set(&mut self, row: usize, col: usize) {
        let wpr = self.wpr();
        let (word, mask) = Self::bit_pos(col);
        self.data[row * wpr + word] |= mask;
    }

    /// Read bit `col` in `row`.
    pub fn get(&self, row: usize, col: usize) -> bool {
        let wpr = self.wpr();
        let (word, mask) = Self::bit_pos(col);
        self.data[row * wpr + word] & mask != 0
    }

    fn swap_rows(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        let wpr = self.wpr();
        for w in 0..wpr {
            let i = a * wpr + w;
            let j = b * wpr + w;
            self.data.swap(i, j);
        }
    }

    fn xor_row_into(&mut self, dst: usize, src: usize) {
        if dst == src {
            return;
        }
        let wpr = self.wpr();
        for w in 0..wpr {
            let bit = self.data[src * wpr + w];
            self.data[dst * wpr + w] ^= bit;
        }
    }
}

/// Solution under free-variable = 0 policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gf2Solution {
    /// Variable assignment.
    pub assignment: Vec<bool>,
    /// Rank after elimination.
    pub rank: usize,
    /// True when `rank == cols`.
    pub unique: bool,
}

/// Gaussian elimination on an augmented GF(2) system.
pub fn solve(matrix: BitMatrix) -> ZbResult<Gf2Solution> {
    solve_with_cancel(matrix, &CancelToken::new())
}

/// Gaussian elimination with cooperative cancel checks on each pivot column.
pub fn solve_with_cancel(mut matrix: BitMatrix, cancel: &CancelToken) -> ZbResult<Gf2Solution> {
    let rows = matrix.rows;
    let cols = matrix.cols;
    let mut pivot_of_col = vec![None; cols];
    let mut origin: Vec<usize> = (0..rows).collect();
    let mut row = 0usize;

    cancel.check()?;
    for col in 0..cols {
        cancel.check()?;
        if row >= rows {
            break;
        }
        let mut pivot = None;
        for r in row..rows {
            if matrix.get(r, col) {
                pivot = Some(r);
                break;
            }
        }
        let Some(piv) = pivot
        else {
            continue;
        };
        matrix.swap_rows(row, piv);
        origin.swap(row, piv);
        for r in 0..rows {
            if r != row && matrix.get(r, col) {
                matrix.xor_row_into(r, row);
            }
        }
        pivot_of_col[col] = Some(row);
        row += 1;
    }

    cancel.check()?;
    for r in 0..rows {
        let mut zero_vars = true;
        for c in 0..cols {
            if matrix.get(r, c) {
                zero_vars = false;
                break;
            }
        }
        if zero_vars && matrix.get(r, cols) {
            return Err(ZbError {
                category: ErrorCategory::ConstraintUnsatisfied,
                code: ErrorCode::Internal,
                message: format!("gf2_inconsistent_row={} GF(2) system is inconsistent (0 = 1)", origin[r]),
            });
        }
    }

    let mut assignment = vec![false; cols];
    let mut rank = 0usize;
    for (col, piv) in pivot_of_col.iter().enumerate() {
        if let Some(r) = *piv {
            assignment[col] = matrix.get(r, cols);
            rank += 1;
        }
    }

    Ok(Gf2Solution { assignment, rank, unique: rank == cols })
}

/// Build and solve from boolean coefficient rows plus RHS.
pub fn solve_rows(coeffs: &[Vec<bool>], rhs: &[bool]) -> ZbResult<Gf2Solution> {
    solve_rows_with_cancel(coeffs, rhs, &CancelToken::new())
}

/// [`solve_rows`] with cooperative cancel checks during elimination.
pub fn solve_rows_with_cancel(coeffs: &[Vec<bool>], rhs: &[bool], cancel: &CancelToken) -> ZbResult<Gf2Solution> {
    cancel.check()?;
    if coeffs.len() != rhs.len() {
        return Err(ZbError::new(ErrorCode::Internal, "GF(2) coeff row count does not match rhs length"));
    }
    let cols = coeffs.first().map(Vec::len).unwrap_or(0);
    if coeffs.iter().any(|row| row.len() != cols) {
        return Err(ZbError::new(ErrorCode::Internal, "GF(2) coeff rows have unequal widths"));
    }
    let mut matrix = BitMatrix::zeros(coeffs.len(), cols);
    for (r, row) in coeffs.iter().enumerate() {
        cancel.check()?;
        for (c, bit) in row.iter().enumerate() {
            if *bit {
                matrix.set(r, c);
            }
        }
        if rhs[r] {
            matrix.set(r, cols);
        }
    }
    solve_with_cancel(matrix, cancel)
}

//! Fixed-skeleton constraint solver for zb recursive ZIP construction.
//!
//! Living `08`: owns GF(2) elimination, CRC affine helpers, and the
//! [`ConstraintProblem`] contract. Does not emit archives, parse DEFLATE, or
//! call Acorn.

#![deny(missing_docs)]

mod cancel;
mod crc;
mod gf2;
mod problem;

pub use cancel::{never as never_cancel, CancelToken};
pub use crc::{
    crc32, crc32_affine_columns, crc32_shared_columns, u32_to_bits, CrcByteSlot, CRC32_CHECK,
    CRC32_POLY,
};
pub use gf2::{solve, solve_rows, solve_rows_with_cancel, solve_with_cancel, BitMatrix, Gf2Solution};
pub use problem::{
    crc_identity_residual_problem, solve_crc_identity_blocks, solve_crc_identity_blocks_with_cancel,
    solve_problem, solve_problem_with_cancel, ByteEquation, ByteExpr, ByteTerm, ChecksumAlgorithm,
    ChecksumConstraint, ConstraintProblem,
    LinearEquation, SolveEvidence,
    SolveOutcome, SolverBudget,
};

/// Stable variable identity inside a frozen skeleton problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariableId(pub u32);

/// Stable equation identity for evidence / conflict reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EquationId(pub u32);

/// Canonical bit assignment after free variables are forced to zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    /// Bit values in stable variable order.
    pub bits: Vec<bool>,
}

impl From<Gf2Solution> for Assignment {
    fn from(value: Gf2Solution) -> Self {
        Self {
            bits: value.assignment,
        }
    }
}

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

pub use cancel::{CancelToken, never as never_cancel};
pub use crc::{CRC32_CHECK, CRC32_POLY, CrcByteSlot, crc32, crc32_affine_columns, crc32_shared_columns, u32_to_bits};
pub use gf2::{BitMatrix, Gf2Solution, solve, solve_rows, solve_rows_with_cancel, solve_with_cancel};
pub use problem::{
    ByteEquation, ByteExpr, ByteTerm, ChecksumAlgorithm, ChecksumConstraint, ConstraintProblem, LinearEquation,
    SharedCrcArchive, SolveEvidence, SolveOutcome, SolverBudget, checksum_from_shared_slots, crc_identity_residual_problem,
    shared_crc_problem, solve_crc_identity_blocks, solve_crc_identity_blocks_with_cancel, solve_problem,
    solve_problem_with_cancel, solve_shared_crc_with_cancel,
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
        Self { bits: value.assignment }
    }
}

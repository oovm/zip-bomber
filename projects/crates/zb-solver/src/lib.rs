//! Fixed-skeleton constraint solver for zb recursive ZIP construction.
//!
//! Living `08`: owns GF(2) elimination and CRC affine helpers. Does not emit
//! archives, parse DEFLATE, or call Acorn.

#![deny(missing_docs)]

mod crc;
mod gf2;

pub use crc::{
    crc32, crc32_affine_columns, crc32_shared_columns, u32_to_bits, CrcByteSlot, CRC32_CHECK,
    CRC32_POLY,
};
pub use gf2::{solve, solve_rows, BitMatrix, Gf2Solution};

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

/// Solver-facing outcome (Living `08`). Artifact emit stays in `zb-generate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveOutcome {
    /// Consistent solution under the free-variable = 0 policy.
    Sat {
        /// Canonical assignment.
        assignment: Assignment,
        /// Matrix rank.
        rank: usize,
        /// Whether the solution is unique.
        unique: bool,
    },
    /// Current fixed skeleton is unsatisfiable.
    Unsat {
        /// Human-readable conflict note.
        message: String,
    },
}

impl SolveOutcome {
    /// Wrap a successful [`Gf2Solution`].
    pub fn from_gf2(sol: Gf2Solution) -> Self {
        Self::Sat {
            unique: sol.unique,
            rank: sol.rank,
            assignment: Assignment::from(sol),
        }
    }
}

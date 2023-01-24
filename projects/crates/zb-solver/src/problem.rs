//! Fixed-skeleton [`ConstraintProblem`] surface (Living `08`).
//!
//! Layout / codec stay in `zb-generate`. This module only validates affine bit
//! equations, enforces solver budgets, and delegates elimination to [`crate::gf2`].

use crate::gf2::{solve, BitMatrix, Gf2Solution};
use crate::{Assignment, EquationId, VariableId};
use zb_types::{ErrorCategory, ErrorCode, ZbError, ZbResult};

/// Per-problem resource ceilings checked before matrix construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolverBudget {
    /// Maximum boolean variable slots.
    pub max_variables: u32,
    /// Maximum linear equations.
    pub max_equations: u32,
    /// Maximum `rows * (cols + 1)` bit cells in the dense matrix.
    pub max_matrix_bits: u64,
}

impl Default for SolverBudget {
    fn default() -> Self {
        Self {
            max_variables: 4096,
            max_equations: 8192,
            max_matrix_bits: 1 << 24,
        }
    }
}

/// One GF(2) XOR equation over named bit variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearEquation {
    /// Stable identity for conflict / evidence reports.
    pub id: EquationId,
    /// Variable indices participating in the XOR (duplicates cancel in pairs).
    pub variables: Vec<VariableId>,
    /// Right-hand side bit.
    pub rhs: bool,
}

/// Frozen affine bit problem handed to the solver after layout is fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintProblem {
    /// Number of boolean variables in stable index order `0..variable_count`.
    pub variable_count: u32,
    /// Ordered linear equations.
    pub equations: Vec<LinearEquation>,
    /// Resource ceilings for this solve.
    pub budget: SolverBudget,
    /// Optional opaque digest of the frozen skeleton (evidence only).
    pub problem_digest: Option<String>,
}

/// Compact evidence returned with a successful solve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveEvidence {
    /// Matrix rank after elimination.
    pub rank: usize,
    /// Free bits under the free-variable = 0 policy (`variable_count - rank`).
    pub free_bits: usize,
    /// Equation count accepted into the matrix.
    pub equations_used: usize,
    /// Echo of the optional problem digest.
    pub problem_digest: Option<String>,
}

/// Solver-facing outcome (Living `08`). Artifact emit stays in `zb-generate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveOutcome {
    /// Consistent solution under the free-variable = 0 policy.
    Sat {
        /// Canonical assignment.
        assignment: Assignment,
        /// Whether the solution is unique (`free_bits == 0`).
        unique: bool,
        /// Elimination evidence.
        evidence: SolveEvidence,
    },
    /// Current fixed skeleton is unsatisfiable.
    Unsat {
        /// Human-readable conflict note.
        message: String,
        /// Equation id when a specific row caused the conflict, if known.
        conflicting_equation: Option<EquationId>,
    },
    /// Budget ceiling would be exceeded.
    ResourceLimited {
        /// Which budget field blocked the solve.
        message: String,
    },
    /// Caller cancelled before completion (reserved for future cancel tokens).
    Cancelled {
        /// Cancellation note.
        message: String,
    },
    /// Ill-formed problem (out-of-range variable, empty identity, …).
    InvalidProblem {
        /// Validation failure.
        message: String,
    },
    /// Unexpected internal failure.
    InternalError {
        /// Diagnostic note.
        message: String,
    },
}

impl SolveOutcome {
    /// Wrap a successful [`Gf2Solution`] with evidence metadata.
    pub fn from_gf2(sol: Gf2Solution, evidence: SolveEvidence) -> Self {
        Self::Sat {
            unique: sol.unique,
            assignment: Assignment::from(sol),
            evidence,
        }
    }
}

impl ConstraintProblem {
    /// Validate identities, ranges, and budgets without solving.
    pub fn validate(&self) -> ZbResult<()> {
        if self.variable_count > self.budget.max_variables {
            return Err(ZbError::new(
                ErrorCode::ResourceLimited,
                format!(
                    "variable_count {} exceeds budget.max_variables {}",
                    self.variable_count, self.budget.max_variables
                ),
            ));
        }
        if self.equations.len() as u32 > self.budget.max_equations {
            return Err(ZbError::new(
                ErrorCode::ResourceLimited,
                format!(
                    "equation count {} exceeds budget.max_equations {}",
                    self.equations.len(),
                    self.budget.max_equations
                ),
            ));
        }
        let cols = self.variable_count as u64;
        let rows = self.equations.len() as u64;
        let cells = rows.saturating_mul(cols.saturating_add(1));
        if cells > self.budget.max_matrix_bits {
            return Err(ZbError::new(
                ErrorCode::ResourceLimited,
                format!(
                    "matrix cells {cells} exceed budget.max_matrix_bits {}",
                    self.budget.max_matrix_bits
                ),
            ));
        }
        for eq in &self.equations {
            for var in &eq.variables {
                if var.0 >= self.variable_count {
                    return Err(ZbError::new(
                        ErrorCode::InvalidTopology,
                        format!(
                            "equation {:?} references variable {} outside 0..{}",
                            eq.id, var.0, self.variable_count
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Build the stacked CRC residual system `(I XOR M) x = b`.
///
/// Each block contributes 32 equations. `columns[j]` bit `i` is `M[i][j]` for
/// that block's rows. Identity is XORed onto columns
/// `[block_index * 32 .. block_index * 32 + 32)` so each block owns its CRC
/// word. `variable_bits` is the shared column width (32 for Quine, 64 for
/// two-node Loopy).
pub fn crc_identity_residual_problem(
    blocks: &[(u32, &[u32])],
    variable_bits: u32,
    budget: SolverBudget,
    problem_digest: Option<String>,
) -> Result<ConstraintProblem, SolveOutcome> {
    if variable_bits == 0 {
        return Err(SolveOutcome::InvalidProblem {
            message: "variable_bits must be positive".into(),
        });
    }
    let mut equations = Vec::with_capacity(blocks.len().saturating_mul(32));
    let mut eq_id = 0u32;
    for (block_index, (constant, columns)) in blocks.iter().enumerate() {
        if columns.len() != variable_bits as usize {
            return Err(SolveOutcome::InvalidProblem {
                message: format!(
                    "CRC block {block_index} has {} columns, expected {variable_bits}",
                    columns.len()
                ),
            });
        }
        let col_base = block_index * 32;
        if col_base + 32 > variable_bits as usize {
            return Err(SolveOutcome::InvalidProblem {
                message: format!(
                    "CRC block {block_index} identity cols [{col_base}..{}) exceed variable_bits {variable_bits}",
                    col_base + 32
                ),
            });
        }
        for row in 0..32 {
            let mut vars = Vec::new();
            for bit in 0..variable_bits as usize {
                if (columns[bit] >> row) & 1 != 0 {
                    vars.push(VariableId(bit as u32));
                }
            }
            // Identity column for this CRC bit.
            let identity = VariableId((col_base + row) as u32);
            if let Some(pos) = vars.iter().position(|v| *v == identity) {
                vars.remove(pos);
            } else {
                vars.push(identity);
            }
            equations.push(LinearEquation {
                id: EquationId(eq_id),
                variables: vars,
                rhs: (constant >> row) & 1 != 0,
            });
            eq_id += 1;
        }
    }
    Ok(ConstraintProblem {
        variable_count: variable_bits,
        equations,
        budget,
        problem_digest,
    })
}

/// Convenience: build [`crc_identity_residual_problem`] then [`solve_problem`].
pub fn solve_crc_identity_blocks(
    blocks: &[(u32, &[u32])],
    variable_bits: u32,
    budget: SolverBudget,
    problem_digest: Option<String>,
) -> SolveOutcome {
    match crc_identity_residual_problem(blocks, variable_bits, budget, problem_digest) {
        Ok(problem) => solve_problem(&problem),
        Err(outcome) => outcome,
    }
}

/// Solve a validated [`ConstraintProblem`].
///
/// Returns a structured [`SolveOutcome`] instead of mapping every failure through
/// [`ZbResult`], so generators can branch on `unsat` vs `resource_limited`
/// without losing equation identity.
pub fn solve_problem(problem: &ConstraintProblem) -> SolveOutcome {
    if let Err(err) = problem.validate() {
        return match err.category {
            ErrorCategory::ResourceLimited => SolveOutcome::ResourceLimited {
                message: err.message,
            },
            ErrorCategory::InvalidRequest => SolveOutcome::InvalidProblem {
                message: err.message,
            },
            _ => SolveOutcome::InternalError {
                message: err.message,
            },
        };
    }

    let cols = problem.variable_count as usize;
    let rows = problem.equations.len();
    let mut matrix = BitMatrix::zeros(rows, cols);
    for (row, eq) in problem.equations.iter().enumerate() {
        // XOR duplicate variable indices so they cancel over GF(2).
        let mut bits = vec![false; cols];
        for var in &eq.variables {
            let idx = var.0 as usize;
            bits[idx] = !bits[idx];
        }
        for (col, bit) in bits.iter().enumerate() {
            if *bit {
                matrix.set(row, col);
            }
        }
        if eq.rhs {
            matrix.set(row, cols);
        }
    }

    match solve(matrix) {
        Ok(sol) => {
            let evidence = SolveEvidence {
                rank: sol.rank,
                free_bits: cols.saturating_sub(sol.rank),
                equations_used: rows,
                problem_digest: problem.problem_digest.clone(),
            };
            SolveOutcome::from_gf2(sol, evidence)
        }
        Err(err) => {
            if err.category == zb_types::ErrorCategory::ConstraintUnsatisfied {
                SolveOutcome::Unsat {
                    message: err.message,
                    conflicting_equation: None,
                }
            } else {
                SolveOutcome::InternalError {
                    message: err.message,
                }
            }
        }
    }
}

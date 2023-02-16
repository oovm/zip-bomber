//! Fixed-skeleton [`ConstraintProblem`] surface (Living `08`).
//!
//! Layout / codec stay in `zb-generate`. This module only validates affine bit
//! equations, enforces solver budgets, and delegates elimination to [`crate::gf2`].

use crate::{
    Assignment, EquationId, VariableId,
    cancel::CancelToken,
    crc::{CrcByteSlot, crc32, crc32_shared_columns},
    gf2::{BitMatrix, Gf2Solution, solve_with_cancel},
};
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
        Self { max_variables: 4096, max_equations: 8192, max_matrix_bits: 1 << 24 }
    }
}

/// One byte of a frozen layout: a constant or a shared byte variable.
///
/// [`ByteTerm::Variable`] indexes a byte, not a bit. Its eight bits occupy
/// `index * 8 .. index * 8 + 8` inside [`ConstraintProblem::variable_count`],
/// least-significant bit first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteTerm {
    /// Fixed byte.
    Constant(u8),
    /// Shared byte variable.
    Variable(u32),
}

/// Equality of two byte terms. Reduced before GF(2) elimination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteEquation {
    /// Stable identity reported on conflict and back-substitution failure.
    pub id: EquationId,
    /// Left side.
    pub left: ByteTerm,
    /// Right side.
    pub right: ByteTerm,
}

/// Registered checksum. Only IEEE CRC-32 is affine in this solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    /// Reflected IEEE CRC-32, init/final XOR `0xffffffff`.
    Crc32Ieee,
}

/// Affine byte: a constant, a shared byte variable, or their XOR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteExpr {
    /// Fixed byte.
    Constant(u8),
    /// Byte variable. Bits occupy `index * 8 .. index * 8 + 8`, least-significant bit first.
    Variable(u32),
    /// XOR of nested affine bytes. Repeated variable indices cancel in pairs.
    Xor(Vec<ByteExpr>),
}

/// IEEE CRC-32 of an affine message equals a 4-byte little-endian affine target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumConstraint {
    /// Stable identity for every residual row and for back-substitution failure.
    pub id: EquationId,
    /// Checksum algorithm. Unknown algorithms are not representable.
    pub algorithm: ChecksumAlgorithm,
    /// Message bytes in order.
    pub message: Vec<ByteExpr>,
    /// Little-endian CRC-32 target.
    pub target: [ByteExpr; 4],
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
    /// Byte equalities reduced before the bit matrix is built.
    pub byte_equations: Vec<ByteEquation>,
    /// Affine checksum constraints expanded into 32 GF(2) rows each.
    pub checksums: Vec<ChecksumConstraint>,
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
    /// Caller cancelled at a cooperative checkpoint.
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
        Self::Sat { unique: sol.unique, assignment: Assignment::from(sol), evidence }
    }
}

impl ConstraintProblem {
    /// Validate identities, ranges, and budgets without solving.
    pub fn validate(&self) -> ZbResult<()> {
        if self.variable_count > self.budget.max_variables {
            return Err(ZbError::new(
                ErrorCode::ResourceLimited,
                format!("variable_count {} exceeds budget.max_variables {}", self.variable_count, self.budget.max_variables),
            ));
        }
        if self.equations.len() as u32 > self.budget.max_equations {
            return Err(ZbError::new(
                ErrorCode::ResourceLimited,
                format!("equation count {} exceeds budget.max_equations {}", self.equations.len(), self.budget.max_equations),
            ));
        }
        let cols = self.variable_count as u64;
        let rows = self.equations.len() as u64;
        let cells = rows.saturating_mul(cols.saturating_add(1));
        if cells > self.budget.max_matrix_bits {
            return Err(ZbError::new(
                ErrorCode::ResourceLimited,
                format!("matrix cells {cells} exceed budget.max_matrix_bits {}", self.budget.max_matrix_bits),
            ));
        }
        for eq in &self.equations {
            for var in &eq.variables {
                if var.0 >= self.variable_count {
                    return Err(ZbError::new(
                        ErrorCode::InvalidTopology,
                        format!("equation {:?} references variable {} outside 0..{}", eq.id, var.0, self.variable_count),
                    ));
                }
            }
        }
        let checksum_rows = self.checksums.len().saturating_mul(32);
        let total_equations = self.equations.len().saturating_add(self.byte_equations.len()).saturating_add(checksum_rows);
        if total_equations as u32 > self.budget.max_equations {
            return Err(ZbError::new(
                ErrorCode::ResourceLimited,
                format!("equation count {total_equations} exceeds budget.max_equations {}", self.budget.max_equations),
            ));
        }
        for eq in &self.byte_equations {
            for term in [eq.left, eq.right] {
                if let ByteTerm::Variable(index) = term {
                    let last = index
                        .checked_mul(8)
                        .and_then(|base| base.checked_add(7))
                        .ok_or_else(|| ZbError::resource_limited("byte variable bit range overflow"))?;
                    if last >= self.variable_count {
                        return Err(ZbError::new(
                            ErrorCode::InvalidTopology,
                            format!(
                                "byte equation {:?} variable {index} needs bits {index}*8..{index}*8+8 inside 0..{}",
                                eq.id, self.variable_count
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

struct ByteClass {
    parent: u32,
    constant: Option<(u8, EquationId)>,
}

fn find_byte(nodes: &mut [ByteClass], mut index: u32) -> u32 {
    // Union-find is acyclic by construction. Bound the walk so a corrupted parent
    // never spins forever.
    for _ in 0..=nodes.len() {
        let parent = nodes[index as usize].parent;
        if parent == index {
            return index;
        }
        let grand = nodes[parent as usize].parent;
        nodes[index as usize].parent = grand;
        index = parent;
    }
    index
}

/// Reduce byte equalities to a constant binding per class plus copy links.
fn reduce_byte_equations(
    equations: &[ByteEquation],
) -> Result<(Vec<(u32, u8, EquationId)>, Vec<(u32, u32, EquationId)>), (EquationId, String)> {
    let mut max_var = 0u32;
    for eq in equations {
        for term in [eq.left, eq.right] {
            if let ByteTerm::Variable(index) = term {
                max_var = max_var.max(index + 1);
            }
        }
    }
    let mut nodes: Vec<ByteClass> = (0..max_var).map(|index| ByteClass { parent: index, constant: None }).collect();
    let mut links = Vec::new();
    for eq in equations {
        match (eq.left, eq.right) {
            (ByteTerm::Constant(left), ByteTerm::Constant(right)) => {
                if left != right {
                    return Err((eq.id, format!("byte constants {left:#04x} and {right:#04x} differ")));
                }
            }
            (ByteTerm::Variable(var), ByteTerm::Constant(value)) | (ByteTerm::Constant(value), ByteTerm::Variable(var)) => {
                let root = find_byte(&mut nodes, var);
                match nodes[root as usize].constant {
                    Some((existing, _)) if existing != value => {
                        return Err((eq.id, format!("byte variable {var} is bound to {existing:#04x} and {value:#04x}")));
                    }
                    Some(_) => {}
                    None => nodes[root as usize].constant = Some((value, eq.id)),
                }
            }
            (ByteTerm::Variable(left), ByteTerm::Variable(right)) => {
                let mut a = find_byte(&mut nodes, left);
                let mut b = find_byte(&mut nodes, right);
                if a == b {
                    continue;
                }
                if a > b {
                    std::mem::swap(&mut a, &mut b);
                }
                match (nodes[a as usize].constant, nodes[b as usize].constant) {
                    (Some((x, _)), Some((y, _))) if x != y => {
                        return Err((eq.id, format!("byte copies are bound to {x:#04x} and {y:#04x}")));
                    }
                    (None, Some(value)) => nodes[a as usize].constant = Some(value),
                    _ => {}
                }
                nodes[b as usize].parent = a;
                nodes[b as usize].constant = None;
                links.push((b, a, eq.id));
            }
        }
    }
    for index in 0..max_var {
        let _ = find_byte(&mut nodes, index);
    }
    let bindings = nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| node.parent == *index as u32)
        .filter_map(|(index, node)| node.constant.map(|(value, id)| (index as u32, value, id)))
        .collect();
    Ok((bindings, links))
}

fn push_linear(rows: &mut Vec<(EquationId, Vec<bool>, bool)>, cols: usize, id: EquationId, variables: &[(usize, bool)]) {
    let mut bits = vec![false; cols];
    let mut rhs = false;
    for &(index, value) in variables {
        bits[index] = !bits[index];
        rhs ^= value;
    }
    rows.push((id, bits, rhs));
}

fn byte_rows(problem: &ConstraintProblem) -> Result<Vec<(EquationId, Vec<bool>, bool)>, SolveOutcome> {
    let (bindings, links) = reduce_byte_equations(&problem.byte_equations)
        .map_err(|(id, message)| SolveOutcome::Unsat { message, conflicting_equation: Some(id) })?;
    let cols = problem.variable_count as usize;
    let mut rows = Vec::new();
    for (index, value, id) in bindings {
        for bit in 0..8 {
            push_linear(&mut rows, cols, id, &[(index as usize * 8 + bit, (value >> bit) & 1 == 1)]);
        }
    }
    for (child, parent, id) in links {
        for bit in 0..8 {
            push_linear(&mut rows, cols, id, &[(child as usize * 8 + bit, false), (parent as usize * 8 + bit, false)]);
        }
    }
    Ok(rows)
}

fn term_byte(term: ByteTerm, bits: &[bool]) -> u8 {
    match term {
        ByteTerm::Constant(value) => value,
        ByteTerm::Variable(index) => {
            let mut value = 0u8;
            for bit in 0..8 {
                if bits[index as usize * 8 + bit] {
                    value |= 1 << bit;
                }
            }
            value
        }
    }
}

const MAX_BYTE_EXPR_NODES: usize = 256;

fn flatten_byte(expr: &ByteExpr) -> Result<(u8, Vec<u32>), SolveOutcome> {
    // Iterative walk with a hard node budget. Nested `Xor` never recurses.
    let mut constant = 0u8;
    let mut vars = Vec::new();
    let mut stack = vec![expr];
    let mut seen = 0usize;
    while let Some(node) = stack.pop() {
        seen = seen.saturating_add(1);
        if seen > MAX_BYTE_EXPR_NODES {
            return Err(SolveOutcome::InvalidProblem {
                message: format!("byte expression exceeds {MAX_BYTE_EXPR_NODES} nodes"),
            });
        }
        match node {
            ByteExpr::Constant(value) => constant ^= value,
            ByteExpr::Variable(index) => {
                if let Some(pos) = vars.iter().position(|existing| *existing == *index) {
                    vars.swap_remove(pos);
                }
                else {
                    vars.push(*index);
                }
            }
            ByteExpr::Xor(parts) => {
                for part in parts.iter().rev() {
                    stack.push(part);
                }
            }
        }
    }
    Ok((constant, vars))
}

fn byte_var_in_range(problem: &ConstraintProblem, index: u32) -> Result<(), SolveOutcome> {
    let last = index.checked_mul(8).and_then(|base| base.checked_add(7));
    match last {
        Some(last) if last < problem.variable_count => Ok(()),
        _ => Err(SolveOutcome::InvalidProblem {
            message: format!("byte variable {index} needs bits {index}*8..{index}*8+8 inside 0..{}", problem.variable_count),
        }),
    }
}

fn eval_byte(expr: &ByteExpr, bits: &[bool]) -> Result<u8, SolveOutcome> {
    let (mut value, vars) = flatten_byte(expr)?;
    for var in vars {
        for bit in 0..8 {
            if bits[var as usize * 8 + bit] {
                value ^= 1 << bit;
            }
        }
    }
    Ok(value)
}

fn checksum_rows(problem: &ConstraintProblem) -> Result<Vec<(EquationId, Vec<bool>, bool)>, SolveOutcome> {
    let cols = problem.variable_count as usize;
    let mut rows = Vec::new();
    for constraint in &problem.checksums {
        if constraint.algorithm != ChecksumAlgorithm::Crc32Ieee {
            return Err(SolveOutcome::InvalidProblem {
                message: "only IEEE CRC-32 checksum constraints are registered".into(),
            });
        }
        let mut constants = Vec::with_capacity(constraint.message.len());
        let mut occurrences = Vec::new();
        for expr in &constraint.message {
            let (constant, vars) = flatten_byte(expr)?;
            for var in &vars {
                byte_var_in_range(problem, *var)?;
            }
            occurrences.push((constants.len(), vars));
            constants.push(constant);
        }
        let crc_const = crc32(&constants);
        let mut influence = vec![0u32; cols];
        let zeros = vec![0u8; constants.len()];
        for (offset, vars) in &occurrences {
            if vars.is_empty() {
                continue;
            }
            // One template for this offset. XOR participants share the same CRC
            // bit columns (duplicate offsets are illegal in crc32_shared_columns).
            let (_, columns) = crc32_shared_columns(&zeros, &[CrcByteSlot { offset: *offset, variable: 0 }], 1)
                .map_err(|err| SolveOutcome::InternalError { message: err.message })?;
            for var in vars {
                for bit in 0..8 {
                    influence[*var as usize * 8 + bit] ^= columns[bit];
                }
            }
        }
        let mut target_const = 0u32;
        let mut target_coeff = vec![0u32; cols];
        for (index, expr) in constraint.target.iter().enumerate() {
            let (constant, vars) = flatten_byte(expr)?;
            for var in &vars {
                byte_var_in_range(problem, *var)?;
            }
            target_const |= u32::from(constant) << (8 * index);
            for var in vars {
                for bit in 0..8 {
                    target_coeff[var as usize * 8 + bit] |= 1u32 << (index * 8 + bit);
                }
            }
        }
        let rhs_word = crc_const ^ target_const;
        for row in 0..32 {
            let mut bits = vec![false; cols];
            for col in 0..cols {
                let from_message = (influence[col] >> row) & 1 == 1;
                let from_target = (target_coeff[col] >> row) & 1 == 1;
                if from_message ^ from_target {
                    bits[col] = true;
                }
            }
            rows.push((constraint.id, bits, (rhs_word >> row) & 1 == 1));
        }
    }
    Ok(rows)
}

fn back_substitute(problem: &ConstraintProblem, bits: &[bool]) -> Option<EquationId> {
    for eq in &problem.equations {
        let mut acc = false;
        for var in &eq.variables {
            acc ^= bits[var.0 as usize];
        }
        if acc != eq.rhs {
            return Some(eq.id);
        }
    }
    for eq in &problem.byte_equations {
        if term_byte(eq.left, bits) != term_byte(eq.right, bits) {
            return Some(eq.id);
        }
    }
    for constraint in &problem.checksums {
        let mut message = Vec::with_capacity(constraint.message.len());
        for expr in &constraint.message {
            match eval_byte(expr, bits) {
                Ok(byte) => message.push(byte),
                Err(_) => return Some(constraint.id),
            }
        }
        let mut target = 0u32;
        for (index, expr) in constraint.target.iter().enumerate() {
            match eval_byte(expr, bits) {
                Ok(byte) => target |= u32::from(byte) << (8 * index),
                Err(_) => return Some(constraint.id),
            }
        }
        if crc32(&message) != target {
            return Some(constraint.id);
        }
    }
    None
}

fn inconsistent_row(message: &str) -> Option<usize> {
    let rest = message.strip_prefix("gf2_inconsistent_row=")?;
    let token = rest.split_whitespace().next()?;
    token.parse().ok()
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
        return Err(SolveOutcome::InvalidProblem { message: "variable_bits must be positive".into() });
    }
    let mut equations = Vec::with_capacity(blocks.len().saturating_mul(32));
    let mut eq_id = 0u32;
    for (block_index, (constant, columns)) in blocks.iter().enumerate() {
        if columns.len() != variable_bits as usize {
            return Err(SolveOutcome::InvalidProblem {
                message: format!("CRC block {block_index} has {} columns, expected {variable_bits}", columns.len()),
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
            }
            else {
                vars.push(identity);
            }
            equations.push(LinearEquation { id: EquationId(eq_id), variables: vars, rhs: (constant >> row) & 1 != 0 });
            eq_id += 1;
        }
    }
    Ok(ConstraintProblem {
        variable_count: variable_bits,
        equations,
        byte_equations: Vec::new(),
        checksums: Vec::new(),
        budget,
        problem_digest,
    })
}

/// One self-referential archive CRC: message bytes with shared slots equal a LE word.
#[derive(Debug, Clone, Copy)]
pub struct SharedCrcArchive<'a> {
    /// Frozen skeleton bytes (CRC slots may hold placeholders).
    pub bytes: &'a [u8],
    /// Variable byte slots inside [`Self::bytes`].
    pub slots: &'a [CrcByteSlot],
    /// Little-endian CRC word as four byte-variable indices.
    pub target_byte_vars: [u32; 4],
}

/// Build a [`ChecksumConstraint`] for `CRC32(message) = target_word`.
pub fn checksum_from_shared_slots(id: EquationId, archive: SharedCrcArchive<'_>) -> Result<ChecksumConstraint, SolveOutcome> {
    let mut message: Vec<ByteExpr> = archive.bytes.iter().map(|&b| ByteExpr::Constant(b)).collect();
    let mut seen = vec![false; archive.bytes.len()];
    for slot in archive.slots {
        if slot.offset >= archive.bytes.len() {
            return Err(SolveOutcome::InvalidProblem { message: format!("CRC slot offset {} out of range", slot.offset) });
        }
        if seen[slot.offset] {
            return Err(SolveOutcome::InvalidProblem { message: format!("duplicate CRC slot offset {}", slot.offset) });
        }
        seen[slot.offset] = true;
        message[slot.offset] = ByteExpr::Variable(slot.variable as u32);
    }
    Ok(ChecksumConstraint {
        id,
        algorithm: ChecksumAlgorithm::Crc32Ieee,
        message,
        target: [
            ByteExpr::Variable(archive.target_byte_vars[0]),
            ByteExpr::Variable(archive.target_byte_vars[1]),
            ByteExpr::Variable(archive.target_byte_vars[2]),
            ByteExpr::Variable(archive.target_byte_vars[3]),
        ],
    })
}

/// Stack one or more self-referential CRC archives into a [`ConstraintProblem`].
pub fn shared_crc_problem(
    archives: &[SharedCrcArchive<'_>],
    variable_bytes: u32,
    budget: SolverBudget,
    problem_digest: Option<String>,
) -> Result<ConstraintProblem, SolveOutcome> {
    if archives.is_empty() {
        return Err(SolveOutcome::InvalidProblem { message: "shared CRC problem requires at least one archive".into() });
    }
    if variable_bytes == 0 {
        return Err(SolveOutcome::InvalidProblem { message: "variable_bytes must be positive".into() });
    }
    let mut checksums = Vec::with_capacity(archives.len());
    for (index, archive) in archives.iter().enumerate() {
        for slot in archive.slots {
            if slot.variable as u32 >= variable_bytes {
                return Err(SolveOutcome::InvalidProblem {
                    message: format!("CRC slot variable {} exceeds variable_bytes {variable_bytes}", slot.variable),
                });
            }
        }
        for &var in &archive.target_byte_vars {
            if var >= variable_bytes {
                return Err(SolveOutcome::InvalidProblem {
                    message: format!("CRC target byte variable {var} exceeds variable_bytes {variable_bytes}"),
                });
            }
        }
        checksums.push(checksum_from_shared_slots(EquationId(index as u32), *archive)?);
    }
    Ok(ConstraintProblem {
        variable_count: variable_bytes.saturating_mul(8),
        equations: Vec::new(),
        byte_equations: Vec::new(),
        checksums,
        budget,
        problem_digest,
    })
}

/// Convenience: build [`shared_crc_problem`] then [`solve_problem_with_cancel`].
pub fn solve_shared_crc_with_cancel(
    archives: &[SharedCrcArchive<'_>],
    variable_bytes: u32,
    budget: SolverBudget,
    problem_digest: Option<String>,
    cancel: &CancelToken,
) -> SolveOutcome {
    match shared_crc_problem(archives, variable_bytes, budget, problem_digest) {
        Ok(problem) => solve_problem_with_cancel(&problem, cancel),
        Err(outcome) => outcome,
    }
}

/// Convenience: build [`crc_identity_residual_problem`] then [`solve_problem`].
pub fn solve_crc_identity_blocks(
    blocks: &[(u32, &[u32])],
    variable_bits: u32,
    budget: SolverBudget,
    problem_digest: Option<String>,
) -> SolveOutcome {
    solve_crc_identity_blocks_with_cancel(blocks, variable_bits, budget, problem_digest, &CancelToken::new())
}

/// [`solve_crc_identity_blocks`] with a shared cancel token.
pub fn solve_crc_identity_blocks_with_cancel(
    blocks: &[(u32, &[u32])],
    variable_bits: u32,
    budget: SolverBudget,
    problem_digest: Option<String>,
    cancel: &CancelToken,
) -> SolveOutcome {
    match crc_identity_residual_problem(blocks, variable_bits, budget, problem_digest) {
        Ok(problem) => solve_problem_with_cancel(&problem, cancel),
        Err(outcome) => outcome,
    }
}

/// Solve a validated [`ConstraintProblem`].
///
/// Returns a structured [`SolveOutcome`] instead of mapping every failure through
/// [`ZbResult`], so generators can branch on `unsat` vs `resource_limited`
/// without losing equation identity.
pub fn solve_problem(problem: &ConstraintProblem) -> SolveOutcome {
    solve_problem_with_cancel(problem, &CancelToken::new())
}

/// [`solve_problem`] with cooperative cancel checks while building and eliminating.
pub fn solve_problem_with_cancel(problem: &ConstraintProblem, cancel: &CancelToken) -> SolveOutcome {
    if let Err(err) = cancel.check() {
        return SolveOutcome::Cancelled { message: err.message };
    }
    if let Err(err) = problem.validate() {
        return match err.category {
            ErrorCategory::ResourceLimited => SolveOutcome::ResourceLimited { message: err.message },
            ErrorCategory::InvalidRequest => SolveOutcome::InvalidProblem { message: err.message },
            ErrorCategory::Cancelled => SolveOutcome::Cancelled { message: err.message },
            _ => SolveOutcome::InternalError { message: err.message },
        };
    }

    let cols = problem.variable_count as usize;
    let byte_derived = match byte_rows(problem) {
        Ok(rows) => rows,
        Err(outcome) => return outcome,
    };
    let checksum_derived = match checksum_rows(problem) {
        Ok(rows) => rows,
        Err(outcome) => return outcome,
    };
    let row_count = problem.equations.len() + byte_derived.len() + checksum_derived.len();
    if row_count as u32 > problem.budget.max_equations {
        return SolveOutcome::ResourceLimited {
            message: format!(
                "expanded equation count {row_count} exceeds budget.max_equations {}",
                problem.budget.max_equations
            ),
        };
    }
    let cells = (row_count as u64).saturating_mul((cols as u64).saturating_add(1));
    if cells > problem.budget.max_matrix_bits {
        return SolveOutcome::ResourceLimited {
            message: format!("matrix cells {cells} exceed budget.max_matrix_bits {}", problem.budget.max_matrix_bits),
        };
    }
    let mut row_ids = Vec::with_capacity(row_count);
    let mut matrix = BitMatrix::zeros(row_count, cols);
    for (row, eq) in problem.equations.iter().enumerate() {
        if let Err(err) = cancel.check() {
            return SolveOutcome::Cancelled { message: err.message };
        }
        row_ids.push(eq.id);
        let mut bits = vec![false; cols];
        for var in &eq.variables {
            bits[var.0 as usize] = !bits[var.0 as usize];
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
    for (offset, (id, bits, rhs)) in byte_derived.iter().enumerate() {
        if let Err(err) = cancel.check() {
            return SolveOutcome::Cancelled { message: err.message };
        }
        let row = problem.equations.len() + offset;
        row_ids.push(*id);
        for (col, bit) in bits.iter().enumerate() {
            if *bit {
                matrix.set(row, col);
            }
        }
        if *rhs {
            matrix.set(row, cols);
        }
    }
    for (offset, (id, bits, rhs)) in checksum_derived.iter().enumerate() {
        if let Err(err) = cancel.check() {
            return SolveOutcome::Cancelled { message: err.message };
        }
        let row = problem.equations.len() + byte_derived.len() + offset;
        row_ids.push(*id);
        for (col, bit) in bits.iter().enumerate() {
            if *bit {
                matrix.set(row, col);
            }
        }
        if *rhs {
            matrix.set(row, cols);
        }
    }

    match solve_with_cancel(matrix, cancel) {
        Ok(sol) => {
            if let Some(id) = back_substitute(problem, &sol.assignment) {
                return SolveOutcome::Unsat {
                    message: format!("back-substitution failed for equation {}", id.0),
                    conflicting_equation: Some(id),
                };
            }
            let evidence = SolveEvidence {
                rank: sol.rank,
                free_bits: cols.saturating_sub(sol.rank),
                equations_used: row_count,
                problem_digest: problem.problem_digest.clone(),
            };
            SolveOutcome::from_gf2(sol, evidence)
        }
        Err(err) => {
            if err.category == ErrorCategory::Cancelled {
                SolveOutcome::Cancelled { message: err.message }
            }
            else if err.category == ErrorCategory::ConstraintUnsatisfied {
                let conflicting_equation = inconsistent_row(&err.message).and_then(|row| row_ids.get(row).copied());
                SolveOutcome::Unsat { message: err.message, conflicting_equation }
            }
            else {
                SolveOutcome::InternalError { message: err.message }
            }
        }
    }
}

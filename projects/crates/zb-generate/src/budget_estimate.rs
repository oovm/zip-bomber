//! Plan-time budget estimates (Living `13` §6.3 C2).

use serde::{Deserialize, Serialize};

/// Estimated index / decode / solver consumption for a `plan()` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlanBudgetEstimate {
    /// Expected Acorn ZIP index passes.
    pub index_passes: u64,
    /// Expected bounded member decodes.
    pub member_decodes: u64,
    /// Expected decoded-byte charge against `max_decoded_bytes`.
    pub decoded_bytes: u64,
    /// Expected read-byte charge against live-memory quota.
    pub read_bytes: u64,
    /// Upper-bound solver word operations for CRC / GF(2) steps.
    pub solver_operations: u64,
    /// Upper-bound constraint bits reserved for CRC systems.
    pub constraint_bits: u64,
}

impl PlanBudgetEstimate {
    /// Combine independent planning stages into one task estimate.
    pub fn merge(self, other: Self) -> Self {
        Self {
            index_passes: self.index_passes.saturating_add(other.index_passes),
            member_decodes: self.member_decodes.saturating_add(other.member_decodes),
            decoded_bytes: self.decoded_bytes.saturating_add(other.decoded_bytes),
            read_bytes: self.read_bytes.saturating_add(other.read_bytes),
            solver_operations: self.solver_operations.max(other.solver_operations),
            constraint_bits: self.constraint_bits.max(other.constraint_bits),
        }
    }
}

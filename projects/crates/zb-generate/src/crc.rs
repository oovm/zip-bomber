//! CRC32 fixed-point planning (stub).

use zb_types::{CrcStrategy, GenerateRequest};

/// Placeholder CRC plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrcPlan {
    /// Strategy selected by the request.
    pub strategy: CrcStrategy,
}

/// Plan CRC resolution. Template echoes strategy only.
pub fn plan(request: &GenerateRequest) -> CrcPlan {
    CrcPlan {
        strategy: request.crc_strategy,
    }
}

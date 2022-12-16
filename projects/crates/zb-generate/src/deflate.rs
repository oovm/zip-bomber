//! DEFLATE block and chained-repeat planning (stub).

use zb_types::GenerateRequest;

/// Placeholder deflate plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeflatePlan {
    /// Whether chained repeat tokens are requested.
    pub chained_repeats: bool,
}

/// Plan DEFLATE emission. Template enables chaining flag only.
pub fn plan(_request: &GenerateRequest) -> DeflatePlan {
    DeflatePlan {
        chained_repeats: true,
    }
}

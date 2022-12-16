//! ZIP local-header / central-directory layout planning (stub).

use zb_types::GenerateRequest;

/// Placeholder layout plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutPlan {
    /// Requested extra-field budget in bytes.
    pub extra_budget: u32,
}

/// Plan ZIP layout. Template returns budget echo only.
pub fn plan(request: &GenerateRequest) -> LayoutPlan {
    LayoutPlan {
        extra_budget: request.extra_budget,
    }
}

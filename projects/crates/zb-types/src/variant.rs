//! Generation variant kinds.

use serde::{Deserialize, Serialize};

/// Supported recursive ZIP / bomb generation variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VariantKind {
    /// Nested or overlapping expansion without a self-reference cycle.
    Bomb,
    /// Strict self-reference `A → A`.
    Quine,
    /// Directed cycle `A → B → … → A` with length at least 2.
    Loopy,
    /// DAG with back-edges and structural self-similarity.
    Fractal,
}

impl VariantKind {
    /// Stable lowercase id used by CLI and reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bomb => "bomb",
            Self::Quine => "quine",
            Self::Loopy => "loopy",
            Self::Fractal => "fractal",
        }
    }

    /// Parse a lowercase variant id.
    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "bomb" => Some(Self::Bomb),
            "quine" => Some(Self::Quine),
            "loopy" => Some(Self::Loopy),
            "fractal" => Some(Self::Fractal),
            _ => None,
        }
    }
}

impl std::fmt::Display for VariantKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// All variants in stable display order.
pub const ALL_VARIANTS: [VariantKind; 4] = [
    VariantKind::Bomb,
    VariantKind::Quine,
    VariantKind::Loopy,
    VariantKind::Fractal,
];

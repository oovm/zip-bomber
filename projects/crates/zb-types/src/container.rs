//! Archive container kinds.

use serde::{Deserialize, Serialize};

/// Supported archive containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerKind {
    /// ZIP / ZIP64 first-wave container.
    Zip,
}

impl ContainerKind {
    /// Stable lowercase id.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zip => "zip",
        }
    }
}

impl std::fmt::Display for ContainerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

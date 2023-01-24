//! Cooperative cancellation for fixed-skeleton solves (Living `04` / `08`).
//!
//! Hosts (Worker termination, `AbortSignal`, CLI interrupt) set the flag.
//! Solvers poll at equation / pivot boundaries. Synchronous native calls still
//! only observe cancellation at those checkpoints until a Worker owns the task.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use zb_types::{ZbError, ZbResult};

/// Shared cancel flag checked at solver step boundaries.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    /// Fresh token that is not cancelled.
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Request cancellation. Idempotent and thread-safe.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Return [`ZbError`] with category `cancelled` when the flag is set.
    pub fn check(&self) -> ZbResult<()> {
        if self.is_cancelled() {
            Err(ZbError::cancelled("solver cancelled"))
        } else {
            Ok(())
        }
    }
}

/// Never-cancelled token for call sites that omit an explicit handle.
pub fn never() -> CancelToken {
    CancelToken::new()
}

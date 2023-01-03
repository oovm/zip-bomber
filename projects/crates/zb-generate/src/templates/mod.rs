//! Verified construction templates (Living `05` / `07`).
//!
//! Templates are finite, named skeletons. Quine / Loopy / Fractal drivers
//! enumerate registered candidates; empty registries yield `search_exhausted`
//! rather than a generic `not_implemented`.

mod lz_quine;
mod quine;

pub use lz_quine::{cox_core_program, interpret as interpret_lz_quine, is_self_reproducing, LzOp};
pub use quine::{list_quine_templates, QuineTemplateMeta};

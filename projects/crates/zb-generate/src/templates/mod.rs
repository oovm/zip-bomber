//! Verified construction templates (Living `05` / `07`).
//!
//! Templates are finite, named skeletons. Quine / Loopy / Fractal drivers
//! enumerate registered candidates; empty registries yield `search_exhausted`
//! rather than a generic `not_implemented`.

mod deflate_quine;
mod lz_quine;
mod quine;

pub use deflate_quine::encode_cox_core_deflate;
pub use lz_quine::{cox_core_program, interpret as interpret_lz_quine, is_self_reproducing, LzOp};
pub use quine::{list_quine_templates, QuineTemplateMeta};
pub(crate) mod recursive;
pub(crate) use recursive::encode_recursive_stream;

pub(crate) mod loopy;

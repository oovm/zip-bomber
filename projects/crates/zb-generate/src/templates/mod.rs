//! Verified construction templates (Living `05` / `07`).
//!
//! Templates are finite, named skeletons. Quine / Loopy / Fractal drivers
//! enumerate registered candidates; empty registries yield `search_exhausted`
//! rather than a generic `not_implemented`.

mod quine;

pub use quine::{list_quine_templates, QuineTemplateMeta};

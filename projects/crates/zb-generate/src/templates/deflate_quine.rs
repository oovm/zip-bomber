//! Complete raw DEFLATE quine, including the self-quoted terminator.
//!
//! The original 100-byte core cannot reproduce its BFINAL bit. This compatibility
//! entry now constructs the complete prefix/core/suffix program with empty outer
//! prefix and suffix. It no longer iterates an unsatisfiable byte assignment.

use zb_types::ZbResult;

/// Build a complete raw DEFLATE stream that inflates to itself.
///
/// This includes a quoted termination sequence, so its size exceeds the old
/// abstract 100-byte core. ZIP generation uses nonempty container boundaries.
pub fn encode_cox_core_deflate() -> ZbResult<Vec<u8>> {
    Ok(super::recursive::encode_recursive_stream(&[], &[], 256)?.bytes)
}

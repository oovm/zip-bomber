//! Raw DEFLATE decode through Acorn codec (verification and post-emit checks).

use acorn_codec::{compression::RawDeflate, CodecLimits};
use zb_types::{ZbError, ZbResult};

/// Decode a raw DEFLATE stream with a bounded output ceiling.
///
/// The entire input must be consumed with no trailing bytes.
pub fn decode_raw_deflate(input: &[u8], max_output: u64) -> ZbResult<Vec<u8>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let limits = CodecLimits::new()
        .with_max_input_bytes(input.len() as u64)
        .with_max_output_bytes(max_output);
    let facts = RawDeflate::decode(input, limits).map_err(|e| {
        ZbError::verification_failed(format!("raw deflate decode failed: {e}"))
    })?;
    if !facts.deflate_stream_complete(input.len()) {
        return Err(ZbError::verification_failed(
            "raw deflate decoder did not consume the complete stream",
        ));
    }
    Ok(facts.value)
}

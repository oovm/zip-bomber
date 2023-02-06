//! Raw DEFLATE decode through Acorn codec (verification and post-emit checks).

use acorn_codec::{compression::RawDeflate, CodecLimits};
use zb_types::{GenerateLimits, ZbError, ZbResult};

use crate::budget::TaskBudgetLedger;

/// Decode a raw DEFLATE stream with a bounded output ceiling.
///
/// The entire input must be consumed with no trailing bytes.
pub fn decode_raw_deflate(input: &[u8], max_output: u64) -> ZbResult<Vec<u8>> {
    let mut ledger = TaskBudgetLedger::small_sample();
    decode_raw_deflate_with_ledger(input, max_output, &mut ledger)
}

/// Decode using an explicit task ledger (shared verify / generate budget).
pub fn decode_raw_deflate_with_ledger(
    input: &[u8],
    max_output: u64,
    ledger: &mut TaskBudgetLedger,
) -> ZbResult<Vec<u8>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let limits = ledger.reserve_raw_deflate(input.len() as u64, max_output)?;
    decode_raw_deflate_with_codec_limits(input, limits, ledger)
}

/// Decode with pre-reserved codec limits and settle the ledger on success.
pub fn decode_raw_deflate_with_codec_limits(
    input: &[u8],
    limits: CodecLimits,
    ledger: &mut TaskBudgetLedger,
) -> ZbResult<Vec<u8>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let facts = RawDeflate::decode(input, limits).map_err(|e| {
        ZbError::verification_failed(format!("raw deflate decode failed: {e}"))
    })?;
    if !facts.deflate_stream_complete(input.len()) {
        return Err(ZbError::verification_failed(
            "raw deflate decoder did not consume the complete stream",
        ));
    }
    ledger.settle_raw_deflate(facts.output_produced as u64, input.len() as u64)?;
    Ok(facts.value)
}

/// Decode using explicit task limits (convenience for tests).
pub fn decode_raw_deflate_with_limits(
    input: &[u8],
    max_output: u64,
    limits: &GenerateLimits,
) -> ZbResult<Vec<u8>> {
    let mut ledger = TaskBudgetLedger::new(limits.clone());
    decode_raw_deflate_with_ledger(input, max_output, &mut ledger)
}

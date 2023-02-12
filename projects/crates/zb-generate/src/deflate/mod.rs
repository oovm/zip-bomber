//! DEFLATE block planning and fixed stored/fixed-Huffman skeleton helpers.
//!
//! The bit-level RFC 1951 primitives live in `acorn-codec`. This module only
//! holds zb's own template-side concepts: repeat chaining, skeleton expansion,
//! and the error mapping between Acorn codec failures and zb errors.

mod decode;

use acorn_codec::{
    bit::BitWriter,
    compression::deflate::{write_fixed_block, write_stored_block, Symbol},
    CodecError,
};
use zb_types::{zip_layout, ErrorCode, GenerateRequest, ZbError, ZbResult};

pub use acorn_codec::compression::deflate::{
    DeflateBlock, DeflateBlockPlan, FixedBlock, StoredBlock,
};
pub use decode::{decode_raw_deflate, decode_raw_deflate_with_ledger};

/// Maps an Acorn codec error raised while emitting a planned DEFLATE block.
///
/// A limit rejection stays a resource limit so the caller does not report a
/// budget problem as an internal failure.
pub(crate) fn block_error(error: CodecError) -> ZbError {
    if error.is_resource_limit() {
        return ZbError::resource_limited(format!("deflate block emit rejected: {error}"));
    }
    ZbError::new(
        ErrorCode::Internal,
        format!("deflate block emit rejected: {error}"),
    )
}

/// Writes one fixed-Huffman block at the writer's current bit position.
pub(crate) fn write_fixed(
    writer: &mut BitWriter,
    symbols: &[Symbol],
    final_block: bool,
) -> ZbResult<()> {
    write_fixed_block(writer, symbols, final_block).map_err(block_error)
}

/// Writes one stored block at the writer's current bit position.
pub(crate) fn write_stored(writer: &mut BitWriter, data: &[u8], final_block: bool) -> ZbResult<()> {
    write_stored_block(writer, data, final_block).map_err(block_error)
}

/// Encodes a payload as a single final stored block.
pub fn encode_stored_stream(payload: &[u8]) -> ZbResult<Vec<u8>> {
    let mut plan = DeflateBlockPlan::new();
    plan.push(
        DeflateBlock::Stored(StoredBlock::new(payload.to_vec())),
        true,
    );
    plan.encode().map_err(block_error)
}

/// Encodes a symbol sequence as a single final fixed-Huffman block.
pub fn encode_fixed_stream(symbols: &[Symbol]) -> ZbResult<Vec<u8>> {
    let mut plan = DeflateBlockPlan::new();
    plan.push(DeflateBlock::Fixed(FixedBlock::new(symbols.to_vec())), true);
    plan.encode().map_err(block_error)
}

/// Placeholder deflate plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeflatePlan {
    /// Whether chained repeat tokens are requested.
    pub chained_repeats: bool,
}

/// Plan DEFLATE emission. Template enables chaining flag only.
pub fn plan(_request: &GenerateRequest) -> DeflatePlan {
    DeflatePlan {
        chained_repeats: true,
    }
}

/// One fixed skeleton instruction (Living `05` §4 initial subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonOp {
    /// Stored-block literal byte (constant).
    StoredLiteral(u8),
    /// Fixed-Huffman literal (constant), not used as a solver variable carrier.
    FixedLiteral(u8),
    /// LZ77 repeat with validated length / distance.
    Repeat {
        /// Match length in `3..=258`.
        length: u16,
        /// Back-reference distance in `1..=32768`.
        distance: u16,
    },
    /// End-of-block marker.
    EndOfBlock,
}

/// Validate a repeat token against RFC 1951 windows and already-emitted bytes.
pub fn validate_repeat(length: u16, distance: u16, emitted: usize) -> ZbResult<()> {
    if !(zip_layout::DEFLATE_LEN_MIN as u16..=zip_layout::DEFLATE_LEN_MAX as u16).contains(&length)
    {
        return Err(ZbError::new(
            ErrorCode::Internal,
            format!("DEFLATE length {length} out of 3..=258"),
        ));
    }
    if distance == 0 || u32::from(distance) > zip_layout::DEFLATE_DIST_MAX {
        return Err(ZbError::new(
            ErrorCode::Internal,
            format!("DEFLATE distance {distance} out of 1..=32768"),
        ));
    }
    if usize::from(distance) > emitted {
        return Err(ZbError::new(
            ErrorCode::Internal,
            format!("DEFLATE distance {distance} exceeds emitted {emitted}"),
        ));
    }
    Ok(())
}

/// Symbolically expand a fixed skeleton into concrete output bytes.
///
/// Only constant literals and validated repeats are accepted.
pub fn expand_skeleton(ops: &[SkeletonOp]) -> ZbResult<Vec<u8>> {
    let mut out = Vec::new();
    for op in ops {
        match *op {
            SkeletonOp::StoredLiteral(b) | SkeletonOp::FixedLiteral(b) => out.push(b),
            SkeletonOp::Repeat { length, distance } => {
                validate_repeat(length, distance, out.len())?;
                for _ in 0..length {
                    let src = out.len() - usize::from(distance);
                    let byte = out[src];
                    out.push(byte);
                }
            }
            SkeletonOp::EndOfBlock => {}
        }
    }
    Ok(out)
}

/// Split a long copy into legal chained repeats (each length ≤ 258).
pub fn chain_repeats(total_length: u32, distance: u16) -> ZbResult<Vec<SkeletonOp>> {
    if total_length == 0 {
        return Ok(Vec::new());
    }
    if total_length < zip_layout::DEFLATE_LEN_MIN {
        return Err(ZbError::new(
            ErrorCode::Internal,
            "chained repeat total length must be at least 3 (or handled as literals)",
        ));
    }
    let mut remaining = total_length;
    let mut ops = Vec::new();
    while remaining > 0 {
        let mut chunk = remaining.min(zip_layout::DEFLATE_LEN_MAX);
        if remaining > chunk && remaining - chunk < zip_layout::DEFLATE_LEN_MIN {
            chunk -= zip_layout::DEFLATE_LEN_MIN - (remaining - chunk);
        }
        if chunk < zip_layout::DEFLATE_LEN_MIN && remaining == chunk {
            return Err(ZbError::new(
                ErrorCode::Internal,
                "cannot encode remainder 1..=2 without adjusting prior segments",
            ));
        }
        let len = if chunk < zip_layout::DEFLATE_LEN_MIN {
            return Err(ZbError::new(
                ErrorCode::Internal,
                "remainder 1..=2 requires prior segment adjustment",
            ));
        } else {
            chunk
        };
        ops.push(SkeletonOp::Repeat {
            length: len as u16,
            distance,
        });
        remaining -= len;
    }
    Ok(ops)
}

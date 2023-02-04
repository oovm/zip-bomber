//! Complete quoted prefix/core/suffix DEFLATE construction.
//!
//! Adapted from the recursive construction in Van Mello and Audenaert (2024).
//! See THIRD_PARTY_NOTICES.md for the accompanying implementation license.

use zb_types::{ErrorCode, ZbError, ZbResult};

use crate::deflate::{chain_repeats, encode_fixed, encode_stored, BitWriter, FixedOp, SkeletonOp};

const R4: [u8; 5] = [0x42, 0x88, 0x21, 0xc4, 0x00];

pub(crate) struct RecursiveStream {
    pub bytes: Vec<u8>,
    pub prefix_copy: usize,
    pub suffix_copy: usize,
    pub length_candidates: u32,
}

pub(crate) fn header(length: usize, final_block: bool) -> ZbResult<Vec<u8>> {
    let length = u16::try_from(length)
        .map_err(|_| ZbError::resource_limited("quoted stored block exceeds ZIP16 length"))?;
    let mut bytes = vec![u8::from(final_block)];
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&(!length).to_le_bytes());
    Ok(bytes)
}

pub(crate) fn stored(payload: &[u8]) -> ZbResult<Vec<u8>> {
    let mut bytes = header(payload.len(), false)?;
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

fn repeat_ops(length: usize) -> ZbResult<Vec<FixedOp>> {
    if length > 32768 {
        return Err(ZbError::resource_limited(
            "recursive copy exceeds DEFLATE window",
        ));
    }
    chain_repeats(length as u32, length as u16)?
        .into_iter()
        .map(|op| match op {
            SkeletonOp::Repeat { length, distance } => Ok(FixedOp::Repeat { length, distance }),
            _ => Err(ZbError::new(ErrorCode::Internal, "unexpected copy opcode")),
        })
        .collect()
}

/// A five-byte repeat fragment may borrow up to five trailing EOB zero bits
/// from the first byte of the following non-final stored header. Nothing is
/// silently padded between ordinary fixed blocks.
pub(crate) fn five_byte_repeat(length: usize) -> ZbResult<Option<Vec<u8>>> {
    if !(6..=516).contains(&length) {
        return Ok(None);
    }
    for left in 3..=258.min(length - 3) {
        let right = length - left;
        if right > 258 {
            continue;
        }
        let mut writer = BitWriter::new();
        encode_fixed(
            &mut writer,
            &[
                FixedOp::Repeat {
                    length: left as u16,
                    distance: length as u16,
                },
                FixedOp::Repeat {
                    length: right as u16,
                    distance: length as u16,
                },
            ],
            false,
        )?;
        if (40..=45).contains(&writer.bit_len()) {
            let mut bytes = writer.finish();
            bytes.truncate(5);
            return Ok(Some(bytes));
        }
    }
    Ok(None)
}

pub(crate) fn aligned_repeat(length: usize) -> ZbResult<Vec<u8>> {
    if let Some(bytes) = five_byte_repeat(length)? {
        return Ok(bytes);
    }
    let mut writer = BitWriter::new();
    encode_fixed(&mut writer, &repeat_ops(length)?, false)?;
    // Include the entire empty block in the quoted fragment: it is a legal
    // zero-output alignment operation, not padding between compressed blocks.
    encode_stored(&mut writer, &[], false)?;
    Ok(writer.finish())
}

fn terminal_repeat(length: usize) -> ZbResult<Vec<u8>> {
    let mut writer = BitWriter::new();
    encode_fixed(&mut writer, &repeat_ops(length)?, false)?;
    encode_stored(&mut writer, &[], false)?;
    encode_stored(&mut writer, &[], true)?;
    Ok(writer.finish())
}

/// Solve the suffix's encoded-width equation by finite integer enumeration.
/// CRC values never influence this search, and no archive bytes are iterated.
pub(crate) fn encode_recursive_stream(
    prefix: &[u8],
    suffix: &[u8],
    max_candidates: u32,
) -> ZbResult<RecursiveStream> {
    if prefix.len() > 32763 || suffix.len() > 32755 {
        return Err(ZbError::resource_limited(
            "recursive prefix/suffix exceeds DEFLATE window",
        ));
    }
    let mut tail = None;
    let mut candidates = 0;
    // A copy of at most 32768 bytes takes at most 128 legal matches; 1024
    // encoded bytes is a conservative bound including both terminator blocks.
    for width in 10..=1024 {
        if candidates >= max_candidates {
            return Err(ZbError::resource_limited(
                "recursive length candidate budget exhausted",
            ));
        }
        candidates += 1;
        if suffix.len() + width > 32768 {
            break;
        }
        let bytes = terminal_repeat(suffix.len() + width)?;
        if bytes.len() == width {
            tail = Some(bytes);
            break;
        }
    }
    let tail =
        tail.ok_or_else(|| ZbError::search_exhausted("recursive suffix width has no candidate"))?;

    let mut first_literal = prefix.to_vec();
    first_literal.extend(header(prefix.len() + 5, false)?);
    let mut bytes = stored(&first_literal)?;
    let prefix_copy = 5;
    let mut repeat = aligned_repeat(first_literal.len())?;
    bytes.extend_from_slice(&repeat);

    // Quote the instruction encoding and its headers until the bridge to the
    // 20-byte core is exactly one five-byte repeat unit. Width shrinks rapidly,
    // but the finite guard protects against an unsupported encoding shape.
    for _ in 0..8 {
        if repeat.len() == 5 {
            break;
        }
        bytes.extend(stored(&repeat)?);
        let quote_header = header(repeat.len(), false)?;
        let mut bridge = repeat.clone();
        bridge.extend(stored(&quote_header)?);
        bytes.extend(stored(&quote_header)?);
        bridge.extend(header(bridge.len() + 5, false)?);
        bytes.extend(stored(&bridge)?);
        repeat = aligned_repeat(bridge.len())?;
        bytes.extend_from_slice(&repeat);
    }
    if repeat.len() != 5 {
        return Err(ZbError::search_exhausted(
            "recursive prefix cannot reach five-byte bridge",
        ));
    }

    bytes.extend(stored(&repeat)?);
    let quote_header = header(5, false)?;
    bytes.extend(stored(&quote_header)?);
    let mut bridge = repeat;
    bridge.extend(stored(&quote_header)?);
    bridge.extend(header(20, false)?);
    bytes.extend(stored(&bridge)?);
    bytes.extend_from_slice(&R4);

    let mut core = R4.to_vec();
    core.extend(header(20, false)?);
    core.extend_from_slice(&R4);
    core.extend(header(20, false)?);
    bytes.extend(stored(&core)?);
    bytes.extend_from_slice(&R4);

    let mut exit = R4.to_vec();
    exit.extend(header(0, false)?);
    exit.extend(header(0, false)?);
    exit.extend(header(tail.len() + suffix.len(), false)?);
    bytes.extend(stored(&exit)?);
    bytes.extend_from_slice(&R4);
    bytes.extend(header(0, false)?);
    bytes.extend(header(0, false)?);
    bytes.extend(header(tail.len() + suffix.len(), false)?);
    bytes.extend_from_slice(&tail);
    let suffix_copy = bytes.len();
    bytes.extend_from_slice(suffix);
    bytes.extend_from_slice(&tail);
    Ok(RecursiveStream {
        bytes,
        prefix_copy,
        suffix_copy,
        length_candidates: candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode_raw_deflate;

    #[test]
    fn complete_prefix_suffix_program_is_self_reproducing() {
        for &(prefix_len, suffix_len) in &[
            (0, 0),
            (37, 75),
            (5, 5),
            (254, 257),
            (300, 300),
            (4096, 4097),
            (32000, 32000),
        ] {
            let prefix: Vec<u8> = (0..prefix_len).map(|i| (i % 251) as u8).collect();
            let suffix: Vec<u8> = (0..suffix_len).map(|i| (i % 239) as u8).collect();
            let stream = encode_recursive_stream(&prefix, &suffix, 1024).unwrap();
            let mut expected = prefix.clone();
            expected.extend_from_slice(&stream.bytes);
            expected.extend_from_slice(&suffix);
            let decoded = decode_raw_deflate(&stream.bytes, expected.len() as u64).unwrap();
            assert_eq!(decoded, expected, "prefix={prefix_len} suffix={suffix_len}");
        }
    }
}

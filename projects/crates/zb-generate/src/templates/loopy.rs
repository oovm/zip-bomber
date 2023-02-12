//! Alternating two-state DEFLATE program, based on Van Mello and Audenaert.
//! The output cursor, rather than encoded offsets, determines replay distances.
use acorn_codec::{
    bit::{BitOrder, BitWriter},
    compression::deflate::Symbol,
};

use super::recursive::{aligned_repeat, header, stored};
use crate::deflate::{chain_repeats, write_fixed, write_stored, SkeletonOp};
use zb_types::{ZbError, ZbResult};

const R4: [u8; 5] = [0x42, 0x88, 0x21, 0xc4, 0x00];

fn ops(length: usize, distance: usize) -> ZbResult<Vec<Symbol>> {
    if distance > 32768 || distance == 0 {
        return Err(ZbError::resource_limited(
            "loop copy exceeds DEFLATE window",
        ));
    }
    chain_repeats(length as u32, distance as u16)?
        .into_iter()
        .map(|op| match op {
            SkeletonOp::Repeat { length, distance } => Ok(Symbol::Match { length, distance }),
            _ => unreachable!(),
        })
        .collect()
}

fn writer() -> BitWriter {
    BitWriter::new(BitOrder::LeastSignificantBitFirst)
}

fn aligned(operations: &[Symbol]) -> ZbResult<Vec<u8>> {
    let mut w = writer();
    write_fixed(&mut w, operations, false)?;
    write_stored(&mut w, &[], false)?;
    Ok(w.finish())
}

/// Encoded alternating program and explicit quoted source/target locations.
pub(crate) struct LoopStream {
    pub bytes: Vec<u8>,
    pub target_copy: usize,
    pub source_copy: usize,
}

/// Construct a stream in source that decodes into target plus its alternate stream.
pub(crate) fn encode_loop(
    source: &[u8],
    target: &[u8],
    source_footer_header: &[u8],
    target_footer_header: &[u8],
    header_len: usize,
    max_candidates: u32,
) -> ZbResult<LoopStream> {
    if source.len() != target.len() || source_footer_header.len() != target_footer_header.len() {
        return Err(ZbError::search_exhausted(
            "loop quote regions must have equal widths",
        ));
    }
    let mut a = target.to_vec();
    a.extend(target_footer_header);
    a.extend(header(a.len() + 5, false)?);
    let mut b = source.to_vec();
    b.extend(source_footer_header);
    b.extend(header(b.len() + 5, false)?);
    let mut bytes = stored(&a)?;
    let target_copy = 5;
    let source_copy = bytes.len() + 5;
    bytes.extend(stored(&b)?);
    let mut cursor = a.len() + b.len();
    let mut initial = vec![Symbol::Match {
        length: 5,
        distance: 5,
    }];
    initial.extend(ops(b.len(), a.len() + b.len() + 5)?);
    let mut repeat = aligned(&initial)?;
    bytes.extend(&repeat);
    cursor += 5 + b.len();
    for _ in 0..8 {
        if repeat.len() == 5 {
            break;
        }
        bytes.extend(stored(&repeat)?);
        cursor += repeat.len();
        let h = header(repeat.len(), false)?;
        bytes.extend(stored(&h)?);
        cursor += h.len();
        let mut bridge = repeat.clone();
        bridge.extend(stored(&h)?);
        bridge.extend(header(bridge.len() + 5, false)?);
        bytes.extend(stored(&bridge)?);
        cursor += bridge.len();
        repeat = aligned_repeat(bridge.len())?;
        bytes.extend(&repeat);
        cursor += bridge.len();
    }
    if repeat.len() != 5 {
        return Err(ZbError::search_exhausted(
            "loop bridge did not reach five bytes",
        ));
    }
    bytes.extend(stored(&repeat)?);
    cursor += 5;
    let h = header(5, false)?;
    bytes.extend(stored(&h)?);
    cursor += 5;
    let mut bridge = repeat;
    bridge.extend(stored(&h)?);
    bridge.extend(header(20, false)?);
    bytes.extend(stored(&bridge)?);
    cursor += bridge.len();
    bytes.extend(R4);
    cursor += 20;
    let mut core = R4.to_vec();
    core.extend(header(20, false)?);
    core.extend(R4);
    core.extend(header(20, false)?);
    bytes.extend(stored(&core)?);
    cursor += 20;
    bytes.extend(R4);
    cursor += 20;
    // The footer to replay starts in the very first decoded literal.
    let footer_start = target.len();
    let footer_len = target_footer_header.len() - header_len;
    let base_distance = cursor + 20 + 20 + 4 - footer_start;
    let mut tail = None;
    for (attempt, width) in (10..=1024).enumerate() {
        if attempt >= max_candidates as usize {
            return Err(ZbError::resource_limited(
                "loop width candidate budget exhausted",
            ));
        }
        let distance = base_distance + 2 * width;
        if distance > 32768 {
            break;
        }
        let mut w = writer();
        write_fixed(&mut w, &ops(width, width)?, false)?;
        write_fixed(&mut w, &ops(footer_len, distance)?, false)?;
        write_stored(&mut w, &[], false)?;
        write_stored(&mut w, &[], true)?;
        let candidate = w.finish();
        if candidate.len() == width {
            tail = Some(candidate);
            break;
        }
    }
    let tail = tail.ok_or_else(|| {
        ZbError::search_exhausted("loop tail width equation has no bounded solution")
    })?;
    let mut exit = R4.to_vec();
    exit.extend(header(0, false)?);
    exit.extend(header(0, false)?);
    exit.extend(header(tail.len() + 4, false)?);
    bytes.extend(stored(&exit)?);
    bytes.extend(R4);
    bytes.extend(header(0, false)?);
    bytes.extend(header(0, false)?);
    bytes.extend(header(tail.len() + 4, false)?);
    bytes.extend([0; 4]);
    bytes.extend(&tail);
    bytes.extend(&tail);
    Ok(LoopStream {
        bytes,
        target_copy,
        source_copy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode_raw_deflate;
    #[test]
    fn alternating_streams_reproduce_complete_opposite_archive() {
        for size in [0, 40, 257, 2048, 12000] {
            let a = vec![b'a'; size];
            let b = vec![b'b'; size];
            let fa = vec![b'x'; 80];
            let fb = vec![b'y'; 80];
            let ha = vec![b'h'; 40];
            let hb = vec![b'i'; 40];
            let mut qa = fa.clone();
            qa.extend(&ha);
            let mut qb = fb.clone();
            qb.extend(&hb);
            let sa = encode_loop(&a, &b, &qa, &qb, 40, 1024).unwrap();
            let sb = encode_loop(&b, &a, &qb, &qa, 40, 1024).unwrap();
            for (stream, prefix, footer, head, other) in
                [(&sa, &b, &fb, &hb, &sb), (&sb, &a, &fa, &ha, &sa)]
            {
                let mut expected = prefix.clone();
                expected.extend(footer);
                expected.extend(head);
                expected.extend(&other.bytes);
                expected.extend(footer);
                let out = decode_raw_deflate(&stream.bytes, expected.len() as u64).unwrap();
                assert_eq!(out, expected, "size={size}");
            }
        }
    }
}

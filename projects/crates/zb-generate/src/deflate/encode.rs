//! Bit-level DEFLATE encoding for stored and fixed-Huffman blocks.

use zb_types::{ErrorCode, ZbError, ZbResult};

use super::bits::BitWriter;

/// One opcode inside a fixed-Huffman block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedOp {
    /// Literal byte `0..=255`.
    Literal(u8),
    /// LZ77 match.
    Repeat {
        /// Match length in `3..=258`.
        length: u16,
        /// Back-reference distance in `1..=32768`.
        distance: u16,
    },
}

/// Encode a complete DEFLATE stream of stored and/or fixed blocks.
pub fn encode_blocks(blocks: &[DeflateBlock]) -> ZbResult<Vec<u8>> {
    if blocks.is_empty() {
        return Err(ZbError::new(
            ErrorCode::Internal,
            "DEFLATE stream requires at least one block",
        ));
    }
    let mut w = BitWriter::new();
    for (i, block) in blocks.iter().enumerate() {
        let is_final = i + 1 == blocks.len();
        match block {
            DeflateBlock::Stored(data) => encode_stored(&mut w, data, is_final)?,
            DeflateBlock::Fixed(ops) => encode_fixed(&mut w, ops, is_final)?,
        }
    }
    Ok(w.finish())
}

/// Encode a skeleton with an explicit final flag for every block.
///
/// Recursive templates need this because a quoted fragment can contain a
/// non-final block even when it is the last block in the fragment. The regular
/// [`encode_blocks`] API deliberately keeps the safe ordinary-stream rule that
/// only the last block is final.
pub fn encode_blocks_with_finality(blocks: &[(DeflateBlock, bool)]) -> ZbResult<Vec<u8>> {
    if blocks.is_empty() {
        return Err(ZbError::new(
            ErrorCode::Internal,
            "DEFLATE stream requires at least one block",
        ));
    }
    let mut w = BitWriter::new();
    for (block, is_final) in blocks {
        match block {
            DeflateBlock::Stored(data) => encode_stored(&mut w, data, *is_final)?,
            DeflateBlock::Fixed(ops) => encode_fixed(&mut w, ops, *is_final)?,
        }
    }
    Ok(w.finish())
}

/// One DEFLATE block (finality is assigned by position in [`encode_blocks`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeflateBlock {
    /// Stored / uncompressed block (`BTYPE=00`).
    Stored(Vec<u8>),
    /// Fixed Huffman block (`BTYPE=01`).
    Fixed(Vec<FixedOp>),
}

/// Encode a single stored block as a complete one-block stream.
pub fn encode_stored_stream(payload: &[u8]) -> ZbResult<Vec<u8>> {
    encode_blocks(&[DeflateBlock::Stored(payload.to_vec())])
}

/// Encode a single fixed-Huffman block as a complete one-block stream.
pub fn encode_fixed_stream(ops: &[FixedOp]) -> ZbResult<Vec<u8>> {
    encode_blocks(&[DeflateBlock::Fixed(ops.to_vec())])
}

pub(crate) fn encode_stored(w: &mut BitWriter, payload: &[u8], final_block: bool) -> ZbResult<()> {
    if payload.len() > 65535 {
        return Err(ZbError::new(
            ErrorCode::Internal,
            "stored block payload exceeds 65535 bytes",
        ));
    }
    w.write_bits(u32::from(final_block), 1);
    w.write_bits(0b00, 2);
    w.align_byte();
    let len = payload.len() as u16;
    let nlen = !len;
    w.write_bytes_aligned(&len.to_le_bytes());
    w.write_bytes_aligned(&nlen.to_le_bytes());
    w.write_bytes_aligned(payload);
    Ok(())
}

pub(crate) fn encode_fixed(w: &mut BitWriter, ops: &[FixedOp], final_block: bool) -> ZbResult<()> {
    w.write_bits(u32::from(final_block), 1);
    w.write_bits(0b01, 2);
    for op in ops {
        match *op {
            FixedOp::Literal(b) => write_literal(w, b),
            FixedOp::Repeat { length, distance } => {
                write_length(w, length)?;
                write_distance(w, distance)?;
            }
        }
    }
    write_lit_code(w, 256); // EOB
    Ok(())
}

fn write_literal(w: &mut BitWriter, lit: u8) {
    write_lit_code(w, u16::from(lit));
}

fn write_lit_code(w: &mut BitWriter, sym: u16) {
    // RFC 1951 §3.2.6 fixed codes.
    if sym <= 143 {
        let code = u32::from(sym) + 0b0011_0000;
        w.write_huffman(code, 8);
    } else if sym <= 255 {
        let code = u32::from(sym - 144) + 0b1_1001_0000;
        w.write_huffman(code, 9);
    } else if sym <= 279 {
        let code = u32::from(sym - 256);
        w.write_huffman(code, 7);
    } else {
        let code = u32::from(sym - 280) + 0b1100_0000;
        w.write_huffman(code, 8);
    }
}

fn write_length(w: &mut BitWriter, length: u16) -> ZbResult<()> {
    let (code, extra_bits, extra) = length_code(length)?;
    write_lit_code(w, code);
    if extra_bits > 0 {
        w.write_bits(extra, extra_bits);
    }
    Ok(())
}

fn write_distance(w: &mut BitWriter, distance: u16) -> ZbResult<()> {
    let (code, extra_bits, extra) = distance_code(distance)?;
    w.write_huffman(u32::from(code), 5);
    if extra_bits > 0 {
        w.write_bits(extra, extra_bits);
    }
    Ok(())
}

fn length_code(length: u16) -> ZbResult<(u16, u8, u32)> {
    // (symbol, extra_bits, base)
    const TABLE: &[(u16, u8, u16)] = &[
        (257, 0, 3),
        (258, 0, 4),
        (259, 0, 5),
        (260, 0, 6),
        (261, 0, 7),
        (262, 0, 8),
        (263, 0, 9),
        (264, 0, 10),
        (265, 1, 11),
        (266, 1, 13),
        (267, 1, 15),
        (268, 1, 17),
        (269, 2, 19),
        (270, 2, 23),
        (271, 2, 27),
        (272, 2, 31),
        (273, 3, 35),
        (274, 3, 43),
        (275, 3, 51),
        (276, 3, 59),
        (277, 4, 67),
        (278, 4, 83),
        (279, 4, 99),
        (280, 4, 115),
        (281, 5, 131),
        (282, 5, 163),
        (283, 5, 195),
        (284, 5, 227),
        (285, 0, 258),
    ];
    if !(3..=258).contains(&length) {
        return Err(ZbError::new(
            ErrorCode::Internal,
            format!("DEFLATE length {length} out of 3..=258"),
        ));
    }
    for &(sym, bits, base) in TABLE {
        let max = if bits == 0 {
            base
        } else {
            base + (1u16 << bits) - 1
        };
        if length >= base && length <= max {
            return Ok((sym, bits, u32::from(length - base)));
        }
    }
    Err(ZbError::new(
        ErrorCode::Internal,
        format!("DEFLATE length {length} has no length code"),
    ))
}

fn distance_code(distance: u16) -> ZbResult<(u16, u8, u32)> {
    const TABLE: &[(u16, u8, u16)] = &[
        (0, 0, 1),
        (1, 0, 2),
        (2, 0, 3),
        (3, 0, 4),
        (4, 1, 5),
        (5, 1, 7),
        (6, 2, 9),
        (7, 2, 13),
        (8, 3, 17),
        (9, 3, 25),
        (10, 4, 33),
        (11, 4, 49),
        (12, 5, 65),
        (13, 5, 97),
        (14, 6, 129),
        (15, 6, 193),
        (16, 7, 257),
        (17, 7, 385),
        (18, 8, 513),
        (19, 8, 769),
        (20, 9, 1025),
        (21, 9, 1537),
        (22, 10, 2049),
        (23, 10, 3073),
        (24, 11, 4097),
        (25, 11, 6145),
        (26, 12, 8193),
        (27, 12, 12289),
        (28, 13, 16385),
        (29, 13, 24577),
    ];
    if distance == 0 || distance > 32768 {
        return Err(ZbError::new(
            ErrorCode::Internal,
            format!("DEFLATE distance {distance} out of 1..=32768"),
        ));
    }
    for &(code, bits, base) in TABLE {
        let max = if bits == 0 {
            u32::from(base)
        } else {
            u32::from(base) + (1u32 << bits) - 1
        };
        if u32::from(distance) >= u32::from(base) && u32::from(distance) <= max {
            return Ok((code, bits, u32::from(distance - base)));
        }
    }
    Err(ZbError::new(
        ErrorCode::Internal,
        format!("DEFLATE distance {distance} has no distance code"),
    ))
}

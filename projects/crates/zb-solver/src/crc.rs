//! IEEE CRC-32 and fixed-length affine column construction (Living `05` / `08`).

/// Reflected IEEE CRC-32 polynomial.
pub const CRC32_POLY: u32 = 0xEDB88320;

/// Known test vector: `CRC32(b"123456789")`.
pub const CRC32_CHECK: u32 = 0xCBF4_3926;

const fn crc_table() -> [u32; 256] {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut state = i as u32;
        let mut bit = 0;
        while bit < 8 {
            state = if state & 1 != 0 {
                (state >> 1) ^ CRC32_POLY
            } else {
                state >> 1
            };
            bit += 1;
        }
        table[i] = state;
        i += 1;
    }
    table
}

const CRC_TABLE: [u32; 256] = crc_table();

/// Compute IEEE CRC-32 (init/final XOR `0xffffffff`).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in bytes {
        crc = crc_state_byte(crc, b);
    }
    !crc
}

/// Build the affine CRC residual for a fixed-length byte string with unknown slots.
///
/// `template[i] == None` means variable byte `vars[k]` (in order). The returned
/// 32-bit residual is `CRC(template_with_zeros) XOR target`, and each column is
/// `CRC(unit_bit) XOR CRC(zeros)` so `F(x) = F(0) XOR M x` holds over GF(2).
pub fn crc32_affine_columns(template: &[Option<u8>], target: u32) -> (u32, Vec<[bool; 32]>) {
    let var_count = template.iter().filter(|b| b.is_none()).count() * 8;
    let mut zeros = vec![0u8; template.len()];
    for (i, slot) in template.iter().enumerate() {
        if let Some(b) = slot {
            zeros[i] = *b;
        }
    }
    let f0 = crc32(&zeros) ^ target;
    let mut columns = vec![[false; 32]; var_count];
    // State differences ignore init/final XOR. Exponentiate the zero-byte
    // transition instead of rescanning the entire buffer per variable bit.
    let powers = zero_byte_powers(template.len());
    let mut variable_start = 0;
    for i in 0..template.len() {
        if template[i].is_none() {
            for bit in 0..8u8 {
                let col = shift_state(
                    crc_state_byte(0, 1u8 << bit),
                    template.len() - i - 1,
                    &powers,
                );
                columns[variable_start + usize::from(bit)] = u32_to_bits(col);
            }
            variable_start += 8;
        }
    }
    (f0, columns)
}

fn crc_state_byte(state: u32, byte: u8) -> u32 {
    (state >> 8) ^ CRC_TABLE[((state ^ u32::from(byte)) & 0xff) as usize]
}

fn apply_map(map: [u32; 32], state: u32) -> u32 {
    let mut out = 0;
    for i in 0..32 {
        if state & (1u32 << i) != 0 {
            out ^= map[i];
        }
    }
    out
}

fn zero_byte_powers(length: usize) -> Vec<[u32; 32]> {
    let mut powers = Vec::new();
    let mut map = std::array::from_fn(|i| crc_state_byte(1u32 << i, 0));
    let mut remaining = length;
    while remaining > 0 {
        powers.push(map);
        map = std::array::from_fn(|i| apply_map(map, map[i]));
        remaining >>= 1;
    }
    powers
}

fn shift_state(mut state: u32, mut bytes: usize, powers: &[[u32; 32]]) -> u32 {
    let mut i = 0;
    while bytes > 0 {
        if bytes & 1 != 0 {
            state = apply_map(powers[i], state);
        }
        bytes >>= 1;
        i += 1;
    }
    state
}

/// One occurrence of a shared variable byte in a frozen byte string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrcByteSlot {
    /// Zero-based byte offset.
    pub offset: usize,
    /// Index of the variable byte shared by all its occurrences.
    pub variable: usize,
}

/// Construct CRC columns for repeated variable slots without per-bit rescans.
///
/// All slots are treated as zero for the constant term. Columns use byte
/// variable order, then least-significant bit first. Duplicate offsets and
/// out-of-range variables are rejected. Complexity is one input scan plus
/// O(slot_count * 8 * 32 * log(length)) bit operations, with no unit buffers.
pub fn crc32_shared_columns(
    bytes: &[u8],
    slots: &[CrcByteSlot],
    variable_bytes: usize,
) -> zb_types::ZbResult<(u32, Vec<u32>)> {
    use zb_types::{ErrorCode, ZbError};
    let mut ordered = slots.to_vec();
    ordered.sort_by_key(|slot| slot.offset);
    if ordered
        .iter()
        .any(|s| s.offset >= bytes.len() || s.variable >= variable_bytes)
        || ordered.windows(2).any(|s| s[0].offset == s[1].offset)
    {
        return Err(ZbError::new(
            ErrorCode::Internal,
            "invalid or overlapping CRC variable slots",
        ));
    }
    let column_count = variable_bytes
        .checked_mul(8)
        .ok_or_else(|| ZbError::resource_limited("CRC column count overflow"))?;
    let mut state = u32::MAX;
    let mut cursor = 0;
    for (offset, &byte) in bytes.iter().enumerate() {
        if ordered
            .get(cursor)
            .is_some_and(|slot| slot.offset == offset)
        {
            state = crc_state_byte(state, 0);
            cursor += 1;
        } else {
            state = crc_state_byte(state, byte);
        }
    }
    let powers = zero_byte_powers(bytes.len());
    let mut columns = vec![0; column_count];
    for slot in &ordered {
        for bit in 0..8 {
            columns[slot.variable * 8 + bit] ^= shift_state(
                crc_state_byte(0, 1u8 << bit),
                bytes.len() - slot.offset - 1,
                &powers,
            );
        }
    }
    Ok((!state, columns))
}

/// Pack a `u32` residual into 32 GF(2) RHS bits (LSB first).
pub fn u32_to_bits(value: u32) -> [bool; 32] {
    let mut bits = [false; 32];
    for b in 0..32 {
        bits[b] = (value >> b) & 1 == 1;
    }
    bits
}

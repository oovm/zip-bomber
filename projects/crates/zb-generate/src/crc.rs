//! CRC32 planning and IEEE CRC-32 affine helpers (Living `05` §5).

use zb_types::{CrcStrategy, GenerateRequest};

/// Placeholder CRC plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrcPlan {
    /// Strategy selected by the request.
    pub strategy: CrcStrategy,
}

/// Plan CRC resolution. Template echoes strategy only.
pub fn plan(request: &GenerateRequest) -> CrcPlan {
    CrcPlan {
        strategy: request.crc_strategy,
    }
}

/// Reflected IEEE CRC-32 polynomial.
pub const CRC32_POLY: u32 = 0xEDB88320;

/// Known test vector: `CRC32(b"123456789")`.
pub const CRC32_CHECK: u32 = 0xCBF4_3926;

/// Compute IEEE CRC-32 (init/final XOR `0xffffffff`).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC32_POLY;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Build the affine CRC residual for a fixed-length byte string with unknown slots.
///
/// `template[i] == None` means variable byte `vars[k]` (in order). The returned
/// 32-bit residual is `CRC(template_with_zeros) XOR target`, and each column is
/// `CRC(unit_bit) XOR CRC(zeros)` so `F(x) = F(0) XOR M x` holds over GF(2).
pub fn crc32_affine_columns(
    template: &[Option<u8>],
    target: u32,
) -> (u32 /* F(0) residual vs target */, Vec<[bool; 32]>) {
    let var_count = template.iter().filter(|b| b.is_none()).count() * 8;
    let mut zeros = vec![0u8; template.len()];
    for (i, slot) in template.iter().enumerate() {
        if let Some(b) = slot {
            zeros[i] = *b;
        }
    }
    let f0 = crc32(&zeros) ^ target;
    let mut columns = Vec::with_capacity(var_count);
    let mut var_index = 0usize;
    for (i, slot) in template.iter().enumerate() {
        if slot.is_some() {
            continue;
        }
        for bit in 0..8u8 {
            let mut unit = zeros.clone();
            unit[i] = 1u8 << bit;
            let col = crc32(&unit) ^ crc32(&zeros);
            let mut bits = [false; 32];
            for b in 0..32 {
                bits[b] = (col >> b) & 1 == 1;
            }
            columns.push(bits);
            var_index += 1;
            let _ = var_index;
        }
    }
    (f0, columns)
}

/// Pack a `u32` residual into 32 GF(2) RHS bits (LSB first).
pub fn u32_to_bits(value: u32) -> [bool; 32] {
    let mut bits = [false; 32];
    for b in 0..32 {
        bits[b] = (value >> b) & 1 == 1;
    }
    bits
}

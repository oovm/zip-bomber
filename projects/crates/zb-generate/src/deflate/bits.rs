//! LSB-first bit packing for DEFLATE (RFC 1951 §3.1.1).

/// Accumulates DEFLATE bits into a byte stream.
#[derive(Debug, Default, Clone)]
pub struct BitWriter {
    buf: Vec<u8>,
    /// Bits already written into the current incomplete byte (`0..=7`).
    bit: u8,
    /// Partial byte being filled from the LSB upward.
    cur: u8,
}

impl BitWriter {
    /// Create an empty writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bytes written so far (excluding an unfinished partial byte).
    pub fn byte_len(&self) -> usize {
        self.buf.len() + if self.bit > 0 { 1 } else { 0 }
    }

    /// Number of meaningful bits, excluding final byte padding.
    pub(crate) fn bit_len(&self) -> usize {
        self.buf.len() * 8 + usize::from(self.bit)
    }

    /// Write `nbits` from the low end of `value` (LSB of `value` first).
    pub fn write_bits(&mut self, mut value: u32, mut nbits: u8) {
        while nbits > 0 {
            let space = 8 - self.bit;
            let take = nbits.min(space);
            let mask = (1u32 << take) - 1;
            let chunk = (value & mask) as u8;
            self.cur |= chunk << self.bit;
            self.bit += take;
            value >>= take;
            nbits -= take;
            if self.bit == 8 {
                self.buf.push(self.cur);
                self.cur = 0;
                self.bit = 0;
            }
        }
    }

    /// Write a Huffman code of `nbits`, starting with the most significant bit.
    pub fn write_huffman(&mut self, code: u32, nbits: u8) {
        for i in (0..nbits).rev() {
            self.write_bits((code >> i) & 1, 1);
        }
    }

    /// Pad with zero bits to the next byte boundary.
    pub fn align_byte(&mut self) {
        if self.bit != 0 {
            self.buf.push(self.cur);
            self.cur = 0;
            self.bit = 0;
        }
    }

    /// Finish and return the encoded bytes (padding the final byte with zeros).
    pub fn finish(mut self) -> Vec<u8> {
        self.align_byte();
        self.buf
    }

    /// Append raw bytes after aligning to a byte boundary.
    pub fn write_bytes_aligned(&mut self, bytes: &[u8]) {
        self.align_byte();
        self.buf.extend_from_slice(bytes);
    }
}

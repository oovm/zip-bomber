//! Abstract Lempel-Ziv self-reproduction (Cox, Living `10` §10).
//!
//! Opcodes are one logical unit each. `L(n)` emits the next `n` program units as
//! literals; `R(n)` copies the last `n` emitted units. The classic core program
//! decompresses to itself.

use zb_types::{ErrorCode, ZbError, ZbResult};

/// One abstract LZ opcode (unit-sized).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LzOp {
    /// Literal of the following `n` program units (`n == 0` is a no-op).
    Literal(u8),
    /// Repeat the last `n` emitted units (`distance == length == n`).
    Repeat(u8),
}

/// Cox's classic self-reproducing program (unit size 1).
///
/// Layout matches the article table whose Code and Output columns are identical.
pub fn cox_core_program() -> Vec<u8> {
    // Encoded as a flat byte stream where:
    //   0x00..=0x40  => Literal(n) with n in the low 6 bits (data follows)
    //   0x80 | n     => Repeat(n)
    // This is an internal test encoding, not DEFLATE.
    let mut p = Vec::new();
    // L0 L0 L0
    p.extend_from_slice(&[lit(0), lit(0), lit(0)]);
    // L4 + four data bytes (L0 L0 L0 L4)
    p.push(lit(4));
    p.extend_from_slice(&[lit(0), lit(0), lit(0), lit(4)]);
    // R4
    p.push(rep(4));
    // L4 + four data bytes (R4 L4 R4 L4)
    p.push(lit(4));
    p.extend_from_slice(&[rep(4), lit(4), rep(4), lit(4)]);
    // R4
    p.push(rep(4));
    // L4 + four data bytes (L0 L0 L0 L0)
    p.push(lit(4));
    p.extend_from_slice(&[lit(0), lit(0), lit(0), lit(0)]);
    p
}

const fn lit(n: u8) -> u8 {
    n & 0x3f
}

const fn rep(n: u8) -> u8 {
    0x80 | (n & 0x3f)
}

/// Interpret a Cox-style flat program into output units.
pub fn interpret(program: &[u8]) -> ZbResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < program.len() {
        let op = program[i];
        i += 1;
        if op & 0x80 != 0 {
            let n = usize::from(op & 0x3f);
            if n == 0 {
                return Err(ZbError::new(ErrorCode::Internal, "abstract Repeat(0) is invalid"));
            }
            if n > out.len() {
                return Err(ZbError::new(ErrorCode::Internal, format!("abstract Repeat({n}) exceeds emitted {}", out.len())));
            }
            for _ in 0..n {
                let b = out[out.len() - n];
                out.push(b);
            }
        }
        else {
            let n = usize::from(op & 0x3f);
            if i + n > program.len() {
                return Err(ZbError::new(ErrorCode::Internal, format!("abstract Literal({n}) overruns program")));
            }
            out.extend_from_slice(&program[i..i + n]);
            i += n;
        }
    }
    Ok(out)
}

/// Whether `program` is a pure LZ quine under [`interpret`].
pub fn is_self_reproducing(program: &[u8]) -> ZbResult<bool> {
    Ok(interpret(program)? == program)
}

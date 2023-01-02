use std::io::Read;

use flate2::read::DeflateDecoder;
use zb_generate::{
    encode_blocks, encode_fixed_stream, encode_stored_stream, DeflateBlock, FixedOp,
};

fn inflate(raw: &[u8]) -> Vec<u8> {
    let mut decoder = DeflateDecoder::new(raw);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).expect("inflate");
    out
}

#[test]
fn stored_stream_roundtrips_via_flate2() {
    let payload = b"hello-zb-stored";
    let raw = encode_stored_stream(payload).unwrap();
    assert_eq!(inflate(&raw), payload);
}

#[test]
fn fixed_literals_roundtrip() {
    let ops: Vec<FixedOp> = b"abcXYZ"
        .iter()
        .copied()
        .map(FixedOp::Literal)
        .collect();
    let raw = encode_fixed_stream(&ops).unwrap();
    assert_eq!(inflate(&raw), b"abcXYZ");
}

#[test]
fn fixed_repeat_roundtrip() {
    let ops = vec![
        FixedOp::Literal(b'a'),
        FixedOp::Literal(b'b'),
        FixedOp::Repeat {
            length: 4,
            distance: 2,
        },
    ];
    let raw = encode_fixed_stream(&ops).unwrap();
    assert_eq!(inflate(&raw), b"ababab");
}

#[test]
fn stored_then_fixed_multi_block() {
    let raw = encode_blocks(&[
        DeflateBlock::Stored(b"pre".to_vec()),
        DeflateBlock::Fixed(vec![
            FixedOp::Literal(b'-'),
            FixedOp::Literal(b'z'),
        ]),
    ])
    .unwrap();
    assert_eq!(inflate(&raw), b"pre-z");
}

#[test]
fn length_258_and_distance_edge() {
    let mut ops = vec![FixedOp::Literal(b'x')];
    ops.push(FixedOp::Repeat {
        length: 258,
        distance: 1,
    });
    let raw = encode_fixed_stream(&ops).unwrap();
    let out = inflate(&raw);
    assert_eq!(out.len(), 259);
    assert!(out.iter().all(|&b| b == b'x'));
}

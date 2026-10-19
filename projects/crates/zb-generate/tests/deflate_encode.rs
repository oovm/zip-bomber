use zb_generate::{
    DeflateBlock, DeflateBlockPlan, FixedBlock, StoredBlock, Symbol, decode_raw_deflate, encode_fixed_stream,
    encode_stored_stream,
};

fn inflate(raw: &[u8]) -> Vec<u8> {
    decode_raw_deflate(raw, 16 * 1024 * 1024).expect("inflate")
}

#[test]
fn stored_stream_roundtrips_via_acorn_codec() {
    let payload = b"hello-zb-stored";
    let raw = encode_stored_stream(payload).unwrap();
    assert_eq!(inflate(&raw), payload);
}

#[test]
fn fixed_literals_roundtrip() {
    let symbols: Vec<Symbol> = b"abcXYZ".iter().copied().map(Symbol::Literal).collect();
    let raw = encode_fixed_stream(&symbols).unwrap();
    assert_eq!(inflate(&raw), b"abcXYZ");
}

#[test]
fn fixed_repeat_roundtrip() {
    let symbols = vec![Symbol::Literal(b'a'), Symbol::Literal(b'b'), Symbol::Match { length: 4, distance: 2 }];
    let raw = encode_fixed_stream(&symbols).unwrap();
    assert_eq!(inflate(&raw), b"ababab");
}

#[test]
fn stored_then_fixed_multi_block() {
    let mut plan = DeflateBlockPlan::new();
    plan.push(DeflateBlock::Stored(StoredBlock::new(b"pre".to_vec())), false);
    plan.push(DeflateBlock::Fixed(FixedBlock::from_literals(b"-z")), true);
    let raw = plan.encode().unwrap();
    assert_eq!(inflate(&raw), b"pre-z");
}

#[test]
fn fixed_then_stored_multi_block_stays_bit_continuous() {
    let mut plan = DeflateBlockPlan::new();
    plan.push(DeflateBlock::Fixed(FixedBlock::from_literals(b"ab")), false);
    plan.push(DeflateBlock::Stored(StoredBlock::new(b"cd".to_vec())), true);
    let raw = plan.encode().unwrap();
    assert_eq!(inflate(&raw), b"abcd");
}

#[test]
fn length_258_and_distance_edge() {
    let mut symbols = vec![Symbol::Literal(b'x')];
    symbols.push(Symbol::Match { length: 258, distance: 1 });
    let raw = encode_fixed_stream(&symbols).unwrap();
    let out = inflate(&raw);
    assert_eq!(out.len(), 259);
    assert!(out.iter().all(|&b| b == b'x'));
}

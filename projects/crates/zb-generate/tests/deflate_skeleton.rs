use zb_generate::{
    chain_repeats, encode_stored_block, expand_skeleton, SkeletonOp,
};

#[test]
fn expand_repeat_with_overlap() {
    let out = expand_skeleton(&[
        SkeletonOp::StoredLiteral(b'a'),
        SkeletonOp::StoredLiteral(b'b'),
        SkeletonOp::Repeat {
            length: 4,
            distance: 2,
        },
        SkeletonOp::EndOfBlock,
    ])
    .unwrap();
    assert_eq!(out, b"ababab");
}

#[test]
fn reject_short_distance() {
    let err = expand_skeleton(&[SkeletonOp::Repeat {
        length: 3,
        distance: 1,
    }])
    .unwrap_err();
    assert!(err.message.contains("exceeds emitted"));
}

#[test]
fn chain_long_repeat() {
    let ops = chain_repeats(300, 8).unwrap();
    assert_eq!(ops.len(), 2);
    match ops[0] {
        SkeletonOp::Repeat { length, distance } => {
            assert_eq!(length, 258);
            assert_eq!(distance, 8);
        }
        _ => panic!("expected repeat"),
    }
    match ops[1] {
        SkeletonOp::Repeat { length, .. } => assert_eq!(length, 42),
        _ => panic!("expected repeat"),
    }
}

#[test]
fn stored_block_is_legal_raw_deflate() {
    use std::io::Read;
    let block = encode_stored_block(b"hi", true).unwrap();
    // BFINAL=1, BTYPE=00, then padded to byte, then LEN/NLEN + data.
    assert_eq!(block[0] & 0b111, 0b001);
    let mut decoder = flate2::read::DeflateDecoder::new(&block[..]);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).unwrap();
    assert_eq!(out, b"hi");
}

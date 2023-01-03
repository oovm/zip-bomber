use std::io::Read;

use flate2::read::DeflateDecoder;
use zb_generate::encode_cox_core_deflate;

fn inflate(raw: &[u8]) -> Vec<u8> {
    let mut decoder = DeflateDecoder::new(raw);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).expect("inflate");
    out
}

#[test]
fn cox_deflate_core_inflates_to_itself() {
    let stream = encode_cox_core_deflate().expect("encode");
    assert!(
        stream.len() > 100,
        "complete termination must also be quoted"
    );
    assert_eq!(inflate(&stream), stream);
}

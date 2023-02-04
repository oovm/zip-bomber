use zb_generate::{decode_raw_deflate, encode_cox_core_deflate};

#[test]
fn cox_deflate_core_inflates_to_itself() {
    let stream = encode_cox_core_deflate().expect("encode");
    assert!(
        stream.len() > 100,
        "complete termination must also be quoted"
    );
    let decoded = decode_raw_deflate(&stream, stream.len() as u64).expect("inflate");
    assert_eq!(decoded, stream);
}

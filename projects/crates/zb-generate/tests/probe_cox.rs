use zb_generate::{decode_raw_deflate, encode_blocks, DeflateBlock, FixedOp};

fn r4() -> Vec<FixedOp> {
    vec![
        FixedOp::Repeat {
            length: 10,
            distance: 20,
        },
        FixedOp::Repeat {
            length: 10,
            distance: 20,
        },
    ]
}

#[test]
fn appending_terminator_does_not_fix_core_identity() {
    // All Cox body blocks non-final; trailing empty stored is final.
    let data1 = vec![0x11u8; 20];
    let data2 = vec![0x22u8; 20];
    let data3 = vec![0x33u8; 20];
    let stream = encode_blocks(&[
        DeflateBlock::Stored(vec![]),
        DeflateBlock::Stored(vec![]),
        DeflateBlock::Stored(vec![]),
        DeflateBlock::Stored(data1),
        DeflateBlock::Fixed(r4()),
        DeflateBlock::Stored(data2),
        DeflateBlock::Fixed(r4()),
        DeflateBlock::Stored(data3),
        DeflateBlock::Stored(vec![]), // final empty
    ])
    .unwrap();
    eprintln!("len with trailing empty final={}", stream.len());
    eprintln!(
        "hdr at 75..80 (should be nonfinal)={:02x?}",
        &stream[75..80]
    );
    eprintln!("last 5 bytes={:02x?}", &stream[stream.len() - 5..]);
    let out = decode_raw_deflate(&stream, stream.len() as u64).expect("inflate");
    eprintln!("inflate len={}", out.len());
    assert_eq!(out.len(), 100);
    assert_eq!(stream.len(), 105);
    assert_ne!(out, stream, "the final block must be reproduced as well");
}

#[test]
fn asymmetric_core_has_a_constant_finality_conflict() {
    let mut data1 = vec![0u8; 20];
    let mut data2 = vec![0u8; 20];
    let mut data3 = vec![0u8; 20];
    for round in 0..8 {
        let stream = encode_blocks(&[
            DeflateBlock::Stored(vec![]),
            DeflateBlock::Stored(vec![]),
            DeflateBlock::Stored(vec![]),
            DeflateBlock::Stored(data1.clone()),
            DeflateBlock::Fixed(r4()),
            DeflateBlock::Stored(data2.clone()),
            DeflateBlock::Fixed(r4()),
            DeflateBlock::Stored(data3.clone()),
        ])
        .unwrap();
        assert_eq!(stream.len(), 100);
        let l0 = stream[0..5].to_vec();
        let l4_nf = stream[15..20].to_vec();
        let r4b = stream[40..45].to_vec();
        let l4_f = stream[75..80].to_vec();
        data1 = [&l0[..], &l0[..], &l0[..], &l4_nf[..]].concat();
        data2 = [&r4b[..], &l4_nf[..], &r4b[..], &l4_f[..]].concat();
        data3 = l0.repeat(4);
        let cand = encode_blocks(&[
            DeflateBlock::Stored(vec![]),
            DeflateBlock::Stored(vec![]),
            DeflateBlock::Stored(vec![]),
            DeflateBlock::Stored(data1.clone()),
            DeflateBlock::Fixed(r4()),
            DeflateBlock::Stored(data2.clone()),
            DeflateBlock::Fixed(r4()),
            DeflateBlock::Stored(data3.clone()),
        ])
        .unwrap();
        let out = decode_raw_deflate(&cand, cand.len() as u64).expect("inflate");
        eprintln!(
            "round {round}: inflate==cand {} (inflen={})",
            out == cand,
            out.len()
        );
        assert_ne!(out, cand, "this asymmetric skeleton is unsatisfiable");
        assert_eq!(out[55], 1);
        assert_eq!(cand[55], 0);
        if out.len() == cand.len() {
            for i in 0..out.len() {
                if out[i] != cand[i] {
                    eprintln!(" diff @{i}: out={:02x} cand={:02x}", out[i], cand[i]);
                    break;
                }
            }
        }
    }
}

use zb_generate::{DeflateBlock, DeflateBlockPlan, FixedBlock, StoredBlock, Symbol, decode_raw_deflate};

fn r4() -> DeflateBlock {
    DeflateBlock::Fixed(FixedBlock::new(vec![
        Symbol::Match { length: 10, distance: 20 },
        Symbol::Match { length: 10, distance: 20 },
    ]))
}

/// Blocks are written back to back, so every entry carries its own finality.
fn plan(blocks: &[(DeflateBlock, bool)]) -> Vec<u8> {
    let mut plan = DeflateBlockPlan::new();
    for (block, final_block) in blocks {
        plan.push(block.clone(), *final_block);
    }
    plan.encode().expect("encode")
}

fn stored(data: &[u8]) -> DeflateBlock {
    DeflateBlock::Stored(StoredBlock::new(data.to_vec()))
}

#[test]
fn appending_terminator_does_not_fix_core_identity() {
    // All Cox body blocks non-final; trailing empty stored is final.
    let data1 = vec![0x11u8; 20];
    let data2 = vec![0x22u8; 20];
    let data3 = vec![0x33u8; 20];
    let stream = plan(&[
        (stored(&[]), false),
        (stored(&[]), false),
        (stored(&[]), false),
        (stored(&data1), false),
        (r4(), false),
        (stored(&data2), false),
        (r4(), false),
        (stored(&data3), false),
        (stored(&[]), true), // final empty
    ]);
    eprintln!("len with trailing empty final={}", stream.len());
    eprintln!("hdr at 75..80 (should be nonfinal)={:02x?}", &stream[75..80]);
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
        let stream = plan(&[
            (stored(&[]), false),
            (stored(&[]), false),
            (stored(&[]), false),
            (stored(&data1), false),
            (r4(), false),
            (stored(&data2), false),
            (r4(), false),
            (stored(&data3), true),
        ]);
        assert_eq!(stream.len(), 100);
        let l0 = stream[0..5].to_vec();
        let l4_nf = stream[15..20].to_vec();
        let r4b = stream[40..45].to_vec();
        let l4_f = stream[75..80].to_vec();
        data1 = [&l0[..], &l0[..], &l0[..], &l4_nf[..]].concat();
        data2 = [&r4b[..], &l4_nf[..], &r4b[..], &l4_f[..]].concat();
        data3 = l0.repeat(4);
        let cand = plan(&[
            (stored(&[]), false),
            (stored(&[]), false),
            (stored(&[]), false),
            (stored(&data1), false),
            (r4(), false),
            (stored(&data2), false),
            (r4(), false),
            (stored(&data3), true),
        ]);
        let out = decode_raw_deflate(&cand, cand.len() as u64).expect("inflate");
        eprintln!("round {round}: inflate==cand {} (inflen={})", out == cand, out.len());
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

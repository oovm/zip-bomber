//! Compare shared CRC influence propagation against full-buffer unit scans.

use std::{hint::black_box, time::Instant};
use zb_solver::{crc32, crc32_shared_columns, CrcByteSlot};

fn reference(bytes: &[u8], slots: &[CrcByteSlot]) -> (u32, Vec<u32>) {
    let mut zero = bytes.to_vec();
    for slot in slots {
        zero[slot.offset] = 0;
    }
    let base = crc32(&zero);
    let columns = (0..32)
        .map(|bit| {
            let mut unit = zero.clone();
            for slot in slots {
                if slot.variable == bit / 8 {
                    unit[slot.offset] = 1 << (bit % 8);
                }
            }
            crc32(&unit) ^ base
        })
        .collect();
    (base, columns)
}

fn main() {
    let bytes: Vec<_> = (0..1048576).map(|i| (i % 251) as u8).collect();
    let slots: Vec<_> = [0, 4096, 524288, 1048572]
        .into_iter()
        .flat_map(|offset| {
            (0..4).map(move |variable| CrcByteSlot {
                offset: offset + variable,
                variable,
            })
        })
        .collect();
    assert_eq!(
        reference(&bytes, &slots),
        crc32_shared_columns(&bytes, &slots, 4).unwrap()
    );
    let rounds = 5;
    let start = Instant::now();
    for _ in 0..rounds {
        black_box(reference(black_box(&bytes), black_box(&slots)));
    }
    let reference_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);
    let start = Instant::now();
    for _ in 0..rounds {
        black_box(crc32_shared_columns(black_box(&bytes), black_box(&slots), 4).unwrap());
    }
    let propagated_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);
    println!("input_bytes={} variable_bits=32 occurrences=4 rounds={rounds} reference_ms={reference_ms:.3} propagated_ms={propagated_ms:.3} speedup={:.2}", bytes.len(), reference_ms / propagated_ms);
}

//! Simple performance probes for zb generation and CRC solving.

use std::{hint::black_box, time::Instant};

use zb_generate::{generate, plan_generate};
use zb_solver::{CrcByteSlot, crc32, crc32_shared_columns};
use zb_types::{GenerateRequest, Outcome, PayloadSpec, TopologyGraph, VariantKind};

fn mean_ms(samples: &[f64]) -> f64 {
    samples.iter().sum::<f64>() / samples.len() as f64
}

fn timed_ms(rounds: u32, mut body: impl FnMut()) -> f64 {
    let start = Instant::now();
    for _ in 0..rounds {
        body();
    }
    start.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds)
}

fn bench_crc_shared_columns(rounds: u32) {
    let bytes: Vec<_> = (0..1_048_576).map(|i| (i % 251) as u8).collect();
    let slots: Vec<_> = [0usize, 4096, 524_288, 1_048_572]
        .into_iter()
        .flat_map(|offset| (0..4).map(move |variable| CrcByteSlot { offset: offset + variable, variable }))
        .collect();

    let reference = |input: &[u8], slots: &[CrcByteSlot]| {
        let mut zero = input.to_vec();
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
            .collect::<Vec<_>>();
        (base, columns)
    };

    let expected = reference(&bytes, &slots);
    let got = crc32_shared_columns(&bytes, &slots, 4).expect("crc columns");
    assert_eq!(expected, got, "CRC column probe mismatch");

    let reference_ms = timed_ms(rounds, || {
        black_box(reference(black_box(&bytes), black_box(&slots)));
    });
    let propagated_ms = timed_ms(rounds, || {
        black_box(crc32_shared_columns(black_box(&bytes), black_box(&slots), 4).unwrap());
    });
    println!(
        "bench=crc_shared_columns input_bytes={} variable_bits=32 occurrences=4 rounds={} reference_ms={:.3} propagated_ms={:.3} speedup={:.2}",
        bytes.len(),
        rounds,
        reference_ms,
        propagated_ms,
        reference_ms / propagated_ms
    );
}

fn attachment_request() -> GenerateRequest {
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.topology = TopologyGraph::quine_self("release.zip", "recurse/again.zip");
    request.payloads = vec![
        PayloadSpec { path: "readme.txt".into(), bytes: b"zb-benchmark attachment probe\n".to_vec() },
        PayloadSpec { path: "assets/说明.txt".into(), bytes: "中文附件".as_bytes().to_vec() },
        PayloadSpec { path: "empty/".into(), bytes: Vec::new() },
        PayloadSpec { path: "metadata/config.json".into(), bytes: br#"{"bench":true}"#.to_vec() },
    ];
    request
}

fn bench_quine_bare(rounds: u32) {
    let request = GenerateRequest::bare(VariantKind::Quine);
    let mut sizes = Vec::new();
    let ms = timed_ms(rounds, || match generate(request.clone()) {
        Outcome::Ok { value, .. } => {
            sizes.push(value.artifacts[0].bytes.len());
            black_box(value);
        }
        Outcome::Err { error, .. } => panic!("bare quine failed: {error}"),
    });
    println!(
        "bench=quine_bare rounds={} mean_ms={:.3} artifact_bytes={} mean_artifact_bytes={:.1}",
        rounds,
        ms,
        sizes.first().copied().unwrap_or(0),
        mean_ms(&sizes.iter().map(|n| *n as f64).collect::<Vec<_>>())
    );
}

fn bench_quine_attachments(rounds: u32) {
    let request = attachment_request();
    match plan_generate(request.clone()) {
        Outcome::Ok { value, .. } => {
            println!("bench=quine_attachments_plan feasible={} summary={}", value.feasible, value.summary);
        }
        Outcome::Err { error, .. } => panic!("attachment quine plan failed: {error}"),
    }

    let mut sizes = Vec::new();
    let ms = timed_ms(rounds, || match generate(request.clone()) {
        Outcome::Ok { value, .. } => {
            sizes.push(value.artifacts[0].bytes.len());
            black_box(value);
        }
        Outcome::Err { error, .. } => panic!("attachment quine failed: {error}"),
    });
    println!(
        "bench=quine_attachments rounds={} mean_ms={:.3} artifact_bytes={} ordinary_payloads=4",
        rounds,
        ms,
        sizes.first().copied().unwrap_or(0)
    );
}

fn main() {
    let rounds: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    println!("zb-benchmark rounds={rounds}");
    bench_crc_shared_columns(rounds);
    bench_quine_bare(rounds);
    bench_quine_attachments(rounds);
}

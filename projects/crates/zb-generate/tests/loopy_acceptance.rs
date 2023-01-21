use zb_generate::generate;
use zb_types::{encode_base64, generate_request_from_json, Outcome};

fn loopy_request(
    path_a: &str,
    path_b: &str,
    file_a: &str,
    file_b: &str,
    payload_a: &[u8],
    payload_b: &[u8],
) -> zb_types::GenerateRequest {
    generate_request_from_json(
        &serde_json::json!({
            "schema":"zb.request/v2",
            "variant":"loopy",
            "profile":"zip.loopy-two-node.gapped/v1",
            "graph":{"root":0,"mode":"recursive","nodes":[
                {"members":[
                    {"kind":"payload","path":file_a,"bytes":encode_base64(payload_a)},
                    {"kind":"archive","path":path_a,"target":1}
                ]},
                {"members":[
                    {"kind":"payload","path":file_b,"bytes":encode_base64(payload_b)},
                    {"kind":"archive","path":path_b,"target":0}
                ]}
            ]}
        })
        .to_string(),
    )
    .unwrap()
}

fn assert_cycle_ok(request: zb_types::GenerateRequest) {
    match generate(request) {
        Outcome::Ok { value, report, .. } => {
            assert_eq!(value.artifacts.len(), 2);
            assert_ne!(value.artifacts[0].bytes, value.artifacts[1].bytes);
            let facts = report.loopy.expect("loopy facts");
            assert_eq!(facts.byte_period, 2);
            assert!(facts.edges.iter().all(|e| e.bytes_equal));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn name_length_deltas_one_two_three() {
    for (a, b) in [
        ("n/x.zip", "n/yy.zip"),
        ("n/x.zip", "n/yyy.zip"),
        ("n/x.zip", "n/yyyy.zip"),
    ] {
        assert_cycle_ok(loopy_request(a, b, "a.txt", "b.txt", b"alpha", b"beta"));
    }
}

#[test]
fn utf8_archive_and_payload_names() {
    assert_cycle_ok(loopy_request(
        "下一/乙.zip",
        "返回/甲.zip",
        "附件/甲.txt",
        "附件/乙.txt",
        "中文附件甲".as_bytes(),
        "中文附件乙".as_bytes(),
    ));
}

#[test]
fn unequal_attachment_lengths() {
    assert_cycle_ok(loopy_request(
        "next/b.zip",
        "back/a.zip",
        "a.txt",
        "b.txt",
        b"short",
        &vec![b'L'; 4096],
    ));
}

#[test]
fn pk_signature_embedded_in_attachment() {
    let mut payload = b"prefix".to_vec();
    payload.extend_from_slice(b"PK\x03\x04");
    payload.extend_from_slice(b"PK\x01\x02");
    payload.extend_from_slice(b"PK\x05\x06");
    payload.extend_from_slice(b"suffix");
    assert_cycle_ok(loopy_request(
        "next/b.zip",
        "back/a.zip",
        "a.txt",
        "b.txt",
        &payload,
        b"other-side",
    ));
}

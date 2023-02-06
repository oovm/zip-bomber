use zb_generate::{generate, plan_generate};
use zb_types::{generate_request_from_json, ErrorCode, Outcome};

fn loopy_with_payload(bytes_b64: &str) -> zb_types::GenerateRequest {
    generate_request_from_json(
        &serde_json::json!({
            "schema":"zb.request/v2",
            "variant":"loopy",
            "profile":"zip.loopy-two-node.gapped/v1",
            "graph":{"root":0,"mode":"recursive","nodes":[
                {"members":[
                    {"kind":"payload","path":"big.bin","bytes":bytes_b64},
                    {"kind":"archive","path":"next/b.zip","target":1}
                ]},
                {"members":[
                    {"kind":"payload","path":"note.txt","bytes":"bm90ZQ=="},
                    {"kind":"archive","path":"back/a.zip","target":0}
                ]}
            ]}
        })
        .to_string(),
    )
    .unwrap()
}

fn assert_quote_window(error: &zb_types::ZbError) {
    assert_eq!(error.code, ErrorCode::ResourceLimited);
    assert!(
        error.message.contains("quote layout") || error.message.contains("window"),
        "{}",
        error.message
    );
}

#[test]
fn oversize_ordinary_member_rejects_quote_window() {
    // Stored local for ~20 KiB exceeds the two-node quote window (`QUOTE_WINDOW`).
    let payload = vec![0u8; 20 * 1024];
    let b64 = zb_types::encode_base64(&payload);
    let request = loopy_with_payload(&b64);
    match plan_generate(request.clone()) {
        Outcome::Err { error, .. } => assert_quote_window(&error),
        other => panic!("plan expected resource_limited, got {other:?}"),
    }
    match generate(request) {
        Outcome::Err { error, .. } => assert_quote_window(&error),
        other => panic!("generate expected resource_limited, got {other:?}"),
    }
}

fn plan_ok(size: usize) -> bool {
    let request = loopy_with_payload(&zb_types::encode_base64(&vec![0u8; size]));
    matches!(plan_generate(request), Outcome::Ok { .. })
}

#[test]
fn quote_and_tail_width_boundary_is_tight() {
    // Binary-search the largest ordinary payload that still plans (window + bounded tail width).
    let mut lo = 0usize;
    let mut hi = 20 * 1024;
    assert!(!plan_ok(hi));
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if plan_ok(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    assert!(plan_ok(lo), "expected size {lo} to fit template layout");
    assert!(!plan_ok(lo + 1), "expected size {} to fail plan", lo + 1);
    match generate(loopy_with_payload(&zb_types::encode_base64(&vec![0u8; lo]))) {
        Outcome::Ok { value, .. } => assert_eq!(value.artifacts.len(), 2),
        other => panic!("boundary generate should succeed, got {other:?}"),
    }
    match plan_generate(loopy_with_payload(&zb_types::encode_base64(&vec![0u8; lo + 1]))) {
        Outcome::Err { error, .. } => {
            assert!(
                matches!(
                    error.code,
                    ErrorCode::ResourceLimited | ErrorCode::SearchExhausted
                ),
                "{error:?}"
            );
        }
        other => panic!("expected layout failure, got {other:?}"),
    }
}

#[test]
fn low_decode_budget_rejects_two_node_layout() {
    let mut request = loopy_with_payload("c2hvcnQ=");
    request.limits.max_decoded_bytes = 64;
    match plan_generate(request.clone()) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.code, ErrorCode::ResourceLimited);
            assert!(
                error.message.contains("budget") || error.message.contains("layout"),
                "{}",
                error.message
            );
        }
        other => panic!("plan expected resource_limited, got {other:?}"),
    }
    match generate(request) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.code, ErrorCode::ResourceLimited);
            assert!(
                error.message.contains("budget") || error.message.contains("layout"),
                "{}",
                error.message
            );
        }
        other => panic!("generate expected resource_limited, got {other:?}"),
    }
}

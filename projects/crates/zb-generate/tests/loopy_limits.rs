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

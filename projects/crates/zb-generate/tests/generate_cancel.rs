use zb_generate::{CancelToken, generate_with_cancel};
use zb_types::{ErrorCategory, GenerateRequest, VariantKind, generate_request_from_json};

#[test]
fn pre_cancelled_quine_generate_returns_cancelled() {
    let request = GenerateRequest::bare(VariantKind::Quine);
    let token = CancelToken::new();
    token.cancel();
    match generate_with_cancel(request, &token) {
        zb_types::Outcome::Err { error, .. } => {
            assert_eq!(error.category, ErrorCategory::Cancelled);
            assert_eq!(error.code.as_str(), "ZB_CANCELLED");
        }
        other => panic!("expected cancelled, got {other:?}"),
    }
}

#[test]
fn pre_cancelled_finite_graph_generate_returns_cancelled() {
    let request = generate_request_from_json(
        &serde_json::json!({
            "schema":"zb.request/v0",
            "variant":"fractal",
            "profile":"zip.profile/v0-stub",
            "graph":{"root":0,"mode":"finite","depth":"2","nodes":[
                {"members":[{"kind":"payload","path":"readme.txt","bytes":"YWJj"},{"kind":"archive","path":"next/b.zip","target":1}],"terminal":[]},
                {"members":[{"kind":"payload","path":"x.txt","bytes":"ZGVmZw=="},{"kind":"archive","path":"back/a.zip","target":0}],"terminal":[]}
            ]}
        })
        .to_string(),
    )
    .unwrap();
    let token = CancelToken::new();
    token.cancel();
    match generate_with_cancel(request, &token) {
        zb_types::Outcome::Err { error, .. } => {
            assert_eq!(error.category, ErrorCategory::Cancelled);
            assert_eq!(error.code.as_str(), "ZB_CANCELLED");
        }
        other => panic!("expected cancelled, got {other:?}"),
    }
}

#[test]
fn pre_cancelled_loopy_generate_returns_cancelled() {
    let request = generate_request_from_json(
        &serde_json::json!({
            "schema":"zb.request/v0",
            "variant":"loopy",
            "profile":"zip.loopy-two-node.gapped/v0",
            "graph":{"root":0,"mode":"recursive","nodes":[
                {"members":[
                    {"kind":"payload","path":"a.txt","bytes":"YWJj"},
                    {"kind":"archive","path":"next/b.zip","target":1}
                ]},
                {"members":[
                    {"kind":"payload","path":"b.txt","bytes":"ZGVm"},
                    {"kind":"archive","path":"back/a.zip","target":0}
                ]}
            ]}
        })
        .to_string(),
    )
    .unwrap();
    let token = CancelToken::new();
    token.cancel();
    match generate_with_cancel(request, &token) {
        zb_types::Outcome::Err { error, .. } => {
            assert_eq!(error.category, ErrorCategory::Cancelled);
        }
        other => panic!("expected cancelled, got {other:?}"),
    }
}

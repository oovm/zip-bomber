use zb_generate::{generate, minimum_byte_period, verify_artifact_set_for_request, verify_artifact_set_with_spec};
use zb_types::{ErrorCategory, ErrorCode, Outcome, VerificationSpec, VerifyEdgeSpec, generate_request_from_json};

fn two_node_loopy() -> zb_types::GenerateRequest {
    generate_request_from_json(
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
    .unwrap()
}

#[test]
fn minimum_period_detects_collapse_and_full_cycle() {
    assert_eq!(minimum_byte_period(&[b"x", b"x"]), 1);
    assert_eq!(minimum_byte_period(&[b"a", b"b"]), 2);
    assert_eq!(minimum_byte_period(&[b"a", b"b", b"a", b"b"]), 2);
    assert_eq!(minimum_byte_period(&[b"a", b"b", b"c"]), 3);
}

#[test]
fn loopy_artifact_set_verifies_edges_and_period() {
    let request = two_node_loopy();
    let Outcome::Ok { value, report, .. } = generate(request.clone())
    else {
        panic!("generate must succeed");
    };
    assert!(report.loopy.as_ref().unwrap().edges.iter().all(|e| e.bytes_equal));

    let verified = verify_artifact_set_for_request(&value, &request).expect("cycle verify");
    assert!(verified.ok);
    let cycle = verified.cycle.expect("cycle facts");
    assert_eq!(cycle.byte_period, 2);
    assert_eq!(cycle.expected_byte_period, Some(2));
    assert_eq!(cycle.edges.len(), 2);
    assert!(cycle.edges.iter().all(|e| e.bytes_equal));
}

#[test]
fn wrong_expected_period_is_period_mismatch() {
    let request = two_node_loopy();
    let Outcome::Ok { value, .. } = generate(request.clone())
    else {
        panic!("generate must succeed");
    };
    let edges = zb_generate::cycle_edges_from_graph(&value, request.graph.as_ref().unwrap()).expect("edges");
    let err = verify_artifact_set_with_spec(&value, &VerificationSpec { edges, expected_byte_period: Some(1) })
        .expect_err("period 1 must fail for distinct two-node loop");
    assert_eq!(err.code, ErrorCode::PeriodMismatch);
    assert_eq!(err.category, ErrorCategory::PeriodMismatch);
}

#[test]
fn wrong_edge_target_fails_verification() {
    let request = two_node_loopy();
    let Outcome::Ok { value, .. } = generate(request)
    else {
        panic!("generate must succeed");
    };
    // Point the recursive member at the source artifact instead of its peer.
    let err = verify_artifact_set_with_spec(
        &value,
        &VerificationSpec {
            edges: vec![VerifyEdgeSpec {
                from: value.root.0.clone(),
                member_path: "next/b.zip".into(),
                to: value.root.0.clone(),
            }],
            expected_byte_period: None,
        },
    )
    .expect_err("wrong target must fail");
    assert_eq!(err.category, ErrorCategory::VerificationFailed);
}

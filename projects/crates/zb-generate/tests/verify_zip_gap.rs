use zb_generate::{AcornZipCapability, StoredMember, emit_stored_archive, generate, verify_artifact_set, verify_zip_bytes};
use zb_types::{ErrorCategory, GenerateRequest, Outcome, VariantKind};

#[test]
fn linked_adapter_supports_v0_gate() {
    let caps = AcornZipCapability::observed();
    assert!(caps.linked);
    assert!(caps.index_zip);
    assert!(caps.member_crc_facts);
    assert!(caps.bounded_decode_complete);
    assert!(caps.decode_crc_pair);
    assert!(caps.parse_budget);
    assert!(caps.supports_v0_full_verify());
    assert!(!caps.missing().contains(&"linked"));
}

#[test]
fn verifies_emitted_stored_archive() {
    let bytes = emit_stored_archive(&[StoredMember { path: "a.txt".into(), payload: b"hello-zb".to_vec() }]).unwrap();
    let report = verify_zip_bytes(&bytes).expect("verify");
    assert!(report.ok);
    assert!(report.notes.iter().any(|n| n.contains("crc_ok")));
}

#[test]
fn verifies_empty_stored_file_and_directory_entry() {
    let bytes = emit_stored_archive(&[
        StoredMember { path: "empty.txt".into(), payload: Vec::new() },
        StoredMember { path: "dir/".into(), payload: Vec::new() },
    ])
    .unwrap();
    let report = verify_zip_bytes(&bytes).expect("verify");
    assert!(report.ok);
    assert!(report.notes.iter().any(|n| n.contains("indexed_members=2")));
    assert!(report.notes.iter().any(|n| n.contains("decoded=0")));
}

#[test]
fn verifies_finite_bomb_root() {
    match generate(GenerateRequest::bare(VariantKind::Bomb)) {
        Outcome::Ok { value, .. } => {
            let report = verify_artifact_set(&value).expect("verify bomb");
            assert!(report.ok);
            assert!(report.notes.iter().any(|n| n.contains("artifact_count=")));
            assert!(report.notes.iter().any(|n| n.contains("n1/0.zip")));
            assert!(value.artifacts.len() >= 2);
        }
        Outcome::Err { error, .. } => panic!("bomb generate failed: {error}"),
    }
}

#[test]
fn verifies_full_loopy_artifact_set() {
    let request = zb_types::generate_request_from_json(
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
    match generate(request) {
        Outcome::Ok { value, .. } => {
            assert_eq!(value.artifacts.len(), 2);
            let report = verify_artifact_set(&value).expect("verify loopy set");
            assert!(report.ok);
            assert!(report.notes.iter().any(|n| n == "artifact_count=2"));
            assert!(!value.logical_node_map.is_empty());
        }
        Outcome::Err { error, .. } => panic!("loopy generate failed: {error}"),
    }
}

#[test]
fn rejects_artifact_set_with_unknown_logical_map_id() {
    match generate(GenerateRequest::bare(VariantKind::Bomb)) {
        Outcome::Ok { mut value, .. } => {
            value.logical_node_map.push("bogus=not-a-real-artifact-id".into());
            let err = verify_artifact_set(&value).unwrap_err();
            assert_eq!(err.category, ErrorCategory::InvalidRequest);
            assert!(err.message.contains("unknown artifact id"));
        }
        Outcome::Err { error, .. } => panic!("bomb generate failed: {error}"),
    }
}

#[test]
fn rejects_non_zip_bytes() {
    let err = verify_zip_bytes(b"not-a-zip").unwrap_err();
    assert_eq!(err.category, ErrorCategory::VerificationFailed);
}

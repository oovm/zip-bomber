use zb_generate::{
    emit_stored_archive, generate, verify_artifact_set, verify_zip_bytes, AcornZipCapability,
    StoredMember,
};
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
    let bytes = emit_stored_archive(&[StoredMember {
        path: "a.txt".into(),
        payload: b"hello-zb".to_vec(),
    }])
    .unwrap();
    let report = verify_zip_bytes(&bytes).expect("verify");
    assert!(report.ok);
    assert!(report.notes.iter().any(|n| n.contains("crc_ok")));
}

#[test]
fn verifies_finite_bomb_root() {
    match generate(GenerateRequest::bare(VariantKind::Bomb)) {
        Outcome::Ok { value, .. } => {
            let report = verify_artifact_set(&value).expect("verify bomb");
            assert!(report.ok);
            assert!(report.notes.iter().any(|n| n.contains("n1/0.zip")));
        }
        Outcome::Err { error, .. } => panic!("bomb generate failed: {error}"),
    }
}

#[test]
fn rejects_non_zip_bytes() {
    let err = verify_zip_bytes(b"not-a-zip").unwrap_err();
    assert_eq!(err.category, ErrorCategory::VerificationFailed);
}

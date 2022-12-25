use zb_generate::{crc32, emit_stored_archive, generate, StoredMember};
use zb_types::{
    ErrorCategory, GenerateLimits, GenerateRequest, Outcome, ReportStatus, VariantKind,
};

#[test]
fn stored_archive_roundtrip_headers() {
    let payload = b"hello-zb";
    let bytes = emit_stored_archive(&[StoredMember {
        path: "a.txt".into(),
        payload: payload.to_vec(),
    }])
    .unwrap();
    assert_eq!(&bytes[0..4], &[0x50, 0x4b, 0x03, 0x04]);
    let crc = u32::from_le_bytes(bytes[14..18].try_into().unwrap());
    assert_eq!(crc, crc32(payload));
    let size = u32::from_le_bytes(bytes[18..22].try_into().unwrap());
    assert_eq!(size as usize, payload.len());
    let eocd = &bytes[bytes.len() - 22..];
    assert_eq!(&eocd[0..4], &[0x50, 0x4b, 0x05, 0x06]);
}

#[test]
fn finite_bomb_depth_two_emits_nested_zip() {
    match generate(GenerateRequest::bare(VariantKind::Bomb)) {
        Outcome::Ok { value, report, .. } => {
            assert_eq!(value.root.0, "bomb-root");
            assert_eq!(value.artifacts.len(), 2);
            let root = value
                .artifacts
                .iter()
                .find(|a| a.id == value.root)
                .expect("root artifact");
            assert!(root.bytes.len() > 30);
            assert_eq!(&root.bytes[0..4], &[0x50, 0x4b, 0x03, 0x04]);
            let hay = String::from_utf8_lossy(&root.bytes);
            assert!(hay.contains("n1/0.zip"));
            assert_eq!(report.status, ReportStatus::Ok);
            assert!(report.archive_bytes.unwrap() > 0);
            assert_eq!(report.declared_uncompressed, Some(8));
        }
        Outcome::Err { error, .. } => panic!("bomb should succeed: {error}"),
    }
}

#[test]
fn finite_bomb_rejects_tiny_artifact_budget() {
    let mut req = GenerateRequest::bare(VariantKind::Bomb);
    req.limits = GenerateLimits {
        max_artifact_bytes: 8,
        ..GenerateLimits::small_sample()
    };
    match generate(req) {
        Outcome::Err { error, report, .. } => {
            assert_eq!(error.category, ErrorCategory::ResourceLimited);
            assert_eq!(report.status, ReportStatus::Error);
        }
        Outcome::Ok { .. } => panic!("expected resource_limited"),
    }
}

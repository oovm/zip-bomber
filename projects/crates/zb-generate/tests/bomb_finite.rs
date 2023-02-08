use zb_generate::{crc32, emit_stored_archive, generate, StoredMember};
use zb_types::{
    ArchiveEdge, EdgeRole, ErrorCategory, ErrorCode, GenerateLimits, GenerateRequest, NodeId,
    Outcome, ReportStatus, TopologyGraph, VariantKind,
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
            let usage = report.budget_usage.expect("budget usage");
            assert!(usage.members_decoded > 0);
            assert!(usage.decoded_bytes_used > 0);
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

#[test]
fn finite_bomb_rejects_logical_budget_before_emit() {
    let mut req = GenerateRequest::bare(VariantKind::Bomb);
    req.limits = GenerateLimits {
        max_decoded_bytes: 1,
        ..GenerateLimits::small_sample()
    };
    match generate(req) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.category, ErrorCategory::ResourceLimited);
            assert!(
                error.message.contains("logical expansion"),
                "message={}",
                error.message
            );
        }
        Outcome::Ok { .. } => panic!("tiny decode budget must fail in plan"),
    }
}

#[test]
fn finite_bomb_rejects_planned_unique_budget_before_emit() {
    let mut req = GenerateRequest::bare(VariantKind::Bomb);
    // Depth 2 default fits in a few hundred bytes; cap total unique below that.
    req.limits = GenerateLimits {
        max_total_artifact_bytes: 40,
        ..GenerateLimits::small_sample()
    };
    match generate(req) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.category, ErrorCategory::ResourceLimited);
            assert!(
                error.message.contains("planned unique")
                    || error.message.contains("unique artifact"),
                "message={}",
                error.message
            );
        }
        Outcome::Ok { .. } => panic!("tiny unique budget must fail in plan"),
    }
}

#[test]
fn topology_dag_emits_nested_member_path() {
    let mut req = GenerateRequest::bare(VariantKind::Bomb);
    req.topology = TopologyGraph::bomb_nest("root.zip", "child.zip", "nested/child.zip");
    match generate(req) {
        Outcome::Ok { value, report, .. } => {
            assert_eq!(value.artifacts.len(), 2);
            let root = value
                .artifacts
                .iter()
                .find(|a| a.id == value.root)
                .expect("root");
            let hay = String::from_utf8_lossy(&root.bytes);
            assert!(hay.contains("nested/child.zip"));
            assert_eq!(report.status, ReportStatus::Ok);
            assert!(report.archive_bytes.unwrap() > 0);
        }
        Outcome::Err { error, .. } => panic!("topology bomb should succeed: {error}"),
    }
}

#[test]
fn topology_cycle_is_rejected() {
    let mut req = GenerateRequest::bare(VariantKind::Bomb);
    let a = NodeId::new("a.zip");
    let b = NodeId::new("b.zip");
    req.topology = TopologyGraph {
        nodes: vec![a.clone(), b.clone()],
        edges: vec![
            ArchiveEdge {
                id: None,
                from: a.clone(),
                member_path: "b.zip".into(),
                to: b.clone(),
                role: EdgeRole::NestedArchive,
            },
            ArchiveEdge {
                id: None,
                from: b,
                member_path: "a.zip".into(),
                to: a,
                role: EdgeRole::NestedArchive,
            },
        ],
        alignment: Default::default(),
    };
    match generate(req) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.code, ErrorCode::InvalidTopology);
        }
        Outcome::Ok { .. } => panic!("cycle must be rejected"),
    }
}

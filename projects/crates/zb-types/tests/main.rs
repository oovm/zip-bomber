use zb_types::{
    decode_base64, encode_base64, generate_request_from_json, generate_request_to_json, zip_layout,
    ArchiveEdge, EdgeRole, GenerateLimits, GenerateRequest, NodeId, PayloadSpec, TopologyGraph,
    VariantKind, ZipProfile, ALL_VARIANTS, REQUEST_SCHEMA_V2, ZIP_PROFILE_V0,
};

#[test]
fn all_variants_roundtrip_ids() {
    for variant in ALL_VARIANTS {
        assert_eq!(VariantKind::parse(variant.as_str()), Some(variant));
    }
}

#[test]
fn zip_convenience_aliases_parse() {
    assert_eq!(VariantKind::parse("zipBomb"), Some(VariantKind::Bomb));
    assert_eq!(VariantKind::parse("zipQuine"), Some(VariantKind::Quine));
    assert_eq!(VariantKind::parse("zipLoopy"), Some(VariantKind::Loopy));
    assert_eq!(VariantKind::parse("zipFractal"), Some(VariantKind::Fractal));
}

#[test]
fn bare_request_uses_v2_schema_and_member_paths() {
    let req = GenerateRequest::bare(VariantKind::Quine);
    assert_eq!(req.schema, REQUEST_SCHEMA_V2);
    assert_eq!(req.topology.edges.len(), 1);
    assert!(!req.topology.edges[0].member_path.is_empty());
}

#[test]
fn archive_edge_carries_member_path() {
    let graph = TopologyGraph {
        nodes: vec![NodeId::new("a"), NodeId::new("b")],
        edges: vec![ArchiveEdge {
            id: Some("e1".into()),
            from: NodeId::new("a"),
            member_path: "nested/b.zip".into(),
            to: NodeId::new("b"),
            role: EdgeRole::NestedArchive,
        }],
        alignment: Default::default(),
    };
    assert_eq!(graph.edges[0].member_path, "nested/b.zip");
}

#[test]
fn zip_profile_v0_and_limits_defaults() {
    let profile = ZipProfile::v0_stub();
    assert_eq!(profile.id, ZIP_PROFILE_V0);
    assert!(!profile.allow_encryption);
    assert_eq!(zip_layout::LFH_FIXED, 30);
    assert_eq!(zip_layout::CD_ENTRY_FIXED, 46);
    let limits = GenerateLimits::small_sample();
    assert_eq!(limits.max_nodes, 64);
    let req = GenerateRequest::bare(VariantKind::Bomb);
    assert_eq!(req.profile, ZIP_PROFILE_V0);
    assert_eq!(req.limits.max_edges, 256);
}

#[test]
fn wire_json_maps_self_member_path_and_base64_payloads() {
    let json = r#"{
        "schema":"zb.request/v2",
        "container":"zip",
        "variant":"quine",
        "selfMemberPath":"recurse/again.zip",
        "payloads":[
            {"archivePath":"readme.txt","bytesBase64":"aGVsbG8="},
            {"archivePath":"empty/"}
        ]
    }"#;
    let request = generate_request_from_json(json).expect("wire decode");
    assert_eq!(request.topology.edges[0].member_path, "recurse/again.zip");
    assert_eq!(request.payloads.len(), 2);
    assert_eq!(request.payloads[0].path, "readme.txt");
    assert_eq!(request.payloads[0].bytes, b"hello");
    assert_eq!(request.payloads[1].path, "empty/");
    assert!(request.payloads[1].bytes.is_empty());

    let mut roundtrip = GenerateRequest::bare(VariantKind::Quine);
    roundtrip.topology = TopologyGraph::quine_self("release.zip", "recurse/again.zip");
    roundtrip.payloads = vec![PayloadSpec {
        path: "assets/说明.txt".into(),
        bytes: "中文".as_bytes().to_vec(),
    }];
    let encoded = generate_request_to_json(&roundtrip).expect("wire encode");
    let decoded = generate_request_from_json(&encoded).expect("wire roundtrip");
    assert_eq!(decoded.payloads[0].bytes, "中文".as_bytes());
    assert_eq!(decode_base64(&encode_base64(b"zb")).unwrap(), b"zb");
}

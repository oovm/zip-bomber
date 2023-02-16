use zb_types::{
    ALL_VARIANTS, ArchiveEdge, CapabilitySet, CapabilityStatus, EdgeRole, GenerateLimits, GenerateRequest, NodeId, PayloadSpec,
    REQUEST_SCHEMA, TopologyClass, TopologyGraph, VariantKind, ZIP_LOOPY_TWO_NODE_GAPPED_V0, ZIP_PROFILE_V0, ZipProfile,
    decode_base64, encode_base64, generate_request_from_json, generate_request_to_json, zip_layout,
};

#[test]
fn all_variants_roundtrip_ids() {
    for variant in ALL_VARIANTS {
        assert_eq!(VariantKind::parse(variant.as_str()), Some(variant));
    }
}

#[test]
fn zip_convenience_aliases_parse() {
    assert_eq!(VariantKind::parse("bomb"), Some(VariantKind::Bomb));
    assert_eq!(VariantKind::parse("quine"), Some(VariantKind::Quine));
    assert_eq!(VariantKind::parse("loopy"), Some(VariantKind::Loopy));
    assert_eq!(VariantKind::parse("fractal"), Some(VariantKind::Fractal));
}

#[test]
fn bare_request_uses_v2_schema_and_member_paths() {
    let req = GenerateRequest::bare(VariantKind::Quine);
    assert_eq!(req.schema, REQUEST_SCHEMA);
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
    assert!(!profile.allow_early_directory_gap);
    assert_eq!(zip_layout::LFH_FIXED, 30);
    assert_eq!(zip_layout::CD_ENTRY_FIXED, 46);
    let gapped = ZipProfile::loopy_two_node_gapped_v1();
    assert_eq!(gapped.id, ZIP_LOOPY_TWO_NODE_GAPPED_V0);
    assert!(gapped.allow_early_directory_gap);
    let limits = GenerateLimits::small_sample();
    assert_eq!(limits.max_nodes, 64);
    let req = GenerateRequest::bare(VariantKind::Bomb);
    assert_eq!(req.profile, ZIP_PROFILE_V0);
    assert_eq!(req.limits.max_edges, 256);
}

#[test]
fn capability_matrix_profiles_and_verified_gates() {
    let caps = CapabilitySet::template_declared();
    assert_eq!(caps.entries.len(), 5);
    let quine = caps.entries.iter().find(|e| e.variant == VariantKind::Quine).expect("quine row");
    assert_eq!(quine.profile, ZIP_PROFILE_V0);
    assert_eq!(quine.status, CapabilityStatus::Verified);
    assert_eq!(quine.topology_class, TopologyClass::SelfLoop);
    let loopy = caps.entries.iter().find(|e| e.variant == VariantKind::Loopy).expect("loopy row");
    assert_eq!(loopy.profile, ZIP_LOOPY_TWO_NODE_GAPPED_V0);
    assert_eq!(loopy.status, CapabilityStatus::Verified);
    let fractal_rec = caps
        .entries
        .iter()
        .find(|e| e.variant == VariantKind::Fractal && e.topology_class == TopologyClass::FractalRecursive)
        .expect("fractal recursive row");
    assert_eq!(fractal_rec.profile, ZIP_LOOPY_TWO_NODE_GAPPED_V0);
    assert_eq!(fractal_rec.status, CapabilityStatus::Implemented);
}

#[test]
fn wire_limits_use_decimal_u64_strings() {
    let json = r#"{
        "schema":"zb.request/v0",
        "container":"zip",
        "variant":"quine",
        "limits":{
            "maxDecodedBytes":"1048576",
            "max_nodes":8,
            "maxConstraintBits":"65536"
        }
    }"#;
    let request = generate_request_from_json(json).expect("wire decode");
    assert_eq!(request.limits.max_decoded_bytes, 1_048_576);
    assert_eq!(request.limits.max_nodes, 8);
    assert_eq!(request.limits.max_constraint_bits, 65_536);
    // Unspecified fields keep small-sample defaults.
    assert_eq!(request.limits.max_edges, 256);

    let encoded = generate_request_to_json(&request).expect("wire encode");
    assert!(encoded.contains("\"maxDecodedBytes\":\"1048576\""));
    assert!(encoded.contains("\"maxNodes\":8"));
    assert!(!encoded.contains("\"max_decoded_bytes\""));

    let bad = r#"{
        "schema":"zb.request/v0",
        "container":"zip",
        "variant":"quine",
        "limits":{"maxDecodedBytes":"01"}
    }"#;
    assert!(generate_request_from_json(bad).is_err());
}

#[test]
fn wire_json_maps_fixed_point_and_base64_payloads() {
    let json = r#"{
        "schema":"zb.request/v0",
        "container":"zip",
        "variant":"quine",
        "fixedPoint":"recurse/again.zip",
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
    roundtrip.payloads = vec![PayloadSpec { path: "assets/说明.txt".into(), bytes: "中文".as_bytes().to_vec() }];
    let encoded = generate_request_to_json(&roundtrip).expect("wire encode");
    let decoded = generate_request_from_json(&encoded).expect("wire roundtrip");
    assert_eq!(decoded.payloads[0].bytes, "中文".as_bytes());
    assert_eq!(decode_base64(&encode_base64(b"zb")).unwrap(), b"zb");
}

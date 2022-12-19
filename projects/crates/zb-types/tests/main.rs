use zb_types::{
    zip_layout, ArchiveEdge, EdgeRole, GenerateLimits, GenerateRequest, NodeId, TopologyGraph,
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

use zb_types::{
    ArchiveEdge, EdgeRole, GenerateRequest, NodeId, TopologyGraph, VariantKind, ALL_VARIANTS,
    REQUEST_SCHEMA_V2,
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

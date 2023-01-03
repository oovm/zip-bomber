use zb_generate::plan_generate;
use zb_types::{GenerateRequest, Outcome, TopologyGraph, VariantKind};

#[test]
fn plans_empty_bomb_as_feasible() {
    match plan_generate(GenerateRequest::bare(VariantKind::Bomb)) {
        Outcome::Ok { value, .. } => {
            assert!(value.feasible);
            assert_eq!(value.variant, "bomb");
            assert!(value.summary.contains("default depth"));
        }
        Outcome::Err { error, .. } => panic!("plan should succeed: {error}"),
    }
}

#[test]
fn plans_topology_bomb_as_feasible() {
    let mut req = GenerateRequest::bare(VariantKind::Bomb);
    req.topology = TopologyGraph::bomb_nest("root.zip", "child.zip", "nested/child.zip");
    match plan_generate(req) {
        Outcome::Ok { value, .. } => {
            assert!(value.feasible);
            assert!(value.summary.contains("topology DAG"));
        }
        Outcome::Err { error, .. } => panic!("plan should succeed: {error}"),
    }
}

#[test]
fn plans_quine_with_complete_template() {
    match plan_generate(GenerateRequest::bare(VariantKind::Quine)) {
        Outcome::Ok { value, .. } => {
            assert!(value.feasible);
            assert!(value.summary.contains("prefix/core/suffix"));
        }
        Outcome::Err { error, .. } => panic!("quine plan failed: {error}"),
    }
}

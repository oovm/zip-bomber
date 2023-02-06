use zb_generate::{generate, plan_generate};
use zb_types::{generate_request_from_json, ErrorCode, Outcome};
fn request(
    variant: &str,
    mode: &str,
    profile: &str,
    depth: Option<&str>,
) -> zb_types::GenerateRequest {
    generate_request_from_json(&serde_json::json!({"schema":"zb.request/v2","variant":variant,"profile":profile,"graph":{"root":0,"mode":mode,"depth":depth,"nodes":[
        {"members":[{"kind":"payload","path":"readme.txt","bytes":"YWJj"},{"kind":"archive","path":"next/b.zip","target":1}],"terminal":[]},
        {"members":[{"kind":"payload","path":"x.txt","bytes":"ZGVmZw=="},{"kind":"archive","path":"back/a.zip","target":0}],"terminal":[]}
    ]}}).to_string()).unwrap()
}

fn three_node_loopy(profile: &str) -> zb_types::GenerateRequest {
    generate_request_from_json(
        &serde_json::json!({
            "schema":"zb.request/v2",
            "variant":"loopy",
            "profile":profile,
            "graph":{"root":0,"mode":"recursive","nodes":[
                {"members":[{"kind":"archive","path":"to/b.zip","target":1}]},
                {"members":[{"kind":"archive","path":"to/c.zip","target":2}]},
                {"members":[{"kind":"archive","path":"to/a.zip","target":0}]}
            ]}
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn finite_and_recursive_two_state_graphs() {
    let finite = request("fractal", "finite", "zip.profile/v0-stub", Some("2"));
    match generate(finite) {
        Outcome::Ok { value, .. } => {
            assert_eq!(value.logical_node_map.len(), 3);
            assert!(value.artifacts.iter().any(|a| a.bytes.len() == 22));
        }
        other => panic!("{other:?}"),
    }
    let loopy = request("loopy", "recursive", "zip.loopy-two-node.gapped/v1", None);
    let fractal = request("fractal", "recursive", "zip.loopy-two-node.gapped/v1", None);
    let a = match generate(loopy) {
        Outcome::Ok { value, .. } => value,
        other => panic!("{other:?}"),
    };
    let b = match generate(fractal) {
        Outcome::Ok { value, .. } => value,
        other => panic!("{other:?}"),
    };
    assert_eq!(a, b);
    assert_eq!(a.artifacts.len(), 2);
}

#[test]
fn loopy_report_carries_structured_facts() {
    let request = request("loopy", "recursive", "zip.loopy-two-node.gapped/v1", None);
    match generate(request) {
        Outcome::Ok { report, .. } => {
            assert_eq!(report.schema, zb_types::REPORT_SCHEMA_V2);
            assert_eq!(
                report.profile.as_deref(),
                Some("zip.loopy-two-node.gapped/v1")
            );
            let solver = report.solver.expect("solver facts");
            assert!(solver.rank > 0);
            assert_eq!(solver.equations_used, 64);
            assert_eq!(
                solver.problem_digest.as_deref(),
                Some("loopy.crc.shared/v1")
            );
            let facts = report.loopy.expect("loopy facts");
            assert_eq!(facts.requested_nodes, 2);
            assert_eq!(facts.byte_period, 2);
            assert_eq!(facts.profile, "zip.loopy-two-node.gapped/v1");
            assert_eq!(facts.ordinary_members, vec![1, 1]);
            assert_eq!(facts.quote_window, 16000);
            assert_eq!(facts.edges.len(), 2);
            assert!(facts.edges.iter().all(|e| e.bytes_equal));
            assert_eq!(facts.crc_rank, solver.rank);
            assert!(report.warnings.iter().all(|w| !w.contains("crc_rank")));
            let usage = report.budget_usage.expect("budget usage");
            assert!(usage.members_decoded > 0);
            assert!(usage.decoded_bytes_used > 0);
            assert!(usage.decoded_bytes_used <= usage.max_decoded_bytes);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn unsupported_profile_does_not_silently_use_gapped_constructor() {
    let r = request("loopy", "recursive", "zip.profile/v0-stub", None);
    match plan_generate(r) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.code, ErrorCode::UnsupportedProfile);
            assert!(error.message.contains("zip.loopy-two-node.gapped/v1"));
        }
        other => panic!("expected unsupported_profile, got {other:?}"),
    }
}

#[test]
fn finite_fractal_rejects_planned_unique_budget_before_emit() {
    let mut req = request("fractal", "finite", "zip.profile/v0-stub", Some("2"));
    req.limits.max_total_artifact_bytes = 30;
    match generate(req) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.category, zb_types::ErrorCategory::ResourceLimited);
            assert!(
                error.message.contains("planned"),
                "message={}",
                error.message
            );
        }
        other => panic!("expected resource_limited plan reject, got {other:?}"),
    }
}

#[test]
fn three_node_loopy_is_unsupported_topology() {
    for profile in ["zip.loopy-two-node.gapped/v1", "zip.profile/v0-stub"] {
        match generate(three_node_loopy(profile)) {
            Outcome::Err { error, .. } => {
                assert_eq!(error.code, ErrorCode::UnsupportedTopology, "{profile}");
                assert!(
                    error.message.contains("3-state"),
                    "profile={profile} message={}",
                    error.message
                );
            }
            other => panic!("expected unsupported_topology for {profile}, got {other:?}"),
        }
    }
}

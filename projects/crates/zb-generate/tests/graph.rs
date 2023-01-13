use zb_generate::{generate, plan_generate};
use zb_types::{generate_request_from_json, Outcome};
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
fn unsupported_profile_does_not_silently_use_gapped_constructor() {
    let r = request("loopy", "recursive", "zip.profile/v0-stub", None);
    assert!(
        matches!(plan_generate(r),Outcome::Err{error,..} if error.code==zb_types::ErrorCode::UnsupportedTopology)
    );
}

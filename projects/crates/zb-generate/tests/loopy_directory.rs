use zb_generate::generate;
use zb_types::{generate_request_from_json, Outcome};

fn request(with_dir: bool) -> zb_types::GenerateRequest {
    let mut nodes = serde_json::json!([
        {"members":[
            {"kind":"payload","path":"readme.txt","bytes":"YWJj"},
            {"kind":"archive","path":"next/b.zip","target":1}
        ]},
        {"members":[
            {"kind":"payload","path":"x.txt","bytes":"ZGVmZw=="},
            {"kind":"archive","path":"back/a.zip","target":0}
        ]}
    ]);
    if with_dir {
        nodes[0]["members"].as_array_mut().unwrap().insert(
            1,
            serde_json::json!({"kind":"payload","path":"empty/","bytes":""}),
        );
    }
    generate_request_from_json(
        &serde_json::json!({
            "schema":"zb.request/v2",
            "variant":"loopy",
            "profile":"zip.loopy-two-node.gapped/v1",
            "graph":{"root":0,"mode":"recursive","nodes":nodes}
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn loopy_without_directory_ok() {
    match generate(request(false)) {
        Outcome::Ok { value, .. } => assert_eq!(value.artifacts.len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn loopy_with_directory_on_one_side() {
    match generate(request(true)) {
        Outcome::Ok { value, .. } => {
            assert_eq!(value.artifacts.len(), 2);
            assert_ne!(value.artifacts[0].bytes, value.artifacts[1].bytes);
        }
        Outcome::Err { error, .. } => {
            panic!(
                "category={:?} code={:?} message={}",
                error.category, error.code, error.message
            );
        }
    }
}

#[test]
fn loopy_with_directory_on_both_sides() {
    let request = generate_request_from_json(
        &serde_json::json!({
            "schema":"zb.request/v2",
            "variant":"loopy",
            "profile":"zip.loopy-two-node.gapped/v1",
            "graph":{"root":0,"mode":"recursive","nodes":[
                {"members":[
                    {"kind":"payload","path":"a.txt","bytes":"YWJj"},
                    {"kind":"payload","path":"empty/","bytes":""},
                    {"kind":"archive","path":"next/b.zip","target":1}
                ]},
                {"members":[
                    {"kind":"payload","path":"b.txt","bytes":"ZGVm"},
                    {"kind":"payload","path":"empty/","bytes":""},
                    {"kind":"archive","path":"back/a.zip","target":0}
                ]}
            ]}
        })
        .to_string(),
    )
    .unwrap();
    match generate(request) {
        Outcome::Ok { value, .. } => assert_eq!(value.artifacts.len(), 2),
        other => panic!("{other:?}"),
    }
}

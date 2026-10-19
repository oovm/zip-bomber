use zb_generate::{decode_raw_deflate, generate, plan_generate, verify_zip_bytes};
use zb_types::{GenerateRequest, Outcome, PayloadSpec, TopologyGraph, VariantKind};

fn inflate_member(archive: &[u8], local_offset: usize) -> Vec<u8> {
    let method = u16::from_le_bytes(archive[local_offset + 8..local_offset + 10].try_into().unwrap());
    let compressed = u32::from_le_bytes(archive[local_offset + 18..local_offset + 22].try_into().unwrap()) as usize;
    let name_len = u16::from_le_bytes(archive[local_offset + 26..local_offset + 28].try_into().unwrap()) as usize;
    let extra = u16::from_le_bytes(archive[local_offset + 28..local_offset + 30].try_into().unwrap()) as usize;
    let start = local_offset + 30 + name_len + extra;
    let data = &archive[start..start + compressed];
    if method == 0 {
        return data.to_vec();
    }
    decode_raw_deflate(data, archive.len() as u64 + 64).expect("inflate attachment")
}

#[test]
fn quine_with_attachments_dirs_and_utf8_names() {
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.topology = TopologyGraph::quine_self("release.zip", "recurse/again.zip");
    request.payloads = vec![
        PayloadSpec { path: "readme.txt".into(), bytes: b"hello quine attachments\n".to_vec() },
        PayloadSpec { path: "assets/说明.txt".into(), bytes: "中文附件内容".as_bytes().to_vec() },
        PayloadSpec { path: "empty/".into(), bytes: Vec::new() },
        PayloadSpec { path: "metadata/config.json".into(), bytes: br#"{"ok":true}"#.to_vec() },
    ];

    let set = match generate(request.clone()) {
        Outcome::Ok { value, report, .. } => {
            println!("{report:?}");
            value
        }
        Outcome::Err { error, .. } => panic!("attachment quine failed: {error}"),
    };
    let bytes = &set.artifacts[0].bytes;
    assert!(verify_zip_bytes(bytes).unwrap().ok);

    // First ordinary member starts at offset 0.
    assert_eq!(inflate_member(bytes, 0), b"hello quine attachments\n");

    // Self member is last local header before the recursive stream. Find by name.
    let self_name = b"recurse/again.zip";
    let mut pos = 0usize;
    let mut self_offset = None;
    while pos + 30 <= bytes.len() {
        if &bytes[pos..pos + 4] != b"PK\x03\x04" {
            break;
        }
        let name_len = u16::from_le_bytes(bytes[pos + 26..pos + 28].try_into().unwrap()) as usize;
        let extra = u16::from_le_bytes(bytes[pos + 28..pos + 30].try_into().unwrap()) as usize;
        let comp = u32::from_le_bytes(bytes[pos + 18..pos + 22].try_into().unwrap()) as usize;
        let name = &bytes[pos + 30..pos + 30 + name_len];
        if name == self_name {
            self_offset = Some(pos);
            break;
        }
        pos += 30 + name_len + extra + comp;
    }
    let self_offset = self_offset.expect("self member local header");
    let decoded = inflate_member(bytes, self_offset);
    assert_eq!(&decoded, bytes);

    match plan_generate(request) {
        Outcome::Ok { value, .. } => {
            assert!(value.feasible);
            assert!(value.summary.contains("ordinary=4"));
            assert!(value.summary.contains("recurse/again.zip"));
        }
        Outcome::Err { error, .. } => panic!("plan failed: {error}"),
    }
}

#[test]
fn explicit_directories_can_contain_files_in_either_order() {
    for reverse in [false, true] {
        let mut request = GenerateRequest::bare(VariantKind::Quine);
        request.topology = TopologyGraph::quine_self("release.zip", "assets/again.zip");
        request.payloads = vec![
            PayloadSpec { path: "assets/".into(), bytes: vec![] },
            PayloadSpec { path: "assets/nested/".into(), bytes: vec![] },
            PayloadSpec { path: "assets/nested/说明.txt".into(), bytes: b"content".to_vec() },
        ];
        if reverse {
            request.payloads.reverse();
        }
        match generate(request) {
            Outcome::Ok { value, .. } => {
                assert!(verify_zip_bytes(&value.artifacts[0].bytes).unwrap().ok)
            }
            Outcome::Err { error, .. } => panic!("directory descendants failed: {error}"),
        }
    }
}

#[test]
fn malformed_directory_slashes_and_file_ancestors_are_rejected() {
    for paths in [vec!["assets//"], vec!["assets", "assets/data.txt"], vec!["a", "a/a.zip"]] {
        let mut request = GenerateRequest::bare(VariantKind::Quine);
        request.payloads = paths.iter().map(|p| PayloadSpec { path: p.to_string(), bytes: vec![] }).collect();
        assert!(matches!(generate(request), Outcome::Err { .. }));
    }
}

#[test]
fn compressed_attachments_still_consume_decode_budget() {
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.payloads = vec![PayloadSpec { path: "large.txt".into(), bytes: vec![b'x'; 100_000] }];
    request.limits.max_decoded_bytes = 50_000;
    match generate(request) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.category, zb_types::ErrorCategory::ResourceLimited)
        }
        Outcome::Ok { .. } => panic!("attachment decode bytes must count toward budget"),
    }
}

#[test]
fn incompressible_attachment_exceeding_window_is_rejected_by_plan_and_generate() {
    let mut seed = 0x12345678u32;
    let bytes: Vec<u8> = (0..40_000)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect();
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.payloads = vec![PayloadSpec { path: "binary.dat".into(), bytes }];
    match plan_generate(request.clone()) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.category, zb_types::ErrorCategory::ResourceLimited)
        }
        Outcome::Ok { .. } => panic!("oversized serialized prefix must fail planning"),
    }
    match generate(request) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.category, zb_types::ErrorCategory::ResourceLimited)
        }
        Outcome::Ok { .. } => panic!("oversized serialized prefix must fail generation"),
    }
}

#[test]
fn quine_rejects_duplicate_and_conflicting_paths() {
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.payloads = vec![
        PayloadSpec { path: "a.txt".into(), bytes: b"1".to_vec() },
        PayloadSpec { path: "a.txt".into(), bytes: b"2".to_vec() },
    ];
    match generate(request) {
        Outcome::Err { error, .. } => assert!(error.message.contains("duplicate")),
        Outcome::Ok { .. } => panic!("duplicate paths must fail"),
    }

    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.topology = TopologyGraph::quine_self("root", "nest/self.zip");
    request.payloads = vec![PayloadSpec { path: "nest".into(), bytes: b"file-vs-dir".to_vec() }];
    match generate(request) {
        Outcome::Err { error, .. } => assert!(error.message.contains("conflict")),
        Outcome::Ok { .. } => panic!("file/dir conflict must fail"),
    }
}

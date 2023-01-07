use flate2::{Decompress, FlushDecompress, Status};
use zb_generate::{generate, plan_generate, verify_zip_bytes};
use zb_types::{
    GenerateRequest, Outcome, PayloadSpec, TopologyGraph, VariantKind,
};

fn inflate_member(archive: &[u8], local_offset: usize) -> Vec<u8> {
    let method = u16::from_le_bytes(archive[local_offset + 8..local_offset + 10].try_into().unwrap());
    let compressed = u32::from_le_bytes(archive[local_offset + 18..local_offset + 22].try_into().unwrap())
        as usize;
    let name_len =
        u16::from_le_bytes(archive[local_offset + 26..local_offset + 28].try_into().unwrap()) as usize;
    let extra =
        u16::from_le_bytes(archive[local_offset + 28..local_offset + 30].try_into().unwrap()) as usize;
    let start = local_offset + 30 + name_len + extra;
    let data = &archive[start..start + compressed];
    if method == 0 {
        return data.to_vec();
    }
    let mut decoder = Decompress::new(false);
    let mut out = vec![0; archive.len() + 64];
    assert_eq!(
        decoder
            .decompress(data, &mut out, FlushDecompress::Finish)
            .unwrap(),
        Status::StreamEnd
    );
    out.truncate(decoder.total_out() as usize);
    out
}

#[test]
fn quine_with_attachments_dirs_and_utf8_names() {
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.topology = TopologyGraph::quine_self("release.zip", "recurse/again.zip");
    request.payloads = vec![
        PayloadSpec {
            path: "readme.txt".into(),
            bytes: b"hello quine attachments\n".to_vec(),
        },
        PayloadSpec {
            path: "assets/说明.txt".into(),
            bytes: "中文附件内容".as_bytes().to_vec(),
        },
        PayloadSpec {
            path: "empty/".into(),
            bytes: Vec::new(),
        },
        PayloadSpec {
            path: "metadata/config.json".into(),
            bytes: br#"{"ok":true}"#.to_vec(),
        },
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
fn quine_rejects_duplicate_and_conflicting_paths() {
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.payloads = vec![
        PayloadSpec {
            path: "a.txt".into(),
            bytes: b"1".to_vec(),
        },
        PayloadSpec {
            path: "a.txt".into(),
            bytes: b"2".to_vec(),
        },
    ];
    match generate(request) {
        Outcome::Err { error, .. } => assert!(error.message.contains("duplicate")),
        Outcome::Ok { .. } => panic!("duplicate paths must fail"),
    }

    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.topology = TopologyGraph::quine_self("root", "nest/self.zip");
    request.payloads = vec![PayloadSpec {
        path: "nest".into(),
        bytes: b"file-vs-dir".to_vec(),
    }];
    match generate(request) {
        Outcome::Err { error, .. } => assert!(error.message.contains("conflict")),
        Outcome::Ok { .. } => panic!("file/dir conflict must fail"),
    }
}

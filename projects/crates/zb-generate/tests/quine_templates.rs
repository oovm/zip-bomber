use flate2::{Decompress, FlushDecompress, Status};
use zb_generate::{generate, list_quine_templates, verify_zip_bytes};
use zb_types::{GenerateRequest, Outcome, VariantKind, ZIP_PROFILE_V0};

#[test]
fn quine_registry_has_complete_prefix_suffix_template() {
    let templates = list_quine_templates(ZIP_PROFILE_V0);
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].id, "quine.stored-fixed.prefix-suffix");
}

#[test]
fn generated_quine_is_complete_and_self_identical() {
    let request = GenerateRequest::bare(VariantKind::Quine);
    let set = match generate(request.clone()) {
        Outcome::Ok { value, report, .. } => {
            println!("{:?}", report);
            value
        }
        Outcome::Err { error, .. } => panic!("quine generation failed: {error}"),
    };
    let bytes = &set.artifacts[0].bytes;
    let start = 30 + request.topology.edges[0].member_path.len();
    let compressed = u32::from_le_bytes(bytes[18..22].try_into().unwrap()) as usize;
    let mut decoder = Decompress::new(false);
    let mut decoded = vec![0; bytes.len() + 1];
    assert_eq!(
        decoder
            .decompress(
                &bytes[start..start + compressed],
                &mut decoded,
                FlushDecompress::Finish
            )
            .unwrap(),
        Status::StreamEnd
    );
    assert_eq!(decoder.total_in() as usize, compressed);
    decoded.truncate(decoder.total_out() as usize);
    assert_eq!(&decoded, bytes);
    assert!(verify_zip_bytes(bytes).unwrap().ok);
}

#[test]
fn quine_is_deterministic_for_utf8_names_and_rejects_budgets() {
    for name in ["a.zip", "递归/自身.zip", "a/longer-name.zip"] {
        let mut request = GenerateRequest::bare(VariantKind::Quine);
        request.topology = zb_types::TopologyGraph::quine_self("root", name);
        let first = generate(request.clone());
        assert!(first.is_ok(), "name={name}: {first:?}");
        assert_eq!(first, generate(request));
    }
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.limits.max_decoded_bytes = 1;
    match generate(request) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.category, zb_types::ErrorCategory::ResourceLimited)
        }
        _ => panic!("tiny decode budget must fail"),
    }
    let mut request = GenerateRequest::bare(VariantKind::Quine);
    request.limits.max_template_candidates = 0;
    match generate(request) {
        Outcome::Err { error, .. } => {
            assert_eq!(error.category, zb_types::ErrorCategory::ResourceLimited)
        }
        _ => panic!("zero candidate budget must fail"),
    }
}

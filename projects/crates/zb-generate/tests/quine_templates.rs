use zb_generate::{generate, list_quine_templates};
use zb_types::{ErrorCategory, GenerateRequest, Outcome, VariantKind, ZIP_PROFILE_V0};

#[test]
fn quine_registry_empty_for_v0() {
    assert!(list_quine_templates(ZIP_PROFILE_V0).is_empty());
}

#[test]
fn quine_generate_reports_search_exhausted() {
    match generate(GenerateRequest::bare(VariantKind::Quine)) {
        Outcome::Err { error, report, .. } => {
            assert_eq!(error.category, ErrorCategory::SearchExhausted);
            assert!(error.message.contains("no verified quine templates"));
            assert_eq!(report.status.as_str(), "error");
        }
        Outcome::Ok { .. } => panic!("quine should not succeed without templates"),
    }
}

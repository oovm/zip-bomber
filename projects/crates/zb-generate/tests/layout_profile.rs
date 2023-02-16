use zb_generate::{MemberLayout, archive_size, plan, resolve_profile};
use zb_types::{GenerateRequest, VariantKind, ZIP_LOOPY_TWO_NODE_GAPPED_V0, ZIP_PROFILE_V0};

#[test]
fn v0_profile_constants_match_living() {
    let req = GenerateRequest::bare(VariantKind::Bomb);
    let plan = plan(&req).expect("plan");
    assert_eq!(plan.lfh_fixed, 30);
    assert_eq!(plan.cd_entry_fixed, 46);
    assert_eq!(plan.eocd_min, 22);
    assert_eq!(plan.profile_id, ZIP_PROFILE_V0);
}

#[test]
fn gapped_loopy_profile_resolves() {
    let mut req = GenerateRequest::bare(VariantKind::Loopy);
    req.profile = ZIP_LOOPY_TWO_NODE_GAPPED_V0.to_string();
    let profile = resolve_profile(&req).expect("gapped profile");
    assert_eq!(profile.id, ZIP_LOOPY_TWO_NODE_GAPPED_V0);
    assert!(profile.allow_early_directory_gap);
    let planned = plan(&req).expect("plan");
    assert_eq!(planned.profile_id, ZIP_LOOPY_TWO_NODE_GAPPED_V0);
}

#[test]
fn archive_size_for_single_stored_member() {
    let member = MemberLayout { name_len: 4, local_extra_len: 0, central_extra_len: 0, comment_len: 0, compressed_len: 10 };
    // LFH 34 + payload 10 + CD 50 + EOCD 22 = 116
    assert_eq!(archive_size(&[member], 0).unwrap(), 116);
}

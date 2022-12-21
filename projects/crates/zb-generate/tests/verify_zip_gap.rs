use zb_generate::{verify_zip_bytes, AcornZipCapability};
use zb_types::ErrorCategory;

#[test]
fn current_gap_blocks_full_verify() {
    let caps = AcornZipCapability::current_gap();
    assert!(!caps.supports_v0_full_verify());
    assert!(caps.missing().contains(&"member_crc_facts"));
    let err = verify_zip_bytes(b"PK\x03\x04").unwrap_err();
    assert_eq!(err.category, ErrorCategory::VerificationCapabilityMissing);
}

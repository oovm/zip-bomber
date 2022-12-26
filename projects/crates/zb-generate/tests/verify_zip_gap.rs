use zb_generate::{verify_zip_bytes, AcornZipCapability};
use zb_types::ErrorCategory;

#[test]
fn observed_surface_still_blocks_full_verify() {
    let caps = AcornZipCapability::observed();
    assert!(caps.index_zip);
    assert!(caps.member_crc_facts);
    assert!(caps.parse_budget);
    assert!(!caps.supports_v0_full_verify());
    assert!(caps.missing().contains(&"bounded_decode_complete"));
    assert!(caps.missing().contains(&"decode_crc_pair"));
    let err = verify_zip_bytes(b"PK\x03\x04").unwrap_err();
    assert_eq!(err.category, ErrorCategory::VerificationCapabilityMissing);
}

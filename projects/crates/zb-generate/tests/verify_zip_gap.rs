use zb_generate::{verify_zip_bytes, AcornZipCapability};
use zb_types::ErrorCategory;

#[test]
fn observed_surface_awaits_linked_adapter() {
    let caps = AcornZipCapability::observed();
    assert!(!caps.linked);
    assert!(caps.index_zip);
    assert!(caps.member_crc_facts);
    assert!(caps.bounded_decode_complete);
    assert!(caps.decode_crc_pair);
    assert!(caps.parse_budget);
    assert!(!caps.supports_v0_full_verify());
    assert!(caps.missing().contains(&"linked"));
    let err = verify_zip_bytes(b"PK\x03\x04").unwrap_err();
    assert_eq!(err.category, ErrorCategory::VerificationCapabilityMissing);
}

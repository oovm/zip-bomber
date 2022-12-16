use zb_types::{VariantKind, ALL_VARIANTS};

#[test]
fn all_variants_roundtrip_ids() {
    for variant in ALL_VARIANTS {
        assert_eq!(VariantKind::parse(variant.as_str()), Some(variant));
    }
}

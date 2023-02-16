//! Acorn `ByteSource` materialization helpers for zb verify paths.

use acorn_source::{ByteSource, MemorySource, SourceRange};
use acorn_zip::MemberDecodeFacts;
use zb_types::{ZbError, ZbResult};

/// Materialize decoded member bytes after [`acorn_zip::decode_member_with_facts`].
///
/// Acorn treats `read_at(offset, &mut [])` as EOF when `offset >= source_len`, so
/// `read_range` with `len == 0` fails with unexpected EOF. Zero-length stored files
/// and directory entries are returned per [`MemberDecodeFacts::decoded_size`] without
/// calling `read_range(0, 0)`.
pub(crate) fn materialize_decoded_bytes(source: &MemorySource, facts: &MemberDecodeFacts) -> ZbResult<Vec<u8>> {
    let len = facts.decoded_size;
    if len == 0 {
        return Ok(Vec::new());
    }
    let source_len = source.length().unwrap_or(0);
    if source_len != len {
        return Err(ZbError::verification_failed(format!(
            "decoded source length {source_len} does not match facts.decoded_size {len}"
        )));
    }
    source
        .read_range(SourceRange { start: 0, len })
        .map_err(|err| ZbError::verification_failed(format!("decoded source read failed: {err}")))
}

#[cfg(test)]
mod tests {
    use acorn_core::Budget;
    use acorn_zip::decode_member_with_facts;

    use super::*;

    #[test]
    fn zero_length_stored_member_materializes_without_read_range() {
        let archive =
            crate::emit::emit_stored_archive(&[crate::emit::StoredMember { path: "empty/".into(), payload: Vec::new() }])
                .expect("emit");
        let index = acorn_zip::index_zip_bytes("test.zip", archive.clone()).expect("index");
        let member = &index.members[0];
        let budget = Budget::default();
        let (source, facts) =
            decode_member_with_facts("test", MemorySource::from_bytes("test.zip", archive), member, &budget).expect("decode");
        assert_eq!(facts.decoded_size, 0);
        let bytes = materialize_decoded_bytes(&source, &facts).expect("materialize");
        assert!(bytes.is_empty());
    }
}

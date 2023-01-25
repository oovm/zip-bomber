//! Decimal-string helpers for Living `04` public u64 fields.

use crate::{ErrorCode, ZbError, ZbResult};

/// Parse a canonical decimal `u64` string (digits only, no sign / fraction / leading zeros).
pub fn parse_decimal_u64(raw: &str) -> ZbResult<u64> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            format!("invalid DecimalU64 `{raw}`"),
        ));
    }
    if raw.len() > 1 && raw.as_bytes()[0] == b'0' {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            format!("DecimalU64 `{raw}` has a leading zero"),
        ));
    }
    raw.parse::<u64>().map_err(|_| {
        ZbError::new(
            ErrorCode::InvalidTopology,
            format!("DecimalU64 `{raw}` exceeds u64"),
        )
    })
}

/// Format a `u64` as a canonical decimal string.
pub fn format_decimal_u64(value: u64) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_forms() {
        assert_eq!(parse_decimal_u64("0").unwrap(), 0);
        assert_eq!(parse_decimal_u64("64").unwrap(), 64);
        assert_eq!(
            parse_decimal_u64("18446744073709551615").unwrap(),
            u64::MAX
        );
    }

    #[test]
    fn rejects_non_canonical() {
        assert!(parse_decimal_u64("").is_err());
        assert!(parse_decimal_u64("+1").is_err());
        assert!(parse_decimal_u64("01").is_err());
        assert!(parse_decimal_u64("1.0").is_err());
        assert!(parse_decimal_u64("1e3").is_err());
    }
}

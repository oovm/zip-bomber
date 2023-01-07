//! POSIX archive member path normalization for Quine / payloads.

use zb_types::{ErrorCode, PayloadSpec, ZbError, ZbResult};

/// Validate one archive-relative member path.
///
/// Directory entries must end with `/`. Ordinary files must not.
pub fn validate_member_path(path: &str, allow_directory: bool) -> ZbResult<()> {
    if path.is_empty() {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "member path must be non-empty",
        ));
    }
    if path.as_bytes().len() > u16::MAX as usize {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            format!("member path `{path}` exceeds ZIP16 name length"),
        ));
    }
    if path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path.contains(':')
    {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            format!("member path `{path}` must be a relative normalized POSIX path"),
        ));
    }
    let is_dir = path.ends_with('/');
    if is_dir && !allow_directory {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            format!("member path `{path}` cannot be a directory entry here"),
        ));
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "member path cannot be empty after removing directory slash",
        ));
    }
    if trimmed
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            format!("member path `{path}` contains empty or relative segments"),
        ));
    }
    Ok(())
}

/// Validate Quine self path plus ordinary payload / directory paths.
pub fn validate_quine_members(self_path: &str, payloads: &[PayloadSpec]) -> ZbResult<()> {
    validate_member_path(self_path, false)?;
    let mut seen = vec![normalize_key(self_path)];
    for payload in payloads {
        let is_dir = payload.path.ends_with('/');
        validate_member_path(&payload.path, is_dir)?;
        if is_dir && !payload.bytes.is_empty() {
            return Err(ZbError::new(
                ErrorCode::InvalidTopology,
                format!(
                    "directory payload `{}` must have empty bytes",
                    payload.path
                ),
            ));
        }
        let key = normalize_key(&payload.path);
        if seen.iter().any(|s| s == &key) {
            return Err(ZbError::new(
                ErrorCode::InvalidTopology,
                format!("duplicate member path `{}`", payload.path),
            ));
        }
        for existing in &seen {
            if paths_conflict(existing, &key) {
                return Err(ZbError::new(
                    ErrorCode::InvalidTopology,
                    format!(
                        "member path `{}` conflicts with existing path `{existing}`",
                        payload.path
                    ),
                ));
            }
        }
        seen.push(key);
    }
    Ok(())
}

fn normalize_key(path: &str) -> String {
    path.trim_end_matches('/').to_string()
}

fn paths_conflict(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (shorter, longer) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    longer.starts_with(shorter) && longer.as_bytes().get(shorter.len()) == Some(&b'/')
}

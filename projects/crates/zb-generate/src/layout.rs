//! ZIP local-header / central-directory layout planning.

use zb_types::{zip_layout, GenerateRequest, ZipProfile, ZbError, ZbResult, ZIP_PROFILE_V0};

/// Planned sizes for one archive member (name + extras already chosen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberLayout {
    /// UTF-8 name byte length.
    pub name_len: u32,
    /// Local Extra Field byte length.
    pub local_extra_len: u32,
    /// Central Extra Field byte length.
    pub central_extra_len: u32,
    /// Central directory comment length.
    pub comment_len: u32,
    /// Compressed payload length.
    pub compressed_len: u32,
}

impl MemberLayout {
    /// Local file header size including name and local extra.
    pub fn lfh_size(self) -> ZbResult<u32> {
        checked_add_u32(zip_layout::LFH_FIXED, self.name_len)?
            .checked_add(self.local_extra_len)
            .ok_or_else(|| overflow("lfh_size"))
    }

    /// Central directory entry size including name, extra, and comment.
    pub fn cd_entry_size(self) -> ZbResult<u32> {
        checked_add_u32(zip_layout::CD_ENTRY_FIXED, self.name_len)?
            .checked_add(self.central_extra_len)
            .and_then(|v| v.checked_add(self.comment_len))
            .ok_or_else(|| overflow("cd_entry_size"))
    }
}

/// Placeholder layout plan for a whole archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutPlan {
    /// Active profile id.
    pub profile_id: String,
    /// Requested extra-field budget in bytes.
    pub extra_budget: u32,
    /// Fixed LFH prefix size from profile constants.
    pub lfh_fixed: u32,
    /// Fixed CD entry prefix size from profile constants.
    pub cd_entry_fixed: u32,
    /// Minimum EOCD size from profile constants.
    pub eocd_min: u32,
}

/// Resolve the ZIP profile named by the request.
pub fn resolve_profile(request: &GenerateRequest) -> ZbResult<ZipProfile> {
    if request.profile == ZIP_PROFILE_V0 {
        Ok(ZipProfile::v0_stub())
    } else {
        Err(ZbError::new(
            zb_types::ErrorCode::UnsupportedProfile,
            format!("unsupported profile `{}`", request.profile),
        ))
    }
}

/// Plan ZIP layout constants for the request profile.
pub fn plan(request: &GenerateRequest) -> ZbResult<LayoutPlan> {
    let profile = resolve_profile(request)?;
    Ok(LayoutPlan {
        profile_id: profile.id,
        extra_budget: request.extra_budget.min(request.limits.max_artifact_bytes as u32),
        lfh_fixed: zip_layout::LFH_FIXED,
        cd_entry_fixed: zip_layout::CD_ENTRY_FIXED,
        eocd_min: zip_layout::EOCD_MIN,
    })
}

/// Compute archive size from ordered member layouts (checked).
pub fn archive_size(members: &[MemberLayout], eocd_extra: u32) -> ZbResult<u64> {
    let mut offset: u64 = 0;
    let mut cd_size: u64 = 0;
    for member in members {
        let lfh = u64::from(member.lfh_size()?);
        let compressed = u64::from(member.compressed_len);
        offset = offset
            .checked_add(lfh)
            .and_then(|v| v.checked_add(compressed))
            .ok_or_else(|| overflow("member_offset"))?;
        cd_size = cd_size
            .checked_add(u64::from(member.cd_entry_size()?))
            .ok_or_else(|| overflow("cd_size"))?;
    }
    let eocd = u64::from(
        zip_layout::EOCD_MIN
            .checked_add(eocd_extra)
            .ok_or_else(|| overflow("eocd"))?,
    );
    offset
        .checked_add(cd_size)
        .and_then(|v| v.checked_add(eocd))
        .ok_or_else(|| overflow("archive_size"))
}

fn checked_add_u32(a: u32, b: u32) -> ZbResult<u32> {
    a.checked_add(b).ok_or_else(|| overflow("u32_add"))
}

fn overflow(what: &str) -> ZbError {
    ZbError::new(
        zb_types::ErrorCode::Internal,
        format!("checked arithmetic overflow in `{what}`"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zb_types::VariantKind;

    #[test]
    fn v0_profile_constants_match_living() {
        let req = zb_types::GenerateRequest::bare(VariantKind::Bomb);
        let plan = plan(&req).expect("plan");
        assert_eq!(plan.lfh_fixed, 30);
        assert_eq!(plan.cd_entry_fixed, 46);
        assert_eq!(plan.eocd_min, 22);
        assert_eq!(plan.profile_id, ZIP_PROFILE_V0);
    }

    #[test]
    fn archive_size_for_single_stored_member() {
        let member = MemberLayout {
            name_len: 4,
            local_extra_len: 0,
            central_extra_len: 0,
            comment_len: 0,
            compressed_len: 10,
        };
        // LFH 34 + payload 10 + CD 50 + EOCD 22 = 116
        assert_eq!(archive_size(&[member], 0).unwrap(), 116);
    }
}

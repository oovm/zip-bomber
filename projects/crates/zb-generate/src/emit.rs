//! Minimal stored-method ZIP emitter (Living `05` finite Bomb baseline).

use zb_solver::crc32;
use zb_types::{zip_layout, ErrorCode, ZbError, ZbResult};

const SIG_LFH: u32 = 0x0403_4b50;
const SIG_CD: u32 = 0x0201_4b50;
const SIG_EOCD: u32 = 0x0605_4b50;
/// General-purpose bit 11: UTF-8 names.
const GP_UTF8: u16 = 1 << 11;
const VERSION_EXTRACT: u16 = 20;
const VERSION_MADE_BY: u16 = 20;
const METHOD_STORED: u16 = 0;

/// One stored member to place in an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMember {
    /// POSIX-style member path (UTF-8).
    pub path: String,
    /// Uncompressed (and stored) payload bytes.
    pub payload: Vec<u8>,
}

/// Emit a single-volume ZIP using only the store method.
pub fn emit_stored_archive(members: &[StoredMember]) -> ZbResult<Vec<u8>> {
    if members.len() > u16::MAX as usize {
        return Err(ZbError::new(
            ErrorCode::Internal,
            "member count exceeds ZIP16 central directory limit",
        ));
    }

    let mut local = Vec::new();
    let mut central = Vec::new();
    let mut offsets: Vec<u32> = Vec::with_capacity(members.len());

    for member in members {
        let name = member.path.as_bytes();
        if name.len() > u16::MAX as usize {
            return Err(ZbError::new(
                ErrorCode::Internal,
                format!("member path `{}` exceeds u16 name length", member.path),
            ));
        }
        if member.payload.len() > u32::MAX as usize {
            return Err(ZbError::new(
                ErrorCode::Internal,
                format!("member `{}` payload exceeds u32 size", member.path),
            ));
        }
        let crc = crc32(&member.payload);
        let size = member.payload.len() as u32;
        let local_offset = u32::try_from(local.len())
            .map_err(|_| ZbError::new(ErrorCode::Internal, "local header offset exceeds u32"))?;
        offsets.push(local_offset);

        write_u32(&mut local, SIG_LFH);
        write_u16(&mut local, VERSION_EXTRACT);
        write_u16(&mut local, GP_UTF8);
        write_u16(&mut local, METHOD_STORED);
        write_u16(&mut local, 0); // mod time
        write_u16(&mut local, 0); // mod date
        write_u32(&mut local, crc);
        write_u32(&mut local, size);
        write_u32(&mut local, size);
        write_u16(&mut local, name.len() as u16);
        write_u16(&mut local, 0); // extra
        local.extend_from_slice(name);
        local.extend_from_slice(&member.payload);

        write_u32(&mut central, SIG_CD);
        write_u16(&mut central, VERSION_MADE_BY);
        write_u16(&mut central, VERSION_EXTRACT);
        write_u16(&mut central, GP_UTF8);
        write_u16(&mut central, METHOD_STORED);
        write_u16(&mut central, 0);
        write_u16(&mut central, 0);
        write_u32(&mut central, crc);
        write_u32(&mut central, size);
        write_u32(&mut central, size);
        write_u16(&mut central, name.len() as u16);
        write_u16(&mut central, 0); // extra
        write_u16(&mut central, 0); // comment
        write_u16(&mut central, 0); // disk start
        write_u16(&mut central, 0); // int attrs
        write_u32(
            &mut central,
            if member.path.ends_with('/') { 0x10 } else { 0 },
        ); // DOS directory
        write_u32(&mut central, local_offset);
        central.extend_from_slice(name);
    }

    let cd_offset = u32::try_from(local.len())
        .map_err(|_| ZbError::new(ErrorCode::Internal, "central directory offset exceeds u32"))?;
    let cd_size = u32::try_from(central.len())
        .map_err(|_| ZbError::new(ErrorCode::Internal, "central directory size exceeds u32"))?;
    let count = members.len() as u16;

    let mut out = local;
    out.extend_from_slice(&central);
    write_u32(&mut out, SIG_EOCD);
    write_u16(&mut out, 0); // disk number
    write_u16(&mut out, 0); // cd start disk
    write_u16(&mut out, count);
    write_u16(&mut out, count);
    write_u32(&mut out, cd_size);
    write_u32(&mut out, cd_offset);
    write_u16(&mut out, 0); // comment

    debug_assert_eq!(
        zip_layout::EOCD_MIN as usize,
        22,
        "EOCD fixed size must match profile"
    );
    let _ = offsets;
    Ok(out)
}

fn write_u16(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

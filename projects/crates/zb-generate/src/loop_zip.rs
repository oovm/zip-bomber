//! Two-node ZIP constructor with explicit inter-record gaps and legal TLV padding.
//! Codec construction follows the alternating program in DOI 10.3390/app14219797.
use crate::templates::loopy::encode_loop;
use crate::{emit_stored_archive, StoredMember};
use zb_solver::{
    crc32, crc32_shared_columns, solve_crc_identity_blocks_with_cancel, CancelToken, CrcByteSlot,
    SolveOutcome, SolverBudget,
};
use zb_types::{
    ArchiveGraphMember as Member, ErrorCode, GenerateRequest, LoopyEdgeFact, LoopyFacts, ZbError,
    ZbResult, ZIP_LOOPY_TWO_NODE_GAPPED_V1,
};
/// Explicit opt-in profile for duplicated early directory records and padding gaps.
pub(crate) const PROFILE: &str = ZIP_LOOPY_TWO_NODE_GAPPED_V1;
/// Maximum `ordinary || early-footer || recursive-LFH` width the alternating quote template can hold.
pub(crate) const QUOTE_WINDOW: usize = 16000;

struct Layout {
    local: Vec<u8>,
    central: Vec<u8>,
    name: Vec<u8>,
    count: u16,
    header_width: usize,
    footer_width: usize,
}
fn put16(b: &mut Vec<u8>, n: u16) {
    b.extend(n.to_le_bytes())
}
fn put32(b: &mut Vec<u8>, n: u32) {
    b.extend(n.to_le_bytes())
}
fn header(l: &Layout, crc: u32, compressed: u32, size: u32) -> Vec<u8> {
    let mut b = Vec::new();
    put32(&mut b, 0x04034b50);
    put16(&mut b, 20);
    put16(&mut b, 1 << 11);
    put16(&mut b, 8);
    b.extend([0; 4]);
    put32(&mut b, crc);
    put32(&mut b, compressed);
    put32(&mut b, size);
    put16(&mut b, l.name.len() as u16);
    let extra = l.header_width - 30 - l.name.len();
    put16(&mut b, extra as u16);
    b.extend(&l.name);
    // APPNOTE 4.6.11: Data Stream Alignment, alignment=1 and zero padding.
    put16(&mut b, 0xa11e);
    put16(&mut b, (extra - 4) as u16);
    put16(&mut b, 1);
    b.resize(l.header_width, 0);
    b
}
fn footer(l: &Layout, local_width: usize, crc: u32, compressed: u32, size: u32) -> Vec<u8> {
    let mut b = l.central.clone();
    put32(&mut b, 0x02014b50);
    put16(&mut b, 20);
    put16(&mut b, 20);
    put16(&mut b, 1 << 11);
    put16(&mut b, 8);
    b.extend([0; 4]);
    put32(&mut b, crc);
    put32(&mut b, compressed);
    put32(&mut b, size);
    put16(&mut b, l.name.len() as u16);
    b.extend([0; 12]);
    put32(&mut b, (local_width + l.footer_width) as u32);
    b.extend(&l.name);
    let cd_size = b.len();
    put32(&mut b, 0x06054b50);
    b.extend([0; 4]);
    put16(&mut b, l.count + 1);
    put16(&mut b, l.count + 1);
    put32(&mut b, cd_size as u32);
    put32(
        &mut b,
        (local_width + l.footer_width + l.header_width + compressed as usize) as u32,
    );
    let comment = l.footer_width - cd_size - 22;
    put16(&mut b, comment as u16);
    b.resize(l.footer_width, 0);
    b
}
fn assemble(
    l: &Layout,
    other: &Layout,
    ow: usize,
    compressed: u32,
    size: u32,
    max_candidates: u32,
) -> ZbResult<(Vec<u8>, Vec<CrcByteSlot>)> {
    let fa = footer(l, ow, 0, compressed, size);
    let fb = footer(other, ow, 0, compressed, size);
    let ha = header(l, 0, compressed, size);
    let hb = header(other, 0, compressed, size);
    let mut qa = fa.clone();
    qa.extend(&ha);
    let mut qb = fb.clone();
    qb.extend(&hb);
    let stream = encode_loop(&l.local, &other.local, &qa, &qb, l.header_width, max_candidates)?;
    let stream_start = ow + l.footer_width + l.header_width;
    let cd_crc = l.central.len() + 16;
    let other_crc = other.central.len() + 16;
    let mut positions = Vec::new();
    // Source headers contain target CRC (variable bytes 4..8).
    for offset in [
        ow + cd_crc,
        ow + l.footer_width + 14,
        stream_start + stream.source_copy + ow + cd_crc,
        stream_start + stream.source_copy + ow + l.footer_width + 14,
        stream_start + stream.bytes.len() + cd_crc,
    ] {
        positions.push((offset, 4));
    }
    // Quoted target headers contain source CRC (variable bytes 0..4).
    for offset in [
        stream_start + stream.target_copy + ow + other_crc,
        stream_start + stream.target_copy + ow + l.footer_width + 14,
    ] {
        positions.push((offset, 0));
    }
    let mut bytes = l.local.clone();
    bytes.extend(fa);
    bytes.extend(ha);
    bytes.extend(&stream.bytes);
    bytes.extend(footer(l, ow, 0, compressed, size));
    let slots = positions
        .into_iter()
        .flat_map(|(offset, v)| {
            (0..4).map(move |i| CrcByteSlot {
                offset: offset + i,
                variable: v + i,
            })
        })
        .collect();
    Ok((bytes, slots))
}
struct Measured {
    layouts: Vec<Layout>,
    ordinary_widths: Vec<u64>,
    ordinary_members: Vec<usize>,
    archive_paths: Vec<String>,
    ow: usize,
    hw: usize,
    fw: usize,
}

fn measure(request: &GenerateRequest, states: &[usize]) -> ZbResult<Measured> {
    let g = request.graph.as_ref().unwrap();
    let mut layouts = Vec::new();
    let mut ordinary_widths = Vec::new();
    let mut ordinary_members = Vec::new();
    let mut archive_paths = Vec::new();
    for &n in states {
        let list = &g.nodes[n].members;
        let (name, path) = match list.last() {
            Some(Member::Archive { path, target, .. })
                if states.contains(target) && *target != n =>
            {
                (path.as_bytes().to_vec(), path.clone())
            }
            _ => {
                return Err(ZbError::new(
                    ErrorCode::UnsupportedTopology,
                    "two-node template requires one cycle edge last",
                ))
            }
        };
        let ordinary = list[..list.len() - 1]
            .iter()
            .map(|m| match m {
                Member::Payload { path, bytes } => Ok(StoredMember {
                    path: path.clone(),
                    payload: zb_types::decode_base64(bytes)?,
                }),
                _ => Err(ZbError::new(
                    ErrorCode::UnsupportedTopology,
                    "two-node template currently supports ordinary attachments before its sole archive edge",
                )),
            })
            .collect::<ZbResult<Vec<_>>>()?;
        let zip = emit_stored_archive(&ordinary)?;
        let end = zip.len() - 22;
        let cd_offset = u32::from_le_bytes(zip[end + 16..end + 20].try_into().unwrap()) as usize;
        ordinary_widths.push(cd_offset as u64);
        ordinary_members.push(ordinary.len());
        archive_paths.push(path);
        layouts.push(Layout {
            local: zip[..cd_offset].to_vec(),
            central: zip[cd_offset..end].to_vec(),
            name,
            count: ordinary.len() as u16,
            header_width: 0,
            footer_width: 0,
        });
    }
    let ow = layouts.iter().map(|l| l.local.len()).max().unwrap();
    let hw = layouts.iter().map(|l| 30 + l.name.len() + 6).max().unwrap();
    let fw = layouts
        .iter()
        .map(|l| l.central.len() + 46 + l.name.len() + 22)
        .max()
        .unwrap();
    if ow + fw + hw > QUOTE_WINDOW || hw > 65535 || fw > 65535 {
        return Err(ZbError::resource_limited(
            "two-node quote layout exceeds supported window",
        ));
    }
    for l in &mut layouts {
        l.local.resize(ow, 0);
        l.header_width = hw;
        l.footer_width = fw;
    }
    Ok(Measured {
        layouts,
        ordinary_widths,
        ordinary_members,
        archive_paths,
        ow,
        hw,
        fw,
    })
}

/// Layout feasibility for plan: quote window plus a bounded tail-width template candidate.
/// CRC search is not run here. Unsatisfiable tail width returns `search_exhausted`.
pub(crate) fn check_layout(request: &GenerateRequest, states: &[usize]) -> ZbResult<()> {
    let measured = measure(request, states)?;
    let Measured { layouts, ow, .. } = measured;
    let _ = assemble(
        &layouts[0],
        &layouts[1],
        ow,
        0,
        0,
        request.limits.max_template_candidates,
    )?;
    Ok(())
}

/// Generate simultaneous byte/CRC fixed points for one two-state SCC.
pub(crate) fn generate(
    request: &GenerateRequest,
    states: &[usize],
    cancel: &CancelToken,
) -> ZbResult<(Vec<(usize, Vec<u8>)>, LoopyFacts, zb_types::SolverFacts)> {
    cancel.check()?;
    let measured = measure(request, states)?;
    let Measured {
        layouts,
        ordinary_widths,
        ordinary_members,
        archive_paths,
        ow,
        hw,
        fw,
    } = measured;
    let (provisional, _) =
        assemble(&layouts[0], &layouts[1], ow, 0, 0, request.limits.max_template_candidates)?;
    let stream_len = provisional.len() - ow - fw - hw - fw;
    let total = provisional.len();
    if total as u64 > request.limits.max_artifact_bytes
        || 2 * total as u64 > request.limits.max_total_artifact_bytes
        || total as u64 * 32 > request.limits.max_live_memory_bytes
        || total as u64 * 4 > request.limits.max_decoded_bytes
        || request.limits.max_constraint_bits < 64 * 65
        || request.limits.max_solver_operations < (total as u64 * 16 + 1048576)
    {
        return Err(ZbError::resource_limited(
            "two-node layout or solver budget exceeded",
        ));
    }
    let (mut a, sa) = assemble(
        &layouts[0],
        &layouts[1],
        ow,
        stream_len as u32,
        total as u32,
        request.limits.max_template_candidates,
    )?;
    let (mut b, mut sb) = assemble(
        &layouts[1],
        &layouts[0],
        ow,
        stream_len as u32,
        total as u32,
        request.limits.max_template_candidates,
    )?;
    // assemble labels source as 0 and target as 1, reversed for the other archive.
    for slot in &mut sb {
        slot.variable = (slot.variable + 4) % 8;
    }
    if a.len() != total || b.len() != total {
        return Err(ZbError::verification_failed(
            "loop layout changed after sizing",
        ));
    }
    let (ca, cols_a) = crc32_shared_columns(&a, &sa, 8)?;
    let (cb, cols_b) = crc32_shared_columns(&b, &sb, 8)?;
    let outcome = solve_crc_identity_blocks_with_cancel(
        &[(ca, cols_a.as_slice()), (cb, cols_b.as_slice())],
        64,
        SolverBudget::default(),
        Some("loopy.crc.shared/v1".into()),
        cancel,
    );
    let SolveOutcome::Sat {
        assignment,
        evidence,
        ..
    } = outcome
    else {
        return Err(match outcome {
            SolveOutcome::Unsat { message, .. } => ZbError {
                category: zb_types::ErrorCategory::ConstraintUnsatisfied,
                code: ErrorCode::Internal,
                message: format!("loopy CRC residual unsat: {message}"),
            },
            SolveOutcome::ResourceLimited { message } => ZbError::resource_limited(message),
            SolveOutcome::Cancelled { message } => ZbError::cancelled(message),
            SolveOutcome::InvalidProblem { message }
            | SolveOutcome::InternalError { message } => ZbError::new(ErrorCode::Internal, message),
            SolveOutcome::Sat { .. } => unreachable!(),
        });
    };
    let mut values = [0u8; 8];
    for (bit, value) in assignment.bits.iter().enumerate() {
        if *value {
            values[bit / 8] |= 1 << (bit % 8)
        }
    }
    for (bytes, slots) in [(&mut a, &sa), (&mut b, &sb)] {
        for slot in slots {
            bytes[slot.offset] = values[slot.variable];
        }
    }
    let crc_a = u32::from_le_bytes(values[..4].try_into().unwrap());
    let crc_b = u32::from_le_bytes(values[4..].try_into().unwrap());
    if crc32(&a) != crc_a || crc32(&b) != crc_b {
        return Err(ZbError::verification_failed(
            "loop CRC solution failed back substitution",
        ));
    }
    for (bytes, expected) in [(&a, &b), (&b, &a)] {
        let data = &bytes[ow + fw + hw..ow + fw + hw + stream_len];
        let mut out = vec![0; total + 1];
        let mut decoder = flate2::Decompress::new(false);
        let status = decoder
            .decompress(data, &mut out, flate2::FlushDecompress::Finish)
            .map_err(|e| ZbError::verification_failed(e.to_string()))?;
        out.truncate(decoder.total_out() as usize);
        if status != flate2::Status::StreamEnd
            || decoder.total_in() as usize != stream_len
            || &out != expected
        {
            return Err(ZbError::verification_failed(
                "loop complete byte equality failed",
            ));
        }
    }
    if request.variant == zb_types::VariantKind::Loopy && a == b {
        return Err(ZbError::new(
            ErrorCode::PeriodMismatch,
            "two-state loop collapses to byte period one",
        ));
    }
    let byte_period = if a == b { 1 } else { 2 };
    let facts = LoopyFacts {
        requested_nodes: states.len(),
        byte_period,
        profile: PROFILE.to_string(),
        ordinary_members,
        quote_window: QUOTE_WINDOW as u64,
        ordinary_widths,
        crc_rank: evidence.rank as u32,
        crc_free_bits: evidence.free_bits as u32,
        edges: vec![
            LoopyEdgeFact {
                from: states[0],
                to: states[1],
                path: archive_paths[0].clone(),
                compressed_len: stream_len as u64,
                target_len: b.len() as u64,
                target_crc: crc_b,
                bytes_equal: true,
            },
            LoopyEdgeFact {
                from: states[1],
                to: states[0],
                path: archive_paths[1].clone(),
                compressed_len: stream_len as u64,
                target_len: a.len() as u64,
                target_crc: crc_a,
                bytes_equal: true,
            },
        ],
    };
    let solver = zb_types::SolverFacts {
        rank: evidence.rank as u32,
        free_bits: evidence.free_bits as u32,
        equations_used: evidence.equations_used as u32,
        problem_digest: evidence.problem_digest.clone(),
    };
    Ok((vec![(states[0], a), (states[1], b)], facts, solver))
}

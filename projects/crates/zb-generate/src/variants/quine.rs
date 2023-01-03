//! Strict self-referential ZIP quine.

use zb_solver::{crc32_shared_columns, solve_rows, CrcByteSlot};
use zb_types::{
    Artifact, ArtifactId, ArtifactSet, ContainerKind, GenerateReport, GenerateRequest, ZbError,
    ZbResult, ZIP_PROFILE_V0,
};

use crate::templates::{encode_recursive_stream, list_quine_templates, QuineTemplateMeta};

use super::ArtifactBundle;

/// Generate a quine archive by enumerating verified templates.
///
/// Returns [`ZbError::search_exhausted`] when the registry has no candidate for
/// the request profile. Topology must already pass [`crate::validate`] quine
/// rules (single self-loop).
pub fn generate(request: GenerateRequest) -> ZbResult<ArtifactBundle> {
    let profile = if request.profile.is_empty() {
        ZIP_PROFILE_V0
    } else {
        request.profile.as_str()
    };
    let candidates = list_quine_templates(profile);
    if candidates.is_empty() {
        return Err(ZbError::search_exhausted(format!(
            "no verified quine templates registered for profile `{profile}` (searched 0 candidates)"
        )));
    }

    let mut attempts = Vec::new();
    for meta in &candidates {
        match try_template(&request, meta) {
            Ok(bundle) => return Ok(bundle),
            Err(err)
                if matches!(
                    err.category,
                    zb_types::ErrorCategory::ConstraintUnsatisfied
                        | zb_types::ErrorCategory::SearchExhausted
                ) =>
            {
                attempts.push(format!("{}: {}", meta.id, err.message))
            }
            Err(err) => return Err(err),
        }
    }

    Err(ZbError::search_exhausted(format!(
        "quine template search exhausted for profile `{profile}` after {} candidates ({})",
        candidates.len(),
        attempts.join("; ")
    )))
}

fn try_template(request: &GenerateRequest, meta: &QuineTemplateMeta) -> ZbResult<ArtifactBundle> {
    if meta.id != "quine.stored-fixed.prefix-suffix" {
        return Err(ZbError::not_implemented("unknown quine template"));
    }
    if !request.payloads.is_empty() {
        return Err(ZbError::not_implemented(
            "this quine template does not yet embed ordinary payload members",
        ));
    }
    if request.limits.max_nodes < 1
        || request.limits.max_edges < 1
        || request.limits.max_members < 1
    {
        return Err(ZbError::resource_limited(
            "quine requires one node, edge, and member",
        ));
    }
    let name = request
        .topology
        .edges
        .first()
        .map(|e| e.member_path.as_str())
        .unwrap_or("self.zip");
    if name.len() > u16::MAX as usize {
        return Err(ZbError::resource_limited("quine member name exceeds ZIP16"));
    }
    let name = name.as_bytes();
    let prefix_len = 30 + name.len();
    let suffix_len = 46 + name.len() + 22;
    let working_bound = (prefix_len + suffix_len) as u64 * 16 + 65536;
    if working_bound > request.limits.max_live_memory_bytes {
        return Err(ZbError::resource_limited(
            "quine working set exceeds max_live_memory_bytes",
        ));
    }
    let zero_crc = [0u8; 4];
    let mut prefix = local_header(name, &zero_crc, 0, 0);
    let mut suffix = central_and_eocd(name, &zero_crc, 0, 0);
    // Measure once. A second compile uses the same frozen width choice and
    // does not count as another outer candidate search.
    let stream = encode_recursive_stream(&prefix, &suffix, request.limits.max_template_candidates)?;
    let archive_len = prefix_len + stream.bytes.len() + suffix_len;
    if archive_len as u64 > request.limits.max_artifact_bytes {
        return Err(ZbError::resource_limited(
            "quine artifact exceeds max_artifact_bytes",
        ));
    }
    if archive_len as u64 > request.limits.max_total_artifact_bytes
        || archive_len as u64 > request.limits.max_decoded_bytes
    {
        return Err(ZbError::resource_limited(
            "quine exceeds total artifact or decode budget",
        ));
    }
    let operation_bound = archive_len as u64 * 8 + 262144;
    if request.limits.max_constraint_bits < 32 * 33
        || request.limits.max_solver_operations < operation_bound
    {
        return Err(ZbError::resource_limited(
            "quine CRC matrix or operation budget exhausted",
        ));
    }
    let compressed_len = stream.bytes.len() as u32;
    let archive_len32 = u32::try_from(archive_len)
        .map_err(|_| ZbError::resource_limited("quine archive exceeds ZIP32"))?;
    prefix = local_header(name, &zero_crc, compressed_len, archive_len32);
    suffix = central_and_eocd(name, &zero_crc, compressed_len, archive_len32);
    let stream = encode_recursive_stream(&prefix, &suffix, request.limits.max_template_candidates)?;
    if stream.bytes.len() != compressed_len as usize {
        return Err(ZbError::search_exhausted(
            "quine layout length changed after header sizing",
        ));
    }
    let mut archive = prefix;
    archive.extend_from_slice(&stream.bytes);
    archive.extend_from_slice(&suffix);
    let cd_offset = 30 + name.len() + stream.bytes.len();
    let crc_offsets = [
        14,
        prefix_len + stream.prefix_copy + 14,
        prefix_len + stream.suffix_copy + 16,
        cd_offset + 16,
    ];
    let slots: Vec<_> = crc_offsets
        .iter()
        .flat_map(|&offset| {
            (0..4).map(move |variable| CrcByteSlot {
                offset: offset + variable,
                variable,
            })
        })
        .collect();
    let (constant, columns) = crc32_shared_columns(&archive, &slots, 4)?;
    let mut coeffs = vec![vec![false; 32]; 32];
    let rhs = zb_solver::u32_to_bits(constant);
    for row in 0..32 {
        for bit in 0..32 {
            coeffs[row][bit] = ((columns[bit] >> row) & 1) != 0;
        }
        coeffs[row][row] ^= true;
    }
    let solution = solve_rows(&coeffs, &rhs)?;
    let mut crc = 0u32;
    for (bit, value) in solution.assignment.iter().enumerate() {
        if *value {
            crc |= 1u32 << bit;
        }
    }
    for offset in crc_offsets {
        archive[offset..offset + 4].copy_from_slice(&crc.to_le_bytes());
    }
    if zb_solver::crc32(&archive) != crc {
        return Err(ZbError::verification_failed(
            "quine CRC solution failed back-substitution",
        ));
    }
    let mut check = flate2::Decompress::new(false);
    let mut decoded = vec![0; archive_len + 1];
    let status = check
        .decompress(
            &archive[prefix_len..prefix_len + stream.bytes.len()],
            &mut decoded,
            flate2::FlushDecompress::Finish,
        )
        .map_err(|e| ZbError::verification_failed(format!("quine inflate failed: {e}")))?;
    if status != flate2::Status::StreamEnd || check.total_in() as usize != stream.bytes.len() {
        return Err(ZbError::verification_failed(
            "quine decoder did not consume the complete stream",
        ));
    }
    decoded.truncate(check.total_out() as usize);
    if decoded != archive {
        return Err(ZbError::verification_failed(
            "quine decoded bytes differ from archive",
        ));
    }
    let verified = crate::verify::verify_zip_bytes(&archive)?;
    if !verified.ok {
        return Err(ZbError::verification_failed(
            "Acorn quine verification failed",
        ));
    }
    let artifact = Artifact {
        id: ArtifactId::new("quine-root"),
        container: ContainerKind::Zip,
        byte_length: archive.len().to_string(),
        digest: String::new(),
        bytes: archive,
    };
    let artifact_id = artifact.id.clone();
    let report = GenerateReport::ok(
        request.variant,
        zb_types::CrcStrategy::GaussianElimination,
        1,
        1,
        artifact.bytes.len() as u64,
        artifact.bytes.len() as u64,
        vec![
            format!("template={}", meta.id),
            format!(
                "crc_rank={}; requested_crc={}; shared_crc_occurrences=4; length_candidates={}",
                solution.rank,
                request.crc_strategy.as_str(),
                stream.length_candidates
            ),
        ],
    );
    Ok(ArtifactBundle {
        artifacts: ArtifactSet {
            root: artifact_id.clone(),
            artifacts: vec![artifact],
            logical_node_map: vec![format!("{}={}", request.topology.nodes[0].0, artifact_id.0)],
        },
        report,
    })
}

fn local_header(name: &[u8], crc: &[u8; 4], compressed: u32, size: u32) -> Vec<u8> {
    let mut b = Vec::with_capacity(30 + name.len());
    b.extend_from_slice(&0x04034b50u32.to_le_bytes());
    b.extend_from_slice(&20u16.to_le_bytes());
    b.extend_from_slice(&0x800u16.to_le_bytes());
    b.extend_from_slice(&8u16.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(crc);
    b.extend_from_slice(&compressed.to_le_bytes());
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&(name.len() as u16).to_le_bytes());
    b.extend_from_slice(&0u16.to_le_bytes());
    b.extend_from_slice(name);
    b
}

fn central_and_eocd(name: &[u8], crc: &[u8; 4], compressed: u32, size: u32) -> Vec<u8> {
    let mut b = Vec::with_capacity(46 + name.len() + 22);
    b.extend_from_slice(&0x02014b50u32.to_le_bytes());
    b.extend_from_slice(&20u16.to_le_bytes());
    b.extend_from_slice(&20u16.to_le_bytes());
    b.extend_from_slice(&0x800u16.to_le_bytes());
    b.extend_from_slice(&8u16.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(crc);
    b.extend_from_slice(&compressed.to_le_bytes());
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&(name.len() as u16).to_le_bytes());
    b.extend_from_slice(&[0; 8]);
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(name);
    b.extend_from_slice(&0x06054b50u32.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&((46 + name.len()) as u32).to_le_bytes());
    b.extend_from_slice(&((30 + name.len()) as u32 + compressed).to_le_bytes());
    b.extend_from_slice(&0u16.to_le_bytes());
    b
}

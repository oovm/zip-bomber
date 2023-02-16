//! Strict self-referential ZIP quine (optional ordinary attachments).

use zb_solver::{CancelToken, CrcByteSlot, SharedCrcArchive, SolveOutcome, SolverBudget, crc32, solve_shared_crc_with_cancel};
use zb_types::{
    Artifact, ArtifactId, ArtifactSet, ContainerKind, GenerateReport, GenerateRequest, PayloadSpec, ZIP_PROFILE_V0, ZbError,
    ZbResult,
};

use acorn_codec::compression::deflate::Symbol;

use crate::{
    PlanBudgetEstimate, TaskBudgetLedger,
    deflate::{decode_raw_deflate, encode_fixed_stream, encode_stored_stream},
    paths::validate_quine_members,
    templates::{QuineTemplateMeta, encode_recursive_stream, list_quine_templates},
};

use super::ArtifactBundle;

const GP_UTF8: u16 = 1 << 11;
const METHOD_STORED: u16 = 0;
const METHOD_DEFLATE: u16 = 8;
const DOS_DIR_ATTR: u32 = 0x10;
/// Prefix may quote at most this many leading bytes under the current template.
const MAX_PREFIX: usize = 32763;
/// Suffix length pre-check used by the recursive encoder.
const MAX_SUFFIX: usize = 32755;

#[derive(Debug, Clone)]
struct OrdinaryMember {
    name: Vec<u8>,
    is_dir: bool,
    method: u16,
    crc: u32,
    uncompressed: u32,
    data: Vec<u8>,
    original: Vec<u8>,
}

/// Generate a quine archive by enumerating verified templates.
pub fn generate(request: GenerateRequest) -> ZbResult<ArtifactBundle> {
    generate_with_cancel(request, &CancelToken::new())
}

/// [`generate`] with cooperative cancel checks around CRC solve steps.
pub fn generate_with_cancel(request: GenerateRequest, cancel: &CancelToken) -> ZbResult<ArtifactBundle> {
    cancel.check()?;
    let profile = if request.profile.is_empty() { ZIP_PROFILE_V0 } else { request.profile.as_str() };
    let candidates = list_quine_templates(profile);
    if candidates.is_empty() {
        return Err(ZbError::search_exhausted(format!(
            "no verified quine templates registered for profile `{profile}` (searched 0 candidates)"
        )));
    }

    let mut attempts = Vec::new();
    for meta in &candidates {
        cancel.check()?;
        match try_template(&request, meta, cancel) {
            Ok(bundle) => return Ok(bundle),
            Err(err)
                if matches!(
                    err.category,
                    zb_types::ErrorCategory::ConstraintUnsatisfied | zb_types::ErrorCategory::SearchExhausted
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

/// Compress ordinary payloads and report layout budgets without emitting CRC.
pub fn plan_layout(request: &GenerateRequest) -> ZbResult<QuineLayoutPlan> {
    prepare_layout(request).map(|(layout, _)| layout)
}

/// Plan-time materialization estimate and the same budget gates as `generate` before CRC solve.
pub fn plan_materialization_budget(request: &GenerateRequest) -> ZbResult<PlanBudgetEstimate> {
    let sizing = quine_materialization_sizing(request)?;
    Ok(PlanBudgetEstimate {
        index_passes: 1,
        member_decodes: sizing.ordinary_members as u64 + 1,
        decoded_bytes: sizing.decoded_bytes,
        read_bytes: sizing.archive_len as u64 * 2,
        solver_operations: sizing.operation_bound,
        constraint_bits: 32 * 33,
    })
}

struct QuineMaterializationSizing {
    archive_len: usize,
    ordinary_members: usize,
    decoded_bytes: u64,
    operation_bound: u64,
}

fn quine_materialization_sizing(request: &GenerateRequest) -> ZbResult<QuineMaterializationSizing> {
    if request.limits.max_nodes < 1 || request.limits.max_edges < 1 || request.limits.max_members < 1 {
        return Err(ZbError::resource_limited("quine requires one node, edge, and member"));
    }
    let (layout, ordinary) = prepare_layout(request)?;
    let self_name = layout.fixed_point.as_bytes();
    let ordinary_local = emit_ordinary_locals(&ordinary)?;
    let self_lfh_offset = ordinary_local.len();
    let input_bytes: u64 = ordinary.iter().map(|m| m.original.len() as u64).sum();
    let working_bound = (layout.prefix_len + layout.suffix_len) as u64 * 16 + input_bytes.saturating_mul(4) + 65536;
    if working_bound > request.limits.max_live_memory_bytes {
        return Err(ZbError::resource_limited("quine working set exceeds max_live_memory_bytes"));
    }
    let zero_crc = [0u8; 4];
    let mut prefix = ordinary_local.clone();
    prefix.extend(local_header(self_name, &zero_crc, 0, 0, METHOD_DEFLATE, false));
    let ordinary_central = emit_ordinary_central(&ordinary)?;
    let mut suffix = ordinary_central;
    suffix.extend(central_header(self_name, &zero_crc, 0, 0, METHOD_DEFLATE, self_lfh_offset as u32, false));
    let provisional_cd_size = suffix.len() as u32;
    suffix.extend(eocd((ordinary.len() + 1) as u16, provisional_cd_size, prefix.len() as u32));
    let stream = encode_recursive_stream(&prefix, &suffix, request.limits.max_template_candidates)?;
    let archive_len = prefix.len() + stream.bytes.len() + suffix.len();
    if archive_len as u64 > request.limits.max_artifact_bytes {
        return Err(ZbError::resource_limited("quine artifact exceeds max_artifact_bytes"));
    }
    let decoded_bytes = (archive_len as u64).saturating_add(input_bytes);
    if archive_len as u64 > request.limits.max_total_artifact_bytes || decoded_bytes > request.limits.max_decoded_bytes {
        return Err(ZbError::resource_limited("quine exceeds total artifact or decode budget"));
    }
    let operation_bound = archive_len as u64 * 8 + 262144;
    if request.limits.max_constraint_bits < 32 * 33 || request.limits.max_solver_operations < operation_bound {
        return Err(ZbError::resource_limited("quine CRC matrix or operation budget exhausted"));
    }
    Ok(QuineMaterializationSizing { archive_len, ordinary_members: ordinary.len(), decoded_bytes, operation_bound })
}

fn prepare_layout(request: &GenerateRequest) -> ZbResult<(QuineLayoutPlan, Vec<OrdinaryMember>)> {
    let self_name = fixed_point(request)?;
    if request.payloads.len() + 1 > request.limits.max_members as usize {
        return Err(ZbError::resource_limited("quine member count exceeds max_members"));
    }
    validate_quine_members(self_name, &request.payloads)?;
    let input_bytes: u64 = request.payloads.iter().map(|p| p.bytes.len() as u64).sum();
    if input_bytes > request.limits.max_input_bytes {
        return Err(ZbError::resource_limited("quine payloads exceed max_input_bytes"));
    }
    if input_bytes.saturating_mul(4).saturating_add(65536) > request.limits.max_live_memory_bytes {
        return Err(ZbError::resource_limited("quine input working set exceeds max_live_memory_bytes"));
    }
    let ordinary = prepare_ordinary_members(&request.payloads, request.limits.max_input_bytes)?;
    let self_name_bytes = self_name.as_bytes();
    let ordinary_local = emit_ordinary_locals(&ordinary)?;
    let ordinary_central = emit_ordinary_central(&ordinary)?;
    let prefix_len = ordinary_local.len() + 30 + self_name_bytes.len();
    let suffix_len = ordinary_central.len() + 46 + self_name_bytes.len() + 22;
    if prefix_len > MAX_PREFIX || suffix_len > MAX_SUFFIX {
        return Err(ZbError::resource_limited(format!(
            "quine layout exceeds recursive window (prefix={prefix_len} suffix={suffix_len})"
        )));
    }
    let compressed_payload_bytes: u64 = ordinary.iter().map(|m| m.data.len() as u64).sum();
    Ok((
        QuineLayoutPlan {
            fixed_point: self_name.to_string(),
            ordinary_members: ordinary.len(),
            prefix_len,
            suffix_len,
            compressed_payload_bytes,
            prefix_budget_remaining: MAX_PREFIX - prefix_len,
            suffix_budget_remaining: MAX_SUFFIX - suffix_len,
        },
        ordinary,
    ))
}

/// Layout estimate used by `plan` and reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuineLayoutPlan {
    /// Self-reference member path.
    pub fixed_point: String,
    /// Ordinary payload / directory count.
    pub ordinary_members: usize,
    /// Serialized prefix length including the self local header.
    pub prefix_len: usize,
    /// Serialized suffix length including central directories and EOCD.
    pub suffix_len: usize,
    /// Sum of compressed ordinary member payloads.
    pub compressed_payload_bytes: u64,
    /// Remaining prefix budget under the current template.
    pub prefix_budget_remaining: usize,
    /// Remaining suffix budget under the current template.
    pub suffix_budget_remaining: usize,
}

fn try_template(request: &GenerateRequest, meta: &QuineTemplateMeta, cancel: &CancelToken) -> ZbResult<ArtifactBundle> {
    if meta.id != "quine.stored-fixed.prefix-suffix" {
        return Err(ZbError::not_implemented("unknown quine template"));
    }
    if request.limits.max_nodes < 1 || request.limits.max_edges < 1 || request.limits.max_members < 1 {
        return Err(ZbError::resource_limited("quine requires one node, edge, and member"));
    }
    let (layout, ordinary) = prepare_layout(request)?;
    let sizing = quine_materialization_sizing(request)?;
    let self_name = layout.fixed_point.as_bytes();
    let ordinary_local = emit_ordinary_locals(&ordinary)?;
    let self_lfh_offset = ordinary_local.len();
    let zero_crc = [0u8; 4];
    let mut prefix = ordinary_local.clone();
    prefix.extend(local_header(self_name, &zero_crc, 0, 0, METHOD_DEFLATE, false));
    let ordinary_central = emit_ordinary_central(&ordinary)?;
    let mut suffix = ordinary_central;
    suffix.extend(central_header(self_name, &zero_crc, 0, 0, METHOD_DEFLATE, self_lfh_offset as u32, false));
    let provisional_cd_size = suffix.len() as u32;
    suffix.extend(eocd((ordinary.len() + 1) as u16, provisional_cd_size, prefix.len() as u32));
    let stream = encode_recursive_stream(&prefix, &suffix, request.limits.max_template_candidates)?;
    let archive_len = sizing.archive_len;
    let compressed_len = stream.bytes.len() as u32;
    let archive_len32 = u32::try_from(archive_len).map_err(|_| ZbError::resource_limited("quine archive exceeds ZIP32"))?;
    prefix = ordinary_local;
    prefix.extend(local_header(self_name, &zero_crc, compressed_len, archive_len32, METHOD_DEFLATE, false));
    let ordinary_central = emit_ordinary_central(&ordinary)?;
    suffix = ordinary_central;
    suffix.extend(central_header(
        self_name,
        &zero_crc,
        compressed_len,
        archive_len32,
        METHOD_DEFLATE,
        self_lfh_offset as u32,
        false,
    ));
    let cd_size = suffix.len() as u32;
    let cd_offset = (prefix.len() + compressed_len as usize) as u32;
    suffix.extend(eocd((ordinary.len() + 1) as u16, cd_size, cd_offset));

    let stream = encode_recursive_stream(&prefix, &suffix, request.limits.max_template_candidates)?;
    if stream.bytes.len() != compressed_len as usize {
        return Err(ZbError::search_exhausted("quine layout length changed after header sizing"));
    }
    let mut archive = prefix;
    archive.extend_from_slice(&stream.bytes);
    archive.extend_from_slice(&suffix);

    let self_crc_in_prefix = self_lfh_offset + 14;
    let cd_self_in_suffix = rebuilt_central_len(&ordinary);
    let prefix_len = self_lfh_offset + 30 + self_name.len();
    let crc_offsets = [
        self_crc_in_prefix,
        prefix_len + stream.prefix_copy + self_crc_in_prefix,
        prefix_len + stream.suffix_copy + cd_self_in_suffix + 16,
        prefix_len + stream.bytes.len() + cd_self_in_suffix + 16,
    ];

    let slots: Vec<_> = crc_offsets
        .iter()
        .flat_map(|&offset| (0..4).map(move |variable| CrcByteSlot { offset: offset + variable, variable }))
        .collect();
    let outcome = solve_shared_crc_with_cancel(
        &[SharedCrcArchive { bytes: &archive, slots: &slots, target_byte_vars: [0, 1, 2, 3] }],
        4,
        SolverBudget::default(),
        Some("quine.crc.shared/v0".into()),
        cancel,
    );
    let SolveOutcome::Sat { assignment, evidence, .. } = outcome
    else {
        return Err(match outcome {
            SolveOutcome::Unsat { message, .. } => ZbError {
                category: zb_types::ErrorCategory::ConstraintUnsatisfied,
                code: zb_types::ErrorCode::Internal,
                message: format!("quine CRC residual unsat: {message}"),
            },
            SolveOutcome::ResourceLimited { message } => ZbError::resource_limited(message),
            SolveOutcome::Cancelled { message } => ZbError::cancelled(message),
            SolveOutcome::InvalidProblem { message } | SolveOutcome::InternalError { message } => {
                ZbError::new(zb_types::ErrorCode::Internal, message)
            }
            SolveOutcome::Sat { .. } => unreachable!(),
        });
    };
    let mut crc = 0u32;
    for (bit, value) in assignment.bits.iter().enumerate() {
        if *value {
            crc |= 1u32 << bit;
        }
    }
    for offset in crc_offsets {
        archive[offset..offset + 4].copy_from_slice(&crc.to_le_bytes());
    }
    if crc32(&archive) != crc {
        return Err(ZbError::verification_failed("quine CRC solution failed back-substitution"));
    }

    let mut verify_ledger = TaskBudgetLedger::new(request.limits.clone());
    let decoded = crate::deflate::decode_raw_deflate_with_ledger(
        &archive[prefix_len..prefix_len + stream.bytes.len()],
        archive_len as u64,
        &mut verify_ledger,
    )?;
    if decoded != archive {
        return Err(ZbError::verification_failed("quine decoded bytes differ from archive"));
    }
    verify_ordinary_members(&archive, &ordinary)?;
    let verified = crate::verify::verify_zip_bytes_with_ledger(&archive, &mut verify_ledger)?;
    if !verified.ok {
        return Err(ZbError::verification_failed("Acorn quine verification failed"));
    }
    let budget_usage = verify_ledger.usage();

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
        sizing.decoded_bytes,
        vec![
            format!("template={}", meta.id),
            format!("fixed_point={}", layout.fixed_point),
            format!("ordinary_members={}", layout.ordinary_members),
            format!(
                "prefix_len={}; suffix_len={}; compressed_payload_bytes={}; prefix_budget_remaining={}; suffix_budget_remaining={}",
                layout.prefix_len,
                layout.suffix_len,
                layout.compressed_payload_bytes,
                layout.prefix_budget_remaining,
                layout.suffix_budget_remaining
            ),
            format!(
                "shared_crc_occurrences=4; length_candidates={}",
                stream.length_candidates
            ),
        ],
    )
    .with_profile(request.profile.clone())
    .with_solver(zb_types::SolverFacts {
        rank: evidence.rank as u32,
        free_bits: evidence.free_bits as u32,
        equations_used: evidence.equations_used as u32,
        problem_digest: evidence.problem_digest.clone(),
    })
    .with_budget_usage(budget_usage);
    Ok(ArtifactBundle {
        artifacts: ArtifactSet {
            root: artifact_id.clone(),
            artifacts: vec![artifact],
            logical_node_map: vec![format!("{}={}", request.topology.nodes[0].0, artifact_id.0)],
        },
        report,
    })
}

fn fixed_point(request: &GenerateRequest) -> ZbResult<&str> {
    request
        .topology
        .edges
        .first()
        .map(|e| e.member_path.as_str())
        .ok_or_else(|| ZbError::new(zb_types::ErrorCode::InvalidTopology, "quine missing edge"))
}

fn prepare_ordinary_members(payloads: &[PayloadSpec], max_input_bytes: u64) -> ZbResult<Vec<OrdinaryMember>> {
    let total: u64 = payloads.iter().map(|p| p.bytes.len() as u64).sum();
    if total > max_input_bytes {
        return Err(ZbError::resource_limited("quine payloads exceed max_input_bytes"));
    }
    let mut out = Vec::with_capacity(payloads.len());
    for payload in payloads {
        let is_dir = payload.path.ends_with('/');
        let name = payload.path.as_bytes().to_vec();
        let original = payload.bytes.clone();
        let (method, data) = if is_dir { (METHOD_STORED, Vec::new()) } else { compress_payload(&original)? };
        let uncompressed =
            u32::try_from(original.len()).map_err(|_| ZbError::resource_limited("payload exceeds ZIP32 size"))?;
        let crc = crc32(&original);
        out.push(OrdinaryMember { name, is_dir, method, crc, uncompressed, data, original });
    }
    Ok(out)
}

fn compress_payload(data: &[u8]) -> ZbResult<(u16, Vec<u8>)> {
    if data.is_empty() {
        return Ok((METHOD_STORED, Vec::new()));
    }
    if data.len() > 65535 {
        return Err(ZbError::resource_limited("attachment exceeds stored DEFLATE block limit"));
    }
    let stored = encode_stored_stream(data)?;
    let literals = data.iter().map(|&b| Symbol::Literal(b)).collect::<Vec<_>>();
    let fixed = encode_fixed_stream(&literals)?;
    let (method, payload) = if fixed.len() < stored.len() {
        (METHOD_DEFLATE, fixed)
    }
    else if stored.len() < data.len() {
        (METHOD_DEFLATE, stored)
    }
    else {
        (METHOD_STORED, data.to_vec())
    };
    Ok((method, payload))
}

fn emit_ordinary_locals(members: &[OrdinaryMember]) -> ZbResult<Vec<u8>> {
    let mut out = Vec::new();
    for member in members {
        out.extend(local_header(
            &member.name,
            &member.crc.to_le_bytes(),
            member.data.len() as u32,
            member.uncompressed,
            member.method,
            member.is_dir,
        ));
        out.extend_from_slice(&member.data);
    }
    Ok(out)
}

fn emit_ordinary_central(members: &[OrdinaryMember]) -> ZbResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut local_cursor = 0usize;
    for member in members {
        out.extend(central_header(
            &member.name,
            &member.crc.to_le_bytes(),
            member.data.len() as u32,
            member.uncompressed,
            member.method,
            local_cursor as u32,
            member.is_dir,
        ));
        local_cursor += 30 + member.name.len() + member.data.len();
    }
    Ok(out)
}

fn rebuilt_central_len(members: &[OrdinaryMember]) -> usize {
    members.iter().map(|m| 46 + m.name.len()).sum()
}

fn verify_ordinary_members(archive: &[u8], members: &[OrdinaryMember]) -> ZbResult<()> {
    let mut offset = 0usize;
    for member in members {
        let name_len = member.name.len();
        let lfh = 30 + name_len;
        if archive.len() < offset + lfh + member.data.len() {
            return Err(ZbError::verification_failed("ordinary member local range out of bounds"));
        }
        if &archive[offset + 30..offset + lfh] != member.name.as_slice() {
            return Err(ZbError::verification_failed("ordinary member name mismatch"));
        }
        let data = &archive[offset + lfh..offset + lfh + member.data.len()];
        let decoded = match member.method {
            METHOD_STORED => data.to_vec(),
            METHOD_DEFLATE => decode_raw_deflate(data, member.original.len() as u64)?,
            _ => return Err(ZbError::verification_failed("unsupported ordinary member method")),
        };
        if decoded != member.original {
            return Err(ZbError::verification_failed(format!(
                "attachment `{}` bytes differ from input snapshot",
                String::from_utf8_lossy(&member.name)
            )));
        }
        offset += lfh + member.data.len();
    }
    Ok(())
}

fn local_header(name: &[u8], crc: &[u8; 4], compressed: u32, size: u32, method: u16, _is_dir: bool) -> Vec<u8> {
    let mut b = Vec::with_capacity(30 + name.len());
    b.extend_from_slice(&0x04034b50u32.to_le_bytes());
    b.extend_from_slice(&20u16.to_le_bytes());
    b.extend_from_slice(&GP_UTF8.to_le_bytes());
    b.extend_from_slice(&method.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(crc);
    b.extend_from_slice(&compressed.to_le_bytes());
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&(name.len() as u16).to_le_bytes());
    b.extend_from_slice(&0u16.to_le_bytes());
    b.extend_from_slice(name);
    b
}

fn central_header(
    name: &[u8],
    crc: &[u8; 4],
    compressed: u32,
    size: u32,
    method: u16,
    local_offset: u32,
    is_dir: bool,
) -> Vec<u8> {
    let mut b = Vec::with_capacity(46 + name.len());
    b.extend_from_slice(&0x02014b50u32.to_le_bytes());
    b.extend_from_slice(&20u16.to_le_bytes());
    b.extend_from_slice(&20u16.to_le_bytes());
    b.extend_from_slice(&GP_UTF8.to_le_bytes());
    b.extend_from_slice(&method.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(crc);
    b.extend_from_slice(&compressed.to_le_bytes());
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&(name.len() as u16).to_le_bytes());
    b.extend_from_slice(&0u16.to_le_bytes()); // extra
    b.extend_from_slice(&0u16.to_le_bytes()); // comment
    b.extend_from_slice(&0u16.to_le_bytes()); // disk
    b.extend_from_slice(&0u16.to_le_bytes()); // int attrs
    let ext = if is_dir { DOS_DIR_ATTR } else { 0 };
    b.extend_from_slice(&ext.to_le_bytes());
    b.extend_from_slice(&local_offset.to_le_bytes());
    b.extend_from_slice(name);
    b
}

fn eocd(count: u16, cd_size: u32, cd_offset: u32) -> Vec<u8> {
    let mut b = Vec::with_capacity(22);
    b.extend_from_slice(&0x06054b50u32.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&count.to_le_bytes());
    b.extend_from_slice(&count.to_le_bytes());
    b.extend_from_slice(&cd_size.to_le_bytes());
    b.extend_from_slice(&cd_offset.to_le_bytes());
    b.extend_from_slice(&0u16.to_le_bytes());
    b
}

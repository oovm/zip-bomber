//! Independent cycle edge and byte-period verification (Living `07` / `11`).

use acorn_source::{ByteSource, MemorySource};
use acorn_zip::decode_member_with_facts;
use zb_types::{
    ArchiveGraphMember, ArchiveGraphSpec, ArtifactSet, CycleVerifyFacts, ErrorCode, FractalMode,
    GenerateLimits, GenerateRequest, LoopyEdgeFact, VerificationSpec, VerifyEdgeSpec, ZbError,
    ZbResult,
};

use crate::budget::TaskBudgetLedger;

use super::zip::{verify_artifact_set_with_ledger, VerificationReport};

/// Minimum positive period of a cyclic byte word.
///
/// `d` must divide `n`, and `archives[i] == archives[(i + d) % n]` for all `i`.
pub fn minimum_byte_period(archives: &[&[u8]]) -> usize {
    let n = archives.len();
    if n == 0 {
        return 0;
    }
    for d in 1..=n {
        if n % d != 0 {
            continue;
        }
        if (0..n).all(|i| archives[i] == archives[(i + d) % n]) {
            return d;
        }
    }
    n
}

/// Build edge specs from a recursive graph and the artifact set's logical map.
///
/// Recursive Loopy / Fractal maps use `node-{n}@0=id`. Finite depth maps are
/// rejected here because period contracts apply to recursive cycles.
pub fn cycle_edges_from_graph(
    set: &ArtifactSet,
    graph: &ArchiveGraphSpec,
) -> ZbResult<Vec<VerifyEdgeSpec>> {
    if graph.mode != FractalMode::Recursive {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "cycle edge verify requires a recursive graph",
        ));
    }
    let mut by_node: std::collections::BTreeMap<usize, String> = std::collections::BTreeMap::new();
    for entry in &set.logical_node_map {
        let Some((label, id)) = entry.rsplit_once('=') else {
            continue;
        };
        // Accept `node-N@0` (graph compiler) and plain `node-N`.
        let Some(rest) = label.strip_prefix("node-") else {
            continue;
        };
        let (node_part, depth_ok) = match rest.split_once('@') {
            Some((n, depth)) => (n, depth == "0"),
            None => (rest, true),
        };
        if !depth_ok {
            continue;
        }
        if let Ok(n) = node_part.parse::<usize>() {
            by_node.insert(n, id.to_string());
        }
    }
    let mut edges = Vec::new();
    for (from, node) in graph.nodes.iter().enumerate() {
        let from_id = by_node.get(&from).ok_or_else(|| {
            ZbError::new(
                ErrorCode::InvalidTopology,
                format!("logical_node_map missing recursive node `{from}`"),
            )
        })?;
        for member in &node.members {
            if let ArchiveGraphMember::Archive { path, target, .. } = member {
                let to_id = by_node.get(target).ok_or_else(|| {
                    ZbError::new(
                        ErrorCode::InvalidTopology,
                        format!("logical_node_map missing recursive target `{target}`"),
                    )
                })?;
                edges.push(VerifyEdgeSpec {
                    from: from_id.clone(),
                    member_path: path.clone(),
                    to: to_id.clone(),
                });
            }
        }
    }
    if edges.is_empty() {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "graph has no archive edges to verify",
        ));
    }
    Ok(edges)
}

/// Verify ZIP structure plus optional cycle edges / period from a generate request.
pub fn verify_artifact_set_for_request(
    set: &ArtifactSet,
    request: &GenerateRequest,
) -> ZbResult<VerificationReport> {
    let Some(graph) = request.graph.as_ref() else {
        return super::zip::verify_artifact_set_with_limits(set, &request.limits);
    };
    let edges = cycle_edges_from_graph(set, graph)?;
    let expected = match request.variant {
        zb_types::VariantKind::Loopy => Some(graph.nodes.len()),
        _ => None,
    };
    verify_artifact_set_with_spec_and_limits(
        set,
        &VerificationSpec {
            edges,
            expected_byte_period: expected,
        },
        &request.limits,
    )
}

/// Verify every artifact, then each declared cycle edge and optional period.
pub fn verify_artifact_set_with_spec(
    set: &ArtifactSet,
    spec: &VerificationSpec,
) -> ZbResult<VerificationReport> {
    verify_artifact_set_with_spec_and_limits(set, spec, &GenerateLimits::small_sample())
}

/// Verify with an explicit shared task ledger across artifacts and cycle edges.
pub fn verify_artifact_set_with_spec_and_limits(
    set: &ArtifactSet,
    spec: &VerificationSpec,
    limits: &GenerateLimits,
) -> ZbResult<VerificationReport> {
    let mut ledger = TaskBudgetLedger::new(limits.clone());
    let mut report = verify_artifact_set_with_ledger(set, &mut ledger)?;
    if spec.edges.is_empty() {
        return Ok(report);
    }

    let ordered = order_simple_cycle(&spec.edges)?;
    let mut edge_facts = Vec::with_capacity(ordered.len());
    let mut cycle_bytes: Vec<Vec<u8>> = Vec::with_capacity(ordered.len());

    for (i, edge) in ordered.iter().enumerate() {
        let from = set
            .artifacts
            .iter()
            .find(|a| a.id.0 == edge.from)
            .ok_or_else(|| {
                ZbError::new(
                    ErrorCode::InvalidTopology,
                    format!("verify edge source `{}` is missing", edge.from),
                )
            })?;
        let to = set
            .artifacts
            .iter()
            .find(|a| a.id.0 == edge.to)
            .ok_or_else(|| {
                ZbError::new(
                    ErrorCode::InvalidTopology,
                    format!("verify edge target `{}` is missing", edge.to),
                )
            })?;
        let mut fact = verify_one_edge(
            &from.bytes,
            &to.bytes,
            &edge.member_path,
            &from.id.0,
            &to.id.0,
            &mut ledger,
        )?;
        fact.from = i;
        fact.to = (i + 1) % ordered.len();
        edge_facts.push(fact);
        cycle_bytes.push(from.bytes.clone());
    }

    let refs: Vec<&[u8]> = cycle_bytes.iter().map(|b| b.as_slice()).collect();
    let byte_period = minimum_byte_period(&refs);
    if let Some(expected) = spec.expected_byte_period {
        if byte_period != expected {
            return Err(ZbError::new(
                ErrorCode::PeriodMismatch,
                format!("byte period {byte_period} does not match expected {expected}"),
            ));
        }
    }

    report.notes.push(format!("cycle_edges={}", edge_facts.len()));
    report.notes.push(format!("byte_period={byte_period}"));
    report.cycle = Some(CycleVerifyFacts {
        byte_period,
        expected_byte_period: spec.expected_byte_period,
        edges: edge_facts,
    });
    Ok(report)
}

fn order_simple_cycle(edges: &[VerifyEdgeSpec]) -> ZbResult<Vec<&VerifyEdgeSpec>> {
    let start = edges[0].from.as_str();
    let mut ordered = Vec::with_capacity(edges.len());
    let mut cursor = start;
    let mut used = std::collections::BTreeSet::new();
    for _ in 0..edges.len() {
        let idx = edges
            .iter()
            .enumerate()
            .find(|(i, e)| e.from == cursor && used.insert(*i))
            .map(|(i, _)| i)
            .ok_or_else(|| {
                ZbError::new(
                    ErrorCode::InvalidTopology,
                    format!("cycle edges are not a simple directed cycle at `{cursor}`"),
                )
            })?;
        ordered.push(&edges[idx]);
        cursor = edges[idx].to.as_str();
    }
    if cursor != start {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "cycle edges do not form a closed simple cycle",
        ));
    }
    Ok(ordered)
}

fn verify_one_edge(
    from_bytes: &[u8],
    to_bytes: &[u8],
    member_path: &str,
    from_id: &str,
    to_id: &str,
    ledger: &mut TaskBudgetLedger,
) -> ZbResult<LoopyEdgeFact> {
    let owned = from_bytes.to_vec();
    let index = acorn_zip::index_zip_bytes("cycle-verify.zip", owned.clone())
        .map_err(|e| ZbError::verification_failed(format!("index `{from_id}`: {e}")))?;
    let member = index
        .members
        .iter()
        .find(|m| m.path == member_path)
        .ok_or_else(|| {
            ZbError::verification_failed(format!(
                "artifact `{from_id}` missing recursive member `{member_path}`"
            ))
        })?;
    let budget = ledger.reserve_member_decode(
        owned.len() as u64,
        member.compressed_size,
        member.uncompressed_size,
    )?;
    let (source, facts) = decode_member_with_facts(
        "cycle-edge",
        MemorySource::from_bytes("cycle-verify.zip", owned),
        member,
        &budget,
    )
    .map_err(|e| {
        ZbError::verification_failed(format!(
            "decode `{from_id}` member `{member_path}` failed: {e}"
        ))
    })?;
    ledger.settle_member_decode(facts.decoded_size as u64, member.compressed_size)?;
    let decoded = match source.length().unwrap_or(0) {
        0 => Vec::new(),
        len => source
            .read_range(acorn_source::SourceRange { start: 0, len })
            .map_err(|e| {
                ZbError::verification_failed(format!(
                    "read `{from_id}` member `{member_path}` failed: {e}"
                ))
            })?,
    };
    if !facts.checksum_ok() || decoded != to_bytes {
        return Err(ZbError::verification_failed(format!(
            "edge `{from_id}` --`{member_path}`--> `{to_id}` inflate mismatch (crc_ok={} decoded={} target={})",
            facts.checksum_ok(),
            decoded.len(),
            to_bytes.len()
        )));
    }
    Ok(LoopyEdgeFact {
        from: 0,
        to: 0,
        path: member_path.to_string(),
        compressed_len: member.compressed_size,
        target_len: to_bytes.len() as u64,
        target_crc: zb_solver::crc32(to_bytes),
        bytes_equal: true,
    })
}

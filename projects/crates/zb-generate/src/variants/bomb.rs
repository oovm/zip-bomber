//! Classic nested expansion bomb (finite stored nesting).

use zb_types::{
    Artifact, ArtifactId, ArtifactSet, ContainerKind, ErrorCode, GenerateReport, GenerateRequest,
    ZbError, ZbResult,
};

use crate::emit::{emit_stored_archive, StoredMember};

use super::ArtifactBundle;

/// Default nesting depth when the request topology is empty.
const DEFAULT_DEPTH: u32 = 2;
/// Default branch factor (members per nest level).
const DEFAULT_BRANCH: u32 = 1;
const DEFAULT_LEAF: &[u8] = b"zb-leaf\n";

/// Generate a finite nested bomb archive.
pub fn generate(request: GenerateRequest) -> ZbResult<ArtifactBundle> {
    let nodes = request.topology.nodes.len();
    let edges = request.topology.edges.len();
    let crc = request.crc_strategy;
    let variant = request.variant;
    let built = emit_finite_nested(&request)?;
    let warnings = vec![format!(
        "finite nested bomb depth={} branch={} unique_bytes={} logical_bytes={}",
        built.depth, built.branch, built.unique_bytes, built.logical_bytes
    )];
    Ok(ArtifactBundle {
        artifacts: built.artifacts,
        report: GenerateReport::ok(
            variant,
            crc,
            nodes.max(built.depth as usize),
            edges,
            built.root_bytes,
            built.logical_bytes,
            warnings,
        ),
    })
}

struct BuiltBomb {
    artifacts: ArtifactSet,
    depth: u32,
    branch: u32,
    root_bytes: u64,
    unique_bytes: u64,
    logical_bytes: u64,
}

fn emit_finite_nested(request: &GenerateRequest) -> ZbResult<BuiltBomb> {
    if !request.topology.edges.is_empty() || !request.topology.nodes.is_empty() {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "finite bomb currently accepts empty topology only (depth/branch defaults)",
        ));
    }

    let depth = DEFAULT_DEPTH.min(request.limits.max_depth).max(1);
    let branch = DEFAULT_BRANCH.max(1);
    if branch > request.limits.max_members {
        return Err(ZbError::resource_limited(format!(
            "branch {branch} exceeds max_members {}",
            request.limits.max_members
        )));
    }

    let leaf = request
        .payloads
        .first()
        .map(|p| p.bytes.as_slice())
        .unwrap_or(DEFAULT_LEAF);

    // Logical unfold: leaf size × branch^depth (each nest level multiplies fan-out).
    let logical = checked_logical(leaf.len() as u64, branch, depth, request.limits.max_decoded_bytes)?;

    let mut level_bytes = emit_stored_archive(&[StoredMember {
        path: "payload.bin".to_string(),
        payload: leaf.to_vec(),
    }])?;
    enforce_artifact_budget(level_bytes.len() as u64, &request.limits)?;

    let mut unique_total = level_bytes.len() as u64;
    let mut layers: Vec<Vec<u8>> = vec![level_bytes.clone()];

    for d in 1..depth {
        let mut members = Vec::with_capacity(branch as usize);
        for i in 0..branch {
            members.push(StoredMember {
                path: format!("n{d}/{i}.zip"),
                payload: level_bytes.clone(),
            });
        }
        level_bytes = emit_stored_archive(&members)?;
        enforce_artifact_budget(level_bytes.len() as u64, &request.limits)?;
        unique_total = unique_total
            .checked_add(level_bytes.len() as u64)
            .ok_or_else(|| ZbError::resource_limited("unique byte counter overflow"))?;
        if unique_total > request.limits.max_total_artifact_bytes {
            return Err(ZbError::resource_limited(format!(
                "unique artifact bytes {unique_total} exceed max_total_artifact_bytes {}",
                request.limits.max_total_artifact_bytes
            )));
        }
        layers.push(level_bytes.clone());
    }

    let root_bytes = level_bytes.len() as u64;
    let root_id = ArtifactId::new("bomb-root");
    let mut artifacts = Vec::with_capacity(layers.len());
    let mut logical_node_map = Vec::with_capacity(layers.len());
    for (i, bytes) in layers.into_iter().enumerate() {
        let id = if i + 1 == depth as usize {
            root_id.clone()
        } else {
            ArtifactId::new(format!("bomb-L{i}"))
        };
        logical_node_map.push(format!("L{i}={}", id.0));
        artifacts.push(Artifact {
            id,
            container: ContainerKind::Zip,
            byte_length: bytes.len().to_string(),
            digest: String::new(),
            bytes,
        });
    }

    Ok(BuiltBomb {
        artifacts: ArtifactSet {
            root: root_id,
            artifacts,
            logical_node_map,
        },
        depth,
        branch,
        root_bytes,
        unique_bytes: unique_total,
        logical_bytes: logical,
    })
}

fn checked_logical(leaf: u64, branch: u32, depth: u32, max_decoded: u64) -> ZbResult<u64> {
    let mut total = leaf;
    for _ in 0..depth {
        total = total
            .checked_mul(u64::from(branch))
            .ok_or_else(|| ZbError::resource_limited("logical expansion overflow"))?;
        if total > max_decoded {
            return Err(ZbError::resource_limited(format!(
                "logical expansion {total} exceeds max_decoded_bytes {max_decoded}"
            )));
        }
    }
    Ok(total)
}

fn enforce_artifact_budget(size: u64, limits: &zb_types::GenerateLimits) -> ZbResult<()> {
    if size > limits.max_artifact_bytes {
        return Err(ZbError::resource_limited(format!(
            "artifact {size} exceeds max_artifact_bytes {}",
            limits.max_artifact_bytes
        )));
    }
    Ok(())
}

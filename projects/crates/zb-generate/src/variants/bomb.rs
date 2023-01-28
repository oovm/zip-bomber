//! Classic nested expansion bomb (finite stored nesting).

use std::collections::{HashMap, HashSet, VecDeque};

use zb_types::{
    Artifact, ArtifactId, ArtifactSet, ContainerKind, EdgeRole, ErrorCode, GenerateReport,
    GenerateRequest, NodeId, ZbError, ZbResult,
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
    let built = if request.topology.nodes.is_empty() && request.topology.edges.is_empty() {
        emit_default_nested(&request)?
    } else {
        emit_topology_dag(&request)?
    };
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
        )
        .with_profile(request.profile.clone()),
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

fn emit_default_nested(request: &GenerateRequest) -> ZbResult<BuiltBomb> {
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

fn emit_topology_dag(request: &GenerateRequest) -> ZbResult<BuiltBomb> {
    let graph = &request.topology;
    if graph.nodes.is_empty() {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "bomb topology requires at least one node when edges are present",
        ));
    }

    let node_set: HashSet<&NodeId> = graph.nodes.iter().collect();
    for edge in &graph.edges {
        if edge.member_path.is_empty() {
            return Err(ZbError::new(
                ErrorCode::InvalidTopology,
                "bomb edges require non-empty member_path",
            ));
        }
        if !node_set.contains(&edge.from) || !node_set.contains(&edge.to) {
            return Err(ZbError::new(
                ErrorCode::InvalidTopology,
                format!(
                    "bomb edge `{}` -> `{}` references unknown node",
                    edge.from.0, edge.to.0
                ),
            ));
        }
        match edge.role {
            EdgeRole::NestedArchive | EdgeRole::Payload => {}
            EdgeRole::SelfReference | EdgeRole::FractalRole => {
                return Err(ZbError::new(
                    ErrorCode::InvalidTopology,
                    format!(
                        "bomb rejects edge role `{}` (nested_archive/payload only)",
                        edge.role.as_str()
                    ),
                ));
            }
        }
    }

    // Nested-archive adjacency for DAG checks and emit order.
    let mut nest_out: HashMap<&NodeId, Vec<&zb_types::ArchiveEdge>> = HashMap::new();
    let mut nest_in: HashMap<&NodeId, usize> = HashMap::new();
    for node in &graph.nodes {
        nest_out.entry(node).or_default();
        nest_in.entry(node).or_insert(0);
    }
    for edge in &graph.edges {
        if edge.role != EdgeRole::NestedArchive {
            continue;
        }
        if edge.from == edge.to {
            return Err(ZbError::new(
                ErrorCode::InvalidTopology,
                "bomb nested_archive edges must be acyclic (no self-loop)",
            ));
        }
        nest_out.get_mut(&edge.from).unwrap().push(edge);
        *nest_in.get_mut(&edge.to).unwrap() += 1;
    }

    // Kahn topological order; cycle ⇒ reject.
    let mut queue: VecDeque<&NodeId> = nest_in
        .iter()
        .filter_map(|(id, deg)| (*deg == 0).then_some(*id))
        .collect();
    let mut order = Vec::with_capacity(graph.nodes.len());
    let mut remaining = nest_in.clone();
    while let Some(node) = queue.pop_front() {
        order.push(node);
        for edge in nest_out.get(node).into_iter().flatten() {
            let deg = remaining.get_mut(&edge.to).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(&edge.to);
            }
        }
    }
    if order.len() != graph.nodes.len() {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "bomb topology must be a DAG (cycle detected on nested_archive edges)",
        ));
    }

    let roots: Vec<&NodeId> = nest_in
        .iter()
        .filter_map(|(id, deg)| (*deg == 0).then_some(*id))
        .collect();
    if roots.len() != 1 {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            format!(
                "bomb topology requires exactly one root (found {})",
                roots.len()
            ),
        ));
    }
    let root = roots[0];

    let payload_by_path: HashMap<&str, &[u8]> = request
        .payloads
        .iter()
        .map(|p| (p.path.as_str(), p.bytes.as_slice()))
        .collect();

    // Emit leaves first: reverse topo so children precede parents.
    order.reverse();
    let mut archive_bytes: HashMap<NodeId, Vec<u8>> = HashMap::new();
    let mut unique_total = 0u64;
    let mut max_depth = 1u32;
    let mut max_branch = 1u32;
    let mut depths: HashMap<&NodeId, u32> = HashMap::new();

    for node in &order {
        let mut members: Vec<StoredMember> = Vec::new();
        let mut nest_count = 0u32;

        for edge in graph.edges.iter().filter(|e| e.from == **node) {
            match edge.role {
                EdgeRole::NestedArchive => {
                    nest_count += 1;
                    let child = archive_bytes.get(&edge.to).ok_or_else(|| {
                        ZbError::new(
                            ErrorCode::Internal,
                            format!("missing child archive `{}`", edge.to.0),
                        )
                    })?;
                    members.push(StoredMember {
                        path: edge.member_path.clone(),
                        payload: child.clone(),
                    });
                }
                EdgeRole::Payload => {
                    let bytes = payload_by_path
                        .get(edge.member_path.as_str())
                        .copied()
                        .unwrap_or(DEFAULT_LEAF);
                    members.push(StoredMember {
                        path: edge.member_path.clone(),
                        payload: bytes.to_vec(),
                    });
                }
                EdgeRole::SelfReference | EdgeRole::FractalRole => unreachable!(),
            }
        }

        if members.is_empty() {
            members.push(StoredMember {
                path: "payload.bin".to_string(),
                payload: request
                    .payloads
                    .first()
                    .map(|p| p.bytes.clone())
                    .unwrap_or_else(|| DEFAULT_LEAF.to_vec()),
            });
        }

        if members.len() as u32 > request.limits.max_members {
            return Err(ZbError::resource_limited(format!(
                "node `{}` has {} members exceeding max_members {}",
                node.0,
                members.len(),
                request.limits.max_members
            )));
        }

        max_branch = max_branch.max(nest_count.max(1));
        let child_depth = nest_out
            .get(node)
            .into_iter()
            .flatten()
            .map(|e| depths.get(&e.to).copied().unwrap_or(1))
            .max()
            .unwrap_or(0);
        let depth = child_depth + 1;
        depths.insert(node, depth);
        max_depth = max_depth.max(depth);
        if max_depth > request.limits.max_depth {
            return Err(ZbError::resource_limited(format!(
                "topology depth {max_depth} exceeds max_depth {}",
                request.limits.max_depth
            )));
        }

        let bytes = emit_stored_archive(&members)?;
        enforce_artifact_budget(bytes.len() as u64, &request.limits)?;
        unique_total = unique_total
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| ZbError::resource_limited("unique byte counter overflow"))?;
        if unique_total > request.limits.max_total_artifact_bytes {
            return Err(ZbError::resource_limited(format!(
                "unique artifact bytes {unique_total} exceed max_total_artifact_bytes {}",
                request.limits.max_total_artifact_bytes
            )));
        }
        archive_bytes.insert((*node).clone(), bytes);
    }

    // Logical unfold along nested edges (shared children counted once per edge).
    let logical = logical_unfold(root, &nest_out, &archive_bytes, request.limits.max_decoded_bytes)?;

    let root_bytes_vec = archive_bytes.get(root).cloned().ok_or_else(|| {
        ZbError::new(ErrorCode::Internal, "root archive missing after emit")
    })?;
    let root_bytes = root_bytes_vec.len() as u64;
    let root_id = ArtifactId::new(format!("bomb-{}", root.0));

    let mut artifacts = Vec::with_capacity(archive_bytes.len());
    let mut logical_node_map = Vec::with_capacity(archive_bytes.len());
    // Stable order: topo parents-first (reverse of emit order).
    order.reverse();
    for node in order {
        let bytes = archive_bytes.remove(node).unwrap();
        let id = if node == root {
            root_id.clone()
        } else {
            ArtifactId::new(format!("bomb-{}", node.0))
        };
        logical_node_map.push(format!("{}={}", node.0, id.0));
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
        depth: max_depth,
        branch: max_branch,
        root_bytes,
        unique_bytes: unique_total,
        logical_bytes: logical,
    })
}

fn logical_unfold(
    root: &NodeId,
    nest_out: &HashMap<&NodeId, Vec<&zb_types::ArchiveEdge>>,
    archives: &HashMap<NodeId, Vec<u8>>,
    max_decoded: u64,
) -> ZbResult<u64> {
    fn walk(
        node: &NodeId,
        nest_out: &HashMap<&NodeId, Vec<&zb_types::ArchiveEdge>>,
        archives: &HashMap<NodeId, Vec<u8>>,
        max_decoded: u64,
        memo: &mut HashMap<NodeId, u64>,
    ) -> ZbResult<u64> {
        if let Some(v) = memo.get(node) {
            return Ok(*v);
        }
        let children = nest_out.get(node).map(|v| v.as_slice()).unwrap_or(&[]);
        let total = if children.is_empty() {
            archives.get(node).map(|b| b.len() as u64).unwrap_or(0)
        } else {
            let mut sum = 0u64;
            for edge in children {
                let child = walk(&edge.to, nest_out, archives, max_decoded, memo)?;
                sum = sum
                    .checked_add(child)
                    .ok_or_else(|| ZbError::resource_limited("logical expansion overflow"))?;
                if sum > max_decoded {
                    return Err(ZbError::resource_limited(format!(
                        "logical expansion {sum} exceeds max_decoded_bytes {max_decoded}"
                    )));
                }
            }
            sum
        };
        memo.insert(node.clone(), total);
        Ok(total)
    }
    walk(root, nest_out, archives, max_decoded, &mut HashMap::new())
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

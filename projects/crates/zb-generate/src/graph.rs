//! Unified ordered graph validation and bounded archive-state compilation.
use crate::{
    emit_stored_archive, PlanBudgetEstimate, PlanSummary, StoredMember, TaskBudgetLedger,
};
use acorn_source::ByteSource;
use std::collections::{BTreeMap, BTreeSet};
use zb_solver::CancelToken;
use zb_types::{
    ArchiveGraphMember as Member, ArchiveGraphSpec, Artifact, ArtifactId, ArtifactSet,
    ContainerKind, ErrorCode, FractalMode, GenerateReport, GenerateRequest, Outcome, PayloadSpec,
    TopologyGraph, VariantKind, ZbError, ZbResult, ZIP_LOOPY_TWO_NODE_GAPPED_V1, ZIP_PROFILE_V0,
};

fn invalid(message: impl Into<String>) -> ZbError {
    ZbError::new(ErrorCode::InvalidTopology, message)
}
fn unsupported(message: impl Into<String>) -> ZbError {
    ZbError::new(ErrorCode::UnsupportedTopology, message)
}
fn edges(g: &ArchiveGraphSpec) -> usize {
    g.nodes
        .iter()
        .flat_map(|n| &n.members)
        .filter(|m| matches!(m, Member::Archive { .. }))
        .count()
}

fn members(request: &GenerateRequest, list: &[Member], terminal: bool) -> ZbResult<u64> {
    if list.len() > request.limits.max_members as usize {
        return Err(ZbError::resource_limited("graph member budget exceeded"));
    }
    let mut paths: Vec<(&str, bool)> = Vec::new();
    let mut input = 0u64;
    for member in list {
        let path = member.path();
        let dir = path.ends_with('/');
        crate::paths::validate_member_path(path, matches!(member, Member::Payload { .. }))?;
        for (previous, previous_dir) in &paths {
            let a = previous.trim_end_matches('/');
            let b = path.trim_end_matches('/');
            if a == b
                || (!previous_dir && b.starts_with(&format!("{a}/")))
                || (!dir && a.starts_with(&format!("{b}/")))
            {
                return Err(invalid(format!(
                    "conflicting graph member paths `{previous}` and `{path}`"
                )));
            }
        }
        paths.push((path, dir));
        match member {
            Member::Payload { bytes, .. } => {
                let decoded = zb_types::decode_base64(bytes)?;
                if dir && !decoded.is_empty() {
                    return Err(invalid("directory payload must be empty"));
                }
                input = input
                    .checked_add(decoded.len() as u64)
                    .ok_or_else(|| ZbError::resource_limited("input size overflow"))?;
            }
            Member::Archive { target, .. } => {
                if terminal {
                    return Err(invalid("terminal cannot contain archive edges"));
                }
                if *target >= request.graph.as_ref().unwrap().nodes.len() {
                    return Err(invalid("graph edge target outside node range"));
                }
            }
        }
    }
    Ok(input)
}

fn validate(request: &GenerateRequest) -> ZbResult<(u32, Vec<Vec<usize>>)> {
    let g = request.graph.as_ref().unwrap();
    if request.schema != zb_types::REQUEST_SCHEMA_V2 || request.container != ContainerKind::Zip {
        return Err(invalid("unsupported graph schema or container"));
    }
    if !matches!(request.variant, VariantKind::Loopy | VariantKind::Fractal) {
        return Err(invalid("graph requires loopy or fractal intent"));
    }
    if request.profile != ZIP_PROFILE_V0 && request.profile != ZIP_LOOPY_TWO_NODE_GAPPED_V1 {
        return Err(ZbError::new(
            ErrorCode::UnsupportedProfile,
            "unknown graph construction profile",
        ));
    }
    if g.nodes.is_empty() || g.root >= g.nodes.len() {
        return Err(invalid("graph requires a valid root"));
    }
    if g.nodes.len() > request.limits.max_nodes as usize
        || edges(g) > request.limits.max_edges as usize
    {
        return Err(ZbError::resource_limited(
            "graph node or edge budget exceeded",
        ));
    }
    let depth = match g.mode {
        FractalMode::Recursive => {
            if g.depth.is_some() {
                return Err(invalid("recursive graph cannot specify finite depth"));
            }
            0
        }
        FractalMode::Finite => {
            if request.variant == VariantKind::Loopy {
                return Err(invalid("loopy requires recursive evaluation"));
            }
            let text = g
                .depth
                .as_ref()
                .ok_or_else(|| invalid("finite graph requires depth"))?;
            if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid("depth must be a decimal integer"));
            }
            let depth = text
                .parse::<u32>()
                .map_err(|_| ZbError::resource_limited("depth exceeds supported range"))?;
            if depth > request.limits.max_depth {
                return Err(ZbError::resource_limited("finite depth exceeds max_depth"));
            }
            depth
        }
    };
    let mut input = 0u64;
    let mut adjacency = Vec::new();
    for node in &g.nodes {
        input = input.saturating_add(members(request, &node.members, false)?);
        if g.mode == FractalMode::Finite {
            if let Some(terminal) = &node.terminal {
                input = input.saturating_add(members(request, terminal, true)?);
            }
        }
        adjacency.push(
            node.members
                .iter()
                .filter_map(|m| match m {
                    Member::Archive { target, .. } => Some(*target),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        );
    }
    if input > request.limits.max_input_bytes
        || input.saturating_mul(4).saturating_add(65536) > request.limits.max_live_memory_bytes
    {
        return Err(ZbError::resource_limited(
            "graph input or working memory budget exceeded",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut stack = vec![g.root];
    while let Some(n) = stack.pop() {
        if seen.insert(n) {
            stack.extend(&adjacency[n]);
        }
    }
    if seen.len() != g.nodes.len() {
        return Err(invalid("graph contains unreachable declared states"));
    }
    let components = scc(&adjacency);
    let cyclic = components
        .iter()
        .any(|c| c.len() > 1 || adjacency[c[0]].contains(&c[0]));
    if g.mode == FractalMode::Recursive && !cyclic {
        return Err(invalid("recursive graph requires a reachable cycle"));
    }
    if request.variant == VariantKind::Loopy {
        if g.nodes.len() < 2 || adjacency.iter().any(|e| e.len() != 1) || components.len() != 1 {
            return Err(invalid(
                "loopy requires one simple cycle with at least two states",
            ));
        }
    }
    Ok((depth, components))
}

// Iterative Kosaraju avoids dependence on the host call-stack depth.
fn scc(a: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut visited = vec![false; a.len()];
    let mut order = Vec::new();
    for start in 0..a.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![(start, 0)];
        while let Some((n, i)) = stack.last_mut() {
            if *i < a[*n].len() {
                let t = a[*n][*i];
                *i += 1;
                if !visited[t] {
                    visited[t] = true;
                    stack.push((t, 0));
                }
            } else {
                order.push(*n);
                stack.pop();
            }
        }
    }
    let mut reverse = vec![Vec::new(); a.len()];
    for (n, e) in a.iter().enumerate() {
        for t in e {
            reverse[*t].push(n);
        }
    }
    visited.fill(false);
    let mut components = Vec::new();
    for start in order.into_iter().rev() {
        if visited[start] {
            continue;
        }
        let mut c = Vec::new();
        let mut stack = vec![start];
        visited[start] = true;
        while let Some(n) = stack.pop() {
            c.push(n);
            for t in &reverse[n] {
                if !visited[*t] {
                    visited[*t] = true;
                    stack.push(*t);
                }
            }
        }
        c.sort_unstable();
        components.push(c);
    }
    components.reverse();
    components
}

fn supported(request: &GenerateRequest, components: &[Vec<usize>]) -> ZbResult<()> {
    if request.graph.as_ref().unwrap().mode == FractalMode::Finite {
        return Ok(());
    }
    for c in components {
        if c.len() > 1 {
            // Living 11: two-node SCC needs the explicit gapped profile. Larger rings have
            // no template yet and must stay unsupported_topology (never silently truncated).
            if c.len() == 2 {
                if request.profile == crate::loop_zip::PROFILE {
                    continue;
                }
                return Err(ZbError::new(
                    ErrorCode::UnsupportedProfile,
                    format!(
                        "two-state recursive SCC requires profile `{}`",
                        crate::loop_zip::PROFILE
                    ),
                ));
            }
            return Err(unsupported(format!(
                "no verified recursive template for {}-state SCC",
                c.len()
            )));
        }
        let n = c[0];
        let list = &request.graph.as_ref().unwrap().nodes[n].members;
        let self_edges = list
            .iter()
            .filter(|m| matches!(m,Member::Archive{target,..} if *target==n))
            .count();
        if self_edges > 1
            || (self_edges == 1
                && !matches!(list.last(),Some(Member::Archive{target,..}) if *target==n))
        {
            return Err(unsupported(
                "single-self template currently requires one self member last in ordered members",
            ));
        }
    }
    Ok(())
}

fn finite_needed_states(
    g: &ArchiveGraphSpec,
    depth: u32,
    limits: &zb_types::GenerateLimits,
) -> ZbResult<BTreeSet<(usize, u32)>> {
    let mut needed = BTreeSet::new();
    let mut pending = vec![(g.root, depth)];
    while let Some((n, d)) = pending.pop() {
        if !needed.insert((n, d)) {
            continue;
        }
        if needed.len() as u64
            > limits.max_nodes as u64 * (limits.max_depth as u64 + 1)
        {
            return Err(ZbError::resource_limited("finite state budget exceeded"));
        }
        if d > 0 {
            for m in &g.nodes[n].members {
                if let Member::Archive { target, .. } = m {
                    pending.push((*target, d - 1));
                }
            }
        }
    }
    Ok(needed)
}

fn estimate_finite_verify(g: &ArchiveGraphSpec, needed: &BTreeSet<(usize, u32)>) -> PlanBudgetEstimate {
    let mut member_decodes = 0u64;
    for &(n, d) in needed {
        let list = if d == 0 {
            g.nodes[n].terminal.as_ref().map(|t| t.as_slice()).unwrap_or(&[])
        } else {
            &g.nodes[n].members
        };
        member_decodes += list.len() as u64;
    }
    PlanBudgetEstimate {
        index_passes: needed.len() as u64 * 2,
        member_decodes: member_decodes * 2,
        decoded_bytes: 0,
        read_bytes: 0,
        solver_operations: 0,
        constraint_bits: 0,
    }
}

/// Plan the normalized graph without emitting archives.
pub(crate) fn plan(request: GenerateRequest) -> Outcome<PlanSummary> {
    let g = request.graph.as_ref().unwrap();
    let nodes = g.nodes.len();
    let count = edges(g);
    let result = validate(&request).and_then(|(depth, components)| {
        supported(&request, &components)?;
        let mut budget_estimate = PlanBudgetEstimate::default();
        if g.mode == FractalMode::Finite {
            let needed = finite_needed_states(g, depth, &request.limits)?;
            plan_finite_expansion(&request, g, depth, &needed)?;
            budget_estimate = budget_estimate.merge(estimate_finite_verify(g, &needed));
        }
        for component in &components {
            if component.len() == 2 && request.profile == crate::loop_zip::PROFILE {
                budget_estimate = budget_estimate.merge(
                    crate::loop_zip::plan_materialization(&request, component)?,
                );
            }
        }
        Ok(budget_estimate)
    });
    match result {
        Ok(budget_estimate) => Outcome::ok(
            PlanSummary {
                feasible: true,
                variant: request.variant.as_str().into(),
                profile: request.profile.clone(),
                summary: format!(
                    "unified graph compiler: {} states, {count} edges, {} evaluation",
                    nodes,
                    g.mode.as_str()
                ),
                warnings: Vec::new(),
                budget_estimate: Some(budget_estimate),
            },
            GenerateReport::ok(
                request.variant,
                request.crc_strategy,
                nodes,
                count,
                0,
                0,
                Vec::new(),
            ),
        ),
        Err(err) => Outcome::err(
            err.clone(),
            GenerateReport::failed(request.variant, request.crc_strategy, nodes, count, &err),
        ),
    }
}

fn ordinary(
    list: &[Member],
    lookup: impl Fn(usize) -> ZbResult<Vec<u8>>,
) -> ZbResult<Vec<StoredMember>> {
    list.iter()
        .map(|m| {
            Ok(StoredMember {
                path: m.path().into(),
                payload: match m {
                    Member::Payload { bytes, .. } => zb_types::decode_base64(bytes)?,
                    Member::Archive { target, .. } => lookup(*target)?,
                },
            })
        })
        .collect()
}
fn size_bound(list: &[StoredMember], request: &GenerateRequest) -> ZbResult<()> {
    let size = 22u64
        + list
            .iter()
            .map(|m| 76u64 + 2 * m.path.len() as u64 + m.payload.len() as u64)
            .sum::<u64>();
    if size > request.limits.max_artifact_bytes
        || size.saturating_mul(4) > request.limits.max_live_memory_bytes
    {
        return Err(ZbError::resource_limited(
            "graph artifact or live memory budget exceeded",
        ));
    }
    Ok(())
}

/// Plan finite fractal unique artifact bytes and consumer logical unfold before emit.
///
/// Multiplicity `c_d(v)` follows Living `05`: from the root at remaining depth `D`,
/// each archive edge to `w` contributes one visit at remaining `d-1`. Unique physical
/// bytes sum planned sizes of each memoized `(v, d)` once. Logical bytes weight child
/// sizes by visit multiplicity and must stay within `max_decoded_bytes`.
fn plan_finite_expansion(
    request: &GenerateRequest,
    g: &ArchiveGraphSpec,
    depth: u32,
    needed: &BTreeSet<(usize, u32)>,
) -> ZbResult<()> {
    let mut sizes: BTreeMap<(usize, u32), u64> = BTreeMap::new();
    let mut unique = 0u64;
    for d in 0..=depth {
        for &(n, rem) in needed.iter().filter(|(_, remaining)| *remaining == d) {
            let list = if rem == 0 {
                g.nodes[n]
                    .terminal
                    .as_ref()
                    .ok_or_else(|| invalid(format!("state {n} missing terminal at cutoff")))?
            } else {
                &g.nodes[n].members
            };
            let mut members = Vec::with_capacity(list.len());
            for m in list {
                match m {
                    Member::Payload { path, bytes } => {
                        let payload = zb_types::decode_base64(bytes)?;
                        members.push((path.len(), payload.len() as u64));
                    }
                    Member::Archive { path, target, .. } => {
                        let child = sizes.get(&(*target, rem.saturating_sub(1))).copied().ok_or_else(
                            || invalid("missing finite planned dependency"),
                        )?;
                        members.push((path.len(), child));
                    }
                }
            }
            let size = 22u64
                + members
                    .iter()
                    .map(|(name_len, payload)| 76u64 + 2 * (*name_len as u64) + *payload)
                    .sum::<u64>();
            if size > request.limits.max_artifact_bytes
                || size.saturating_mul(4) > request.limits.max_live_memory_bytes
            {
                return Err(ZbError::resource_limited(
                    "planned graph artifact or live memory budget exceeded",
                ));
            }
            unique = unique
                .checked_add(size)
                .ok_or_else(|| ZbError::resource_limited("unique byte counter overflow"))?;
            if unique > request.limits.max_total_artifact_bytes {
                return Err(ZbError::resource_limited(format!(
                    "planned unique artifact bytes {unique} exceed max_total_artifact_bytes {}",
                    request.limits.max_total_artifact_bytes
                )));
            }
            sizes.insert((n, rem), size);
        }
    }

    // Multiplicity from root: c[root][depth] = 1, push along archive edges.
    let mut counts: BTreeMap<(usize, u32), u64> = BTreeMap::new();
    counts.insert((g.root, depth), 1);
    let mut logical = 0u64;
    for rem in (0..=depth).rev() {
        let layer: Vec<(usize, u32)> = needed
            .iter()
            .copied()
            .filter(|(_, r)| *r == rem)
            .collect();
        for (n, _) in layer {
            let visits = counts.get(&(n, rem)).copied().unwrap_or(0);
            if visits == 0 {
                continue;
            }
            let list = if rem == 0 {
                g.nodes[n]
                    .terminal
                    .as_ref()
                    .ok_or_else(|| invalid(format!("state {n} missing terminal at cutoff")))?
            } else {
                &g.nodes[n].members
            };
            let mut has_archive = false;
            for m in list {
                if let Member::Archive { target, .. } = m {
                    has_archive = true;
                    if rem > 0 {
                        let key = (*target, rem - 1);
                        let next = counts
                            .get(&key)
                            .copied()
                            .unwrap_or(0)
                            .checked_add(visits)
                            .ok_or_else(|| {
                                ZbError::resource_limited("finite expansion multiplicity overflow")
                            })?;
                        counts.insert(key, next);
                    }
                }
            }
            if !has_archive {
                let size = sizes.get(&(n, rem)).copied().unwrap_or(0);
                logical = logical
                    .checked_add(
                        visits
                            .checked_mul(size)
                            .ok_or_else(|| ZbError::resource_limited("logical expansion overflow"))?,
                    )
                    .ok_or_else(|| ZbError::resource_limited("logical expansion overflow"))?;
                if logical > request.limits.max_decoded_bytes {
                    return Err(ZbError::resource_limited(format!(
                        "planned logical expansion {logical} exceeds max_decoded_bytes {}",
                        request.limits.max_decoded_bytes
                    )));
                }
            }
        }
    }
    Ok(())
}

struct CompiledGraph {
    set: ArtifactSet,
    loopy: Option<zb_types::LoopyFacts>,
    solver: Option<zb_types::SolverFacts>,
    budget_usage: zb_types::BudgetUsage,
}

fn compile(request: &GenerateRequest, cancel: &CancelToken) -> ZbResult<CompiledGraph> {
    cancel.check()?;
    let (depth, components) = validate(request)?;
    supported(request, &components)?;
    let g = request.graph.as_ref().unwrap();
    let mut compiled: BTreeMap<(usize, u32), Vec<u8>> = BTreeMap::new();
    let mut total = 0u64;
    let mut decoded = 0u64;
    let mut loopy_facts = None;
    let mut solver_facts = None;
    let mut save =
        |key, bytes: Vec<u8>, compiled: &mut BTreeMap<(usize, u32), Vec<u8>>| -> ZbResult<()> {
            total = total.saturating_add(bytes.len() as u64);
            if total > request.limits.max_total_artifact_bytes
                || total.saturating_mul(2) > request.limits.max_live_memory_bytes
            {
                return Err(ZbError::resource_limited(
                    "graph materialization budget exceeded",
                ));
            }
            if !(bytes.len() == 22 && bytes[0..4] == [0x50, 0x4b, 0x05, 0x06]) {
                crate::verify_zip_bytes(&bytes)
                    .map_err(|e| ZbError::verification_failed(format!("state {:?}: {e}", key)))?;
            }
            compiled.insert(key, bytes);
            Ok(())
        };
    if g.mode == FractalMode::Finite {
        let mut needed = BTreeSet::new();
        let mut pending = vec![(g.root, depth)];
        while let Some((n, d)) = pending.pop() {
            if !needed.insert((n, d)) {
                continue;
            }
            if needed.len() as u64
                > request.limits.max_nodes as u64 * (request.limits.max_depth as u64 + 1)
            {
                return Err(ZbError::resource_limited("finite state budget exceeded"));
            }
            if d > 0 {
                for m in &g.nodes[n].members {
                    if let Member::Archive { target, .. } = m {
                        pending.push((*target, d - 1));
                    }
                }
            }
        }
        // Living 05: plan expansion multiplicity and unique bytes before emit.
        plan_finite_expansion(request, g, depth, &needed)?;
        for d in 0..=depth {
            for &(n, _) in needed.iter().filter(|(_, remaining)| *remaining == d) {
                let list = if d == 0 {
                    g.nodes[n]
                        .terminal
                        .as_ref()
                        .ok_or_else(|| invalid(format!("state {n} missing terminal at cutoff")))?
                } else {
                    &g.nodes[n].members
                };
                let members = ordinary(list, |t| {
                    compiled
                        .get(&(t, d - 1))
                        .cloned()
                        .ok_or_else(|| invalid("missing finite dependency"))
                })?;
                size_bound(&members, request)?;
                save((n, d), emit_stored_archive(&members)?, &mut compiled)?;
            }
        }
    } else {
        for c in components {
            if c.len() == 2 {
                let (archives, facts, solver) = crate::loop_zip::generate(request, &c, cancel)?;
                loopy_facts = Some(facts);
                solver_facts = Some(solver);
                for (n, bytes) in archives {
                    save((n, 0), bytes, &mut compiled)?;
                }
                continue;
            }
            let n = c[0];
            let list = &g.nodes[n].members;
            let self_edge = list
                .iter()
                .find(|m| matches!(m,Member::Archive{target,..} if *target==n));
            let bytes = if let Some(edge) = self_edge {
                let payloads = ordinary(&list[..list.len() - 1], |t| {
                    compiled
                        .get(&(t, 0))
                        .cloned()
                        .ok_or_else(|| invalid("missing SCC dependency"))
                })?;
                let mut r = GenerateRequest::bare(VariantKind::Quine);
                r.profile = request.profile.clone();
                r.limits = request.limits.clone();
                r.crc_strategy = request.crc_strategy;
                r.topology = TopologyGraph::quine_self(format!("node-{n}"), edge.path());
                r.payloads = payloads
                    .into_iter()
                    .map(|m| PayloadSpec {
                        path: m.path,
                        bytes: m.payload,
                    })
                    .collect();
                crate::variants::quine::generate_with_cancel(r, cancel)?
                    .artifacts
                    .artifacts
                    .remove(0)
                    .bytes
            } else {
                let members = ordinary(list, |t| {
                    compiled
                        .get(&(t, 0))
                        .cloned()
                        .ok_or_else(|| invalid("missing SCC dependency"))
                })?;
                size_bound(&members, request)?;
                emit_stored_archive(&members)?
            };
            save((n, 0), bytes, &mut compiled)?;
        }
    }
    // Independent parser verifies every declared member, including the target identity.
    let mut verify_ledger = TaskBudgetLedger::new(request.limits.clone());
    for (&(n, d), bytes) in &compiled {
        let list = if g.mode == FractalMode::Finite && d == 0 {
            g.nodes[n].terminal.as_ref().unwrap()
        } else {
            &g.nodes[n].members
        };
        let expected = ordinary(list, |t| {
            Ok(compiled[&(
                t,
                if g.mode == FractalMode::Finite {
                    d - 1
                } else {
                    0
                },
            )]
                .clone())
        })?;
        decoded =
            decoded.saturating_add(expected.iter().map(|m| m.payload.len() as u64).sum::<u64>());
        if decoded > request.limits.max_decoded_bytes {
            return Err(ZbError::resource_limited(
                "graph verification decoded budget exceeded",
            ));
        }
        verify_members(bytes, &expected, &mut verify_ledger)?;
    }
    let root_key = (g.root, depth);
    let mut artifacts: Vec<Artifact> = Vec::new();
    let mut mapping = BTreeMap::new();
    for (key, bytes) in compiled {
        let id = if let Some(existing) = artifacts.iter().find(|a| a.bytes == bytes) {
            existing.id.clone()
        } else {
            let id = ArtifactId::new(format!("state-{}-depth-{}", key.0, key.1));
            artifacts.push(Artifact {
                id: id.clone(),
                container: ContainerKind::Zip,
                byte_length: bytes.len().to_string(),
                digest: format!("crc32:{:08x}", zb_solver::crc32(&bytes)),
                bytes,
            });
            id
        };
        mapping.insert(key, id);
    }
    Ok(CompiledGraph {
        set: ArtifactSet {
            root: mapping[&root_key].clone(),
            artifacts,
            logical_node_map: mapping
                .iter()
                .map(|((n, d), id)| format!("node-{n}@{d}={}", id.0))
                .collect(),
        },
        loopy: loopy_facts,
        solver: solver_facts,
        budget_usage: verify_ledger.usage(),
    })
}

fn verify_members(
    bytes: &[u8],
    expected: &[StoredMember],
    ledger: &mut TaskBudgetLedger,
) -> ZbResult<()> {
    if expected.is_empty() {
        if bytes.len() == 22 && bytes[0..4] == [0x50, 0x4b, 0x05, 0x06] {
            return Ok(());
        }
        return Err(ZbError::verification_failed(
            "empty graph archive is not a minimal EOCD",
        ));
    }
    let index = acorn_zip::index_zip_bytes("graph.zip", bytes.to_vec())
        .map_err(|e| ZbError::verification_failed(e.to_string()))?;
    if index.members.len() != expected.len() {
        return Err(ZbError::verification_failed("graph member count mismatch"));
    }
    for (actual, expected) in index.members.iter().zip(expected) {
        if actual.path != expected.path {
            return Err(ZbError::verification_failed(
                "graph member order or path mismatch",
            ));
        }
        let archive_len = bytes.len() as u64;
        let budget = ledger.reserve_member_decode(
            archive_len,
            actual.compressed_size,
            actual.uncompressed_size,
        )?;
        let (source, facts) = acorn_zip::decode_member_with_facts(
            "graph-member",
            acorn_source::MemorySource::from_bytes("graph.zip", bytes.to_vec()),
            actual,
            &budget,
        )
        .map_err(|e| {
            ZbError::verification_failed(format!(
                "graph member `{}` decode failed: {e}",
                expected.path
            ))
        })?;
        ledger.settle_member_decode(facts.decoded_size as u64, actual.compressed_size)?;
        // Empty stored files and directory entries decode to a zero-length source.
        // Acorn rejects `read_range(0, 0)` with unexpected eof, so skip that call.
        let decoded = match source.length().unwrap_or(0) {
            0 => Vec::new(),
            len => source
                .read_range(acorn_source::SourceRange { start: 0, len })
                .map_err(|e| {
                    ZbError::verification_failed(format!(
                        "graph member `{}` read failed: {e}",
                        expected.path
                    ))
                })?,
        };
        if !facts.checksum_ok() || decoded != expected.payload {
            return Err(ZbError::verification_failed(format!(
                "graph member `{}` differs from declared target",
                expected.path
            )));
        }
    }
    Ok(())
}

/// Generate through the shared graph compiler.
pub(crate) fn generate(request: GenerateRequest) -> Outcome<ArtifactSet> {
    generate_with_cancel(request, &CancelToken::new())
}

/// [`generate`] with a shared cancel token for SCC / Quine CRC steps.
pub(crate) fn generate_with_cancel(
    request: GenerateRequest,
    cancel: &CancelToken,
) -> Outcome<ArtifactSet> {
    let g = request.graph.as_ref().unwrap();
    let nodes = g.nodes.len();
    let count = edges(g);
    match compile(&request, cancel) {
        Ok(CompiledGraph {
            set,
            loopy,
            solver,
            budget_usage,
        }) => {
            let total = set.artifacts.iter().map(|a| a.bytes.len() as u64).sum();
            let mut report = GenerateReport::ok(
                request.variant,
                request.crc_strategy,
                nodes,
                count,
                set.artifacts.iter().find(|a| a.id == set.root).unwrap().bytes.len() as u64,
                total,
                vec!["unified graph: every member independently decoded and byte-compared".into()],
            )
            .with_profile(request.profile.clone());
            if let Some(facts) = solver {
                report = report.with_solver(facts);
            }
            if let Some(facts) = loopy {
                report = report.with_loopy(facts);
            }
            report = report.with_budget_usage(budget_usage);
            Outcome::ok(set, report)
        }
        Err(err) => Outcome::err(
            err.clone(),
            GenerateReport::failed(request.variant, request.crc_strategy, nodes, count, &err),
        ),
    }
}

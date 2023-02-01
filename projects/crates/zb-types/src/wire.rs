//! Wire DTOs for Node / WASM JSON request transport.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    format_decimal_u64, parse_decimal_u64, zip_layout, AlignmentPolicy, ArchiveEdge, Artifact,
    ArtifactId, ArtifactSet, ContainerKind, CrcStrategy, FractalSpec, GenerateLimits,
    GenerateRequest, NodeId, PayloadSpec, TopologyGraph, VariantKind, ZbError, ZbResult,
    REQUEST_SCHEMA_V2, ZIP_PROFILE_V0,
};

/// Wire payload: archive path plus base64 or raw byte array.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirePayloadSpec {
    /// Archive member path (`archivePath` on the public TS surface).
    #[serde(alias = "archivePath", alias = "archive_path")]
    pub path: String,
    /// Raw bytes when the host already expanded them (tests / N-API objects).
    #[serde(default)]
    pub bytes: Option<Vec<u8>>,
    /// Base64 bytes preferred on the isomorphic JSON wire.
    #[serde(default, alias = "bytesBase64", alias = "bytes_base64")]
    pub bytes_base64: Option<String>,
}

/// Wire topology edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireArchiveEdge {
    /// Optional edge id.
    #[serde(default)]
    pub id: Option<String>,
    /// Source node id string.
    pub from: String,
    /// Member path inside `from`.
    #[serde(alias = "memberPath", alias = "member_path")]
    pub member_path: String,
    /// Target node id string.
    pub to: String,
    /// Edge role string (snake_case).
    #[serde(default = "default_edge_role")]
    pub role: String,
}

fn default_edge_role() -> String {
    "nested_archive".to_string()
}

/// Wire topology graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireTopologyGraph {
    /// Node id strings.
    #[serde(default)]
    pub nodes: Vec<String>,
    /// Edges.
    #[serde(default)]
    pub edges: Vec<WireArchiveEdge>,
    /// Optional alignment policy.
    #[serde(default)]
    pub alignment: Option<AlignmentPolicy>,
}

/// Partial generate request accepted from TS / CLI JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireGenerateRequest {
    /// Schema id. Defaults to [`REQUEST_SCHEMA_V2`].
    #[serde(default = "default_schema")]
    pub schema: String,
    /// Container id. Defaults to `zip`.
    #[serde(default = "default_container")]
    pub container: String,
    /// Variant id.
    pub variant: String,
    /// Ordered archive-state graph from staged builders.
    #[serde(default)]
    pub graph: Option<crate::ArchiveGraphSpec>,
    /// Optional topology. When omitted, bare defaults are used.
    #[serde(default)]
    pub topology: Option<WireTopologyGraph>,
    /// Convenience self-member path for Quine (maps onto the sole self-loop edge).
    #[serde(default, alias = "selfMemberPath", alias = "self_member_path")]
    pub self_member_path: Option<String>,
    /// Ordinary payloads / directory entries.
    #[serde(default)]
    pub payloads: Vec<WirePayloadSpec>,
    /// Optional fractal grammar.
    #[serde(default)]
    pub fractal: Option<FractalSpec>,
    /// Profile id.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// Optional limits overlay (`DecimalU64` strings for large budgets on the wire).
    #[serde(default)]
    pub limits: Option<WireGenerateLimits>,
    /// CRC strategy.
    #[serde(default)]
    pub crc_strategy: Option<CrcStrategy>,
    /// Soft extra budget.
    #[serde(default)]
    pub extra_budget: Option<u32>,
}

/// Wire form of [`GenerateLimits`]. Large budgets use decimal strings (Living `04`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WireGenerateLimits {
    /// Max topology nodes.
    #[serde(default, rename = "maxNodes", alias = "max_nodes")]
    pub max_nodes: Option<u32>,
    /// Max topology edges.
    #[serde(default, rename = "maxEdges", alias = "max_edges")]
    pub max_edges: Option<u32>,
    /// Max members per archive.
    #[serde(default, rename = "maxMembers", alias = "max_members")]
    pub max_members: Option<u32>,
    /// Max total input payload bytes.
    #[serde(
        default,
        rename = "maxInputBytes",
        alias = "max_input_bytes",
        serialize_with = "ser_opt_decimal_u64",
        deserialize_with = "de_opt_decimal_u64"
    )]
    pub max_input_bytes: Option<u64>,
    /// Max single artifact bytes.
    #[serde(
        default,
        rename = "maxArtifactBytes",
        alias = "max_artifact_bytes",
        serialize_with = "ser_opt_decimal_u64",
        deserialize_with = "de_opt_decimal_u64"
    )]
    pub max_artifact_bytes: Option<u64>,
    /// Max sum of artifact bytes.
    #[serde(
        default,
        rename = "maxTotalArtifactBytes",
        alias = "max_total_artifact_bytes",
        serialize_with = "ser_opt_decimal_u64",
        deserialize_with = "de_opt_decimal_u64"
    )]
    pub max_total_artifact_bytes: Option<u64>,
    /// Max decode budget for verification unfold.
    #[serde(
        default,
        rename = "maxDecodedBytes",
        alias = "max_decoded_bytes",
        serialize_with = "ser_opt_decimal_u64",
        deserialize_with = "de_opt_decimal_u64"
    )]
    pub max_decoded_bytes: Option<u64>,
    /// Max nesting / fractal depth.
    #[serde(default, rename = "maxDepth", alias = "max_depth")]
    pub max_depth: Option<u32>,
    /// Max outer template candidates.
    #[serde(default, rename = "maxTemplateCandidates", alias = "max_template_candidates")]
    pub max_template_candidates: Option<u32>,
    /// Max GF(2) constraint bits.
    #[serde(
        default,
        rename = "maxConstraintBits",
        alias = "max_constraint_bits",
        serialize_with = "ser_opt_decimal_u64",
        deserialize_with = "de_opt_decimal_u64"
    )]
    pub max_constraint_bits: Option<u64>,
    /// Max solver word operations.
    #[serde(
        default,
        rename = "maxSolverOperations",
        alias = "max_solver_operations",
        serialize_with = "ser_opt_decimal_u64",
        deserialize_with = "de_opt_decimal_u64"
    )]
    pub max_solver_operations: Option<u64>,
    /// Max live working memory.
    #[serde(
        default,
        rename = "maxLiveMemoryBytes",
        alias = "max_live_memory_bytes",
        serialize_with = "ser_opt_decimal_u64",
        deserialize_with = "de_opt_decimal_u64"
    )]
    pub max_live_memory_bytes: Option<u64>,
}

impl WireGenerateLimits {
    /// Overlay onto small-sample defaults.
    pub fn into_limits(self) -> ZbResult<GenerateLimits> {
        let mut limits = GenerateLimits::small_sample();
        if let Some(v) = self.max_nodes {
            limits.max_nodes = v;
        }
        if let Some(v) = self.max_edges {
            limits.max_edges = v;
        }
        if let Some(v) = self.max_members {
            limits.max_members = v;
        }
        if let Some(v) = self.max_input_bytes {
            limits.max_input_bytes = v;
        }
        if let Some(v) = self.max_artifact_bytes {
            limits.max_artifact_bytes = v;
        }
        if let Some(v) = self.max_total_artifact_bytes {
            limits.max_total_artifact_bytes = v;
        }
        if let Some(v) = self.max_decoded_bytes {
            limits.max_decoded_bytes = v;
        }
        if let Some(v) = self.max_depth {
            limits.max_depth = v;
        }
        if let Some(v) = self.max_template_candidates {
            limits.max_template_candidates = v;
        }
        if let Some(v) = self.max_constraint_bits {
            limits.max_constraint_bits = v;
        }
        if let Some(v) = self.max_solver_operations {
            limits.max_solver_operations = v;
        }
        if let Some(v) = self.max_live_memory_bytes {
            limits.max_live_memory_bytes = v;
        }
        Ok(limits)
    }

    /// Capture a core limits snapshot for JSON wire emission.
    pub fn from_limits(limits: &GenerateLimits) -> Self {
        Self {
            max_nodes: Some(limits.max_nodes),
            max_edges: Some(limits.max_edges),
            max_members: Some(limits.max_members),
            max_input_bytes: Some(limits.max_input_bytes),
            max_artifact_bytes: Some(limits.max_artifact_bytes),
            max_total_artifact_bytes: Some(limits.max_total_artifact_bytes),
            max_decoded_bytes: Some(limits.max_decoded_bytes),
            max_depth: Some(limits.max_depth),
            max_template_candidates: Some(limits.max_template_candidates),
            max_constraint_bits: Some(limits.max_constraint_bits),
            max_solver_operations: Some(limits.max_solver_operations),
            max_live_memory_bytes: Some(limits.max_live_memory_bytes),
        }
    }
}

fn ser_opt_decimal_u64<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_str(&format_decimal_u64(*v)),
        None => serializer.serialize_none(),
    }
}

fn de_opt_decimal_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Str(String),
        Num(u64),
        Null,
    }
    match Option::<Raw>::deserialize(deserializer)? {
        None | Some(Raw::Null) => Ok(None),
        Some(Raw::Num(v)) => Ok(Some(v)),
        Some(Raw::Str(s)) => parse_decimal_u64(&s).map(Some).map_err(serde::de::Error::custom),
    }
}

fn default_schema() -> String {
    REQUEST_SCHEMA_V2.to_string()
}

fn default_container() -> String {
    "zip".to_string()
}

fn default_profile() -> String {
    ZIP_PROFILE_V0.to_string()
}

/// Decode a wire JSON generate request into the core contract.
pub fn generate_request_from_json(json: &str) -> ZbResult<GenerateRequest> {
    let wire: WireGenerateRequest = serde_json::from_str(json).map_err(|err| {
        ZbError::new(
            crate::ErrorCode::InvalidTopology,
            format!("invalid generate request JSON: {err}"),
        )
    })?;
    wire.into_request()
}

impl WireGenerateRequest {
    /// Convert wire DTO into [`GenerateRequest`].
    pub fn into_request(self) -> ZbResult<GenerateRequest> {
        let variant = VariantKind::parse(&self.variant).ok_or_else(|| {
            ZbError::new(
                crate::ErrorCode::UnknownVariant,
                format!("unknown variant `{}`", self.variant),
            )
        })?;
        if self.schema != REQUEST_SCHEMA_V2 {
            return Err(ZbError::new(
                crate::ErrorCode::InvalidTopology,
                format!("unsupported request schema `{}`", self.schema),
            ));
        }
        if self.container != ContainerKind::Zip.as_str() {
            return Err(ZbError::new(
                crate::ErrorCode::InvalidTopology,
                format!("unsupported container `{}`", self.container),
            ));
        }

        let mut request = GenerateRequest::bare(variant);
        request.graph = self.graph;
        request.profile = self.profile;
        if let Some(limits) = self.limits {
            request.limits = limits.into_limits()?;
        }
        if let Some(crc) = self.crc_strategy {
            request.crc_strategy = crc;
        }
        if let Some(extra) = self.extra_budget {
            request.extra_budget = extra;
        } else {
            request.extra_budget = zip_layout::EXTRA_BUDGET_DEFAULT;
        }
        request.fractal = self.fractal.or(request.fractal);

        if let Some(topo) = self.topology {
            request.topology = topo.into_topology()?;
        }
        if let Some(path) = self.self_member_path {
            if variant != VariantKind::Quine {
                return Err(ZbError::new(
                    crate::ErrorCode::InvalidTopology,
                    "selfMemberPath is only valid for the quine variant",
                ));
            }
            let root = request
                .topology
                .nodes
                .first()
                .cloned()
                .unwrap_or_else(|| NodeId::new("a.zip"));
            request.topology = TopologyGraph::quine_self(root.0, path);
        }

        request.payloads = self
            .payloads
            .into_iter()
            .map(WirePayloadSpec::into_payload)
            .collect::<ZbResult<Vec<_>>>()?;
        Ok(request)
    }
}

impl WirePayloadSpec {
    fn into_payload(self) -> ZbResult<PayloadSpec> {
        let bytes = match (self.bytes, self.bytes_base64) {
            (Some(bytes), None) => bytes,
            (None, Some(b64)) => decode_base64(&b64)?,
            (Some(bytes), Some(_)) => bytes,
            (None, None) => {
                if self.path.ends_with('/') {
                    Vec::new()
                } else {
                    return Err(ZbError::new(
                        crate::ErrorCode::InvalidTopology,
                        format!("payload `{}` requires bytes or bytesBase64", self.path),
                    ));
                }
            }
        };
        Ok(PayloadSpec {
            path: self.path,
            bytes,
        })
    }
}

impl WireTopologyGraph {
    fn into_topology(self) -> ZbResult<TopologyGraph> {
        let nodes = self.nodes.into_iter().map(NodeId::new).collect::<Vec<_>>();
        let mut edges = Vec::with_capacity(self.edges.len());
        for edge in self.edges {
            let role = match edge.role.as_str() {
                "nested_archive" => crate::EdgeRole::NestedArchive,
                "self_reference" => crate::EdgeRole::SelfReference,
                "payload" => crate::EdgeRole::Payload,
                "fractal_role" => crate::EdgeRole::FractalRole,
                other => {
                    return Err(ZbError::new(
                        crate::ErrorCode::InvalidTopology,
                        format!("unknown edge role `{other}`"),
                    ))
                }
            };
            edges.push(ArchiveEdge {
                id: edge.id,
                from: NodeId::new(edge.from),
                member_path: edge.member_path,
                to: NodeId::new(edge.to),
                role,
            });
        }
        Ok(TopologyGraph {
            nodes,
            edges,
            alignment: self.alignment.unwrap_or_default(),
        })
    }
}

/// Minimal base64 decoder for the isomorphic wire.
pub fn decode_base64(input: &str) -> ZbResult<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut values = Vec::with_capacity(input.len());
    for ch in input.bytes() {
        if ch == b'=' || ch.is_ascii_whitespace() {
            if ch == b'=' {
                break;
            }
            continue;
        }
        let idx = TABLE
            .iter()
            .position(|b| *b == ch)
            .ok_or_else(|| ZbError::new(crate::ErrorCode::InvalidTopology, "invalid base64"))?;
        values.push(idx as u8);
    }
    let mut out = Vec::new();
    for chunk in values.chunks(4) {
        let v0 = u32::from(*chunk.first().unwrap_or(&0));
        let v1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let v2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let v3 = u32::from(*chunk.get(3).unwrap_or(&0));
        let n = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        out.push(((n >> 16) & 0xff) as u8);
        if chunk.len() > 2 {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if chunk.len() > 3 {
            out.push((n & 0xff) as u8);
        }
    }
    Ok(out)
}

/// Encode bytes as base64 for the isomorphic wire.
pub fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Serialize a core request into wire JSON with base64 payloads.
pub fn generate_request_to_json(request: &GenerateRequest) -> ZbResult<String> {
    let wire = WireGenerateRequest {
        schema: request.schema.clone(),
        container: request.container.as_str().to_string(),
        variant: request.variant.as_str().to_string(),
        topology: Some(WireTopologyGraph {
            nodes: request.topology.nodes.iter().map(|n| n.0.clone()).collect(),
            edges: request
                .topology
                .edges
                .iter()
                .map(|e| WireArchiveEdge {
                    id: e.id.clone(),
                    from: e.from.0.clone(),
                    member_path: e.member_path.clone(),
                    to: e.to.0.clone(),
                    role: e.role.as_str().to_string(),
                })
                .collect(),
            alignment: Some(request.topology.alignment.clone()),
        }),
        graph: request.graph.clone(),
        self_member_path: None,
        payloads: request
            .payloads
            .iter()
            .map(|p| WirePayloadSpec {
                path: p.path.clone(),
                bytes: None,
                bytes_base64: Some(encode_base64(&p.bytes)),
            })
            .collect(),
        fractal: request.fractal.clone(),
        profile: request.profile.clone(),
        limits: Some(WireGenerateLimits::from_limits(&request.limits)),
        crc_strategy: Some(request.crc_strategy),
        extra_budget: Some(request.extra_budget),
    };
    serde_json::to_string(&wire).map_err(|err| {
        ZbError::new(
            crate::ErrorCode::Internal,
            format!("failed to serialize wire request: {err}"),
        )
    })
}

/// Wire artifact with base64 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireArtifact {
    /// Artifact id.
    pub id: String,
    /// Optional container id (defaults to zip).
    #[serde(default)]
    pub container: Option<String>,
    /// Base64 archive bytes.
    #[serde(alias = "bytes_base64")]
    pub bytes_base64: String,
    /// Decimal byte length.
    #[serde(default, alias = "byte_length")]
    pub byte_length: Option<String>,
    /// Content digest.
    #[serde(default)]
    pub digest: Option<String>,
}

/// Wire artifact set for Node / WASM verify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireArtifactSet {
    /// Root artifact id.
    pub root: String,
    /// Artifacts in stable order.
    pub artifacts: Vec<WireArtifact>,
    /// Logical map entries `label=id`.
    #[serde(default, alias = "logical_node_map")]
    pub logical_node_map: Vec<String>,
}

impl WireArtifactSet {
    /// Encode a core artifact set for JSON transport.
    pub fn from_set(set: &ArtifactSet) -> Self {
        Self {
            root: set.root.0.clone(),
            artifacts: set
                .artifacts
                .iter()
                .map(|a| WireArtifact {
                    id: a.id.0.clone(),
                    container: Some(a.container.as_str().to_string()),
                    bytes_base64: encode_base64(&a.bytes),
                    byte_length: Some(a.byte_length.clone()),
                    digest: Some(a.digest.clone()),
                })
                .collect(),
            logical_node_map: set.logical_node_map.clone(),
        }
    }

    /// Decode into a core artifact set.
    pub fn into_set(self) -> ZbResult<ArtifactSet> {
        let mut artifacts = Vec::with_capacity(self.artifacts.len());
        for item in self.artifacts {
            let bytes = decode_base64(&item.bytes_base64)?;
            let byte_length = item
                .byte_length
                .unwrap_or_else(|| bytes.len().to_string());
            artifacts.push(Artifact {
                id: ArtifactId::new(item.id),
                container: ContainerKind::Zip,
                bytes,
                byte_length,
                digest: item.digest.unwrap_or_default(),
            });
        }
        Ok(ArtifactSet {
            root: ArtifactId::new(self.root),
            artifacts,
            logical_node_map: self.logical_node_map,
        })
    }
}

/// Serialize an artifact set for isomorphic verify.
pub fn artifact_set_to_json(set: &ArtifactSet) -> ZbResult<String> {
    serde_json::to_string(&WireArtifactSet::from_set(set)).map_err(|err| {
        ZbError::new(
            crate::ErrorCode::Internal,
            format!("failed to serialize artifact set: {err}"),
        )
    })
}

/// Parse an artifact set from isomorphic JSON.
pub fn artifact_set_from_json(json: &str) -> ZbResult<ArtifactSet> {
    let wire: WireArtifactSet = serde_json::from_str(json).map_err(|err| {
        ZbError::new(
            crate::ErrorCode::InvalidTopology,
            format!("invalid artifact set JSON: {err}"),
        )
    })?;
    wire.into_set()
}

/// Parse a [`crate::VerificationSpec`] from isomorphic JSON.
pub fn verification_spec_from_json(json: &str) -> ZbResult<crate::VerificationSpec> {
    serde_json::from_str(json).map_err(|err| {
        ZbError::new(
            crate::ErrorCode::InvalidTopology,
            format!("invalid verification spec JSON: {err}"),
        )
    })
}

/// Serialize a [`crate::VerificationSpec`] for isomorphic verify.
pub fn verification_spec_to_json(spec: &crate::VerificationSpec) -> ZbResult<String> {
    serde_json::to_string(spec).map_err(|err| {
        ZbError::new(
            crate::ErrorCode::Internal,
            format!("failed to serialize verification spec: {err}"),
        )
    })
}

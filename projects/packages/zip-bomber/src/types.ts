/** Stable variant ids (container-orthogonal). */
export type VariantId = "bomb" | "quine" | "loopy" | "fractal";

/** First-wave container. */
export type ContainerId = "zip";

/** All variants in display order. */
export const VARIANTS: readonly VariantId[] = [
    "bomb",
    "quine",
    "loopy",
    "fractal",
] as const;

/** Decimal u64 on the wire. */
export type DecimalU64 = string;

export type ErrorCategory =
    | "invalid_request"
    | "unsupported_container"
    | "unsupported_variant"
    | "unsupported_topology"
    | "unsupported_profile"
    | "backend_unavailable"
    | "binding_mismatch"
    | "resource_limited"
    | "search_exhausted"
    | "constraint_unsatisfied"
    | "period_mismatch"
    | "verification_failed"
    | "verification_capability_missing"
    | "cancelled"
    | "internal_error"
    | "not_implemented";

export type ZbError = {
    category: ErrorCategory;
    code: string;
    message: string;
};

export type LoopyEdgeFact = {
    from: number;
    to: number;
    path: string;
    compressed_len: number;
    target_len: number;
    target_crc: number;
    bytes_equal: boolean;
};

export type LoopyFacts = {
    requested_nodes: number;
    byte_period: number;
    profile: string;
    ordinary_members: number[];
    quote_window: number;
    ordinary_widths: number[];
    crc_rank: number;
    crc_free_bits: number;
    edges: LoopyEdgeFact[];
};

export type SolverFacts = {
    rank: number;
    free_bits: number;
    equations_used: number;
    problem_digest?: string | null;
};

export type GenerateReport = {
    schema: string;
    container: ContainerId;
    variant: VariantId;
    status: "ok" | "not_implemented" | "error";
    profile?: string | null;
    topology_nodes: number;
    topology_edges: number;
    warnings: string[];
    error_code?: string | null;
    error_message?: string | null;
    crc_strategy?: string;
    solver?: SolverFacts | null;
    loopy?: LoopyFacts | null;
    budget_usage?: BudgetUsage | null;
};

export type BudgetUsage = {
    max_decoded_bytes: number;
    decoded_bytes_used: number;
    max_live_memory_bytes: number;
    read_bytes_used: number;
    members_decoded: number;
};

export type Artifact = {
    id: string;
    container: ContainerId;
    bytes: Uint8Array;
    byteLength: DecimalU64;
    digest: string;
};

export type ArtifactSet = {
    root: string;
    artifacts: Artifact[];
    logicalNodeMap: string[];
};

export type Outcome<T> =
    | { ok: true; value: T; report: GenerateReport }
    | { ok: false; error: ZbError; report: GenerateReport };

export type CapabilityStatus = "declared" | "implemented" | "verified";

export type CapabilityEntry = {
    container: ContainerId;
    variant: VariantId;
    topologyClass: string;
    profile: string;
    status: CapabilityStatus;
};

export type CapabilitySet = {
    entries: CapabilityEntry[];
};

/** Ordinary attachment / directory entry on the public TS surface. */
export type PayloadSpec = {
    /** Archive-relative POSIX path (`empty/` denotes a directory entry). */
    archivePath: string;
    /** Frozen input snapshot bytes. */
    bytes?: Uint8Array;
};

export type GenerateRequest = {
    schema: "zb.request/v2";
    container: ContainerId;
    variant: VariantId;
    graph?: unknown;
    profile?: string;
    limits?: Partial<GenerateLimits>;
    /** Opaque topology object for template stubs. */
    topology?: unknown;
    /** Quine self-reference member path. Maps onto the sole self-loop edge. */
    selfMemberPath?: string;
    /** Ordinary attachments and explicit directory entries. */
    payloads?: PayloadSpec[];
};

export type PlanBudgetEstimate = {
    index_passes: number;
    member_decodes: number;
    decoded_bytes: number;
    read_bytes: number;
    solver_operations: number;
    constraint_bits: number;
};

export type PlanSummary = {
    feasible: boolean;
    variant: VariantId;
    profile: string;
    summary: string;
    warnings: string[];
    budget_estimate?: PlanBudgetEstimate | null;
};

export type VerifyEdgeSpec = {
    from: string;
    memberPath: string;
    to: string;
};

export type VerificationSpec = {
    edges?: VerifyEdgeSpec[];
    expectedBytePeriod?: number;
};

export type CycleVerifyFacts = {
    byte_period: number;
    expected_byte_period?: number | null;
    edges: LoopyEdgeFact[];
};

export type VerifySummary = {
    verified: boolean;
    notes: string[];
    missingCapabilities: string[];
    /** Present when cycle edges / period were checked. */
    cycle?: CycleVerifyFacts | null;
};

export type ZipBomberApi = {
    capabilities(): Promise<CapabilitySet>;
    plan(request: GenerateRequest | GraphSpec): Promise<Outcome<PlanSummary>>;
    generate(
        request: GenerateRequest | GraphSpec,
        options?: { signal?: AbortSignal },
    ): Promise<Outcome<ArtifactSet>>;
    verify(
        input: ArtifactSet,
        spec?: VerificationSpec | GenerateRequest,
    ): Promise<Outcome<VerifySummary>>;
    zipBomb(options?: Partial<GenerateRequest>): Promise<Outcome<ArtifactSet>>;
    zipQuine(options?: Partial<GenerateRequest>): Promise<Outcome<ArtifactSet>>;
    zipLoopy(options?: Partial<GenerateRequest> | ZipLoopySpec): Promise<Outcome<ArtifactSet>>;
    zipFractal(options?: Partial<GenerateRequest> | ZipFractalSpec): Promise<Outcome<ArtifactSet>>;
    dispose(): Promise<void>;
};

/** Graph-local staged builder diagnostic. */
export type BuildDiagnostic = { code: string; message: string; node?: number; archivePath?: string };
export type BuildOutcome<T> = { ok: true; value: T; diagnostics: BuildDiagnostic[] } |
    { ok: false; error: ZbError; diagnostics: BuildDiagnostic[] };
/** Public generate budgets. u64 fields are `DecimalU64` strings (Living `04`). */
export type GenerateLimits = {
    maxNodes: number;
    maxEdges: number;
    maxMembers: number;
    maxInputBytes: DecimalU64;
    maxArtifactBytes: DecimalU64;
    maxTotalArtifactBytes: DecimalU64;
    maxDecodedBytes: DecimalU64;
    maxDepth: number;
    maxTemplateCandidates: number;
    maxConstraintBits: DecimalU64;
    maxSolverOperations: DecimalU64;
    maxLiveMemoryBytes: DecimalU64;
};
export type BuildOptions = { profile?: string; limits?: Partial<GenerateLimits> };
declare const graphSpecBrand: unique symbol;
export type ZipLoopySpec = { readonly variant: "loopy"; readonly [graphSpecBrand]: true };
export type ZipFractalSpec = { readonly variant: "fractal"; readonly [graphSpecBrand]: true };
export type GraphSpec = ZipLoopySpec | ZipFractalSpec;
export type NodeOptions = { label?: string; payloads?: readonly PayloadSpec[] };
export type ArchiveLink<N> = { archivePath: string; to: N; role?: string };
export interface LoopyNode {
    attach(payload: PayloadSpec): this;
    directory(archivePath: string): this;
    link(edge: ArchiveLink<LoopyNode>): this;
}
export interface FractalNode {
    attach(payload: PayloadSpec): this;
    directory(archivePath: string): this;
    link(edge: ArchiveLink<FractalNode>): this;
    terminal(options: { payloads: readonly PayloadSpec[] }): this;
}
export interface ZipLoopyBuilder {
    node(options?: NodeOptions): LoopyNode;
    root(node: LoopyNode): this;
    build(options?: BuildOptions): BuildOutcome<ZipLoopySpec>;
}
export interface ZipFractalBuilder {
    node(options?: NodeOptions): FractalNode;
    root(node: FractalNode): this;
    build(options: BuildOptions & ({mode: "recursive"} | {mode: "finite"; depth: DecimalU64})): BuildOutcome<ZipFractalSpec>;
}

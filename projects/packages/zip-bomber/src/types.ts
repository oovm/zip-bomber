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

export type GenerateReport = {
    schema: string;
    container: ContainerId;
    variant: VariantId;
    status: "ok" | "not_implemented" | "error";
    topology_nodes: number;
    topology_edges: number;
    warnings: string[];
    error_code?: string | null;
    error_message?: string | null;
    crc_strategy?: string;
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
    /** Opaque topology object for template stubs. */
    topology?: unknown;
    /** Quine self-reference member path. Maps onto the sole self-loop edge. */
    selfMemberPath?: string;
    /** Ordinary attachments and explicit directory entries. */
    payloads?: PayloadSpec[];
};

export type PlanSummary = {
    feasible: boolean;
    variant: VariantId;
    profile: string;
    summary: string;
    warnings: string[];
};

export type VerifySummary = {
    verified: boolean;
    notes: string[];
    missingCapabilities: string[];
};

export type ZipBomberApi = {
    capabilities(): Promise<CapabilitySet>;
    plan(request: GenerateRequest): Promise<Outcome<PlanSummary>>;
    generate(request: GenerateRequest): Promise<Outcome<ArtifactSet>>;
    verify(
        input: ArtifactSet,
        spec?: unknown,
    ): Promise<Outcome<VerifySummary>>;
    zipBomb(options?: Partial<GenerateRequest>): Promise<Outcome<ArtifactSet>>;
    zipQuine(options?: Partial<GenerateRequest>): Promise<Outcome<ArtifactSet>>;
    zipLoopy(options?: Partial<GenerateRequest>): Promise<Outcome<ArtifactSet>>;
    zipFractal(options?: Partial<GenerateRequest>): Promise<Outcome<ArtifactSet>>;
    dispose(): Promise<void>;
};

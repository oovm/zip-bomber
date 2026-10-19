export type {
    Artifact,
    ArtifactSet,
    CapabilityEntry,
    CapabilitySet,
    CapabilityStatus,
    ContainerId,
    DecimalU64,
    ErrorCategory,
    GenerateReport,
    GenerateRequest,
    LoopyEdgeFact,
    LoopyFacts,
    Outcome,
    PayloadSpec,
    VariantId,
    ZipBomberApi,
    ZbError,
} from "./types.js";
export { VARIANTS } from "./types.js";
export {
    createSharedZipBomberApi,
    bomb,
    fractal,
    loopy,
    quine,
} from "./facade.js";
export type { BackendGenerateResult, ZipBomberBackendHooks } from "./facade.js";
export type { ZipBomberApiFactory, ZipBomberBackend } from "./api.js";
export { notImplementedError, runtimeDisposedError } from "./errors.js";
export {
    encodeArtifactSetJson,
    encodeBase64,
    encodeGenerateRequestJson,
    toUint8Array,
} from "./wire.js";

export { createLoopy, createFractal } from "./builders.js";
export type { LoopySpec, FractalSpec, LoopyBuilder, FractalBuilder, LoopyNode, FractalNode,
    BuildOptions, BuildOutcome, BuildDiagnostic, NodeOptions, ArchiveLink, GenerateLimits } from "./types.js";

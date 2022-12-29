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
    Outcome,
    VariantId,
    ZipBomberApi,
    ZbError,
} from "./types.js";
export { VARIANTS } from "./types.js";
export {
    createSharedZipBomberApi,
    zipBomb,
    zipFractal,
    zipLoopy,
    zipQuine,
} from "./facade.js";
export type { BackendGenerateResult, ZipBomberBackendHooks } from "./facade.js";
export type { ZipBomberApiFactory, ZipBomberBackend } from "./api.js";
export { notImplementedError, runtimeDisposedError } from "./errors.js";
export { toUint8Array } from "./wire.js";

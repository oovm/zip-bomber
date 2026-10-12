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
} from "./shared/types.js";
export { VARIANTS } from "./shared/types.js";
export {
    createSharedZipBomberApi,
    zipBomb,
    zipFractal,
    zipLoopy,
    zipQuine,
} from "./shared/facade.js";

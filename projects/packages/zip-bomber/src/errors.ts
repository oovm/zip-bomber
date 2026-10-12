import type { VariantId, ZbError } from "./types.js";

/** Structured not-implemented error for a variant. */
export function notImplementedError(variant: VariantId): ZbError {
    return {
        category: "not_implemented",
        code: "ZB_NOT_IMPLEMENTED",
        message: `variant \`${variant}\` is recognized but not implemented`,
    };
}

/** Structured disposed-runtime error. */
export function runtimeDisposedError(): ZbError {
    return {
        category: "internal_error",
        code: "ZB_RUNTIME_DISPOSED",
        message: "runtime disposed",
    };
}

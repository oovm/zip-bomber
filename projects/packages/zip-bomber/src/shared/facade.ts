import {
    VARIANTS,
    type ArtifactSet,
    type CapabilitySet,
    type GenerateRequest,
    type GenerateReport,
    type Outcome,
    type VariantId,
    type ZipBomberApi,
    type ZbError,
} from "./types.js";

function notImplementedReport(variant: VariantId): GenerateReport {
    return {
        schema: "zb.report/v1",
        container: "zip",
        variant,
        status: "not_implemented",
        topology_nodes: 0,
        topology_edges: 0,
        warnings: [`variant \`${variant}\` generator is not implemented yet`],
        error_code: "ZB_NOT_IMPLEMENTED",
        error_message: `variant \`${variant}\` is recognized but not implemented`,
        crc_strategy: "reset_bytes",
    };
}

function notImplementedError(variant: VariantId): ZbError {
    return {
        category: "not_implemented",
        code: "ZB_NOT_IMPLEMENTED",
        message: `variant \`${variant}\` is recognized but not implemented`,
    };
}

function failVariant(variant: VariantId): Outcome<ArtifactSet> {
    return {
        ok: false,
        error: notImplementedError(variant),
        report: notImplementedReport(variant),
    };
}

function bareRequest(variant: VariantId): GenerateRequest {
    return {
        schema: "zb.request/v2",
        container: "zip",
        variant,
    };
}

/** Shared isomorphic API used by Node and WASM loaders. */
export function createSharedZipBomberApi(backend: "node" | "wasm" | "stub"): ZipBomberApi {
    let disposed = false;

    const guard = async <T>(run: () => Promise<Outcome<T>>): Promise<Outcome<T>> => {
        if (disposed) {
            const report = notImplementedReport("bomb");
            report.status = "error";
            report.error_code = "ZB_RUNTIME_DISPOSED";
            report.error_message = "runtime disposed";
            return {
                ok: false,
                error: {
                    category: "internal_error",
                    code: "ZB_RUNTIME_DISPOSED",
                    message: "runtime disposed",
                },
                report,
            };
        }
        return run();
    };

    return {
        async capabilities(): Promise<CapabilitySet> {
            return {
                entries: [
                    ...VARIANTS.map((variant) => ({
                        container: "zip" as const,
                        variant,
                        topologyClass:
                            variant === "bomb"
                                ? "nested_tree"
                                : variant === "quine"
                                  ? "self_loop"
                                  : variant === "loopy"
                                    ? "directed_cycle"
                                    : "fractal_finite",
                        profile: "zip.profile/v0-stub",
                        status: "declared" as const,
                    })),
                    {
                        container: "zip",
                        variant: "fractal",
                        topologyClass: "fractal_recursive",
                        profile: "zip.profile/v0-stub",
                        status: "declared",
                    },
                ],
            };
        },
        plan(request) {
            return guard(async () => ({
                ok: false,
                error: notImplementedError(request.variant),
                report: notImplementedReport(request.variant),
            }));
        },
        generate(request) {
            return guard(async () => failVariant(request.variant));
        },
        verify() {
            return guard(async () => ({
                ok: false,
                error: {
                    category: "not_implemented",
                    code: "ZB_NOT_IMPLEMENTED",
                    message: "verify is not implemented yet",
                },
                report: notImplementedReport("bomb"),
            }));
        },
        zipBomb() {
            return guard(async () => failVariant("bomb"));
        },
        zipQuine() {
            return guard(async () => failVariant("quine"));
        },
        zipLoopy() {
            return guard(async () => failVariant("loopy"));
        },
        zipFractal() {
            return guard(async () => failVariant("fractal"));
        },
        async dispose() {
            disposed = true;
            void backend;
        },
    };
}

/** Convenience helpers shared by node/wasm entrypoints. */
export async function zipBomb(): Promise<Outcome<ArtifactSet>> {
    const api = createSharedZipBomberApi("stub");
    try {
        return await api.zipBomb();
    } finally {
        await api.dispose();
    }
}

export async function zipQuine(): Promise<Outcome<ArtifactSet>> {
    const api = createSharedZipBomberApi("stub");
    try {
        return await api.zipQuine();
    } finally {
        await api.dispose();
    }
}

export async function zipLoopy(): Promise<Outcome<ArtifactSet>> {
    const api = createSharedZipBomberApi("stub");
    try {
        return await api.zipLoopy();
    } finally {
        await api.dispose();
    }
}

export async function zipFractal(): Promise<Outcome<ArtifactSet>> {
    const api = createSharedZipBomberApi("stub");
    try {
        return await api.zipFractal();
    } finally {
        await api.dispose();
    }
}

export { bareRequest };

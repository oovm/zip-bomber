import {
    VARIANTS,
    type ArtifactSet,
    type CapabilitySet,
    type CapabilityStatus,
    type GenerateRequest,
    type GenerateReport,
    type Outcome,
    type VariantId,
    type ZipBomberApi,
    type ZbError,
} from "./types.js";

/** Native / WASM generate hook result (root archive bytes). */
export type BackendGenerateResult = {
    exitCode: number;
    ok: boolean;
    bytes?: Uint8Array;
    reportJson: string;
    errorCategory?: string;
    errorCode?: string;
    errorMessage?: string;
};

/** Optional backend hooks so Node/WASM loaders stay isomorphic. */
export type ZipBomberBackendHooks = {
    generateVariant?: (variant: string) => BackendGenerateResult;
    capabilities?: () => CapabilitySet;
};

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

function defaultCapabilities(bombStatus: CapabilityStatus): CapabilitySet {
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
                status: (variant === "bomb" ? bombStatus : "declared") as CapabilityStatus,
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
}

function parseReport(reportJson: string, fallback: VariantId): GenerateReport {
    try {
        const parsed = JSON.parse(reportJson) as Partial<GenerateReport>;
        return {
            schema: parsed.schema ?? "zb.report/v1",
            container: "zip",
            variant: (parsed.variant as VariantId) ?? fallback,
            status: parsed.status ?? "error",
            topology_nodes: parsed.topology_nodes ?? 0,
            topology_edges: parsed.topology_edges ?? 0,
            warnings: parsed.warnings ?? [],
            error_code: parsed.error_code,
            error_message: parsed.error_message,
            crc_strategy: parsed.crc_strategy,
        };
    } catch {
        const report = notImplementedReport(fallback);
        report.status = "error";
        report.error_code = "ZB_BINDING_MISMATCH";
        report.error_message = "failed to parse native report JSON";
        return report;
    }
}

function artifactSetFromRoot(
    bytes: Uint8Array | undefined,
    variant: VariantId,
): ArtifactSet {
    const root = `${variant}-root`;
    if (!bytes || bytes.length === 0) {
        return { root, artifacts: [], logicalNodeMap: [] };
    }
    return {
        root,
        artifacts: [
            {
                id: root,
                container: "zip",
                bytes,
                byteLength: String(bytes.length),
                digest: "",
            },
        ],
        logicalNodeMap: [`root=${root}`],
    };
}

function outcomeFromBackend(
    variant: VariantId,
    result: BackendGenerateResult,
): Outcome<ArtifactSet> {
    const report = parseReport(result.reportJson, variant);
    if (result.ok) {
        return {
            ok: true,
            value: artifactSetFromRoot(result.bytes, variant),
            report,
        };
    }
    return {
        ok: false,
        error: {
            category: (result.errorCategory as ZbError["category"]) ?? "not_implemented",
            code: result.errorCode ?? "ZB_NOT_IMPLEMENTED",
            message: result.errorMessage ?? `variant \`${variant}\` failed`,
        },
        report,
    };
}

/** Shared isomorphic API used by Node and WASM loaders. */
export function createSharedZipBomberApi(
    backend: "node" | "wasm" | "stub",
    hooks: ZipBomberBackendHooks = {},
): ZipBomberApi {
    let disposed = false;
    const hasGenerate = typeof hooks.generateVariant === "function";

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

    const runVariant = (variant: VariantId): Outcome<ArtifactSet> => {
        if (hasGenerate) {
            return outcomeFromBackend(variant, hooks.generateVariant!(variant));
        }
        return failVariant(variant);
    };

    return {
        async capabilities(): Promise<CapabilitySet> {
            if (hooks.capabilities) {
                return hooks.capabilities();
            }
            return defaultCapabilities(hasGenerate ? "implemented" : "declared");
        },
        plan(request) {
            return guard(async () => ({
                ok: false,
                error: notImplementedError(request.variant),
                report: notImplementedReport(request.variant),
            }));
        },
        generate(request) {
            return guard(async () => runVariant(request.variant));
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
            return guard(async () => runVariant("bomb"));
        },
        zipQuine() {
            return guard(async () => runVariant("quine"));
        },
        zipLoopy() {
            return guard(async () => runVariant("loopy"));
        },
        zipFractal() {
            return guard(async () => runVariant("fractal"));
        },
        async dispose() {
            disposed = true;
            void backend;
        },
    };
}

/** Convenience helpers shared by node/wasm entrypoints (stub backend). */
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

import { graphRequest, isGraphSpec } from "./builders.js";
import {
    VARIANTS,
    type ArtifactSet,
    type CapabilitySet,
    type CapabilityStatus,
    type GenerateRequest,
    type GraphSpec,
    type GenerateReport,
    type Outcome,
    type PlanSummary,
    type VariantId,
    type VerifySummary,
    type ZipBomberApi,
    type ZbError,
} from "./types.js";
import { encodeGenerateRequestJson } from "./wire.js";

/** Native / WASM generate hook result (root archive bytes). */
export type BackendGenerateResult = {
    exitCode: number;
    ok: boolean;
    root?: string;
    bytes?: Uint8Array;
    artifacts?: Array<{ id: string; bytes: Uint8Array; byteLength?: string; digest?: string }>;
    logicalNodeMap?: string[];
    reportJson: string;
    errorCategory?: string;
    errorCode?: string;
    errorMessage?: string;
};

/** Native / WASM plan hook result. */
export type BackendPlanResult = {
    exitCode: number;
    ok: boolean;
    feasible?: boolean;
    summary?: string;
    profile?: string;
    reportJson: string;
    errorCategory?: string;
    errorCode?: string;
    errorMessage?: string;
};

/** Native / WASM verify hook result. */
export type BackendVerifyResult = {
    exitCode: number;
    ok: boolean;
    reportJson: string;
    errorCategory?: string;
    errorCode?: string;
    errorMessage?: string;
};

/** Optional backend hooks so Node/WASM loaders stay isomorphic. */
export type ZipBomberBackendHooks = {
    /** Preferred full-request path (`zb.request/v2` JSON). */
    generateRequestJson?: (requestJson: string) => BackendGenerateResult;
    /**
     * Libuv-pool generate. `signal` abort flips the shared cancel flag.
     * WASM has no mid-call interrupt. The facade still honors an already-aborted signal.
     */
    generateRequestCancellable?: (
        requestJson: string,
        signal?: AbortSignal,
    ) => Promise<BackendGenerateResult>;
    planRequestJson?: (requestJson: string) => BackendPlanResult;
    /** Bare-variant fallback used by older bindings. */
    generateVariant?: (variant: string) => BackendGenerateResult;
    planVariant?: (variant: string) => BackendPlanResult;
    verifyZipBytes?: (bytes: Uint8Array) => BackendVerifyResult;
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

function cancelledOutcome(variant: VariantId): Outcome<ArtifactSet> {
    const report = notImplementedReport(variant);
    report.status = "error";
    report.error_code = "ZB_CANCELLED";
    report.error_message = "solver cancelled";
    report.warnings = [];
    return {
        ok: false,
        error: {
            category: "cancelled",
            code: "ZB_CANCELLED",
            message: "solver cancelled",
        },
        report,
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

function mergeRequest(
    variant: VariantId,
    options?: Partial<GenerateRequest> | GraphSpec,
): GenerateRequest {
    if (options && isGraphSpec(options)) {
        if (options.variant !== variant) throw new TypeError("graph specification variant mismatch");
        return graphRequest(options);
    }
    return {
        schema: "zb.request/v2",
        container: "zip",
        variant,
        graph: options?.graph,
        profile: options?.profile,
        limits: options?.limits,
        topology: options?.topology,
        selfMemberPath: options?.selfMemberPath,
        payloads: options?.payloads,
    };
}

function requestNeedsJson(request: GenerateRequest): boolean {
    return Boolean(
        request.graph !== undefined || request.profile !== undefined || request.limits !== undefined ||
        request.selfMemberPath !== undefined ||
            (request.payloads && request.payloads.length > 0) ||
            request.topology !== undefined,
    );
}

function toWireJson(request: GenerateRequest): string {
    return encodeGenerateRequestJson({
        variant: request.variant,
        graph: request.graph, profile: request.profile, limits: request.limits,
        selfMemberPath: request.selfMemberPath,
        payloads: request.payloads,
        topology: request.topology,
    });
}

function defaultCapabilities(bombStatus: CapabilityStatus): CapabilitySet {
    // Stub backends stay fully `declared`. When a real binding is absent but a
    // bomb hook is present, mirror the Rust matrix profiles and promote only
    // the rows that already cleared Living `07` verified gates.
    const live = bombStatus !== "declared";
    const row = (
        variant: (typeof VARIANTS)[number],
        topologyClass: CapabilitySet["entries"][number]["topologyClass"],
        profile: string,
        status: CapabilityStatus,
    ) => ({
        container: "zip" as const,
        variant,
        topologyClass,
        profile,
        status: live ? status : ("declared" as const),
    });
    return {
        entries: [
            row("bomb", "nested_tree", "zip.profile/v0-stub", "implemented"),
            row("quine", "self_loop", "zip.profile/v0-stub", "verified"),
            row("loopy", "directed_cycle", "zip.loopy-two-node.gapped/v1", "verified"),
            row("fractal", "fractal_finite", "zip.profile/v0-stub", "implemented"),
            row("fractal", "fractal_recursive", "zip.loopy-two-node.gapped/v1", "implemented"),
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
            profile: parsed.profile ?? undefined,
            topology_nodes: parsed.topology_nodes ?? 0,
            topology_edges: parsed.topology_edges ?? 0,
            warnings: parsed.warnings ?? [],
            error_code: parsed.error_code,
            error_message: parsed.error_message,
            crc_strategy: parsed.crc_strategy,
            solver: parsed.solver ?? undefined,
            loopy: parsed.loopy ?? undefined,
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
    artifacts?: BackendGenerateResult["artifacts"],
    logicalNodeMap?: string[],
    actualRoot?: string,
): ArtifactSet {
    const root = actualRoot ?? `${variant}-root`;
    const full = artifacts?.length ? artifacts.map((a) => ({ id:a.id, container:"zip" as const, bytes:a.bytes, byteLength:a.byteLength ?? String(a.bytes.length), digest:a.digest ?? "" })) : (bytes && bytes.length ? [{ id:root, container:"zip" as const, bytes, byteLength:String(bytes.length), digest:"" }] : []);
    return { root, artifacts: full, logicalNodeMap: logicalNodeMap ?? [`root=${root}`] };
}

function outcomeFromBackend(
    variant: VariantId,
    result: BackendGenerateResult,
): Outcome<ArtifactSet> {
    const report = parseReport(result.reportJson, variant);
    if (result.ok) {
        return {
            ok: true,
            value: artifactSetFromRoot(result.bytes, variant, result.artifacts, result.logicalNodeMap, result.root),
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
    const hasGenerate =
        typeof hooks.generateRequestCancellable === "function" ||
        typeof hooks.generateRequestJson === "function" ||
        typeof hooks.generateVariant === "function";

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

    const runRequest = async (
        request: GenerateRequest,
        signal?: AbortSignal,
    ): Promise<Outcome<ArtifactSet>> => {
        if (signal?.aborted) {
            return cancelledOutcome(request.variant);
        }
        if (hooks.generateRequestCancellable) {
            return outcomeFromBackend(
                request.variant,
                await hooks.generateRequestCancellable(toWireJson(request), signal),
            );
        }
        if (hooks.generateRequestJson && requestNeedsJson(request)) {
            return outcomeFromBackend(
                request.variant,
                hooks.generateRequestJson(toWireJson(request)),
            );
        }
        if (hooks.generateRequestJson) {
            return outcomeFromBackend(
                request.variant,
                hooks.generateRequestJson(toWireJson(request)),
            );
        }
        if (hooks.generateVariant) {
            if (requestNeedsJson(request)) {
                return {
                    ok: false,
                    error: {
                        category: "backend_unavailable",
                        code: "ZB_BACKEND_UNAVAILABLE",
                        message:
                            "backend lacks generateRequestJson for selfMemberPath/payloads",
                    },
                    report: notImplementedReport(request.variant),
                };
            }
            return outcomeFromBackend(
                request.variant,
                hooks.generateVariant(request.variant),
            );
        }
        return failVariant(request.variant);
    };

    const planRequest = (request: GenerateRequest): Outcome<PlanSummary> => {
        const runJson = hooks.planRequestJson
            ? () => hooks.planRequestJson!(toWireJson(request))
            : hooks.planVariant && !requestNeedsJson(request)
              ? () => hooks.planVariant!(request.variant)
              : undefined;
        if (runJson) {
            const result = runJson();
            const report = parseReport(result.reportJson, request.variant);
            if (result.ok) {
                return {
                    ok: true,
                    value: {
                        feasible: result.feasible ?? true,
                        variant: request.variant,
                        profile: result.profile ?? "zip.profile/v0-stub",
                        summary: result.summary ?? "planned",
                        warnings: report.warnings ?? [],
                    },
                    report,
                };
            }
            return {
                ok: false,
                error: {
                    category: (result.errorCategory as ZbError["category"]) ?? "not_implemented",
                    code: result.errorCode ?? "ZB_NOT_IMPLEMENTED",
                    message: result.errorMessage ?? `plan for \`${request.variant}\` failed`,
                },
                report,
            };
        }
        return {
            ok: false,
            error: notImplementedError(request.variant),
            report: notImplementedReport(request.variant),
        };
    };

    const verifyBytes = (input: ArtifactSet): Outcome<VerifySummary> => {
        const root =
            input.artifacts.find((a) => a.id === input.root) ?? input.artifacts[0];
        if (!root || root.bytes.length === 0) {
            const report = notImplementedReport("bomb");
            report.status = "error";
            report.error_code = "ZB_INVALID_TOPOLOGY";
            report.error_message = "artifact set has no root bytes to verify";
            return {
                ok: false,
                error: {
                    category: "invalid_request",
                    code: "ZB_INVALID_TOPOLOGY",
                    message: "artifact set has no root bytes to verify",
                },
                report,
            };
        }
        if (hooks.verifyZipBytes) {
            const result = hooks.verifyZipBytes(root.bytes);
            const report = parseReport(result.reportJson, "bomb");
            if (result.ok) {
                let notes: string[] = [];
                let missingCapabilities: string[] = [];
                try {
                    const parsed = JSON.parse(result.reportJson) as {
                        notes?: string[];
                        missing_capabilities?: string[];
                    };
                    notes = parsed.notes ?? [];
                    missingCapabilities = parsed.missing_capabilities ?? [];
                } catch {
                    // keep empty notes
                }
                return {
                    ok: true,
                    value: { verified: true, notes, missingCapabilities },
                    report,
                };
            }
            return {
                ok: false,
                error: {
                    category:
                        (result.errorCategory as ZbError["category"]) ?? "verification_failed",
                    code: result.errorCode ?? "ZB_VERIFICATION_FAILED",
                    message: result.errorMessage ?? "verification failed",
                },
                report,
            };
        }
        return {
            ok: false,
            error: {
                category: "not_implemented",
                code: "ZB_NOT_IMPLEMENTED",
                message: "verify is not implemented yet",
            },
            report: notImplementedReport("bomb"),
        };
    };

    return {
        async capabilities(): Promise<CapabilitySet> {
            if (hooks.capabilities) {
                return hooks.capabilities();
            }
            return defaultCapabilities(hasGenerate ? "implemented" : "declared");
        },
        plan(request) {
            return guard(async () => planRequest(isGraphSpec(request) ? graphRequest(request) : request));
        },
        generate(request, options) {
            const spec = isGraphSpec(request) ? graphRequest(request) : request;
            return guard(async () => runRequest(spec, options?.signal));
        },
        verify(input) {
            return guard(async () => verifyBytes(input));
        },
        zipBomb(options) {
            return guard(async () => runRequest(mergeRequest("bomb", options)));
        },
        zipQuine(options) {
            return guard(async () => runRequest(mergeRequest("quine", options)));
        },
        zipLoopy(options) {
            return guard(async () => runRequest(mergeRequest("loopy", options)));
        },
        zipFractal(options) {
            return guard(async () => runRequest(mergeRequest("fractal", options)));
        },
        async dispose() {
            disposed = true;
            void backend;
        },
    };
}

/** Convenience helpers shared by node/wasm entrypoints (stub backend). */
export async function zipBomb(
    options?: Partial<GenerateRequest>,
): Promise<Outcome<ArtifactSet>> {
    const api = createSharedZipBomberApi("stub");
    try {
        return await api.zipBomb(options);
    } finally {
        await api.dispose();
    }
}

export async function zipQuine(
    options?: Partial<GenerateRequest>,
): Promise<Outcome<ArtifactSet>> {
    const api = createSharedZipBomberApi("stub");
    try {
        return await api.zipQuine(options);
    } finally {
        await api.dispose();
    }
}

export async function zipLoopy(
    options?: Partial<GenerateRequest> | GraphSpec,
): Promise<Outcome<ArtifactSet>> {
    const api = createSharedZipBomberApi("stub");
    try {
        return await api.zipLoopy(options);
    } finally {
        await api.dispose();
    }
}

export async function zipFractal(
    options?: Partial<GenerateRequest> | GraphSpec,
): Promise<Outcome<ArtifactSet>> {
    const api = createSharedZipBomberApi("stub");
    try {
        return await api.zipFractal(options);
    } finally {
        await api.dispose();
    }
}

export { bareRequest };

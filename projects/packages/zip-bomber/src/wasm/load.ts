import {
    createSharedZipBomberApi,
    type ZipBomberBackendHooks,
} from "../facade.js";
import {
    VARIANTS,
    type ArtifactSet,
    type CapabilitySet,
    type GenerateRequest,
    type GraphSpec,
    type Outcome,
    type VariantId,
    type ZipBomberApi,
} from "../types.js";
import {
    cancelledWasmGenerateResult,
    mapGenerateWasmJson,
    type WasmGenerateJson,
    bytesToB64,
    b64ToBytes,
} from "./result.js";
import {
    runGenerateInWorker,
    supportsWasmWorker,
} from "./cancellable.js";
import { initWasmBinding, type WasmBinding } from "./worker-runtime.js";

const WORKER_URL = new URL("./generate-worker.js", import.meta.url);

export type ZipBomberWasmApi = {
    listVariants: () => VariantId[];
    createZipBomber: () => Promise<ZipBomberApi>;
};

type WasmBindingFull = WasmBinding & {
    capabilitiesJson?: () => string;
    generateVariantJson?: (variant: string) => string;
    planVariantJson?: (variant: string) => string;
    planRequestJson?: (requestJson: string) => string;
    verifyZipBytesJson?: (bytesBase64: string) => string;
    verifyArtifactSetJson?: (setJson: string, specJson?: string | null) => string;
    verifyArtifactSetForRequestJson?: (setJson: string, requestJson: string) => string;
    default?: WasmBinding["default"] & {
        listVariants?: () => string;
        capabilitiesJson?: () => string;
        generateVariantJson?: (variant: string) => string;
        planRequestJson?: (requestJson: string) => string;
        verifyArtifactSetJson?: (setJson: string, specJson?: string | null) => string;
        verifyArtifactSetForRequestJson?: (setJson: string, requestJson: string) => string;
    };
};

function hooksFromWasm(binding: WasmBindingFull): ZipBomberBackendHooks {
    const fromDefault = typeof binding.default === "object" ? binding.default : undefined;
    const generateVariantJson =
        binding.generateVariantJson ?? fromDefault?.generateVariantJson;
    const generateRequestJson =
        binding.generateRequestJson ?? fromDefault?.generateRequestJson;
    const planRequestJson = binding.planRequestJson ?? fromDefault?.planRequestJson;
    const capabilitiesJson =
        binding.capabilitiesJson ?? fromDefault?.capabilitiesJson;
    const verifyArtifactSetJson =
        binding.verifyArtifactSetJson ?? fromDefault?.verifyArtifactSetJson;
    const verifyArtifactSetForRequestJson =
        binding.verifyArtifactSetForRequestJson ??
        fromDefault?.verifyArtifactSetForRequestJson;
    return {
        planRequestJson: planRequestJson
            ? (requestJson) => {
                  const raw = JSON.parse(planRequestJson(requestJson)) as {
                      ok: boolean;
                      feasible?: boolean;
                      summary?: string;
                      profile?: string;
                      budgetEstimate?: import("../types.js").PlanBudgetEstimate;
                      report?: unknown;
                      error?: { category?: string; code?: string; message?: string };
                  };
                  return {
                      exitCode: raw.ok ? 0 : 2,
                      ok: raw.ok,
                      feasible: raw.feasible,
                      summary: raw.summary,
                      profile: raw.profile,
                      budgetEstimate: raw.budgetEstimate,
                      reportJson: JSON.stringify(raw.report ?? {}),
                      errorCategory: raw.error?.category,
                      errorCode: raw.error?.code,
                      errorMessage: raw.error?.message,
                  };
              }
            : undefined,
        planVariant: binding.planVariantJson
            ? (variant) => {
                  const raw = JSON.parse(binding.planVariantJson!(variant));
                  return {
                      exitCode: raw.ok ? 0 : 2, ok: raw.ok,
                      feasible: raw.feasible, summary: raw.summary, profile: raw.profile,
                      reportJson: JSON.stringify(raw.report ?? {}),
                      errorCategory: raw.error?.category, errorCode: raw.error?.code,
                      errorMessage: raw.error?.message,
                  };
              } : undefined,
        verifyZipBytes: binding.verifyZipBytesJson
            ? (bytes) => {
                  const raw = JSON.parse(binding.verifyZipBytesJson!(bytesToB64(bytes)));
                  return {
                      exitCode: raw.ok ? 0 : 1, ok: raw.ok,
                      reportJson: JSON.stringify(raw.report ?? {}),
                      errorCategory: raw.error?.category, errorCode: raw.error?.code,
                      errorMessage: raw.error?.message,
                  };
              } : undefined,
        verifyArtifactSetJson: verifyArtifactSetJson
            ? (setJson, specJson) => {
                  const raw = JSON.parse(
                      verifyArtifactSetJson(setJson, specJson ?? undefined),
                  );
                  return {
                      exitCode: raw.ok ? 0 : 1,
                      ok: raw.ok,
                      reportJson: JSON.stringify(raw.report ?? {}),
                      errorCategory: raw.error?.category,
                      errorCode: raw.error?.code,
                      errorMessage: raw.error?.message,
                  };
              }
            : undefined,
        verifyArtifactSetForRequestJson: verifyArtifactSetForRequestJson
            ? (setJson, requestJson) => {
                  const raw = JSON.parse(
                      verifyArtifactSetForRequestJson(setJson, requestJson),
                  );
                  return {
                      exitCode: raw.ok ? 0 : 1,
                      ok: raw.ok,
                      reportJson: JSON.stringify(raw.report ?? {}),
                      errorCategory: raw.error?.category,
                      errorCode: raw.error?.code,
                      errorMessage: raw.error?.message,
                  };
              }
            : undefined,
        generateRequestJson: generateRequestJson
            ? (requestJson) =>
                  mapGenerateWasmJson(
                      JSON.parse(generateRequestJson(requestJson)) as WasmGenerateJson,
                  )
            : undefined,
        generateRequestCancellable: generateRequestJson
            ? (requestJson, signal) => {
                  if (signal?.aborted) {
                      return Promise.resolve(cancelledWasmGenerateResult());
                  }
                  if (signal && supportsWasmWorker()) {
                      return runGenerateInWorker(WORKER_URL, requestJson, signal);
                  }
                  return Promise.resolve(
                      mapGenerateWasmJson(
                          JSON.parse(
                              generateRequestJson(requestJson),
                          ) as WasmGenerateJson,
                      ),
                  );
              }
            : undefined,
        generateVariant: generateVariantJson
            ? (variant) => {
                  const raw = JSON.parse(generateVariantJson(variant)) as {
                      ok: boolean;
                      bytesBase64?: string;
                      report?: unknown;
                      error?: {
                          category?: string;
                          code?: string;
                          message?: string;
                      };
                  };
                  return {
                      exitCode: raw.ok ? 0 : 2,
                      ok: raw.ok,
                      bytes: raw.bytesBase64 ? b64ToBytes(raw.bytesBase64) : undefined,
                      reportJson: JSON.stringify(raw.report ?? {}),
                      errorCategory: raw.error?.category,
                      errorCode: raw.error?.code,
                      errorMessage: raw.error?.message,
                  };
              }
            : undefined,
        capabilities: capabilitiesJson
            ? () => {
                  const parsed = JSON.parse(capabilitiesJson()) as {
                      entries: CapabilitySet["entries"];
                  };
                  return {
                      entries: parsed.entries.map((e) => ({
                          container: e.container,
                          variant: e.variant,
                          profile: e.profile,
                          status: e.status,
                          topologyClass:
                              (e as { topology_class?: string }).topology_class ??
                              e.topologyClass,
                      })),
                  };
              }
            : undefined,
    };
}

/** WASM capability probe + isomorphic API factory. */
export async function loadZipBomberWasm(): Promise<ZipBomberWasmApi> {
    let listed: VariantId[] | null = null;
    let hooks: ZipBomberBackendHooks = {};
    try {
        const mod = (await initWasmBinding()) as WasmBindingFull;
        const listVariants = mod.listVariants ?? (typeof mod.default === "object" ? mod.default.listVariants : undefined);
        if (typeof listVariants === "function") {
            listed = listVariants()
                .split(",")
                .filter(Boolean) as VariantId[];
        }
        hooks = hooksFromWasm(mod);
    } catch {
        // fall through to stub
    }
    return {
        listVariants: () => listed ?? [...VARIANTS],
        createZipBomber: async () =>
            createSharedZipBomberApi(
                Object.keys(hooks).length > 0 ? "wasm" : "stub",
                hooks,
            ),
    };
}

/** Isomorphic factory for WASM / browser. */
export async function createZipBomber(): Promise<ZipBomberApi> {
    const api = await loadZipBomberWasm();
    return api.createZipBomber();
}

export async function bomb(
    options?: Partial<GenerateRequest>,
): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.bomb(options);
    } finally {
        await api.dispose();
    }
}

export async function quine(
    options?: Partial<GenerateRequest>,
): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.quine(options);
    } finally {
        await api.dispose();
    }
}

export async function loopy(
    options?: Partial<GenerateRequest> | GraphSpec,
): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.loopy(options);
    } finally {
        await api.dispose();
    }
}

export async function fractal(
    options?: Partial<GenerateRequest> | GraphSpec,
): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.fractal(options);
    } finally {
        await api.dispose();
    }
}

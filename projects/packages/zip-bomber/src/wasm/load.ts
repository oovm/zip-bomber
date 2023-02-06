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

const ARTIFACT_PACKAGE = "@sxo/zip-bomber-unknown-wasm32";

export type ZipBomberWasmApi = {
    listVariants: () => VariantId[];
    createZipBomber: () => Promise<ZipBomberApi>;
};

type WasmBinding = {
    listVariants?: () => string;
    capabilitiesJson?: () => string;
    generateVariantJson?: (variant: string) => string;
    generateRequestJson?: (requestJson: string) => string;
    planVariantJson?: (variant: string) => string;
    planRequestJson?: (requestJson: string) => string;
    verifyZipBytesJson?: (bytesBase64: string) => string;
    verifyArtifactSetJson?: (setJson: string, specJson?: string | null) => string;
    verifyArtifactSetForRequestJson?: (setJson: string, requestJson: string) => string;
    default?: ((input?: unknown) => Promise<unknown>) | {
        listVariants?: () => string;
        capabilitiesJson?: () => string;
        generateVariantJson?: (variant: string) => string;
        generateRequestJson?: (requestJson: string) => string;
        planRequestJson?: (requestJson: string) => string;
        verifyArtifactSetJson?: (setJson: string, specJson?: string | null) => string;
        verifyArtifactSetForRequestJson?: (setJson: string, requestJson: string) => string;
    };
};

function bytesToB64(bytes: Uint8Array): string {
    let text = "";
    for (const byte of bytes) text += String.fromCharCode(byte);
    return btoa(text);
}

function b64ToBytes(b64: string): Uint8Array {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i += 1) {
        out[i] = bin.charCodeAt(i);
    }
    return out;
}

function hooksFromWasm(binding: WasmBinding): ZipBomberBackendHooks {
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
            ? (requestJson) => {
                  const raw = JSON.parse(generateRequestJson(requestJson)) as {
                      ok: boolean;
                      bytesBase64?: string;
                      root?: string;
                      artifacts?: Array<{id:string;bytesBase64:string;byteLength?:number;digest?:string}>;
                      logicalNodeMap?: string[];
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
                      root: raw.root,
                      artifacts: raw.artifacts?.map(a=>({id:a.id,bytes:b64ToBytes(a.bytesBase64),byteLength:String(a.byteLength ?? 0),digest:a.digest ?? ""})),
                      logicalNodeMap: raw.logicalNodeMap,
                      reportJson: JSON.stringify(raw.report ?? {}),
                      errorCategory: raw.error?.category,
                      errorCode: raw.error?.code,
                      errorMessage: raw.error?.message,
                  };
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
        const mod = (await import(ARTIFACT_PACKAGE)) as WasmBinding;
        if (typeof mod.default === "function") {
            if (typeof process !== "undefined" && process.versions?.node) {
                const { readFile } = await import("node:fs/promises");
                const { createRequire } = await import("node:module");
                const { pathToFileURL } = await import("node:url");
                const moduleUrl = pathToFileURL(createRequire(import.meta.url).resolve(ARTIFACT_PACKAGE));
                const bytes = await readFile(new URL("./zb_wasm_bg.wasm", moduleUrl));
                await mod.default({ module_or_path: bytes });
            } else {
                await mod.default();
            }
        }
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

export async function zipBomb(
    options?: Partial<GenerateRequest>,
): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.zipBomb(options);
    } finally {
        await api.dispose();
    }
}

export async function zipQuine(
    options?: Partial<GenerateRequest>,
): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.zipQuine(options);
    } finally {
        await api.dispose();
    }
}

export async function zipLoopy(
    options?: Partial<GenerateRequest> | GraphSpec,
): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.zipLoopy(options);
    } finally {
        await api.dispose();
    }
}

export async function zipFractal(
    options?: Partial<GenerateRequest> | GraphSpec,
): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.zipFractal(options);
    } finally {
        await api.dispose();
    }
}

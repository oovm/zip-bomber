import {
    createSharedZipBomberApi,
    type ZipBomberBackendHooks,
} from "../facade.js";
import {
    VARIANTS,
    type ArtifactSet,
    type CapabilitySet,
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
    default?: {
        listVariants?: () => string;
        capabilitiesJson?: () => string;
        generateVariantJson?: (variant: string) => string;
    };
};

function b64ToBytes(b64: string): Uint8Array {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i += 1) {
        out[i] = bin.charCodeAt(i);
    }
    return out;
}

function hooksFromWasm(binding: WasmBinding): ZipBomberBackendHooks {
    const generateVariantJson =
        binding.generateVariantJson ?? binding.default?.generateVariantJson;
    const capabilitiesJson =
        binding.capabilitiesJson ?? binding.default?.capabilitiesJson;
    return {
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
                          ...e,
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
        const listVariants = mod.listVariants ?? mod.default?.listVariants;
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

export async function zipBomb(): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.zipBomb();
    } finally {
        await api.dispose();
    }
}

export async function zipQuine(): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.zipQuine();
    } finally {
        await api.dispose();
    }
}

export async function zipLoopy(): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.zipLoopy();
    } finally {
        await api.dispose();
    }
}

export async function zipFractal(): Promise<Outcome<ArtifactSet>> {
    const api = await createZipBomber();
    try {
        return await api.zipFractal();
    } finally {
        await api.dispose();
    }
}

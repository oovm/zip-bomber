import { createRequire } from "node:module";
import {
    createSharedZipBomberApi,
    type ZipBomberBackendHooks,
} from "../shared/facade.js";
import {
    VARIANTS,
    type CapabilitySet,
    type Outcome,
    type ArtifactSet,
    type VariantId,
    type ZipBomberApi,
} from "../shared/types.js";

export type NativeGenerateResult = {
    exitCode: number;
    ok: boolean;
    bytes?: Buffer;
    reportJson: string;
    errorCategory?: string;
    errorCode?: string;
    errorMessage?: string;
};

export type ZipBomberNodeApi = {
    listVariants: () => VariantId[];
    createZipBomber: () => Promise<ZipBomberApi>;
    generateVariant?: (variant: string) => NativeGenerateResult;
};

type NativeBinding = {
    listVariantsNapi?: () => string[];
    capabilitiesNapi?: () => Array<{
        container: string;
        variant: string;
        topologyClass?: string;
        topology_class?: string;
        profile: string;
        status: string;
    }>;
    generateVariant?: (variant: string) => {
        exitCode: number;
        ok: boolean;
        bytes?: Buffer;
        reportJson: string;
        errorCategory?: string;
        errorCode?: string;
        errorMessage?: string;
    };
};

function mapCapabilities(binding: NativeBinding): CapabilitySet | undefined {
    if (!binding.capabilitiesNapi) {
        return undefined;
    }
    const rows = binding.capabilitiesNapi();
    return {
        entries: rows.map((row) => ({
            container: "zip" as const,
            variant: row.variant as VariantId,
            topologyClass: row.topologyClass ?? row.topology_class ?? "nested_tree",
            profile: row.profile,
            status: row.status as CapabilitySet["entries"][number]["status"],
        })),
    };
}

function hooksFromBinding(binding: NativeBinding): ZipBomberBackendHooks {
    const caps = mapCapabilities(binding);
    return {
        generateVariant: binding.generateVariant
            ? (variant) => {
                  const result = binding.generateVariant!(variant);
                  return {
                      exitCode: result.exitCode,
                      ok: result.ok,
                      bytes: result.bytes,
                      reportJson: result.reportJson,
                      errorCategory: result.errorCategory,
                      errorCode: result.errorCode,
                      errorMessage: result.errorMessage,
                  };
              }
            : undefined,
        capabilities: caps ? () => caps : undefined,
    };
}

/** Load native zip-bomber bindings when present; otherwise return a TS-only stub. */
export function loadZipBomberNode(): ZipBomberNodeApi {
    const require = createRequire(import.meta.url);
    const candidates = [
        "@sxo/zip-bomber-win32-x64",
        "@sxo/zip-bomber-linux-x64",
        "@sxo/zip-bomber-linux-arm64",
        "@sxo/zip-bomber-darwin-x64",
        "@sxo/zip-bomber-darwin-arm64",
    ];
    for (const name of candidates) {
        try {
            const binding = require(name) as NativeBinding;
            const hooks = hooksFromBinding(binding);
            return {
                listVariants: () =>
                    (binding.listVariantsNapi?.() ?? [...VARIANTS]) as VariantId[],
                createZipBomber: async () => createSharedZipBomberApi("node", hooks),
                generateVariant: binding.generateVariant
                    ? (variant) => binding.generateVariant!(variant)
                    : undefined,
            };
        } catch {
            // try next platform package
        }
    }
    return {
        listVariants: () => [...VARIANTS],
        createZipBomber: async () => createSharedZipBomberApi("stub"),
    };
}

/** Isomorphic factory for Node. */
export async function createZipBomber(): Promise<ZipBomberApi> {
    return loadZipBomberNode().createZipBomber();
}

/** Convenience: prefer native Bomb emit when the platform package is present. */
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

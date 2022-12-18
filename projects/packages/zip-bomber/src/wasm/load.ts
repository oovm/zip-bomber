import {
    createSharedZipBomberApi,
    zipBomb,
    zipFractal,
    zipLoopy,
    zipQuine,
} from "../shared/facade.js";
import { VARIANTS, type VariantId, type ZipBomberApi } from "../shared/types.js";

const ARTIFACT_PACKAGE = "@sxo/zip-bomber-unknown-wasm32";

export type ZipBomberWasmApi = {
    listVariants: () => VariantId[];
    createZipBomber: () => Promise<ZipBomberApi>;
};

type WasmBinding = {
    listVariants?: () => string;
    default?: { listVariants?: () => string };
};

/** WASM capability probe + isomorphic API factory. */
export async function loadZipBomberWasm(): Promise<ZipBomberWasmApi> {
    let listed: VariantId[] | null = null;
    try {
        const mod = (await import(ARTIFACT_PACKAGE)) as WasmBinding;
        const listVariants = mod.listVariants ?? mod.default?.listVariants;
        if (typeof listVariants === "function") {
            listed = listVariants()
                .split(",")
                .filter(Boolean) as VariantId[];
        }
    } catch {
        // fall through
    }
    return {
        listVariants: () => listed ?? [...VARIANTS],
        createZipBomber: async () => createSharedZipBomberApi("wasm"),
    };
}

/** Isomorphic factory for WASM / browser. */
export async function createZipBomber(): Promise<ZipBomberApi> {
    const api = await loadZipBomberWasm();
    return api.createZipBomber();
}

export { zipBomb, zipQuine, zipLoopy, zipFractal };

import { VARIANTS, type VariantId } from "../types.js";

const ARTIFACT_PACKAGE = "@sxo/zip-bomber-unknown-wasm32";

export type ZipBomberWasmApi = {
    listVariants: () => VariantId[];
};

type WasmBinding = {
    listVariants?: () => string;
    default?: { listVariants?: () => string };
};

/** WASM capability probe. Falls back to the static variant list. */
export async function loadZipBomberWasm(): Promise<ZipBomberWasmApi> {
    try {
        const mod = (await import(ARTIFACT_PACKAGE)) as WasmBinding;
        const listVariants = mod.listVariants ?? mod.default?.listVariants;
        if (typeof listVariants === "function") {
            return {
                listVariants: () =>
                    listVariants()
                        .split(",")
                        .filter(Boolean) as VariantId[],
            };
        }
    } catch {
        // fall through
    }
    return {
        listVariants: () => [...VARIANTS],
    };
}

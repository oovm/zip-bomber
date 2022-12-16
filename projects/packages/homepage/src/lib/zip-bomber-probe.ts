import { loadZipBomberWasm } from "@sxo/zip-bomber/wasm";
import type { VariantId } from "@sxo/zip-bomber";

export type ZipBomberProbe = {
    variants: VariantId[];
    backend: "node" | "wasm";
};

/** Node probe for CI and local verify without a browser. */
export async function probeZipBomberNode(): Promise<ZipBomberProbe> {
    const { loadZipBomberNode } = await import("@sxo/zip-bomber/node");
    const binding = loadZipBomberNode();
    return {
        variants: binding.listVariants(),
        backend: "node",
    };
}

/** Browser WASM probe via `@sxo/zip-bomber/wasm`. */
export async function probeZipBomberWasm(): Promise<ZipBomberProbe> {
    const binding = await loadZipBomberWasm();
    return {
        variants: binding.listVariants(),
        backend: "wasm",
    };
}

/** Prefer WASM in browsers, Node bindings elsewhere. */
export async function probeZipBomber(): Promise<ZipBomberProbe> {
    if (typeof window !== "undefined") {
        return probeZipBomberWasm();
    }
    return probeZipBomberNode();
}

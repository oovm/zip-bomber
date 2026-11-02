export type ZipBomberProbe = {
    variants: string[];
    backend: "node" | "wasm" | "browser-unavailable";
};

/**
 * Browser snapshot. The live WASM probe is disabled because the assembler packs
 * every statically visible import for the browser and cannot yet vendor the
 * multi-file `@sxo/zip-bomber` package. Track the compiler fix, then restore
 * `probeZipBomberWasm` / `probeZipBomberNode`.
 */
export function probeZipBomberBrowserUnavailable(): ZipBomberProbe {
    return {
        variants: ["bomb", "quine", "loopy", "fractal"],
        backend: "browser-unavailable",
    };
}

/** Homepage runs in the browser, so the probe is always the unavailable snapshot. */
export async function probeZipBomber(): Promise<ZipBomberProbe> {
    return probeZipBomberBrowserUnavailable();
}

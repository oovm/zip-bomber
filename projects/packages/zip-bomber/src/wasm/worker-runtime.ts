const ARTIFACT_PACKAGE = "@sxo/zip-bomber-unknown-wasm32";

export type WasmBinding = {
    listVariants?: () => string;
    generateRequestJson?: (requestJson: string) => string;
    default?: ((input?: unknown) => Promise<unknown>) | {
        generateRequestJson?: (requestJson: string) => string;
    };
};

export async function initWasmBinding(): Promise<WasmBinding> {
    const mod = (await import(ARTIFACT_PACKAGE)) as WasmBinding;
    if (typeof mod.default === "function") {
        if (typeof process !== "undefined" && process.versions?.node) {
            const { readFile } = await import("node:fs/promises");
            const { createRequire } = await import("node:module");
            const { pathToFileURL } = await import("node:url");
            const moduleUrl = pathToFileURL(
                createRequire(import.meta.url).resolve(ARTIFACT_PACKAGE),
            );
            const bytes = await readFile(new URL("./zb_wasm_bg.wasm", moduleUrl));
            await mod.default({ module_or_path: bytes });
        } else {
            await mod.default();
        }
    }
    return mod;
}

export function generateRequestJsonFromBinding(
    binding: WasmBinding,
    requestJson: string,
): string {
    const fromDefault =
        typeof binding.default === "object" ? binding.default : undefined;
    const generateRequestJson =
        binding.generateRequestJson ?? fromDefault?.generateRequestJson;
    if (!generateRequestJson) {
        throw new Error("wasm binding missing generateRequestJson");
    }
    return generateRequestJson(requestJson);
}

import { createRequire } from "node:module";
import { VARIANTS, type VariantId } from "../types.js";

export type ZipBomberNodeApi = {
    listVariants: () => VariantId[];
    generateVariant?: (variant: string) => {
        exitCode: number;
        bytes?: Buffer;
        reportJson: string;
    };
};

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
            const binding = require(name) as {
                listVariantsNapi?: () => string[];
                generateVariant?: (variant: string) => {
                    exitCode: number;
                    bytes?: Buffer;
                    reportJson: string;
                };
            };
            return {
                listVariants: () =>
                    (binding.listVariantsNapi?.() ?? [...VARIANTS]) as VariantId[],
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
    };
}

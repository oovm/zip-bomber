import { createRequire } from "node:module";
import {
    createSharedZipBomberApi,
    zipBomb,
    zipFractal,
    zipLoopy,
    zipQuine,
} from "../shared/facade.js";
import { VARIANTS, type VariantId, type ZipBomberApi } from "../shared/types.js";

export type ZipBomberNodeApi = {
    listVariants: () => VariantId[];
    createZipBomber: () => Promise<ZipBomberApi>;
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
                    ok: boolean;
                    bytes?: Buffer;
                    reportJson: string;
                    errorCategory?: string;
                    errorCode?: string;
                    errorMessage?: string;
                };
            };
            return {
                listVariants: () =>
                    (binding.listVariantsNapi?.() ?? [...VARIANTS]) as VariantId[],
                createZipBomber: async () => createSharedZipBomberApi("node"),
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

export { zipBomb, zipQuine, zipLoopy, zipFractal };

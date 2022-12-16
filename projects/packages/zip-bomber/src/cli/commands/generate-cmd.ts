import { writeFileSync } from "node:fs";
import { loadZipBomberNode } from "../../node/load.js";
import { VARIANTS, type VariantId } from "../../types.js";

function parseArgs(argv: string[]): { variant?: string; output?: string } {
    let variant: string | undefined;
    let output: string | undefined;
    for (let i = 0; i < argv.length; i += 1) {
        const arg = argv[i];
        if (arg === "--variant" || arg === "-v") {
            variant = argv[++i];
        } else if (arg === "-o" || arg === "--output") {
            output = argv[++i];
        }
    }
    return { variant, output };
}

/** Generate a variant archive (stub until native algorithms land). */
export async function runGenerate(argv: string[]): Promise<void> {
    const { variant, output } = parseArgs(argv);
    if (!variant || !output) {
        console.error("usage: zip-bomber generate --variant <id> -o <path>");
        process.exitCode = 1;
        return;
    }
    if (!VARIANTS.includes(variant as VariantId)) {
        console.error(`unknown variant: ${variant}`);
        console.error(`expected one of: ${VARIANTS.join(", ")}`);
        process.exitCode = 1;
        return;
    }

    const node = loadZipBomberNode();
    if (!node.generateVariant) {
        const report = {
            schema: "zb.report/v1",
            variant,
            status: "not_implemented",
            warnings: [
                "native zip-bomber binding is not built; run build:napi or wait for algorithm gates",
            ],
            error_code: "ZB_NOT_IMPLEMENTED",
            error_message: `variant \`${variant}\` is recognized but not implemented`,
        };
        console.log(JSON.stringify(report, null, 2));
        process.exitCode = 2;
        return;
    }

    const result = node.generateVariant(variant);
    console.log(result.reportJson);
    if (result.bytes && result.bytes.length > 0) {
        writeFileSync(output, result.bytes);
        console.error(`wrote ${output} (${result.bytes.length} bytes)`);
    }
    process.exitCode = result.exitCode;
}

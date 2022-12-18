import { writeFileSync } from "node:fs";
import { loadZipBomberNode } from "../../node/load.js";
import { zipBomb, zipFractal, zipLoopy, zipQuine } from "../../shared/facade.js";
import { VARIANTS, type VariantId } from "../../shared/types.js";

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

async function runShared(variant: VariantId) {
    switch (variant) {
        case "bomb":
            return zipBomb();
        case "quine":
            return zipQuine();
        case "loopy":
            return zipLoopy();
        case "fractal":
            return zipFractal();
    }
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
    if (node.generateVariant) {
        const result = node.generateVariant(variant);
        console.log(result.reportJson);
        if (result.bytes && result.bytes.length > 0) {
            writeFileSync(output, result.bytes);
            console.error(`wrote ${output} (${result.bytes.length} bytes)`);
        }
        process.exitCode = result.exitCode;
        return;
    }

    const outcome = await runShared(variant as VariantId);
    console.log(JSON.stringify(outcome, null, 2));
    if (outcome.ok && outcome.value.artifacts[0]?.bytes.length) {
        writeFileSync(output, outcome.value.artifacts[0].bytes);
        console.error(`wrote ${output}`);
        process.exitCode = 0;
        return;
    }
    process.exitCode = 2;
}

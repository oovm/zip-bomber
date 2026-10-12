import { writeFileSync } from "node:fs";

import type { Cli, ParsedOptions } from "@vmz/commander";

import {
    loadZipBomberNode,
    zipBomb,
    zipFractal,
    zipLoopy,
    zipQuine,
} from "../../node/load.js";
import { VARIANTS, type VariantId } from "../../types.js";

export function registerGenerateCommand(cli: Cli): void {
    cli.command("generate", "zb.cli.cmd.generate")
        .option("-v, --variant <id>", "zb.cli.opt.variant")
        .option("-o, --output <path>", "zb.cli.opt.output")
        .action((options) => runGenerate(options));
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

/** Generate a variant archive through the Node API / native binding. */
export async function runGenerate(options: ParsedOptions): Promise<number> {
    const variant = typeof options.variant === "string" ? options.variant : undefined;
    const output = typeof options.output === "string" ? options.output : undefined;
    if (!variant || !output) {
        console.error("generate requires --variant and --output");
        return 1;
    }
    if (!VARIANTS.includes(variant as VariantId)) {
        console.error(`unknown variant: ${variant}`);
        console.error(`expected one of: ${VARIANTS.join(", ")}`);
        return 1;
    }

    const node = loadZipBomberNode();
    if (node.generateVariant) {
        const result = node.generateVariant(variant);
        console.log(result.reportJson);
        if (result.bytes && result.bytes.length > 0) {
            writeFileSync(output, result.bytes);
            console.error(`wrote ${output} (${result.bytes.length} bytes)`);
        }
        return result.exitCode;
    }

    const outcome = await runShared(variant as VariantId);
    console.log(JSON.stringify(outcome, null, 2));
    if (outcome.ok && outcome.value.artifacts[0]?.bytes.length) {
        writeFileSync(output, outcome.value.artifacts[0].bytes);
        console.error(`wrote ${output}`);
        return 0;
    }
    return 2;
}

import type { Cli, ParsedOptions } from "@vmz/commander";

import { loadZipBomberNode } from "../../node/load.js";
import { createSharedZipBomberApi, bareRequest } from "../../facade.js";
import { VARIANTS, type VariantId } from "../../types.js";

export function registerPlanCommand(cli: Cli): void {
    cli.command("plan", "zb.cli.cmd.plan")
        .option("-v, --variant <id>", "zb.cli.opt.variant")
        .action((options) => runPlan(options));
}

/** Plan feasibility for a bare variant without writing bytes. */
export async function runPlan(options: ParsedOptions): Promise<number> {
    const variant = typeof options.variant === "string" ? options.variant : undefined;
    if (!variant) {
        console.error("plan requires --variant");
        return 1;
    }
    if (!VARIANTS.includes(variant as VariantId)) {
        console.error(`unknown variant: ${variant}`);
        console.error(`expected one of: ${VARIANTS.join(", ")}`);
        return 1;
    }

    const node = loadZipBomberNode();
    if (node.planVariant) {
        const result = node.planVariant(variant);
        console.log(result.reportJson);
        if (result.ok && result.summary) {
            console.error(
                `feasible=${result.feasible} profile=${result.profile ?? "-"} ${result.summary}`,
            );
        }
        return result.exitCode;
    }

    const api = createSharedZipBomberApi("stub");
    try {
        const outcome = await api.plan(bareRequest(variant as VariantId));
        console.log(JSON.stringify(outcome, null, 2));
        return outcome.ok ? 0 : 2;
    } finally {
        await api.dispose();
    }
}

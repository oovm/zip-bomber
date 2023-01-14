import type { Cli, ParsedOptions } from "@vmz/commander";
import { createZipBomber } from "../../node/index.js";
import { readHostRequest, requestOptions } from "../request.js";
import { inputFailure } from "../locale.js";

export function registerPlanCommand(cli: Cli): void {
    requestOptions(cli.command("plan", "zb.cli.cmd.plan")
        .option("-v, --variant <id>", "zb.cli.opt.variant"))
        .action(runPlan);
}

/** Plan the same attachment snapshots and layout used for generation. */
export async function runPlan(options: ParsedOptions): Promise<number> {
    try {
        const host = await readHostRequest(options);
        const request = host.kind === "flags" ? host.request : host.job.request;
        const api = await createZipBomber();
        try {
            const outcome = await api.plan(request);
            console.log(JSON.stringify(outcome));
            return outcome.ok ? 0 : 2;
        } finally {
            await api.dispose();
        }
    } catch (error) {
        return inputFailure(error, options);
    }
}

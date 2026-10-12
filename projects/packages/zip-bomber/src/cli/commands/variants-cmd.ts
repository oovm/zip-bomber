import type { Cli, ParsedOptions } from "@vmz/commander";

import { loadZipBomberNode } from "../../node/load.js";

export function registerVariantsCommand(cli: Cli): void {
    cli.command("variants", "zb.cli.cmd.variants")
        .option("--json", "zb.cli.opt.json")
        .action((options) => runVariants(options));
}

/** List supported variant ids. */
export async function runVariants(options: ParsedOptions): Promise<number> {
    const json = Boolean(options.json);
    const variants = loadZipBomberNode().listVariants();
    if (json) {
        console.log(JSON.stringify({ variants }, null, 2));
        return 0;
    }
    for (const id of variants) {
        console.log(id);
    }
    return 0;
}

import { readFileSync } from "node:fs";

import type { Cli, ParsedOptions } from "@vmz/commander";

import { loadZipBomberNode } from "../../node/load.js";
import { createSharedZipBomberApi } from "../../facade.js";

export function registerVerifyCommand(cli: Cli): void {
    cli.command("verify", "zb.cli.cmd.verify")
        .option("-i, --input <path>", "zb.cli.opt.input")
        .action((options) => runVerify(options));
}

/** Independently verify a ZIP archive on disk. */
export async function runVerify(options: ParsedOptions): Promise<number> {
    const input = typeof options.input === "string" ? options.input : undefined;
    if (!input) {
        console.error("verify requires --input");
        return 1;
    }

    let bytes: Buffer;
    try {
        bytes = readFileSync(input);
    } catch (error) {
        console.error(`failed to read ${input}: ${error instanceof Error ? error.message : error}`);
        return 1;
    }

    const node = loadZipBomberNode();
    if (node.verifyZipBytes) {
        const result = node.verifyZipBytes(bytes);
        console.log(result.reportJson);
        if (!result.ok && result.errorMessage) {
            console.error(result.errorMessage);
        }
        return result.exitCode;
    }

    const api = createSharedZipBomberApi("stub");
    try {
        const outcome = await api.verify({
            root: "cli-root",
            artifacts: [
                {
                    id: "cli-root",
                    container: "zip",
                    bytes: new Uint8Array(bytes),
                    byteLength: String(bytes.length),
                    digest: "",
                },
            ],
            logicalNodeMap: ["root=cli-root"],
        });
        console.log(JSON.stringify(outcome, null, 2));
        return outcome.ok ? 0 : 2;
    } finally {
        await api.dispose();
    }
}

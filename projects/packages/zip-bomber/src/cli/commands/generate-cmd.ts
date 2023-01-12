import { writeFile } from "node:fs/promises";
import type { Cli, ParsedOptions } from "@vmz/commander";
import { createZipBomber } from "../../node/index.js";
import { InputError, readRequest, requestOptions } from "../request.js";
import { inputFailure, translator } from "../locale.js";

export function registerGenerateCommand(cli: Cli): void {
    requestOptions(cli.command("generate", "zb.cli.cmd.generate")
        .option("-v, --variant <id>", "zb.cli.opt.variant")
        .option("-o, --output <path>", "zb.cli.opt.output"))
        .action(runGenerate);
}

/** Generate through the public Node API, preserving all request fields. */
export async function runGenerate(options: ParsedOptions): Promise<number> {
    try {
        if (typeof options.output !== "string" || !options.output) {
            throw new InputError("zb.cli.err.output");
        }
        const request = await readRequest(options);
        const api = await createZipBomber();
        try {
            const outcome = await api.generate(request);
            if (!outcome.ok) {
                console.log(JSON.stringify(outcome));
                return 2;
            }
            const artifact = outcome.value.artifacts.find(a => a.id === outcome.value.root);
            if (!artifact || outcome.value.artifacts.length !== 1) {
                throw new InputError("zb.cli.err.singleArtifact");
            }
            // Do not overwrite an existing output or an attachment source.
            await writeFile(options.output, artifact.bytes, { flag: "wx" });
            console.log(JSON.stringify({
                ok: true,
                value: {
                    root: outcome.value.root,
                    artifacts: [{ id: artifact.id, byteLength: artifact.byteLength, digest: artifact.digest }],
                    logicalNodeMap: outcome.value.logicalNodeMap,
                },
                report: outcome.report,
            }));
            console.error(translator(options)("zb.cli.result.wrote", {
                path: options.output, bytes: artifact.byteLength,
            }));
            return 0;
        } finally {
            await api.dispose();
        }
    } catch (error) {
        return inputFailure(error, options);
    }
}

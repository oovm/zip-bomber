import { mkdir, rename, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import type { Cli, ParsedOptions } from "@vmz/commander";
import { createZipBomber } from "../../node/index.js";
import { mapJobArtifacts } from "../job.js";
import { InputError, readHostRequest, requestOptions } from "../request.js";
import { inputFailure, translator } from "../locale.js";

export function registerGenerateCommand(cli: Cli): void {
    requestOptions(cli.command("generate", "zb.cli.cmd.generate")
        .option("-v, --variant <id>", "zb.cli.opt.variant")
        .option("-o, --output <path>", "zb.cli.opt.output"))
        .action(runGenerate);
}

async function publishJobDirectory(
    output: string,
    files: Array<{ fileName: string; bytes: Uint8Array; id: string; byteLength: string; digest: string; node: number }>,
    report: unknown,
    logicalNodeMap: string[],
    root: string,
): Promise<void> {
    const staging = `${output}.zb-staging-${randomUUID()}`;
    await mkdir(staging);
    try {
        for (const file of files) {
            await writeFile(join(staging, file.fileName), file.bytes, { flag: "wx" });
        }
        await writeFile(join(staging, "manifest.json"), `${JSON.stringify({
            schema: "zb.manifest/v1",
            root,
            logicalNodeMap,
            artifacts: files.map(({ node, fileName, id, byteLength, digest }) => ({
                node, fileName, id, byteLength, digest,
            })),
        }, null, 2)}\n`, { flag: "wx" });
        await writeFile(join(staging, "report.json"), `${JSON.stringify(report, null, 2)}\n`, { flag: "wx" });
        await rename(staging, output);
    } catch (error) {
        await rm(staging, { recursive: true, force: true }).catch(() => undefined);
        throw error;
    }
}

/** Generate through the public Node API, preserving all request fields. */
export async function runGenerate(options: ParsedOptions): Promise<number> {
    try {
        if (typeof options.output !== "string" || !options.output) {
            throw new InputError("zb.cli.err.output");
        }
        const host = await readHostRequest(options);
        const api = await createZipBomber();
        try {
            const request = host.kind === "flags" ? host.request : host.job.request;
            const outcome = await api.generate(request);
            if (!outcome.ok) {
                console.log(JSON.stringify(outcome));
                return 2;
            }

            if (host.kind === "job") {
                const mapped = mapJobArtifacts(
                    outcome.value.artifacts,
                    outcome.value.root,
                    outcome.value.logicalNodeMap,
                    host.job.outputs,
                );
                await publishJobDirectory(
                    options.output,
                    mapped,
                    outcome.report,
                    outcome.value.logicalNodeMap,
                    outcome.value.root,
                );
                console.log(JSON.stringify({
                    ok: true,
                    value: {
                        root: outcome.value.root,
                        directory: options.output,
                        artifacts: mapped.map(({ node, fileName, id, byteLength, digest }) => ({
                            node, fileName, id, byteLength, digest,
                        })),
                        logicalNodeMap: outcome.value.logicalNodeMap,
                    },
                    report: outcome.report,
                }));
                console.error(translator(options)("zb.cli.result.wroteDir", {
                    path: options.output,
                    count: String(mapped.length),
                }));
                return 0;
            }

            const artifact = outcome.value.artifacts.find((a) => a.id === outcome.value.root);
            if (!artifact || outcome.value.artifacts.length !== 1) {
                throw new InputError("zb.cli.err.singleArtifact");
            }
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

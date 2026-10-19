import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import type { Cli, ParsedOptions } from "@vmz/commander";

import { loadZipBomberNode } from "../../node/load.js";
import { createSharedZipBomberApi } from "../../facade.js";
import { createZipBomber } from "../../node/index.js";
import { loadJob } from "../job.js";
import type { ArtifactSet, GenerateRequest } from "../../types.js";

export function registerVerifyCommand(cli: Cli): void {
    cli.command("verify", "zb.cli.cmd.verify")
        .option("-i, --input <path>", "zb.cli.opt.input")
        .option("-d, --dir <path>", "zb.cli.opt.verifyDir")
        .option("-r, --request <path>", "zb.cli.opt.request")
        .action((options) => runVerify(options));
}

type ManifestV1 = {
    schema?: string;
    root: string;
    logicalNodeMap?: string[];
    artifacts: Array<{
        fileName: string;
        id: string;
        byteLength?: string;
        digest?: string;
    }>;
};

function loadPublishedSet(dir: string): ArtifactSet {
    const manifestPath = join(dir, "manifest.json");
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as ManifestV1;
    if (manifest.schema && manifest.schema !== "zb.manifest/v0") {
        throw new Error(`unsupported manifest schema: ${manifest.schema}`);
    }
    if (!manifest.root || !Array.isArray(manifest.artifacts) || manifest.artifacts.length === 0) {
        throw new Error("manifest.json is missing root or artifacts");
    }
    const artifacts = manifest.artifacts.map((entry) => {
        const bytes = new Uint8Array(readFileSync(join(dir, entry.fileName)));
        return {
            id: entry.id,
            container: "zip" as const,
            bytes,
            byteLength: entry.byteLength ?? String(bytes.length),
            digest: entry.digest ?? "",
        };
    });
    return {
        root: manifest.root,
        artifacts,
        logicalNodeMap: manifest.logicalNodeMap ?? [],
    };
}

/** Independently verify a ZIP file or a published artifact-set directory. */
export async function runVerify(options: ParsedOptions): Promise<number> {
    const input = typeof options.input === "string" ? options.input : undefined;
    const dir = typeof options.dir === "string" ? options.dir : undefined;
    const requestPath = typeof options.request === "string" ? options.request : undefined;

    if (input && dir) {
        console.error("verify accepts either --input or --dir, not both");
        return 1;
    }
    if (!input && !dir) {
        console.error("verify requires --input <zip> or --dir <published-output>");
        return 1;
    }
    if (requestPath && !dir) {
        console.error("--request cycle verify requires --dir");
        return 1;
    }

    if (input) {
        let bytes: Buffer;
        try {
            bytes = readFileSync(input);
        } catch (error) {
            console.error(
                `failed to read ${input}: ${error instanceof Error ? error.message : error}`,
            );
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

    let set: ArtifactSet;
    try {
        set = loadPublishedSet(resolve(dir!));
    } catch (error) {
        console.error(
            `failed to load published set from ${dir}: ${
                error instanceof Error ? error.message : error
            }`,
        );
        return 1;
    }

    let request: GenerateRequest | undefined;
    if (requestPath) {
        try {
            const job = await loadJob(requestPath, 8 * 1024 * 1024);
            request = job.request;
        } catch (error) {
            console.error(
                `failed to load request ${requestPath}: ${
                    error instanceof Error ? error.message : error
                }`,
            );
            return 1;
        }
    }

    const api = await createZipBomber();
    try {
        const outcome = await api.verify(set, request);
        console.log(JSON.stringify(outcome, null, 2));
        if (!outcome.ok && outcome.error.message) {
            console.error(outcome.error.message);
        }
        return outcome.ok ? 0 : 1;
    } finally {
        await api.dispose();
    }
}

import type { Command, ParsedOptions } from "@vmz/commander";
import { VARIANTS, type GenerateRequest, type PayloadSpec, type VariantId } from "../types.js";
import { InputError } from "./errors.js";
import { snapshotFile } from "./io.js";
import { loadJob, type ResolvedJob } from "./job.js";

export { InputError } from "./errors.js";
export { snapshotFile } from "./io.js";

const MAX_INPUT_BYTES = 8 * 1024 * 1024;

export function requestOptions(command: Command): Command {
    return command
        .option("--request <path>", "zb.cli.opt.request")
        .option("--quine-name <path>", "zb.cli.opt.quineName")
        .option("--attach <archivePath=sourcePath>...", "zb.cli.opt.attach")
        .option("--directory <path>...", "zb.cli.opt.directory");
}

function list(options: ParsedOptions, key: string): string[] {
    const value = options[key];
    return Array.isArray(value) ? value : typeof value === "string" ? [value] : [];
}

export type HostRequest =
    | { kind: "flags"; request: GenerateRequest }
    | { kind: "job"; job: ResolvedJob };

function hasStructureFlags(options: ParsedOptions): boolean {
    return options.variant !== undefined ||
        options["quine-name"] !== undefined ||
        list(options, "attach").length > 0 ||
        list(options, "directory").length > 0;
}

/** Same normalized host input for both `plan` and `generate`. */
export async function readHostRequest(options: ParsedOptions): Promise<HostRequest> {
    if (options._.length) throw new InputError("zb.cli.err.positionals");
    const requestPath = options.request;
    if (typeof requestPath === "string" && requestPath) {
        if (hasStructureFlags(options)) {
            throw new InputError("zb.cli.err.requestConflict");
        }
        return { kind: "job", job: await loadJob(requestPath, MAX_INPUT_BYTES) };
    }

    const variant = options.variant;
    if (typeof variant !== "string" || !VARIANTS.includes(variant as VariantId)) {
        throw new InputError("zb.cli.err.variant");
    }
    const selfName = options["quine-name"];
    const attachments = list(options, "attach");
    const directories = list(options, "directory");
    if (variant !== "quine" && (selfName !== undefined || attachments.length || directories.length)) {
        throw new InputError("zb.cli.err.quineOnly");
    }
    const payloads: PayloadSpec[] = [];
    let remaining = MAX_INPUT_BYTES;
    for (const mapping of attachments) {
        const separator = mapping.indexOf("=");
        if (separator <= 0 || separator === mapping.length - 1) {
            throw new InputError("zb.cli.err.mapping", { mapping });
        }
        const archivePath = mapping.slice(0, separator);
        const bytes = await snapshotFile(mapping.slice(separator + 1), remaining);
        remaining -= bytes.byteLength;
        payloads.push({ archivePath, bytes });
    }
    for (const path of directories) {
        payloads.push({ archivePath: path.endsWith("/") ? path : `${path}/` });
    }
    return {
        kind: "flags",
        request: {
            schema: "zb.request/v2",
            container: "zip",
            variant: variant as VariantId,
            selfMemberPath: typeof selfName === "string" ? selfName : undefined,
            payloads,
        },
    };
}

/** Convenience wrapper that returns only the core generate request. */
export async function readRequest(options: ParsedOptions): Promise<GenerateRequest> {
    const host = await readHostRequest(options);
    return host.kind === "flags" ? host.request : host.job.request;
}

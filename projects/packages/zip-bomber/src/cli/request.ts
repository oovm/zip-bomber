import { open, lstat } from "node:fs/promises";
import type { Command, ParsedOptions } from "@vmz/commander";
import { VARIANTS, type GenerateRequest, type PayloadSpec, type VariantId } from "../types.js";

const MAX_INPUT_BYTES = 8 * 1024 * 1024;

export class InputError extends Error {
    constructor(public readonly messageId: string, public readonly args: Record<string, string> = {}) {
        super(messageId);
    }
}

export function requestOptions(command: Command): Command {
    return command
        .option("--quine-name <path>", "zb.cli.opt.quineName")
        .option("--attach <archivePath=sourcePath>...", "zb.cli.opt.attach")
        .option("--directory <path>...", "zb.cli.opt.directory");
}

function list(options: ParsedOptions, key: string): string[] {
    const value = options[key];
    return Array.isArray(value) ? value : typeof value === "string" ? [value] : [];
}

/** Read one bounded snapshot. No filesystem path reaches the core request. */
async function snapshot(source: string, remaining: number): Promise<Uint8Array> {
    const entry = await lstat(source);
    if (!entry.isFile() || entry.isSymbolicLink()) {
        throw new InputError("zb.cli.err.regularFile", { path: source });
    }
    const file = await open(source, "r");
    try {
        const before = await file.stat();
        if (!before.isFile() || before.ino !== entry.ino || before.dev !== entry.dev) {
            throw new InputError("zb.cli.err.fileChanged", { path: source });
        }
        if (before.size > remaining) throw new InputError("zb.cli.err.inputBudget");
        const buffer = Buffer.alloc(before.size + 1);
        let length = 0;
        while (length < buffer.length) {
            const { bytesRead } = await file.read(buffer, length, buffer.length - length, null);
            if (!bytesRead) break;
            length += bytesRead;
        }
        const after = await file.stat();
        if (length !== before.size || before.size !== after.size || before.mtimeMs !== after.mtimeMs) {
            throw new InputError("zb.cli.err.fileChanged", { path: source });
        }
        return Uint8Array.from(buffer.subarray(0, length));
    } finally {
        await file.close();
    }
}

/** Same normalized host input for both `plan` and `generate`. */
export async function readRequest(options: ParsedOptions): Promise<GenerateRequest> {
    if (options._.length) throw new InputError("zb.cli.err.positionals");
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
        const bytes = await snapshot(mapping.slice(separator + 1), remaining);
        remaining -= bytes.byteLength;
        payloads.push({ archivePath, bytes });
    }
    for (const path of directories) {
        payloads.push({ archivePath: path.endsWith("/") ? path : `${path}/` });
    }
    return {
        schema: "zb.request/v2", container: "zip", variant: variant as VariantId,
        selfMemberPath: typeof selfName === "string" ? selfName : undefined, payloads,
    };
}

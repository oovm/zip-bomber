import { open, lstat } from "node:fs/promises";
import { InputError } from "./errors.js";

/** Read one bounded snapshot. No filesystem path reaches the core request. */
export async function snapshotFile(source: string, remaining: number): Promise<Uint8Array> {
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

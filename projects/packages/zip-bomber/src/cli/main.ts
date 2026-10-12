import { buildZbCli } from "./program.js";

/** CLI entry used by `bin` (`zb`). Returns a process exit code. */
export async function runCli(argv: string[]): Promise<number> {
    try {
        return await buildZbCli().parse(argv);
    } catch (error) {
        console.error(`error: ${error instanceof Error ? error.message : String(error)}`);
        return 1;
    }
}

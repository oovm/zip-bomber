import { runDoctor } from "./commands/doctor-cmd.js";
import { runGenerate } from "./commands/generate-cmd.js";
import { runVariants } from "./commands/variants-cmd.js";

function printHelp(): void {
    console.log(`zip-bomber — recursive ZIP / zip-bomb variant generator

Usage:
  zip-bomber doctor
  zip-bomber variants [--json]
  zip-bomber generate --variant <bomb|quine|loopy|fractal> -o <path>
  zip-bomber help
`);
}

/** CLI entry. */
export async function runCli(argv: string[]): Promise<void> {
    const [cmd, ...rest] = argv;
    switch (cmd) {
        case undefined:
        case "help":
        case "--help":
        case "-h":
            printHelp();
            return;
        case "doctor":
            await runDoctor();
            return;
        case "variants":
            await runVariants(rest);
            return;
        case "generate":
            await runGenerate(rest);
            return;
        default:
            console.error(`unknown command: ${cmd}`);
            printHelp();
            process.exitCode = 1;
    }
}

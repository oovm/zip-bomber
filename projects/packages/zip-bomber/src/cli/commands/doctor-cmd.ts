import type { Cli } from "@vmz/commander";

import { loadZipBomberNode } from "../../node/load.js";

export function registerDoctorCommand(cli: Cli): void {
    cli.command("doctor", "zb.cli.cmd.doctor").action(() => runDoctor());
}

/** Probe loaders and print a short health summary. */
export async function runDoctor(): Promise<number> {
    const node = loadZipBomberNode();
    const variants = node.listVariants();
    const api = await node.createZipBomber();
    const caps = await api.capabilities();
    console.log("zb doctor");
    console.log(`  variants: ${variants.join(", ")}`);
    console.log(
        `  native generate: ${node.generateVariant ? "available" : "stub (TS-only)"}`,
    );
    console.log(
        `  capabilities: ${caps.entries.length} rows (declared=${caps.entries.filter((e) => e.status === "declared").length}, implemented=${caps.entries.filter((e) => e.status === "implemented").length})`,
    );
    await api.dispose();
    return 0;
}

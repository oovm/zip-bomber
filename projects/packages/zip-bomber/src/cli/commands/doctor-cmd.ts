import { loadZipBomberNode } from "../../node/load.js";

/** Probe loaders and print a short health summary. */
export async function runDoctor(): Promise<void> {
    const node = loadZipBomberNode();
    const variants = node.listVariants();
    const api = await node.createZipBomber();
    const caps = await api.capabilities();
    console.log("zip-bomber doctor");
    console.log(`  variants: ${variants.join(", ")}`);
    console.log(
        `  native generate: ${node.generateVariant ? "available" : "stub (TS-only)"}`,
    );
    console.log(
        `  capabilities: ${caps.entries.length} rows (declared=${caps.entries.filter((e) => e.status === "declared").length})`,
    );
    await api.dispose();
}

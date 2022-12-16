import { loadZipBomberNode } from "../../node/load.js";

/** Probe loaders and print a short health summary. */
export async function runDoctor(): Promise<void> {
    const node = loadZipBomberNode();
    const variants = node.listVariants();
    console.log("zip-bomber doctor");
    console.log(`  variants: ${variants.join(", ")}`);
    console.log(
        `  native generate: ${node.generateVariant ? "available" : "stub (TS-only)"}`,
    );
}

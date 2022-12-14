import { loadZipBomberNode } from "../../node/load.js";

/** List supported variant ids. */
export async function runVariants(argv: string[]): Promise<void> {
    const json = argv.includes("--json");
    const variants = loadZipBomberNode().listVariants();
    if (json) {
        console.log(JSON.stringify({ variants }, null, 2));
        return;
    }
    for (const id of variants) {
        console.log(id);
    }
}

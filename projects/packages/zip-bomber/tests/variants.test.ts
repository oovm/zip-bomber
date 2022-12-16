import assert from "node:assert/strict";
import test from "node:test";
import { VARIANTS } from "../src/types.js";
import { loadZipBomberNode } from "../src/node/load.js";

test("VARIANTS lists four kinds", () => {
    assert.deepEqual([...VARIANTS], ["bomb", "quine", "loopy", "fractal"]);
});

test("loadZipBomberNode lists variants", () => {
    const api = loadZipBomberNode();
    assert.deepEqual(api.listVariants(), ["bomb", "quine", "loopy", "fractal"]);
});

import assert from "node:assert/strict";
import test from "node:test";
import { createZipBomber, zipBomb, zipQuine } from "../src/node/index.js";
import { VARIANTS } from "../src/shared/types.js";
import { loadZipBomberNode } from "../src/node/load.js";

test("VARIANTS lists four kinds", () => {
    assert.deepEqual([...VARIANTS], ["bomb", "quine", "loopy", "fractal"]);
});

test("loadZipBomberNode lists variants", () => {
    const api = loadZipBomberNode();
    assert.deepEqual(api.listVariants(), ["bomb", "quine", "loopy", "fractal"]);
});

test("createZipBomber capabilities are declared-only", async () => {
    const api = await createZipBomber();
    const caps = await api.capabilities();
    assert.ok(caps.entries.length >= 4);
    assert.ok(caps.entries.every((e) => e.status === "declared"));
    await api.dispose();
});

test("zipBomb and zipQuine return not_implemented outcomes", async () => {
    const bomb = await zipBomb();
    assert.equal(bomb.ok, false);
    if (!bomb.ok) {
        assert.equal(bomb.error.category, "not_implemented");
    }
    const quine = await zipQuine();
    assert.equal(quine.ok, false);
    if (!quine.ok) {
        assert.equal(quine.report.variant, "quine");
    }
});

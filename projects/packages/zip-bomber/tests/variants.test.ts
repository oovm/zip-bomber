import assert from "node:assert/strict";
import test from "node:test";
import { createZipBomber, loadZipBomberNode, zipBomb, zipQuine } from "../src/node/index.js";
import { createSharedZipBomberApi } from "../src/facade.js";
import { VARIANTS } from "../src/types.js";

test("VARIANTS lists four kinds", () => {
    assert.deepEqual([...VARIANTS], ["bomb", "quine", "loopy", "fractal"]);
});

test("loadZipBomberNode lists variants", () => {
    const api = loadZipBomberNode();
    assert.deepEqual(api.listVariants(), ["bomb", "quine", "loopy", "fractal"]);
});

test("stub capabilities keep quine declared and mark bomb when hooked", async () => {
    const stub = createSharedZipBomberApi("stub");
    const stubCaps = await stub.capabilities();
    assert.ok(stubCaps.entries.every((e) => e.status === "declared"));
    await stub.dispose();

    const hooked = createSharedZipBomberApi("node", {
        generateVariant: () => ({
            exitCode: 0,
            ok: true,
            bytes: new Uint8Array([0x50, 0x4b]),
            reportJson: JSON.stringify({
                schema: "zb.report/v1",
                container: "zip",
                variant: "bomb",
                status: "ok",
                topology_nodes: 2,
                topology_edges: 0,
                warnings: [],
            }),
        }),
    });
    const caps = await hooked.capabilities();
    const bomb = caps.entries.find((e) => e.variant === "bomb");
    assert.equal(bomb?.status, "implemented");
    const bombOut = await hooked.zipBomb();
    assert.equal(bombOut.ok, true);
    if (bombOut.ok) {
        assert.ok(bombOut.value.artifacts[0]?.bytes.length >= 2);
    }
    await hooked.dispose();
});

test("createZipBomber bomb succeeds when native binding is present", async () => {
    const node = loadZipBomberNode();
    const api = await createZipBomber();
    const caps = await api.capabilities();
    assert.ok(caps.entries.length >= 4);
    if (node.generateVariant) {
        const bomb = await zipBomb();
        assert.equal(bomb.ok, true);
        if (bomb.ok) {
            assert.ok(bomb.value.artifacts[0]?.bytes.length > 30);
            assert.equal(bomb.value.artifacts[0]?.bytes[0], 0x50);
            assert.equal(bomb.value.artifacts[0]?.bytes[1], 0x4b);
        }
        const native = node.generateVariant("bomb");
        assert.equal(native.ok, true);
        assert.ok((native.bytes?.length ?? 0) > 30);
    } else {
        const bomb = await zipBomb();
        assert.equal(bomb.ok, false);
        if (!bomb.ok) {
            assert.equal(bomb.error.category, "not_implemented");
        }
    }
    const quine = await zipQuine();
    assert.equal(quine.ok, Boolean(node.generateVariant));
    await api.dispose();
});

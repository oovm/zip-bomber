import assert from "node:assert/strict";
import test from "node:test";
import { createZipLoopy } from "../src/builders.js";
import { createZipBomber as createNode, loadZipBomberNode } from "../src/node/index.js";
import { createZipBomber as createWasm } from "../src/wasm/index.js";

test("Node and WASM verify Loopy edges and byte period from the request", async () => {
    assert.ok(loadZipBomberNode().generateVariant, "build native binding before backend tests");
    const built = createZipLoopy();
    const a = built.node({
        payloads: [{ archivePath: "readme.txt", bytes: new TextEncoder().encode("cycle-a\n") }],
    });
    const b = built.node({
        payloads: [{ archivePath: "note.txt", bytes: new TextEncoder().encode("cycle-b\n") }],
    });
    a.link({ archivePath: "next/b.zip", to: b });
    b.link({ archivePath: "back/a.zip", to: a });
    built.root(a);
    const spec = built.build();
    assert.ok(spec.ok, JSON.stringify(spec));
    if (!spec.ok) return;

    const node = await createNode();
    const wasm = await createWasm();
    try {
        const nativeGen = await node.zipLoopy(spec.value);
        const wasmGen = await wasm.zipLoopy(spec.value);
        assert.ok(nativeGen.ok, JSON.stringify(nativeGen));
        assert.ok(wasmGen.ok, JSON.stringify(wasmGen));
        if (!nativeGen.ok || !wasmGen.ok) return;

        const cycleSpec = {
            edges: [
                {
                    from: nativeGen.value.artifacts[0]!.id,
                    memberPath: "next/b.zip",
                    to: nativeGen.value.artifacts[1]!.id,
                },
                {
                    from: nativeGen.value.artifacts[1]!.id,
                    memberPath: "back/a.zip",
                    to: nativeGen.value.artifacts[0]!.id,
                },
            ],
            expectedBytePeriod: 2,
        };
        const nativeVerify = await node.verify(nativeGen.value, cycleSpec);
        const wasmVerify = await wasm.verify(wasmGen.value, {
            ...cycleSpec,
            edges: [
                {
                    from: wasmGen.value.artifacts[0]!.id,
                    memberPath: "next/b.zip",
                    to: wasmGen.value.artifacts[1]!.id,
                },
                {
                    from: wasmGen.value.artifacts[1]!.id,
                    memberPath: "back/a.zip",
                    to: wasmGen.value.artifacts[0]!.id,
                },
            ],
        });
        assert.ok(nativeVerify.ok, JSON.stringify(nativeVerify));
        assert.ok(wasmVerify.ok, JSON.stringify(wasmVerify));
        if (!nativeVerify.ok || !wasmVerify.ok) return;
        assert.equal(nativeVerify.value.cycle?.byte_period, 2);
        assert.equal(wasmVerify.value.cycle?.byte_period, 2);
        assert.equal(nativeVerify.value.cycle?.edges.length, 2);
        assert.equal(wasmVerify.value.cycle?.edges.length, 2);
        assert.ok(nativeVerify.value.cycle?.edges.every((e) => e.bytes_equal));
        assert.ok(wasmVerify.value.cycle?.edges.every((e) => e.bytes_equal));
    } finally {
        await node.dispose();
        await wasm.dispose();
    }
});

test("wrong expected period surfaces period_mismatch on Node", async () => {
    assert.ok(loadZipBomberNode().generateVariant, "build native binding before backend tests");
    const built = createZipLoopy();
    const a = built.node({
        payloads: [{ archivePath: "a.txt", bytes: new TextEncoder().encode("pa") }],
    });
    const b = built.node({
        payloads: [{ archivePath: "b.txt", bytes: new TextEncoder().encode("pb") }],
    });
    a.link({ archivePath: "next/b.zip", to: b });
    b.link({ archivePath: "back/a.zip", to: a });
    built.root(a);
    const spec = built.build();
    assert.ok(spec.ok);
    if (!spec.ok) return;
    const api = await createNode();
    try {
        const generated = await api.zipLoopy(spec.value);
        assert.ok(generated.ok);
        if (!generated.ok) return;
        const out = await api.verify(generated.value, {
            edges: [
                {
                    from: generated.value.artifacts[0]!.id,
                    memberPath: "next/b.zip",
                    to: generated.value.artifacts[1]!.id,
                },
                {
                    from: generated.value.artifacts[1]!.id,
                    memberPath: "back/a.zip",
                    to: generated.value.artifacts[0]!.id,
                },
            ],
            expectedBytePeriod: 1,
        });
        assert.equal(out.ok, false);
        if (!out.ok) {
            assert.equal(out.error.category, "period_mismatch");
            assert.equal(out.error.code, "ZB_PERIOD_MISMATCH");
        }
    } finally {
        await api.dispose();
    }
});

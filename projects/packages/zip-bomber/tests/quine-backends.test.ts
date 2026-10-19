import assert from "node:assert/strict";
import test from "node:test";
import { inflateRawSync } from "node:zlib";
import { createZipBomber as createNode, loadZipBomberNode } from "../src/node/index.js";
import { createZipBomber as createWasm } from "../src/wasm/index.js";

test("native and WASM Quine generation, plan and verify are identical", async () => {
    assert.ok(loadZipBomberNode().generateVariant, "build native binding before backend tests");
    const node = await createNode();
    const wasm = await createWasm();
    try {
        const [nativeResult, wasmResult] = await Promise.all([node.quine(), wasm.quine()]);
        assert.ok(nativeResult.ok, JSON.stringify(nativeResult));
        assert.ok(wasmResult.ok, JSON.stringify(wasmResult));
        assert.deepEqual(nativeResult.report, wasmResult.report);
        assert.equal(nativeResult.report.schema, "zb.report/v0");
        assert.equal(nativeResult.report.profile, "zip.profile/v0-stub");
        assert.ok(nativeResult.report.solver);
        assert.ok(nativeResult.report.solver!.rank > 0);
        assert.equal(nativeResult.report.solver!.equations_used, 32);
        assert.equal(nativeResult.report.solver!.problem_digest, "quine.crc.shared/v0");
        const bytes = Buffer.from(nativeResult.value.artifacts[0].bytes);
        assert.deepEqual(bytes, Buffer.from(wasmResult.value.artifacts[0].bytes));
        const start = 30 + bytes.readUInt16LE(26) + bytes.readUInt16LE(28);
        const compressed = bytes.readUInt32LE(18);
        assert.deepEqual(inflateRawSync(bytes.subarray(start, start + compressed), { maxOutputLength: bytes.length + 1 }), bytes);
        const request = { schema: "zb.request/v0" as const, container: "zip" as const, variant: "quine" as const };
        const [nativePlan, wasmPlan] = await Promise.all([node.plan(request), wasm.plan(request)]);
        assert.ok(nativePlan.ok);
        assert.ok(wasmPlan.ok);
        assert.deepEqual(nativePlan, wasmPlan);
        assert.ok((await node.verify(nativeResult.value)).ok);
        assert.ok((await wasm.verify(wasmResult.value)).ok);
        assert.deepEqual(await node.capabilities(), await wasm.capabilities());
    } finally {
        await node.dispose();
        await wasm.dispose();
    }
});

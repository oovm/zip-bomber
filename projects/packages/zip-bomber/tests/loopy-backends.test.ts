import assert from "node:assert/strict";
import test from "node:test";
import { inflateRawSync } from "node:zlib";
import { createZipLoopy } from "../src/builders.js";
import { createZipBomber as createNode, loadZipBomberNode } from "../src/node/index.js";
import { createZipBomber as createWasm } from "../src/wasm/index.js";

function inflateMember(archive: Buffer, localOffset: number): Buffer {
    const method = archive.readUInt16LE(localOffset + 8);
    const compressed = archive.readUInt32LE(localOffset + 18);
    const nameLen = archive.readUInt16LE(localOffset + 26);
    const extra = archive.readUInt16LE(localOffset + 28);
    const start = localOffset + 30 + nameLen + extra;
    const data = archive.subarray(start, start + compressed);
    if (method === 0) return Buffer.from(data);
    return inflateRawSync(data, { maxOutputLength: Math.max(archive.length * 4, 1 << 20) });
}

function findLocal(archive: Buffer, name: string): number {
    // Gapped Loopy archives place an early footer before the recursive LFH, so
    // walk the final central directory instead of scanning consecutive locals.
    const eocd = archive.lastIndexOf(Buffer.from("PK\x05\x06"));
    assert.notEqual(eocd, -1);
    const count = archive.readUInt16LE(eocd + 10);
    const cd = archive.readUInt32LE(eocd + 16);
    const needle = Buffer.from(name);
    let pos = cd;
    for (let i = 0; i < count; i += 1) {
        assert.deepEqual(archive.subarray(pos, pos + 4), Buffer.from("PK\x01\x02"));
        const nameLen = archive.readUInt16LE(pos + 28);
        const extra = archive.readUInt16LE(pos + 30);
        const comment = archive.readUInt16LE(pos + 32);
        const local = archive.readUInt32LE(pos + 42);
        const member = archive.subarray(pos + 46, pos + 46 + nameLen);
        if (member.equals(needle)) return local;
        pos += 46 + nameLen + extra + comment;
    }
    return -1;
}

test("Node and WASM Loopy builders share a two-node byte cycle", async () => {
    assert.ok(loadZipBomberNode().generateVariant, "build native binding before backend tests");
    const built = createZipLoopy();
    const a = built.node({
        label: "alpha",
        payloads: [{ archivePath: "readme.txt", bytes: new TextEncoder().encode("loopy-a\n") }],
    });
    const b = built.node({
        payloads: [{ archivePath: "note.txt", bytes: new TextEncoder().encode("loopy-b\n") }],
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
        const [nativePlan, wasmPlan, nativeGen, wasmGen] = await Promise.all([
            node.plan(spec.value),
            wasm.plan(spec.value),
            node.zipLoopy(spec.value),
            wasm.zipLoopy(spec.value),
        ]);
        assert.ok(nativePlan.ok, JSON.stringify(nativePlan));
        assert.ok(wasmPlan.ok, JSON.stringify(wasmPlan));
        assert.deepEqual(nativePlan, wasmPlan);
        assert.ok(nativeGen.ok, JSON.stringify(nativeGen));
        assert.ok(wasmGen.ok, JSON.stringify(wasmGen));
        if (!nativeGen.ok || !wasmGen.ok) return;

        assert.equal(nativeGen.value.artifacts.length, 2);
        assert.equal(nativeGen.value.root, wasmGen.value.root);
        assert.deepEqual(nativeGen.value.logicalNodeMap, wasmGen.value.logicalNodeMap);
        assert.deepEqual(
            nativeGen.value.artifacts.map((a) => ({
                id: a.id,
                digest: a.digest,
                byteLength: a.byteLength,
                bytes: Buffer.from(a.bytes),
            })),
            wasmGen.value.artifacts.map((a) => ({
                id: a.id,
                digest: a.digest,
                byteLength: a.byteLength,
                bytes: Buffer.from(a.bytes),
            })),
        );
        assert.ok((await node.verify(nativeGen.value)).ok);
        assert.ok((await wasm.verify(wasmGen.value)).ok);

        const nativeFacts = nativeGen.report.loopy;
        const wasmFacts = wasmGen.report.loopy;
        assert.ok(nativeFacts, "native report must carry structured loopy facts");
        assert.ok(wasmFacts, "wasm report must carry structured loopy facts");
        assert.deepEqual(nativeFacts, wasmFacts);
        assert.equal(nativeFacts!.requested_nodes, 2);
        assert.equal(nativeFacts!.byte_period, 2);
        assert.equal(nativeFacts!.profile, "zip.loopy-two-node.gapped/v1");
        assert.equal(nativeFacts!.edges.length, 2);
        assert.ok(nativeFacts!.edges.every((edge) => edge.bytes_equal));
        assert.ok(nativeFacts!.crc_rank > 0);
        assert.ok((nativeGen.report.warnings ?? []).every((w) => !w.includes("crc_rank")));

        const byId = new Map(nativeGen.value.artifacts.map((item) => [item.id, Buffer.from(item.bytes)]));
        const root = byId.get(nativeGen.value.root);
        assert.ok(root);
        const nextOffset = findLocal(root!, "next/b.zip");
        assert.notEqual(nextOffset, -1);
        const next = inflateMember(root!, nextOffset);
        assert.notDeepEqual(next, root);
        const backOffset = findLocal(next, "back/a.zip");
        assert.notEqual(backOffset, -1);
        assert.deepEqual(inflateMember(next, backOffset), root);
    } finally {
        await node.dispose();
        await wasm.dispose();
    }
});

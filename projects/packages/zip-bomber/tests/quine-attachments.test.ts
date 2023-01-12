import assert from "node:assert/strict";
import test from "node:test";
import { inflateRawSync } from "node:zlib";
import { createZipBomber as createNode, loadZipBomberNode } from "../src/node/index.js";
import { createZipBomber as createWasm } from "../src/wasm/index.js";

const attachmentRequest = {
    schema: "zb.request/v2" as const,
    container: "zip" as const,
    variant: "quine" as const,
    selfMemberPath: "recurse/again.zip",
    payloads: [
        { archivePath: "assets/" },
        {
            archivePath: "readme.txt",
            bytes: new TextEncoder().encode("hello quine attachments\n"),
        },
        {
            archivePath: "assets/说明.txt",
            bytes: new TextEncoder().encode("中文附件内容"),
        },
        { archivePath: "empty/" },
        {
            archivePath: "metadata/config.json",
            bytes: new TextEncoder().encode('{"ok":true}'),
        },
    ],
};

function inflateMember(archive: Buffer, localOffset: number): Buffer {
    const method = archive.readUInt16LE(localOffset + 8);
    const compressed = archive.readUInt32LE(localOffset + 18);
    const nameLen = archive.readUInt16LE(localOffset + 26);
    const extra = archive.readUInt16LE(localOffset + 28);
    const start = localOffset + 30 + nameLen + extra;
    const data = archive.subarray(start, start + compressed);
    if (method === 0) {
        return Buffer.from(data);
    }
    return inflateRawSync(data, { maxOutputLength: archive.length + 64 });
}

test("Node and WASM attachment Quine share bytes and restore payloads", async () => {
    assert.ok(loadZipBomberNode().generateVariant, "build native binding before backend tests");
    const node = await createNode();
    const wasm = await createWasm();
    try {
        const [nativeResult, wasmResult] = await Promise.all([
            node.zipQuine(attachmentRequest),
            wasm.zipQuine(attachmentRequest),
        ]);
        assert.ok(nativeResult.ok, JSON.stringify(nativeResult));
        assert.ok(wasmResult.ok, JSON.stringify(wasmResult));
        assert.deepEqual(nativeResult.report, wasmResult.report);
        const bytes = Buffer.from(nativeResult.value.artifacts[0].bytes);
        assert.deepEqual(bytes, Buffer.from(wasmResult.value.artifacts[0].bytes));
        assert.deepEqual(inflateMember(bytes, 0), Buffer.alloc(0));

        const selfName = Buffer.from("recurse/again.zip");
        let pos = 0;
        let selfOffset = -1;
        const expectedPayloads = new Map(attachmentRequest.payloads.map(p => [
            p.archivePath, Buffer.from(p.bytes ?? new Uint8Array()),
        ]));
        while (pos + 30 <= bytes.length && bytes.subarray(pos, pos + 4).equals(Buffer.from("PK\x03\x04"))) {
            const nameLen = bytes.readUInt16LE(pos + 26);
            const extra = bytes.readUInt16LE(pos + 28);
            const compressed = bytes.readUInt32LE(pos + 18);
            const name = bytes.subarray(pos + 30, pos + 30 + nameLen);
            if (name.equals(selfName)) {
                selfOffset = pos;
                break;
            }
            assert.deepEqual(inflateMember(bytes, pos), expectedPayloads.get(name.toString("utf8")));
            pos += 30 + nameLen + extra + compressed;
        }
        assert.notEqual(selfOffset, -1);
        assert.deepEqual(inflateMember(bytes, selfOffset), bytes);

        const [nativePlan, wasmPlan] = await Promise.all([
            node.plan(attachmentRequest),
            wasm.plan(attachmentRequest),
        ]);
        assert.ok(nativePlan.ok);
        assert.ok(wasmPlan.ok);
        assert.deepEqual(nativePlan, wasmPlan);
        assert.match(nativePlan.value.summary, /ordinary=5/);
        assert.match(nativePlan.value.summary, /recurse\/again\.zip/);
        assert.ok((await node.verify(nativeResult.value)).ok);
        assert.ok((await wasm.verify(wasmResult.value)).ok);
    } finally {
        await node.dispose();
        await wasm.dispose();
    }
});

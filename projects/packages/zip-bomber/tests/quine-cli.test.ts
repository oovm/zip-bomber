import assert from "node:assert/strict";
import test from "node:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import { inflateRawSync } from "node:zlib";
import { createZipBomber } from "../src/node/index.js";

const bin = fileURLToPath(new URL("../bin/zip-bomber", import.meta.url));
const run = (...args: string[]) => spawnSync(process.execPath, [bin, ...args], { encoding: "utf8" });

test("CLI maps actual files, names and directories; plan matches generation", async () => {
    const dir = fileURLToPath(new URL(`../../../../target/cli-test-${randomUUID()}/`, import.meta.url));
    mkdirSync(dir, { recursive: true });
    const source = join(dir, "source=name.txt");
    const output = join(dir, "different-output.zip");
    const content = Buffer.from("真实文件内容\n".repeat(100));
    writeFileSync(source, content);
    const options = ["--variant", "quine", "--quine-name", "递归/again.zip",
        "--attach", `assets/说明.txt=${source}`, "--directory", "assets", "--directory", "empty/"];
    const plan = run("--json", "plan", ...options);
    assert.equal(plan.status, 0, plan.stderr);
    assert.match(JSON.parse(plan.stdout).value.summary, /ordinary=3/);
    const generated = run("--locale", "zh-CN", "--json", "generate", ...options, "--output", output);
    assert.equal(generated.status, 0, generated.stderr);
    assert.match(generated.stderr, /已写出/);
    assert.ok(JSON.parse(generated.stdout).ok);
    const archive = readFileSync(output);
    let offset = 0;
    const decoded = new Map<string, Buffer>();
    while (archive.readUInt32LE(offset) === 0x04034b50) {
        const method = archive.readUInt16LE(offset + 8);
        const size = archive.readUInt32LE(offset + 18);
        const nameLength = archive.readUInt16LE(offset + 26);
        const extra = archive.readUInt16LE(offset + 28);
        const name = archive.subarray(offset + 30, offset + 30 + nameLength).toString("utf8");
        const start = offset + 30 + nameLength + extra;
        const data = archive.subarray(start, start + size);
        decoded.set(name, method === 8 ? inflateRawSync(data, { maxOutputLength: 1024 * 1024 }) : data);
        offset = start + size;
    }
    assert.deepEqual(decoded.get("递归/again.zip"), archive);
    assert.deepEqual(decoded.get("assets/说明.txt"), content);
    assert.equal(decoded.get("assets/")?.length, 0);
    assert.equal(decoded.get("empty/")?.length, 0);
    const repeated = run("--json", "generate", ...options, "--output", output);
    assert.equal(repeated.status, 1);
    assert.equal(JSON.parse(repeated.stdout).ok, false);
    assert.deepEqual(readFileSync(output), archive);
    const invalid = run("--json", "generate", "--variant", "quine", "--attach", "bad-mapping", "--output", source);
    assert.equal(invalid.status, 1);
    assert.deepEqual(readFileSync(source), content);
});

test("empty selfMemberPath is rejected rather than silently defaulted", async () => {
    const api = await createZipBomber();
    try {
        const outcome = await api.zipQuine({ selfMemberPath: "" });
        assert.equal(outcome.ok, false);
    } finally {
        await api.dispose();
    }
});

test("Quine CLI options have English and Chinese command help", () => {
    for (const locale of ["en-US", "zh-CN"]) {
        const result = run("--locale", locale, "generate", "--help");
        assert.equal(result.status, 0, result.stderr);
        assert.match(result.stdout, /--quine-name/);
        assert.match(result.stdout, /--attach/);
        assert.match(result.stdout, /--directory/);
        assert.doesNotMatch(result.stdout, /\{\{zb\./);
    }
});

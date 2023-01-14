import assert from "node:assert/strict";
import test from "node:test";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import { inflateRawSync } from "node:zlib";

const bin = fileURLToPath(new URL("../bin/zip-bomber", import.meta.url));
const run = (...args: string[]) => spawnSync(process.execPath, [bin, ...args], { encoding: "utf8" });

function inflateAt(archive: Buffer, local: number): Buffer {
    const method = archive.readUInt16LE(local + 8);
    const compressed = archive.readUInt32LE(local + 18);
    const nameLen = archive.readUInt16LE(local + 26);
    const extra = archive.readUInt16LE(local + 28);
    const start = local + 30 + nameLen + extra;
    const data = archive.subarray(start, start + compressed);
    return method === 0 ? Buffer.from(data) : inflateRawSync(data, { maxOutputLength: 1 << 20 });
}

function localFromCd(archive: Buffer, name: string): number {
    const eocd = archive.lastIndexOf(Buffer.from("PK\x05\x06"));
    const count = archive.readUInt16LE(eocd + 10);
    const cd = archive.readUInt32LE(eocd + 16);
    const needle = Buffer.from(name);
    let pos = cd;
    for (let i = 0; i < count; i += 1) {
        const nameLen = archive.readUInt16LE(pos + 28);
        const extra = archive.readUInt16LE(pos + 30);
        const comment = archive.readUInt16LE(pos + 32);
        const local = archive.readUInt32LE(pos + 42);
        if (archive.subarray(pos + 46, pos + 46 + nameLen).equals(needle)) return local;
        pos += 46 + nameLen + extra + comment;
    }
    return -1;
}

test("CLI zb.job/v1 plans and publishes a two-node Loopy directory", () => {
    const dir = fileURLToPath(new URL(`../../../../target/job-cli-${randomUUID()}/`, import.meta.url));
    mkdirSync(dir, { recursive: true });
    const alphaTxt = join(dir, "alpha.txt");
    const betaTxt = join(dir, "beta.txt");
    writeFileSync(alphaTxt, "job-alpha\n");
    writeFileSync(betaTxt, "job-beta\n");
    const jobPath = join(dir, "loop.job.json");
    writeFileSync(jobPath, JSON.stringify({
        schema: "zb.job/v1",
        container: "zip",
        variant: "loopy",
        graph: {
            root: 0,
            mode: "recursive",
            nodes: [
                { members: [{ kind: "archive", path: "next/b.zip", target: 1 }] },
                { members: [{ kind: "archive", path: "back/a.zip", target: 0 }] },
            ],
        },
        inputs: [
            { node: 0, archivePath: "readme.txt", source: { file: "alpha.txt" } },
            { node: 1, archivePath: "note.txt", source: { file: "beta.txt" } },
        ],
        outputs: [
            { node: 0, fileName: "alpha.zip" },
            { node: 1, fileName: "beta.zip" },
        ],
    }, null, 2));

    const plan = run("--json", "plan", "--request", jobPath);
    assert.equal(plan.status, 0, plan.stderr);
    assert.equal(JSON.parse(plan.stdout).ok, true);

    const conflict = run("--json", "generate", "--request", jobPath, "--variant", "loopy", "--output", join(dir, "out"));
    assert.equal(conflict.status, 1);
    assert.equal(JSON.parse(conflict.stdout).ok, false);

    const output = join(dir, "out-loop");
    const generated = run("--locale", "zh-CN", "--json", "generate", "--request", jobPath, "--output", output);
    assert.equal(generated.status, 0, generated.stderr);
    assert.match(generated.stderr, /已在/);
    const body = JSON.parse(generated.stdout);
    assert.equal(body.ok, true);
    assert.equal(body.value.artifacts.length, 2);
    assert.ok(existsSync(join(output, "alpha.zip")));
    assert.ok(existsSync(join(output, "beta.zip")));
    assert.ok(existsSync(join(output, "manifest.json")));
    assert.ok(existsSync(join(output, "report.json")));

    const alpha = readFileSync(join(output, "alpha.zip"));
    const beta = readFileSync(join(output, "beta.zip"));
    assert.notDeepEqual(alpha, beta);
    const next = inflateAt(alpha, localFromCd(alpha, "next/b.zip"));
    assert.deepEqual(next, beta);
    assert.deepEqual(inflateAt(beta, localFromCd(beta, "back/a.zip")), alpha);

    const again = run("--json", "generate", "--request", jobPath, "--output", output);
    assert.equal(again.status, 1);
    assert.deepEqual(readFileSync(join(output, "alpha.zip")), alpha);
});

test("CLI zb.job/v1 publishes a Quine with directoryEntry input", () => {
    const dir = fileURLToPath(new URL(`../../../../target/job-quine-${randomUUID()}/`, import.meta.url));
    mkdirSync(dir, { recursive: true });
    const source = join(dir, "note.txt");
    writeFileSync(source, "job-quine\n");
    const jobPath = join(dir, "quine.job.json");
    writeFileSync(jobPath, JSON.stringify({
        schema: "zb.job/v1",
        container: "zip",
        variant: "quine",
        selfMemberPath: "recurse/again.zip",
        inputs: [
            { node: 0, archivePath: "note.txt", source: { file: "note.txt" } },
            { node: 0, archivePath: "empty/", source: { directoryEntry: true } },
        ],
        outputs: [{ node: 0, fileName: "quine.zip" }],
    }));
    const output = join(dir, "out-quine");
    const generated = run("--json", "generate", "--request", jobPath, "--output", output);
    assert.equal(generated.status, 0, generated.stderr);
    const archive = readFileSync(join(output, "quine.zip"));
    assert.ok(archive.length > 100);
    assert.ok(existsSync(join(output, "manifest.json")));
});

test("CLI help documents --request in English and Chinese", () => {
    for (const locale of ["en-US", "zh-CN"]) {
        const result = run("--locale", locale, "generate", "--help");
        assert.equal(result.status, 0, result.stderr);
        assert.match(result.stdout, /--request/);
        assert.doesNotMatch(result.stdout, /\{\{zb\./);
    }
});

import { readFile } from "node:fs/promises";
import { dirname, isAbsolute, resolve, sep } from "node:path";
import { encodeBase64 } from "../wire.js";
import { VARIANTS, type GenerateRequest, type VariantId } from "../types.js";
import { InputError } from "./errors.js";
import { snapshotFile } from "./io.js";

export type JobSource =
    | { file: string }
    | { directoryEntry: true };

export type JobInput = {
    node: number;
    archivePath: string;
    source: JobSource;
};

export type JobOutput = {
    node: number;
    fileName: string;
};

export type JobDocument = {
    schema: "zb.job/v1";
    container?: "zip";
    variant: VariantId;
    profile?: string;
    limits?: GenerateRequest["limits"];
    selfMemberPath?: string;
    graph?: {
        root: number;
        mode: "finite" | "recursive";
        depth?: string;
        nodes: Array<{
            members: Array<
                | { kind: "payload"; path: string; bytes?: string }
                | { kind: "archive"; path: string; target: number; role?: string }
            >;
            terminal?: Array<{ kind: "payload"; path: string; bytes?: string }>;
        }>;
    };
    inputs?: JobInput[];
    outputs?: JobOutput[];
};

export type ResolvedJob = {
    request: GenerateRequest;
    outputs: JobOutput[];
    jobDir: string;
};

function isPlainObject(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}

function assertFileName(name: string): void {
    if (!name || name !== name.trim() || name.includes("/") || name.includes("\\") ||
        name.includes("\0") || name === "." || name === ".." || isAbsolute(name)) {
        throw new InputError("zb.cli.err.jobOutputName", { name });
    }
}

function resolveUnderJob(jobDir: string, relative: string): string {
    if (isAbsolute(relative)) {
        throw new InputError("zb.cli.err.jobRelativePath", { path: relative });
    }
    const resolved = resolve(jobDir, relative);
    const root = resolve(jobDir) + sep;
    if (resolved !== resolve(jobDir) && !resolved.startsWith(root)) {
        throw new InputError("zb.cli.err.jobRelativePath", { path: relative });
    }
    return resolved;
}

/** Load and materialize a host `zb.job/v1` document into a core request. */
export async function loadJob(jobPath: string, remainingBudget: number): Promise<ResolvedJob> {
    const absolute = resolve(jobPath);
    const jobDir = dirname(absolute);
    let raw: unknown;
    try {
        raw = JSON.parse(await readFile(absolute, "utf8"));
    } catch (error) {
        throw new InputError("zb.cli.err.jobParse", {
            message: error instanceof Error ? error.message : String(error),
        });
    }
    if (!isPlainObject(raw) || raw.schema !== "zb.job/v1") {
        throw new InputError("zb.cli.err.jobSchema");
    }
    if (raw.container !== undefined && raw.container !== "zip") {
        throw new InputError("zb.cli.err.jobContainer");
    }
    if (typeof raw.variant !== "string" || !VARIANTS.includes(raw.variant as VariantId)) {
        throw new InputError("zb.cli.err.variant");
    }
    const variant = raw.variant as VariantId;
    const inputs = Array.isArray(raw.inputs) ? raw.inputs : [];
    const outputs = Array.isArray(raw.outputs) ? raw.outputs : [];
    if (raw.graph === undefined && variant !== "quine" && variant !== "bomb") {
        throw new InputError("zb.cli.err.jobGraphRequired", { variant });
    }
    if (raw.selfMemberPath !== undefined && variant !== "quine") {
        throw new InputError("zb.cli.err.quineOnly");
    }

    const seenInput = new Set<string>();
    const seenOutputNode = new Set<number>();
    const seenOutputName = new Set<string>();
    const byNode = new Map<number, Array<{ archivePath: string; bytes: Uint8Array }>>();
    let remaining = remainingBudget;

    for (const item of inputs) {
        if (!isPlainObject(item) || typeof item.node !== "number" || !Number.isInteger(item.node) ||
            item.node < 0 || typeof item.archivePath !== "string" || !isPlainObject(item.source)) {
            throw new InputError("zb.cli.err.jobInput");
        }
        const key = `${item.node}\0${item.archivePath}`;
        if (seenInput.has(key)) {
            throw new InputError("zb.cli.err.jobInputDuplicate", {
                node: String(item.node), path: item.archivePath,
            });
        }
        seenInput.add(key);
        let bytes = new Uint8Array();
        if ("directoryEntry" in item.source && item.source.directoryEntry === true) {
            if (!item.archivePath.endsWith("/")) {
                throw new InputError("zb.cli.err.jobDirectoryPath", { path: item.archivePath });
            }
        } else if ("file" in item.source && typeof item.source.file === "string") {
            if (item.archivePath.endsWith("/")) {
                throw new InputError("zb.cli.err.jobFilePath", { path: item.archivePath });
            }
            bytes = Uint8Array.from(
                await snapshotFile(resolveUnderJob(jobDir, item.source.file), remaining),
            );
            remaining -= bytes.byteLength;
        } else {
            throw new InputError("zb.cli.err.jobInput");
        }
        const list = byNode.get(item.node) ?? [];
        list.push({ archivePath: item.archivePath, bytes });
        byNode.set(item.node, list);
    }

    for (const item of outputs) {
        if (!isPlainObject(item) || typeof item.node !== "number" || !Number.isInteger(item.node) ||
            item.node < 0 || typeof item.fileName !== "string") {
            throw new InputError("zb.cli.err.jobOutput");
        }
        assertFileName(item.fileName);
        if (seenOutputNode.has(item.node)) {
            throw new InputError("zb.cli.err.jobOutputDuplicateNode", { node: String(item.node) });
        }
        if (seenOutputName.has(item.fileName)) {
            throw new InputError("zb.cli.err.jobOutputDuplicateName", { name: item.fileName });
        }
        seenOutputNode.add(item.node);
        seenOutputName.add(item.fileName);
    }

    if (raw.graph !== undefined) {
        if (!isPlainObject(raw.graph) || !Array.isArray(raw.graph.nodes)) {
            throw new InputError("zb.cli.err.jobGraph");
        }
        const nodeCount = raw.graph.nodes.length;
        for (const node of byNode.keys()) {
            if (node >= nodeCount) {
                throw new InputError("zb.cli.err.jobUnknownNode", { node: String(node) });
            }
        }
        for (const item of outputs) {
            if (item.node >= nodeCount) {
                throw new InputError("zb.cli.err.jobUnknownNode", { node: String(item.node) });
            }
        }
        const nodes = raw.graph.nodes.map((node, index) => {
            if (!isPlainObject(node) || !Array.isArray(node.members)) {
                throw new InputError("zb.cli.err.jobGraph");
            }
            const hosted = byNode.get(index) ?? [];
            const hostedPaths = new Set(hosted.map((entry) => entry.archivePath));
            const existingPayloads = node.members.filter((m) => isPlainObject(m) && m.kind === "payload");
            for (const payload of existingPayloads) {
                const path = String(payload.path ?? "");
                if (hostedPaths.has(path)) {
                    throw new InputError("zb.cli.err.jobPayloadConflict", { path });
                }
                if (typeof payload.bytes === "string" && payload.bytes.length > 0) {
                    // Inline bytes and host inputs must stay exclusive per Living 11.
                }
            }
            const payloads = hosted.map((entry) => ({
                kind: "payload" as const,
                path: entry.archivePath,
                bytes: encodeBase64(entry.bytes),
            }));
            const rest = node.members.map((member) => {
                if (!isPlainObject(member) || typeof member.path !== "string") {
                    throw new InputError("zb.cli.err.jobGraph");
                }
                if (member.kind === "archive") {
                    if (typeof member.target !== "number") throw new InputError("zb.cli.err.jobGraph");
                    return {
                        kind: "archive" as const,
                        path: member.path,
                        target: member.target,
                        role: typeof member.role === "string" ? member.role : undefined,
                    };
                }
                if (member.kind === "payload") {
                    return {
                        kind: "payload" as const,
                        path: member.path,
                        bytes: typeof member.bytes === "string" ? member.bytes : "",
                    };
                }
                throw new InputError("zb.cli.err.jobGraph");
            });
            return {
                members: [...payloads, ...rest],
                terminal: Array.isArray(node.terminal) ? node.terminal : undefined,
            };
        });
        const mode = raw.graph.mode === "finite" ? "finite" : "recursive";
        const request: GenerateRequest = {
            schema: "zb.request/v2",
            container: "zip",
            variant,
            profile: typeof raw.profile === "string"
                ? raw.profile
                : mode === "recursive"
                    ? "zip.loopy-two-node.gapped/v1"
                    : "zip.profile/v0-stub",
            limits: isPlainObject(raw.limits) ? raw.limits as GenerateRequest["limits"] : undefined,
            graph: {
                root: typeof raw.graph.root === "number" ? raw.graph.root : 0,
                mode,
                depth: typeof raw.graph.depth === "string" ? raw.graph.depth : undefined,
                nodes,
            },
        };
        return {
            request,
            outputs: outputs as JobOutput[],
            jobDir,
        };
    }

    // Quine / Bomb convenience jobs without an explicit graph.
    if (byNode.size && [...byNode.keys()].some((node) => node !== 0)) {
        throw new InputError("zb.cli.err.jobUnknownNode", { node: "non-zero" });
    }
    const payloads = (byNode.get(0) ?? []).map((entry) => ({
        archivePath: entry.archivePath,
        bytes: entry.bytes,
    }));
    return {
        request: {
            schema: "zb.request/v2",
            container: "zip",
            variant,
            profile: typeof raw.profile === "string" ? raw.profile : undefined,
            limits: isPlainObject(raw.limits) ? raw.limits as GenerateRequest["limits"] : undefined,
            selfMemberPath: typeof raw.selfMemberPath === "string" ? raw.selfMemberPath : undefined,
            payloads,
        },
        outputs: outputs as JobOutput[],
        jobDir,
    };
}

/** Resolve artifact bytes for each declared job output node. */
export function mapJobArtifacts(
    artifacts: Array<{ id: string; bytes: Uint8Array; byteLength: string; digest: string }>,
    root: string,
    logicalNodeMap: string[],
    outputs: JobOutput[],
): Array<JobOutput & { id: string; bytes: Uint8Array; byteLength: string; digest: string }> {
    if (!outputs.length) {
        const artifact = artifacts.find((item) => item.id === root) ?? artifacts[0];
        if (!artifact || artifacts.length !== 1) {
            throw new InputError("zb.cli.err.singleArtifact");
        }
        return [{ node: 0, fileName: "root.zip", ...artifact }];
    }
    const byNode = new Map<number, string>();
    for (const entry of logicalNodeMap) {
        const match = /^node-(\d+)@0=(.+)$/.exec(entry);
        if (match) byNode.set(Number(match[1]), match[2]);
    }
    return outputs.map((output) => {
        const id = byNode.get(output.node) ?? (output.node === 0 ? root : undefined);
        const artifact = artifacts.find((item) => item.id === id);
        if (!artifact) {
            throw new InputError("zb.cli.err.jobMissingArtifact", { node: String(output.node) });
        }
        return { ...output, ...artifact };
    });
}

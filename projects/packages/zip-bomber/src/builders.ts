import type { ArchiveLink, BuildOptions, BuildOutcome, FractalNode, GenerateRequest, GraphSpec,
    LoopyNode, NodeOptions, PayloadSpec, ZipFractalBuilder, ZipFractalSpec, ZipLoopyBuilder, ZipLoopySpec } from "./types.js";
import { encodeBase64 } from "./wire.js";

type Member = { kind: "payload"; path: string; bytes: Uint8Array } |
    { kind: "archive"; path: string; target: DraftNode; role: string };
const snapshots = new WeakMap<object, string>();

/** Materialize a fresh request copy; private snapshots never expose mutable bytes. */
export function graphRequest(spec: GraphSpec): GenerateRequest {
    const json = snapshots.get(spec);
    if (!json) throw new TypeError("not a built archive graph specification");
    return JSON.parse(json) as GenerateRequest;
}

export function isGraphSpec(value: object): value is GraphSpec { return snapshots.has(value); }

class DraftNode implements FractalNode {
    readonly members: Member[] = [];
    terminalMembers?: Member[];
    constructor(readonly owner: Draft, readonly label?: string) {}
    attach(payload: PayloadSpec): this { this.members.push(ordinary(payload)); return this; }
    directory(path: string): this { return this.attach({ archivePath: path.endsWith("/") ? path : `${path}/` }); }
    link(edge: ArchiveLink<FractalNode>): this {
        if (!(edge.to instanceof DraftNode)) throw new TypeError("target must be a node handle");
        this.members.push({ kind: "archive", path: edge.archivePath, target: edge.to, role: edge.role ?? edge.archivePath });
        return this;
    }
    terminal(options: {payloads: readonly PayloadSpec[]}): this {
        this.terminalMembers = options.payloads.map(ordinary); return this;
    }
}
function ordinary(payload: PayloadSpec): Member {
    if (payload.bytes !== undefined && !(payload.bytes instanceof Uint8Array)) throw new TypeError("payload bytes must be Uint8Array");
    if (payload.bytes === undefined && !payload.archivePath.endsWith("/")) throw new TypeError("file payload requires bytes");
    return { kind: "payload", path: payload.archivePath, bytes: Uint8Array.from(payload.bytes ?? []) };
}
function checkPaths(members: Member[]): string | undefined {
    const names = new Map<string, boolean>();
    for (const m of members) {
        const dir = m.path.endsWith("/");
        const key = dir ? m.path.slice(0, -1) : m.path;
        if (!key || /[\\\0:]/.test(key) || key.startsWith("/") || key.split("/").some(p => !p || p === "." || p === "..") ||
            new TextEncoder().encode(m.path).length > 65535 || (dir && (m.kind === "archive" || m.bytes.length))) return `invalid member path: ${m.path}`;
        if (names.has(key)) return `duplicate member path: ${m.path}`;
        for (const [other, otherDir] of names) {
            if ((!otherDir && key.startsWith(`${other}/`)) || (!dir && other.startsWith(`${key}/`))) return `file/directory conflict: ${m.path}`;
        }
        names.set(key, dir);
    }
}

class Draft {
    readonly nodes: DraftNode[] = [];
    selected?: DraftNode;
    rootDeclarations = 0;
    node(options: NodeOptions = {}): DraftNode {
        const n = new DraftNode(this, options.label);
        for (const payload of options.payloads ?? []) n.attach(payload);
        this.nodes.push(n); return n;
    }
    root(node: FractalNode): this {
        if (!(node instanceof DraftNode)) throw new TypeError("root must be a node handle");
        this.selected = node; this.rootDeclarations++; return this;
    }
    build(variant: "loopy" | "fractal", options: BuildOptions & {mode?: "finite" | "recursive"; depth?: string}): BuildOutcome<GraphSpec> {
        const fail = (message: string): BuildOutcome<GraphSpec> => ({ ok: false,
            error: { category: "invalid_request", code: "ZB_INVALID_TOPOLOGY", message },
            diagnostics: [{ code: "ZB_INVALID_TOPOLOGY", message }] });
        if (!this.nodes.length) return fail("empty graph");
        const root = this.selected ?? this.nodes[0];
        if (root.owner !== this || this.rootDeclarations > 1) return fail("root owner mismatch or repeated root declaration");
        for (const n of this.nodes) {
            for (const m of n.members) if (m.kind === "archive" && m.target.owner !== this) return fail("cross-builder node reference");
            const error = checkPaths(n.members); if (error) return fail(error);
            if (n.terminalMembers) { const e = checkPaths(n.terminalMembers); if (e) return fail(e); }
        }
        const order: DraftNode[] = []; const seen = new Set<DraftNode>();
        const visit = (n: DraftNode) => { if (seen.has(n)) return; seen.add(n); order.push(n);
            for (const m of n.members) if (m.kind === "archive") visit(m.target); };
        visit(root);
        if (order.length !== this.nodes.length) return fail("unreachable declared node");
        const mode = options.mode ?? "recursive";
        if (mode === "finite" && (typeof options.depth !== "string" || !/^(0|[1-9][0-9]*)$/.test(options.depth) || BigInt(options.depth) > 0xffffffffn)) return fail("invalid finite depth");
        const edges = (n: DraftNode) => n.members.filter((m): m is Extract<Member,{kind:"archive"}> => m.kind === "archive");
        if (variant === "loopy") {
            if (order.length < 2 || order.some(n => edges(n).length !== 1) || edges(order.at(-1)!)[0].target !== root) return fail("loopy requires a single simple cycle of at least two nodes");
        } else if (mode === "recursive") {
            const active = new Set<DraftNode>(); const done = new Set<DraftNode>();
            const cyclic = (n: DraftNode): boolean => {
                if (active.has(n)) return true; if (done.has(n)) return false; active.add(n);
                for (const e of edges(n)) if (cyclic(e.target)) return true;
                active.delete(n); done.add(n); return false;
            };
            if (!cyclic(root)) return fail("recursive fractal requires a reachable cycle");
        } else {
            let layer = new Set([root]); const depth = Number(options.depth);
            // A frontier repeat allows skipping equivalent levels without deep recursion.
            const frontiers = new Map<string, number>();
            for (let d = 0; d < depth; d++) {
                const key = [...layer].map(n => order.indexOf(n)).sort((a,b)=>a-b).join(",");
                const old = frontiers.get(key);
                if (old !== undefined) { const period = d-old; const skip = Math.floor((depth-d)/period)*period; if (skip) { d += skip-1; continue; } }
                frontiers.set(key,d); layer = new Set([...layer].flatMap(n => edges(n).map(e=>e.target)));
            }
            if ([...layer].some(n => n.terminalMembers === undefined)) return fail("missing terminal definition at finite cutoff");
        }
        const member = (m: Member) => m.kind === "payload" ? {kind:m.kind,path:m.path,bytes:encodeBase64(m.bytes)} :
            {kind:m.kind,path:m.path,target:order.indexOf(m.target),role:m.role};
        const graph = {root:0,mode,depth:mode === "finite" ? options.depth : undefined,nodes:order.map(n=>({
            members:n.members.map(member),terminal:mode === "finite" ? n.terminalMembers?.map(member) : undefined,
        }))};
        const spec = Object.freeze({ variant }) as GraphSpec;
        snapshots.set(spec,JSON.stringify({schema:"zb.request/v2",container:"zip",variant,graph,profile:options.profile,limits:options.limits}));
        return {ok:true,value:spec,diagnostics:[]};
    }
}

/** Declare nodes, connect a simple cycle, then build an immutable specification. */
export function createZipLoopy(): ZipLoopyBuilder {
    const d = new Draft();
    return {node: options=>d.node(options),root(node){d.root(node as FractalNode);return this;},
        build: (options={})=>d.build("loopy",options) as BuildOutcome<ZipLoopySpec>};
}
/** Declare shared states and branches, then choose recursive or finite evaluation. */
export function createZipFractal(): ZipFractalBuilder {
    const d = new Draft();
    return {node: options=>d.node(options),root(node){d.root(node);return this;},
        build: options=>d.build("fractal",options) as BuildOutcome<ZipFractalSpec>};
}

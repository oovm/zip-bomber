import type { BackendGenerateResult } from "../facade.js";

export type WasmGenerateJson = {
    ok: boolean;
    bytesBase64?: string;
    root?: string;
    artifacts?: Array<{
        id: string;
        bytesBase64: string;
        byteLength?: number;
        digest?: string;
    }>;
    logicalNodeMap?: string[];
    report?: unknown;
    error?: {
        category?: string;
        code?: string;
        message?: string;
    };
};

export function bytesToB64(bytes: Uint8Array): string {
    let text = "";
    for (const byte of bytes) text += String.fromCharCode(byte);
    return btoa(text);
}

export function b64ToBytes(b64: string): Uint8Array {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i += 1) {
        out[i] = bin.charCodeAt(i);
    }
    return out;
}

export function mapGenerateWasmJson(raw: WasmGenerateJson): BackendGenerateResult {
    return {
        exitCode: raw.ok ? 0 : 2,
        ok: raw.ok,
        bytes: raw.bytesBase64 ? b64ToBytes(raw.bytesBase64) : undefined,
        root: raw.root,
        artifacts: raw.artifacts?.map((a) => ({
            id: a.id,
            bytes: b64ToBytes(a.bytesBase64),
            byteLength: String(a.byteLength ?? 0),
            digest: a.digest ?? "",
        })),
        logicalNodeMap: raw.logicalNodeMap,
        reportJson: JSON.stringify(raw.report ?? {}),
        errorCategory: raw.error?.category,
        errorCode: raw.error?.code,
        errorMessage: raw.error?.message,
    };
}

export function cancelledWasmGenerateResult(): BackendGenerateResult {
    return {
        exitCode: 2,
        ok: false,
        reportJson: JSON.stringify({
            schema: "zb.report/v0",
            container: "zip",
            variant: "quine",
            status: "error",
            topology_nodes: 0,
            topology_edges: 0,
            warnings: [],
            error_code: "ZB_CANCELLED",
            error_message: "solver cancelled",
            crc_strategy: "reset_bytes",
        }),
        errorCategory: "cancelled",
        errorCode: "ZB_CANCELLED",
        errorMessage: "solver cancelled",
    };
}

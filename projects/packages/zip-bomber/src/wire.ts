/** Normalize Node `Buffer` / views to a plain `Uint8Array` for the public API. */
export function toUint8Array(
    bytes: Uint8Array | ArrayBufferView | null | undefined,
): Uint8Array | undefined {
    if (!bytes) {
        return undefined;
    }
    if (bytes instanceof Uint8Array) {
        return bytes;
    }
    return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
}

/** Encode bytes as standard base64 for the isomorphic JSON wire. */
export function encodeBase64(bytes: Uint8Array): string {
    let binary = "";
    for (const byte of bytes) {
        binary += String.fromCharCode(byte);
    }
    if (typeof btoa === "function") {
        return btoa(binary);
    }
    return Buffer.from(bytes).toString("base64");
}

export type WirePayload = {
    archivePath: string;
    bytesBase64?: string;
};

export type WireGenerateRequest = {
    schema: "zb.request/v2";
    container: "zip";
    variant: string;
    selfMemberPath?: string;
    payloads?: WirePayload[];
    topology?: unknown;
    graph?: unknown;
    profile?: string;
    limits?: unknown;
};

/** Build a `zb.request/v2` JSON document with base64 payload bytes. */
export function encodeGenerateRequestJson(input: {
    variant: string;
    selfMemberPath?: string;
    payloads?: Array<{ archivePath: string; bytes?: Uint8Array }>;
    topology?: unknown;
    graph?: unknown;
    profile?: string;
    limits?: unknown;
}): string {
    const wire: WireGenerateRequest = {
        schema: "zb.request/v2",
        container: "zip",
        variant: input.variant,
    };
    wire.graph = input.graph; wire.profile = input.profile; wire.limits = input.limits;
    if (input.selfMemberPath !== undefined) {
        wire.selfMemberPath = input.selfMemberPath;
    }
    if (input.topology !== undefined) {
        wire.topology = input.topology;
    }
    if (input.payloads && input.payloads.length > 0) {
        wire.payloads = input.payloads.map((payload) => ({
            archivePath: payload.archivePath,
            bytesBase64: payload.bytes ? encodeBase64(payload.bytes) : undefined,
        }));
    }
    return JSON.stringify(wire);
}

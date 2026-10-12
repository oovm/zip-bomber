/** Normalize Node `Buffer` / views to a plain `Uint8Array` for the public API. */
export function toUint8Array(bytes: Uint8Array | ArrayBufferView | null | undefined): Uint8Array | undefined {
    if (!bytes) {
        return undefined;
    }
    if (bytes instanceof Uint8Array) {
        return bytes;
    }
    return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
}

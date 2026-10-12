import type { ZipBomberApi } from "./types.js";
import type { ZipBomberBackendHooks } from "./facade.js";

/** Backend kind used when constructing a runtime. */
export type ZipBomberBackend = "node" | "wasm" | "stub";

/** Factory contract for Node / WASM loaders. */
export type ZipBomberApiFactory = (
    backend: ZipBomberBackend,
    hooks?: ZipBomberBackendHooks,
) => ZipBomberApi;

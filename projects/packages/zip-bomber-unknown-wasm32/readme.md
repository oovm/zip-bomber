# 🧩 @sxo/zip-bomber-unknown-wasm32

WebAssembly build of zip-bomber, published for the browser and edge entries.

Use it through the main package rather than depending on it directly:

```js
import {createZipBomber} from "@sxo/zip-bomber/wasm";
```

The bundle carries the full generator — `zipBomb`, `zipQuine`, `zipLoopy`, `zipFractal`, `plan`, `verify`, and
`capabilities` — on top of the built-in Rust core, and it produces byte-identical archives to the Node binding. One
limitation is deliberate: an abort that arrives after the call starts cannot interrupt the running WASM call, so only a
signal that is already aborted before `generate` is honoured. Cancel by discarding the result, or run it in a worker you
can terminate.

The JS glue and the `.wasm` payload live in `lib/`.

# 🧵 zb-napi

Node-API binding over `zb-generate`.

Exposes the full surface rather than a single entry point: generate and plan from a `zb.request/v0` document,
cancellable generate that maps an `AbortSignal` onto the solver cancellation token, raw-bytes verify, artifact-set
verify with an optional verification spec, capabilities, and variant listing.

Reports travel as JSON so the Node and WebAssembly bindings can be compared field for field.

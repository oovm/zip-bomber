# 🐧 @sxo/zip-bomber-linux-x64

Node-API binary that backs `@sxo/zip-bomber` on Linux x64, built from the same Rust core as the WebAssembly entry.

It is declared as an optional dependency of the main package, so package managers pick the matching build automatically.
Install it directly only when you need to pin a platform artifact by hand — the JavaScript package is the one that
exposes the API.

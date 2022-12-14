# zip-bomber

[![License](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](https://www.mozilla.org/MPL/2.0/)

Generate recursive ZIP / zip-bomb variants for archive robustness testing and format research.

Variants: **bomb**, **quine**, **loopy**, **fractal**.

- Rust crates use the short prefix `zb-*` (`zb-types`, `zb-generate`, `zb-napi`, `zb-wasm`).
- npm packages use the full name `@sxo/zip-bomber` and `packages/zip-bomber-*`.

```bash
npm install -g @sxo/zip-bomber
zip-bomber doctor
zip-bomber variants
zip-bomber generate --variant quine -o ./out.zip
```

Template builds list variants and return `not_implemented` until algorithm gates land. Prefer a VM or sandbox when experimenting with recursive extractors.

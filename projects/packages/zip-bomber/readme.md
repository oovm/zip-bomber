# @sxo/zip-bomber

Generate recursive ZIP / zip-bomb variants for archive robustness testing.

```bash
npm install -g @sxo/zip-bomber
zip-bomber variants
zip-bomber generate --variant quine -o ./out.zip
```

Variants: `bomb`, `quine`, `loopy`, `fractal`.

Rust crates use the short prefix `zb-*`. npm packages stay `@sxo/zip-bomber*`.

Template builds list variants and return `not_implemented` until algorithm gates land. Prefer a VM or sandbox when experimenting with recursive extractors.

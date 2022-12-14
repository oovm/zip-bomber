# zip-bomber

Generate recursive ZIP / zip-bomb variants for archive robustness testing.

## When to use

- Need a classic expansion bomb, ZIP quine (`A→A`), loopy cycle (`A→B→A`), or fractal self-similar nest
- Testing unzippers, scanners, or recursive extractors against cyclic / nested archives

## Commands

```bash
zip-bomber doctor
zip-bomber variants
zip-bomber generate --variant quine -o ./out.zip
```

Variants: `bomb`, `quine`, `loopy`, `fractal`.

## Safety

Prefer a VM or sandbox. Blind recursive extraction can exhaust disk or memory. Template builds may return `not_implemented` until algorithm gates land.

## Naming

- npm: `@sxo/zip-bomber`, packages `zip-bomber-*`
- Rust crates: `zb-*` (short prefix only)

# @sxo/zip-bomber

Generate recursive ZIP / zip-bomb variants for archive robustness testing.

```bash
npm install -g @sxo/zip-bomber
zb variants
zb generate --variant quine -o ./out.zip
zb plan --variant quine --quine-name recurse/again.zip --attach "assets/readme.txt=./readme.txt"
zb generate --variant quine --quine-name recurse/again.zip --attach "assets/readme.txt=./readme.txt" --directory assets --directory empty -o ./release.zip
```

Variants: `bomb`, `quine`, `loopy`, `fractal`.

Rust crates use the short prefix `zb-*`. npm packages stay `@sxo/zip-bomber*`.

Quine supports an independent self-member path, UTF-8 attachment paths, regular-file snapshots, and explicit directories. Repeat `--attach` and `--directory` to add entries. Existing output files are never overwritten. Attachments must fit the template's serialized prefix/suffix window budgets. Loopy and Fractal remain unimplemented.

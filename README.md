# 🗜️ zip-bomber

Generate recursive ZIP / zip-bomb variants — archives that contain themselves, contain each other, or unfold without
bound — behind a capability matrix that refuses to claim more than the code proves.

[![check-rust](https://github.com/oovm/zip-bomber/actions/workflows/check-rust.yml/badge.svg)](https://github.com/oovm/zip-bomber/actions/workflows/check-rust.yml)
[![check-typescript](https://github.com/oovm/zip-bomber/actions/workflows/check-typescript.yml/badge.svg)](https://github.com/oovm/zip-bomber/actions/workflows/check-typescript.yml)

## 🧬 Variants

| Variant   | Topology               | What it produces                                                         | Status       |
|-----------|------------------------|--------------------------------------------------------------------------|--------------|
| `bomb`    | nested tree            | depth-planned nesting with an expansion budget checked before allocation | implemented  |
| `quine`   | self loop              | one member whose decompressed bytes are the archive itself               | **verified** |
| `loopy`   | directed cycle         | each node's recursive member decompresses to the whole of the other node | **verified** |
| `fractal` | finite / recursive DAG | a self-similar spec with back edges, materialised per depth              | implemented  |

**verified** means the gate passed end to end: generate, independent decode, byte equality on both edges, and cycle /
period checks. **implemented** means the code path exists and is budget-planned but is not registered as independently
verified. `capabilities()` publishes the same matrix at runtime.

Known boundaries are rejected rather than approximated: a cycle longer than two nodes returns `unsupported_topology`,
and a two-node cycle that does not select the `zip.loopy-two-node.gapped/v0` profile returns `unsupported_profile`.

## 🚀 Install and run

```bash
npm install -g @sxo/zip-bomber

zb doctor                                    # backend, capability matrix, catalog coverage
zb variants
zb plan --variant quine                      # feasibility and budget estimate, no bytes written
zb generate --variant quine -o ./self.zip
zb verify --input ./self.zip
```

Structured requests carry the topology instead of flags:

```bash
zb plan --request ./loop.job.json
zb generate --request ./loop.job.json -o ./out-loop
zb verify --dir ./out-loop --request ./loop.job.json
```

`generate` writes into a fresh directory via staging and rename, and refuses to overwrite an existing one.

## 🏗️ Repository layout

```text
projects/crates/     zb-types · zb-solver · zb-generate · zb-napi · zb-wasm · zb-benchmark
projects/packages/   @sxo/zip-bomber, per-platform binaries, wasm build, agent skills, homepage
scripts/             build-napi.mjs, build:wasm helper, CI publishers
```

`zb-generate` owns topology, layout, templates, and verification orchestration; `zb-solver` owns the GF (2) / CRC
constraint algebra; `zb-napi` and `zb-wasm` are thin bindings; the npm packages re-export one shared facade.

Compression and ZIP parsing are not implemented here: the workspace consumes the Acorn crates (`acorn-codec` for
Deflate, `acorn-zip` for archive structure), so there is no external `zip` or `flate2` anywhere in the dependency graph.
A sibling Acorn checkout is required to build.

## 🧪 Build and test

```bash
cargo test --workspace --all-features     # contracts, solver, templates, verification
cargo fmt --all -- --check
pnpm build:napi                           # native Node-API binary
pnpm build:wasm                           # wasm32-unknown-unknown package
pnpm typecheck
pnpm test:e2e                             # Node and WASM backends must agree byte for byte
```

The end-to-end gate is the honest one: both backends run the same request, the archives are compared byte-for-byte, and
every declared `verified` row is re-checked through independent decoding rather than through the generator's own code
path.

## 📦 Releases

npm publishing is OIDC-only through the `publish-npm.yml` workflow and the `NPM_PUBLISH` environment; no long-lived npm
token exists in CI, and real versions are never published locally. Unpublished package names are claimed once as `0.0.0`
so the Trusted Publisher binding can be registered before the first real tag.

## ⚠️ Handle with care

These artifacts are explodey on purpose. Extract them in a VM, container, or sandbox with hard time and size limits, not
on a workstation. Scanning them with indexers, AV engines, and archive browsers is the intended use; feeding them into a
production pipeline is not.

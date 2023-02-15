# 🗜️ zip-bomber

Recursive ZIP generators for the archives that are meant to break things: a ZIP that contains itself, two ZIPs that
contain each other, and nested archives that unfold without bound.

Made for extractor robustness testing, archive-format research, and scanner / antivirus regression corpora. Every
compressed byte comes from an in-repo Deflate implementation, so the same request produces the same archive bytes on
Node, in the browser, and from the CLI.

## 🧬 Variants

| Variant   | Topology               | What it produces                                                         | Status       |
|-----------|------------------------|--------------------------------------------------------------------------|--------------|
| `quine`   | self loop              | one member whose decompressed bytes are the archive itself               | **verified** |
| `loopy`   | directed cycle         | each node's recursive member decompresses to the whole of the other node | **verified** |
| `bomb`    | nested tree            | depth-planned nesting with an expansion budget checked before allocation | implemented  |
| `fractal` | finite / recursive DAG | a self-similar spec with back edges, materialised per depth              | implemented  |

**verified** means the CI gate passed end to end: generate, independent decode, byte equality on both edges, and cycle /
period checks. **implemented** means the code path exists and is budget-planned, but is not registered as independently
verified. `capabilities()` returns exactly this matrix, and no report claims more than the code proves.

Honest boundaries: a cycle longer than two nodes returns `unsupported_topology` instead of being silently truncated to
two, and a two-node cycle that does not explicitly select the `zip.loopy-two-node.gapped/v1` profile returns
`unsupported_profile` instead of quietly switching constructors.

## 🚀 Quick start

`zb` is the installed command. Every subcommand accepts `--json` for machine-readable output and `--locale` for the
message catalog.

```bash
zb doctor                                             # backends, capability matrix, catalog coverage
zb variants                                           # the four declared variants
zb plan --variant quine                               # feasibility and budget estimate, no bytes written
zb generate --variant quine -o ./self.zip             # the archive that decompresses to itself
```

Two-node cycles and finite fractals are described as documents rather than as a wall of flags, so the generator never
has to guess what you meant:

```bash
zb plan --request ./loop.job.json
zb generate --request ./loop.job.json -o ./out-loop      # publish the whole artifact set
zb verify --dir ./out-loop --request ./loop.job.json     # re-check every edge and the cycle period
```

`generate` publishes into a fresh directory (staging, then rename) and refuses an existing one, so a partial run never
looks finished. request files, `--variant`, and the attachment flags are mutually exclusive.

## 🧰 CLI

| Command       | Purpose                                                                    |
|---------------|----------------------------------------------------------------------------|
| `zb doctor`   | probe the loaded backend, print the capability matrix and catalog coverage |
| `zb variants` | list declared variants                                                     |
| `zb plan`     | feasibility, resolved profile, and a per-phase budget estimate             |
| `zb generate` | write the archive set, `manifest.json`, and `report.json`                  |
| `zb verify`   | re-verify an archive, or a published directory with `--dir`                |

Useful options: `--variant`, `--output`, `--quine-name` (self-member path), repeated `--attach archivePath=sourcePath`,
repeated `--directory`, `--request`, `--input`, `--dir`, `--json`, `--locale`.

## 📦 Node and browser

```js
import { zipQuine, zipLoopy } from "@sxo/zip-bomber";
import { createZipBomber } from "@sxo/zip-bomber/node";
import { createZipBomber as createWasm } from "@sxo/zip-bomber/wasm";

const api = await createZipBomber();
const result = await api.zipQuine({ selfMemberPath: "a/a.zip" });
if (result.ok) {
    for (const artifact of result.value.artifacts) {
        console.log(artifact.id, artifact.bytes.byteLength);
    }
}
await api.dispose();
```

The same facade backs all three entries: `zipBomb`, `zipQuine`, `zipLoopy`, `zipFractal`, `plan`, `verify`, and
`capabilities`. `createZipLoopy()` and `createZipFractal()` build a staged spec with owner-bound node handles, so you
link nodes by handle instead of inventing ids.

Structured results instead of thrown strings:

- `Outcome` carries `ok`, the value, and a `zb.report/v2` report.
- Errors use stable `ZB_*` codes and categories, so they can be branched on rather than matched as text.
- Success reports carry the resolved profile, the solver facts (`rank`, `free_bits`, `equations_used`,
  `problem_digest`), per-edge facts, and `budget_usage`.

Long-running generation is cancellable: `generate(request, { signal })` honours an `AbortSignal` on Node through the
libuv pool, returning `ZB_CANCELLED`. The browser entry observes an abort that already happened before the call.

## 🛡️ Budgets and determinism

`GenerateLimits` is enforced before allocation, not after:

- total and per-artifact byte ceilings, decoded-byte ceiling, live-memory ratio;
- GF (2) constraint bits and solver operation ceilings;
- nesting depth, member count, template candidate count.

Values that can exceed `MAX_SAFE_INTEGER` travel as decimal strings on the wire, so a large budget never silently rounds
through a JavaScript `number`. Exceeding a ceiling returns `resource_limited` with the phase that refused, which is a
different outcome from `verification_failed`.

Determinism is a contract, not a coincidence: the Deflate encoder, the attachment profile, the candidate enumeration
order, and the CRC solve are all fixed. Identical requests produce identical bytes, and the Node and WASM backends are
asserted byte-for-byte equal in CI.

## ⚠️ Handle with care

These are working exploit-adjacent artifacts by design.

- Do not extract generated archives on a machine you care about. Use a VM, a container, or a dedicated sandbox with a
  hard time and size cap.
- Scanning them with indexers, AV engines, and archive browsers is the point. Feeding them to a production pipeline is
  not.
- `bomb` and recursive `fractal` requests are rejected before they allocate when the planned expansion exceeds your
  ceilings, but you still want an external watchdog.

## 📚 Layout

Rust crates use the `zb-*` prefix (`zb-types`, `zb-solver`, `zb-generate`, `zb-napi`, `zb-wasm`). The npm side is this
package plus one optional binary package per platform, and a `@sxo/zip-bomber-skills` companion for agent skills. CI
gates are `check-rust`, `check-typescript`, and `publish-npm`, and the release path is OIDC-only.

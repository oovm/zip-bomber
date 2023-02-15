# 📋 zb-types

Shared value contracts for the whole toolchain: variant kinds, archive topology graphs, generate requests and limits,
artifacts, capability rows, and the report schemas.

Reports use `zb.report/v2` when bytes were emitted, exposing the resolved profile, solver facts, per-edge facts, and
`budget_usage`. Stub and error outcomes keep the older `zb.report/v1` shape. Large budgets travel as decimal strings on
the wire, so a ceiling above `MAX_SAFE_INTEGER` does not round through a JavaScript `number`.

This crate has no generator logic and no I/O.

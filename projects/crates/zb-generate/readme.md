# 🏗️ zb-generate

Topology to archives: the pipeline that turns a request into a verified artifact set.

Stages: normalise the graph, compute SCCs, plan sizes before allocating, pick a template, lay out the ZIP records, solve
the fixed point through `zb-solver`, emit, then verify through Acorn. Template streaming is built from `acorn-codec`
block primitives, and unfinalised blocks stay bit-continuous, so the generator chooses the block sequence while the
codec owns the bitstream.

Budgets are reserved phase by phase through a task ledger, and a request that cannot fit is refused before the first
large allocation. Emitted reports carry the profile, solver facts, per-edge equality, and the budget snapshot.

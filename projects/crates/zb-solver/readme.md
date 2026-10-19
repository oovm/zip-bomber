# 🧮 zb-solver

Constraint algebra for fixed-skeleton archive construction.

Owns GF (2) bitset elimination, IEEE CRC-32 affine column construction, shared-slot CRC problems where one 32-bit value
is stored in several places, and byte/checksum equalities reduced to residual rows before they reach the bit matrix.
Rank, free bits, and a problem digest come back as evidence so a caller can report how a fixed point was found.

Cooperative cancellation runs through a token checked at pivots and column construction, so a long elimination can be
stopped without leaving a half-solved matrix behind.

It depends on `zb-types` only. It never emits archives and never calls Acorn.

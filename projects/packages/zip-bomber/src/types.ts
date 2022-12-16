/** Stable variant ids exposed by CLI and bindings. */
export type VariantId = "bomb" | "quine" | "loopy" | "fractal";

/** All variants in display order. */
export const VARIANTS: readonly VariantId[] = [
    "bomb",
    "quine",
    "loopy",
    "fractal",
] as const;

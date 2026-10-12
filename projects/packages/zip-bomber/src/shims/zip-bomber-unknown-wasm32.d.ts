declare module "@sxo/zip-bomber-unknown-wasm32" {
    export function listVariants(): string;
    export function capabilitiesJson(): string;
    export function generateVariantJson(variant: string): string;
    const _default: {
        listVariants?: () => string;
        capabilitiesJson?: () => string;
        generateVariantJson?: (variant: string) => string;
    };
    export default _default;
}

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createLocalizeFromLocales, type ParsedOptions } from "@vmz/commander";
import { InputError } from "./errors.js";

export function localesRoot(): string {
    return join(dirname(fileURLToPath(import.meta.url)), "../../locales");
}

export function translator(options: ParsedOptions) {
    return createLocalizeFromLocales({
        root: localesRoot(), locale: typeof options.locale === "string" ? options.locale : undefined,
        envKeys: ["ZB_LOCALE", "LC_ALL", "LANG"],
    }).t;
}

export function inputFailure(error: unknown, options: ParsedOptions): number {
    const t = translator(options);
    const code = error instanceof InputError ? error.messageId : "zb.cli.err.hostIo";
    const args = error instanceof InputError ? error.args : {
        message: error instanceof Error ? error.message : String(error),
    };
    if (options.json) console.log(JSON.stringify({ ok: false, error: { category: "invalid_request", code, args } }));
    console.error(t(code, args));
    return 1;
}

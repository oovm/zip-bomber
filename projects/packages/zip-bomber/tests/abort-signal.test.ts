import assert from "node:assert/strict";
import test from "node:test";
import { createSharedZipBomberApi } from "../src/facade.js";
import type { GenerateRequest } from "../src/types.js";
import { createZipBomber, loadZipBomberNode } from "../src/node/index.js";

const request: GenerateRequest = {
    schema: "zb.request/v2",
    container: "zip",
    variant: "quine",
};

test("already-aborted signal returns cancelled before the backend", async () => {
    const api = createSharedZipBomberApi("node", {
        generateRequestCancellable: async () => {
            throw new Error("backend must not run");
        },
    });
    const out = await api.generate(request, { signal: AbortSignal.abort() });
    assert.equal(out.ok, false);
    if (!out.ok) {
        assert.equal(out.error.category, "cancelled");
        assert.equal(out.error.code, "ZB_CANCELLED");
    }
    await api.dispose();
});

test("abort notifies the cancellable generate hook", async () => {
    const api = createSharedZipBomberApi("node", {
        generateRequestCancellable: (_json, signal) =>
            new Promise((resolve) => {
                const finish = () =>
                    resolve({
                        exitCode: 1,
                        ok: false,
                        reportJson: JSON.stringify({
                            schema: "zb.report/v1",
                            container: "zip",
                            variant: "quine",
                            status: "error",
                            warnings: [],
                        }),
                        errorCategory: "cancelled",
                        errorCode: "ZB_CANCELLED",
                        errorMessage: "solver cancelled",
                    });
                if (signal?.aborted) finish();
                else signal?.addEventListener("abort", finish, { once: true });
            }),
    });
    const controller = new AbortController();
    const pending = api.generate(request, { signal: controller.signal });
    controller.abort();
    const out = await pending;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.category, "cancelled");
    await api.dispose();
});

test("native cancel flag rejects a pooled generate", async () => {
    const { createRequire } = await import("node:module");
    const require = createRequire(import.meta.url);
    let binding: {
        GenerateCancel?: new () => { cancel: () => void };
        generateRequestJsonCancellable?: (
            requestJson: string,
            cancel: { cancel: () => void },
        ) => Promise<{ ok: boolean; errorCategory?: string; errorCode?: string }>;
    };
    try {
        const loaded = require("@sxo/zip-bomber-win32-x64") as {
            default?: typeof binding;
        } & typeof binding;
        binding = loaded.default ?? loaded;
    } catch {
        return;
    }
    if (!binding.GenerateCancel || !binding.generateRequestJsonCancellable) return;
    const handle = new binding.GenerateCancel();
    handle.cancel();
    const result = await binding.generateRequestJsonCancellable(
        JSON.stringify(request),
        handle,
    );
    assert.equal(result.ok, false);
    assert.equal(result.errorCategory, "cancelled");
    assert.equal(result.errorCode, "ZB_CANCELLED");
});

test("native pre-cancelled generate returns cancelled", async () => {
    const node = loadZipBomberNode();
    const binding = node as unknown as {
        generateVariant?: unknown;
    };
    if (!binding.generateVariant) return;
    const api = await createZipBomber();
    const out = await api.generate(request, { signal: AbortSignal.abort() });
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.category, "cancelled");
    await api.dispose();
});

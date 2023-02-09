import assert from "node:assert/strict";
import test from "node:test";
import { runGenerateInWorker } from "../src/wasm/cancellable.js";

class HungWorker {
    terminated = false;
    onmessage: ((event: MessageEvent) => void) | null = null;
    onerror: ((event: ErrorEvent) => void) | null = null;

    terminate() {
        this.terminated = true;
    }

    postMessage(_data: unknown) {
        // Intentionally never responds so abort must terminate the worker.
    }
}

test("wasm worker abort returns cancelled and terminates the worker", async () => {
    const controller = new AbortController();
    const worker = new HungWorker();
    const pending = runGenerateInWorker(
        new URL("file:///generate-worker.js"),
        JSON.stringify({ schema: "zb.request/v2", variant: "quine" }),
        controller.signal,
        () => worker as unknown as Worker,
    );
    controller.abort();
    const result = await pending;
    assert.equal(result.ok, false);
    assert.equal(result.errorCategory, "cancelled");
    assert.equal(result.errorCode, "ZB_CANCELLED");
    assert.equal(worker.terminated, true);
});

test("wasm worker success maps generate payload", async () => {
    class OkWorker {
        onmessage: ((event: MessageEvent) => void) | null = null;
        terminate() {}
        postMessage() {
            this.onmessage?.({
                data: {
                    kind: "ok",
                    payload: JSON.stringify({
                        ok: true,
                        bytesBase64: "aGk=",
                        report: { schema: "zb.report/v2", status: "ok", warnings: [] },
                    }),
                },
            } as MessageEvent);
        }
    }
    const result = await runGenerateInWorker(
        new URL("file:///generate-worker.js"),
        "{}",
        undefined,
        () => new OkWorker() as unknown as Worker,
    );
    assert.equal(result.ok, true);
    assert.deepEqual(result.bytes, new TextEncoder().encode("hi"));
});

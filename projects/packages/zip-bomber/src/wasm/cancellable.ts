import type { BackendGenerateResult } from "../facade.js";
import {
    cancelledWasmGenerateResult,
    mapGenerateWasmJson,
    type WasmGenerateJson,
} from "./result.js";

export type WorkerFactory = (url: URL) => Worker;

type WorkerResponse =
    | { kind: "ok"; payload: string }
    | { kind: "error"; message: string };

export function supportsWasmWorker(): boolean {
    return typeof Worker !== "undefined";
}

export function runGenerateInWorker(
    workerUrl: URL,
    requestJson: string,
    signal?: AbortSignal,
    workerFactory?: WorkerFactory,
): Promise<BackendGenerateResult> {
    if (signal?.aborted) {
        return Promise.resolve(cancelledWasmGenerateResult());
    }
    const createWorker = workerFactory ?? ((url) => new Worker(url, { type: "module" }));
    return new Promise((resolve) => {
        const worker = createWorker(workerUrl);
        let settled = false;
        const finish = (result: BackendGenerateResult) => {
            if (settled) return;
            settled = true;
            signal?.removeEventListener("abort", onAbort);
            worker.terminate();
            resolve(result);
        };
        const onAbort = () => finish(cancelledWasmGenerateResult());
        signal?.addEventListener("abort", onAbort, { once: true });
        worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
            const data = event.data;
            if (data.kind === "ok") {
                finish(
                    mapGenerateWasmJson(
                        JSON.parse(data.payload) as WasmGenerateJson,
                    ),
                );
                return;
            }
            finish({
                exitCode: 2,
                ok: false,
                reportJson: JSON.stringify({
                    schema: "zb.report/v1",
                    status: "error",
                    warnings: [],
                    error_code: "ZB_INTERNAL",
                    error_message: data.message,
                }),
                errorCategory: "internal_error",
                errorCode: "ZB_INTERNAL",
                errorMessage: data.message,
            });
        };
        worker.onerror = () => {
            finish({
                exitCode: 2,
                ok: false,
                reportJson: JSON.stringify({
                    schema: "zb.report/v1",
                    status: "error",
                    warnings: [],
                    error_code: "ZB_INTERNAL",
                    error_message: "wasm worker failed",
                }),
                errorCategory: "internal_error",
                errorCode: "ZB_INTERNAL",
                errorMessage: "wasm worker failed",
            });
        };
        worker.postMessage({ requestJson });
    });
}

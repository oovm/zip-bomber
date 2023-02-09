/// <reference lib="webworker" />
import {
    generateRequestJsonFromBinding,
    initWasmBinding,
} from "./worker-runtime.js";

type WorkerRequest = { requestJson: string };
type WorkerResponse =
    | { kind: "ok"; payload: string }
    | { kind: "error"; message: string };

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
    try {
        const binding = await initWasmBinding();
        const payload = generateRequestJsonFromBinding(
            binding,
            event.data.requestJson,
        );
        const response: WorkerResponse = { kind: "ok", payload };
        self.postMessage(response);
    } catch (err) {
        const response: WorkerResponse = {
            kind: "error",
            message: err instanceof Error ? err.message : String(err),
        };
        self.postMessage(response);
    }
};

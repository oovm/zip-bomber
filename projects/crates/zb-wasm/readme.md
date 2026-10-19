# 🌐 zb-wasm

WebAssembly binding over `zb-generate`, for browsers and edge runtimes.

Carries the same generate, plan, verify, and capability surface as the native binding, with the same bytes for the same
request. Mid-call cancellation is not promised: only a signal that was already aborted before the call is honoured, and
anything else has to be cut off from the outside, for example by terminating the worker that hosts the instance.

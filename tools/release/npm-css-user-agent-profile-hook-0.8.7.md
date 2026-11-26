# Optional computed-document UA provider for #296

A private shared-adapter profile may now supply `userAgentSource(backend)` as an
owned decoded UA bundle. Without that hook, the old no-argument bridge and decoder
run exactly as before. The adapter does not assume custom providers immutable.

Actual native, Node WASM and web WASM controls use Root's computed preferences and
document UA to match the six source-hashed Chrome159 initial/inherited font/color
cases, including inherited Times on the controlled macOS profile. Warm reads use
zero legacy-UA bridge calls, one preferences request and one compilation; reset,
backend/preference identity changes, mutable custom UA and original errors retain
their contracts. Caller context must include preference identity in its environment
key; no factory-cache guard was relaxed.

The canonical probe/cases/oracle live in the binding crate's own test fixtures and
are shared by npm and Native tests without duplicate JSON. This commit excludes the
separate Native wrapper source. Full Node tests and original coverage floors pass;
exact counts, dependencies and source/artifact hashes are in the adjacent receipt.
The runtime default and private capture authority are unchanged.

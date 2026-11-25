# Optional shared CSS orchestration checkpoint

This increment adds an internal orchestration factory and optional canonical-UA-source discovery. It does not change the published getComputedStyle implementation; issue296 remains open.

The factory owns immutable CSS models without retaining DOM nodes in its weak resource registry. Explicit caller callbacks supply authenticated environment/render/result certificates, matching facts, limits and final CSSOM values. Uncertified stylesheet stores are fingerprinted on every read. Provider/typed failures propagate unchanged; mandatory finalization precedes copied primitive result ownership. Retained proxies survive reset/disposal. Missing WeakRef/finalization support is an explicit factory capability failure, not a weaker ownership mode.

Actual native and node-WASM component controls show unchanged warm reads and detached construction add no bridge calls; direct plain CSSOM changes recompile even when the document stamp is unchanged, connected changes resolve again, and disposal order remains prepared then compiled. These controls use explicit static fixture facts and already-finalized literal colors, so they do not establish full runtime or used-value parity. The exact artifact identities and counts are in the adjacent JSON.

The isolated Rust1.98 parser rebuild includes native and optimized node/web WASM. Full runtime tests pass901/901, zero skips; coverage100% lines/functions and97.62% branches; CC9 passes. The first gate attempt failed only because a spawned npm packaging control selected the restricted global cache; the unchanged gate passed with the owned npm cache. No user cache permissions were changed.

Canonical UA retrieval is optional at this boundary. Discovery validates a supplied function without invoking it; absent support remains valid capability discovery. Native/WASM forced-mode selection, minimal parser injection and existing fallback behavior remain unchanged. Actual source export and the complete Rust cascade/scene/default integration are separate pending work.

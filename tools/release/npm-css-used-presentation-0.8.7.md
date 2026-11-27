# Optional lazy used-property presentation (0.8.7)

This source increment depends on `1be2e4f2d309eb767200778ca9738d43a37b39f0`. It adds an optional internal `resolveProperty` callback to the shared computed-style factory/proxy. It does **not** switch the published getComputedStyle default or close #296. No JS width/height table, new runtime flag or parser requirement is added.

Only canonical keys in Rust-returned `row.usedProperties` can invoke the callback; missing/bad metadata with a supplied callback throws TypeError. Color, length, item, iteration and priority reads perform no used-property callback/layout. A successful string is cached once per proxy; original failures remain pending and retryable. Renderer values and batch rows remain unchanged.

Empty metadata performs no full batch/provider-context copy. A per-factory WeakMap memo owns and freezes pure-data row/batch/provider snapshots once, discarding partial memo entries on errors. NEW callback provider contexts must be stateless pure data; callers strip DOM/native handles/functions. Shallow-frozen read-context snapshots preserve owner/certificate identity without freezing a live Document. Returned proxies remain readable after reset/disposal and receive no compiled/prepared handles. No cross-version presentation reuse or performance neutrality is claimed.

Full JS/conformance: 1000 pass, zero failures; line/function coverage 100%, branches 97.32%; Vitest 29, Jest 2, CC9, format and diff checks pass. Existing plain HTML parser artifacts suffice; this child ran no Rust/addon compiler or benchmark. The two source files stayed frozen during combined-artifact controls.

The combined-source receipt asserts all nine groups green: original 18, compatibility 15 and inheritance 4 across native/Node WASM/web WASM. Native SHA256: `35edaaf7737438559416cf8dda96bb647a4c30f2be6b2b6413a9711dad4355db`; Node/web WASM SHA256: `08235348d3551f3799c2ff26a08f2fc0c23e532429fd7654ae0773e7e8a8bfde`. Six result receipts embed the exact matching proxy/factory hashes. Original18 receipts lack embedded manifests; their hashed harness explicitly imports this frozen factory and its adjacent proxy. This is private adapter proof, not default cutover or general layout parity. Full hashes, paths, checks and verification scope are in the adjacent JSON.

Held brand/performance and fresh-builder experiments are excluded. Parent owns release integration/push; no tag/publication/main merge is included. Raw logs remain in `.evidence/issue-296/used-proxy/`.

Threshold audit: the first normal hook accepted 998 tests while reporting 97.29% branches. That rounded CLI acceptance was not used as exact-floor evidence. Two test-only malformed-factory-metadata and function-context regressions now give 1000 passing tests and 97.32% branches at the hook's actual concurrency, with both runtime source files unchanged.

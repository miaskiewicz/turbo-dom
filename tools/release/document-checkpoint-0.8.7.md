# Connected-document checkpoints — v0.8.7 / #552

React's MUI Autocomplete render reads `anchorEl.clientWidth` while constructing the next detached subtree. The full native storage revision correctly advances for those disconnected operations. Using that revision as the sole native-document solved-layout cache identity caused a complete solve for each new anchor.

The actual consumer trace had reusable strict geometry (`queryIndependent=true`), cacheable effects, no lifecycle exception, identical non-tree state and precisely +80 full-version increments between expensive reads. The intercepted JavaScript operation count was 77, not a one-to-one mapping to those 80 internal increments. A second witness classified every intervening receiver as disconnected. Independent public offline control isolates the cause: six unchanged width reads produce one solve/five hits; adding only a retained detached `document.createElement('div')` between reads produces six solves/zero hits, identical width 750 and no document mutation records. Connected text/style/attachment controls still change geometry and require new solves.

## Additive APIs

Core `turbo_dom::rtdom::Tree::document_checkpoint() -> Option<DocumentCheckpoint>` and `document_unchanged_since(checkpoint) -> bool` provide opaque tree-incarnation/connected-change identity. Host `turbo_dom_browser_env::host_internals::document_checkpoint(scope, native_document) -> Option<DocumentCheckpoint>` additionally validates the exact native Document, installed creation realm and browsing ownership under explicit microtask checkpoints. The host token has `Clone`, `Copy`, `Debug`, `Eq` and `PartialEq`; it holds no V8 handles. No guest API is added.

Compare freshly acquired `Some` tokens for native connected-input cache equivalence. `None` retains the previous full-version cache behavior. Uncertain-shadow/inert/retired/uncertain owners, unaccounted versions and malformed document ancestry do not supply proof. Native field/private ownership reads do not invoke guest properties, cascade, resources, lifecycle or layout.

`Tree.version` and selector counters retain their existing mutation increments. Audited detached allocation/attributes/text/form-state/child construction preserve the new checkpoint. Connected changes, global or unknown invalidation, quiet style invalidation and successful storage collection invalidate it. Detaching from a connected parent invalidates before ownership is lost; inserting the detached component invalidates normally. Full mutation audit and hashed consumer baseline receipts are under `.evidence/document-checkpoint/`.

## Consumer boundary

This API establishes connected native-input equivalence only. Retain all CSSOM/live-state, host, rules, composition, inline policy, font/image epoch, viewport, query-admission and animation/cacheability checks. Never promote a query-dependent result without admission validation. Never reuse lifecycle-derived or noncacheable results.

Capture refresh and every new live-tree layout still check the original full `Tree.version`. A checkpoint is not permission to pass an outdated `LiveCapture` to live layout. The consumer must acquire and consume keys in its existing coherent synchronous native observation; style/resource work that changes that observation must retain normal refresh/revalidation.

## Validation

Final per-command test/coverage/commit receipts are recorded in `.evidence/document-checkpoint/FINAL-GATES.json` and `INTEGRATION.json`. The floors remain core 100% and binding 97.6%, with the existing CC9 limit. Producer API controls include detached React-like HTML/SVG construction, connected attribute/text/UTF16/form/custom state, quiet/global invalidation, old/new parent transitions, storage collection/replacement, independent trees/realms, invalid brands, inert/retired/shadow owners, unaccounted versions, malformed low-level root reparenting, default install and startup-snapshot rehydration.

Producer validation does not establish that the original application mapping step fits its unchanged consumer execution budget. Surf owns coherent repinning, actual artifact rebuild and that acceptance replay.

Verified implementation: `5dd8ef9d42974d4eb52f2c2d5519c13292443f9a`.
Final frozen-source workspace: **3,219 passed, 0 failed, 13 existing ignored**;
default binding: **838 passed, 0 failed, 4 existing ignored**;
all-feature layout: **1,299 passed, 0 failed, 0 ignored**. Strict workspace and
all-feature Clippy, formatting, CC9, version/single-V8 guards passed. Core LLVM
coverage **13,479/13,479 (100%)**; binding **22,742/23,291 (97.642866%)**, above the
unchanged 97.6% floor. Allocation controls match the prior receipt: churn 372
allocations / 82,270 bytes per operation (ceilings 380 / 85,000); warmed sibling
ranks 0 allocations / 0 bytes for 1,024 matches. No budgets or assertions changed.

Connected-shadow capability is extended under #554; see [the shadow ownership receipt](document-checkpoint-shadow-0.8.7.md). The original implementation/gate identity above records the initial conservative contract.

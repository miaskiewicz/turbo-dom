# Pure mutation-batch claim (v0.8.7)

Base: `e62c9dcfe8a8f1d940e1c9f5379d92df9d145feb`. Follow-up to the separate empty-document fanout found alongside #579. No cross-call cache, sampled-value approximation, ownership waiver, skipped drain, or changed recording guard.

## Cause and change

`mutation_observers::flush` previously entered and restored each saved document merely to call `Tree::take_mutation_events_utf16`. Each scope swap moved DOM/STYLE/LISTENERS/ACTIVE state through the document map, including when the event vector was empty. The tree operation itself only drains owned Rust vectors/maps and converts owned UTF16 payloads; it has no TLS, V8, consumer or guest callback.

The new claim phase directly drains every saved tree, then the current tree last. All batches are claimed and all RefCell borrows released before any consumer is invoked. Actual nonempty delivery retains its original DomScope, native consumers, observer queues, CE handling and script processing. The early radio-owner, recording and termination checks are unchanged.

Within one pass, saved documents follow the existing map iteration order and current is last. HashMap order across separate calls is NOT promised: the old swapping implementation itself removed/reinserted entries, while the new read/drain phase does not alter the map.

## Causal operation control

The 24-row static matrix varies N16/64 selected targets, D4/16 ancestor depth, M0/1/8 unrelated retained documents, and recording OFF/ON. Each row samples twice at a fixed clock, with equal owned presentation payloads, unchanged native world and zero geometry-provider calls. The switch counter observes real DomScope enter/drop under a test-only claim-phase guard, not estimated switch cost. Diagnostic-origin work is recorded separately; state is isolate-owned primitive Rust data, with only a scoped weak bridge to the active native test phase and no V8 roots.

At D16/N64/M8 with recording ON:

| Operation | Before | After |
|---|---:|---:|
| Production capture switches | 406,464 | 0 |
| Diagnostic capture switches | 20,752 | 0 |
| Foreign empty drains | 203,232 | 203,232 |
| Total empty drains | 228,636 | 228,636 |
| Flush attempts | 25,404 | 25,404 |

All 24 rows retain the original drain and flush counts. The earlier 43dd audit used a different release base and counted 203,200 foreign drains; the source-matched baseline here is 203,232. The scope-switch counter's positive inverse still records two real switches when entering/restoring a foreign document outside the pure claim.

## Semantic inverses

A native two-document control queues nonempty attribute records with an unpaired UTF16 surrogate old value in BOTH trees before the flush. Trusted consumers mutate during delivery and reenter the fanout. Original records remain delivered exactly once, ownerDocument matches the actual realm, foreign precedes current within the pass, nested records are correct, and observer callbacks stay deferred until the explicit microtask checkpoint. The existing cross-document custom-element adoption/connected callbacks and the full normative suite are retained.

No live app timeout resolution is claimed from these operation counters. Actual optimized Surf artifact acceptance remains separate.

## Final gates

- Focused causal/control suite: 4 passed, 0 failed (baseline switch expectation was genuinely RED).
- Full host binding crate: **1,145 passed, 0 failed, 4 existing ignored**, 34 suites.
- Default binding library profile: **845 passed, 0 failed, 4 existing ignored**.
- Strict Clippy, all targets, host-internals/test-support, `-D warnings`: PASS.
- Formatting and diff checks: PASS. New helpers/tests CC≤9; the two pre-existing mutation_observers functions child_list/flush remain at their unchanged baseline10. No limit/exclusion changes.
- Binding coverage has not been remeasured in this lane; no coverage-floor change.

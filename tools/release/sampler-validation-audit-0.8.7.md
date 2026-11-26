# Sampler validated-import operation fix (#579 follow-up)

Base: `43dd96e795f7f8d633d89b8e820facd97d591cf7`. The production change shares the original complete validation-and-local-import operation between lazy-default reads and ordinary imports. No cross-call cache, validity memo, checkpoint substitution, selected-root shortcut or flush bypass is introduced.

The motivation is the actual Surf 43dd profile: 216 canonical samples, 59.515 seconds host sample duration, maximum 145 native CSS candidates. CPU ancestry overlaps: sampling 58.613 seconds, parent-source 32.611, sheet application 26.796, entry validation 15.507. Those are not invocation counts and must not be added.

## A: validation origin

The four synthetic N16/64, D4/16 scenes sample twice with the same world, fixed canonical clock, equal complete sampler payloads and zero geometry-provider calls. A raw native test hook records isolate-owned primitive counters without ownership callback machinery. Diagnostic callbacks/flushes are classified separately.

At N64/D16 each sample performs 20,565 entry validations, including 9,639 immediately adjacent successful lazy-validation then original-import validations of the SAME entry. Maximum immediate dependency fanout is one; no entry repeats within one recursive operation. This is a dependency chain repeated across operations, not a branching DAG pathology. Each of owner/probe/world is read 41,294 times.

Totals are B+L+I: B=(N-1)(D+2), L=N(D+1)(D+2)/2, I=(N-1)(D+1)(D+2)/2. At N64/D16 these are 1,134+9,792+9,639.

Implemented change: share the existing full validate-and-import operation between the lazy-default path and ordinary import path. Import only after all existing guards; the immediate next local lookup then needs no duplicate validation. No cross-call memo, persistent certificate, checkpoint substitution or guard omission. Preserve the selected-root fresh invariant and original fallback behavior.

## B: native empty document fanout

A separate explicit-normal longhand scene removes the inherited-default trigger. Twenty-four rows vary N/D, retained unrelated documents M0/1/8, and mutation recording OFF/ON. Worlds and complete payloads match between samples; provider calls remain zero.

At N64/D16 there are 25,400 production flush attempts regardless M. Recording OFF performs zero foreign-document drains. Recording ON performs 0 / 25,400 / 203,200 actual empty foreign-document drains at M0/1/8 respectively. Diagnostic-origin attempts (1,297) are separately excluded. This establishes the read wrapper's cost amplification, not permission to bypass custom-element/observer semantics.

The real cross-document inverse passes: a native media callback creates a foreign custom element and adopts it into the main document; connected/adopted ownership callbacks remain correct, native nonempty drains are observed, MutationObserver delivery stays zero before the explicit checkpoint and occurs afterward. Raw diagnostics retain no V8 references.

## Verification status

A: causal RED (0 passed / 1 failed) becomes GREEN (1 passed / 0 failed), all four rows retained. Largest validation total 20,565→10,926; each of world/owner/probe reads 41,294→22,016; adjacent duplicate validations 9,639→0. No freshness cascade is removed.
B: original diagnostic RED (0 passed / 1 failed), all 24 rows retained. No B optimization is implemented. The final positive test verifies exact counter accounting for the native fanout; it does not claim that the fanout is eliminated.
Cross-document ownership/CE/MO inverse: GREEN (1 passed / 0 failed).
Final gate counts are recorded below. Existing #579 mutation/media/font, volatile/failed-box, active ancestor/seek/custom-property/reference-size, snapshot/privacy and termination controls are retained. No live app timeout resolution claim.

## Guard and ordering argument

The lazy path already excludes its selected root and requires the exact current private batch plus exact local-snapshot identity when present. Its shared helper now performs the ORIGINAL complete EntryValid operation, then the ORIGINAL local snapshot import. There is no callback between successful guard completion and import, and no callback between returning that map and the next local lookup. The next lookup therefore consumes the imported snapshot instead of performing the same proof a second time. Failure still returns the original fresh-map path. World/probe/owner pre/post checks, recursive dependencies, media revalidation, volatile/failed-box rejection, lifecycle mode, and termination behavior remain in the original validator.

This does not assert that validity survives arbitrary future work: no certificate escapes, no validation result is memoized, and separate later operations continue to validate normally.

## Final gates

- Full host binding crate: **1,110 passed, 0 failed, 4 existing ignored**, 32 suites including integration/doc targets.
- Default binding library profile: **840 passed, 0 failed, 4 existing ignored**.
- Strict Clippy, all targets, host-internals/test-support, `-D warnings`: PASS.
- Formatting and diff checks: PASS. New Rust test/helper files CC≤9: PASS.
- Binding JS complexity: exactly the same 101 pre-existing violations as base, no new violation; global configured JS gate does not include this binding file. New helper complexity stays within nine.
- Binding coverage has not been remeasured in this lane; no floor or exclusion changed.

The raw diagnostic hook is native test-only and deleted/captured by bootstrap. Its callbacks do not enter the generic ownership wrapper; observer/CE callback work attributable to pre-existing diagnostic hooks is separately classified, not silently included in production-origin counts. The native counters keep primitive isolate-owned state and no V8 handles.

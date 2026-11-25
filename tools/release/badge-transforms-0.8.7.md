# Canonical variable/percentage transitions — v0.8.7

A real MUI badge uses `scale(0) translate(var(--Badge-translate))` to
`scale(1) translate(var(--Badge-translate))`, with `--Badge-translate:50%,-50%`.
Producer410 rejected that real, unmodified transition before the first geometry
read. The identical static transform already worked in native layout.

The existing shared UTF16 custom-token kernel now resolves transform variables
when CSS transition endpoints are created. Private declaration provenance retains
only immutable source/token objects and strings, not DOM targets or snapshot maps.
Old endpoint tokens remain frozen when a variable changes. A replacement transition
starts from the currently presented old contribution sampled at the same clock
before cancellation, rather than its old underlying target.

Canonical translate primitives retain finite px/% coefficients and function order.
Matching-list interpolation emits a symbolic transform through the existing public
PresentedProperty path before native cascade/inheritance/layout. Layout still owns
the current local box basis; sampling never guesses dimensions or calls a layout
provider. Unresolved reference-dependent matrix decomposition, relative-font and
unproved registered/rollback/dynamic-custom contexts remain unsupported. Uncertainty
propagates through the existing composed-parent snapshot provenance, including
open and closed shadow hosts. Rotation/skew/perspective primitive spelling remains
intact so symbolic mixtures cannot discard rotation winding. Zero percentages in
z coordinates remain invalid; valid x/y percentage coordinates remain supported.

The newly supported token operation has an explicit conservative ceiling of
1048576 UTF16 units for total/output work and nesting16. Aggregate input is checked
before custom-map copies. A shared token classifier can prove the unchanged no-var
path without expansion, preserving literal transform input beyond the new operation
ceiling; classifier refusal remains unproved, not a false no-var result. Resource
failures are typed, and error formatting preserves actual V8 termination instead
of manufacturing a replacement RangeError. No existing execution/resource budget,
coverage floor or capability guard was raised or weakened.

## Regression receipts

Implementation87e125435ad61df573ffce5572cff5bacd4c853d (including ancestor27507e1c)
passes83 presented controls, startup snapshot1 and transition7 passing; additional
absolute5/presentation5/lifecycle12/shadow6 adapter controls pass. New and changed
scoped helpers passCC9. The native error-formatter guard is the separate followup
f3c346c0cf0ff3b9f8b8ee0458387c8b90cd7ecd, included in combined code1759e0ca.

Independent Chromium159.0.8154.6 fixtures cover the first unmodified read and five
sampled rectangles. Surf's exact unchanged original and retarget geometry tests
pass2/2 on immutable87 (1.01s), while275 is causally1pass/1fail for retargeting.
The combined recovery+Badge merge51d4e0b0 passes the same2 geometry tests and the
unchanged26-test host-input binary together. Consumer receipts are under
`turbo-surf/.recovery/badge-87e-private/` and `combined-51d-private/`.

Producer combined workspace51d4e0b0 completes3248 reported passes, zero failures,
13 ignored tests across242 result summaries. The final native termination and
ordinary-RangeError inverse executes1/1 on combined1759e0ca. An initial incorrectly
qualified exact-name filter ran zero tests and is not counted. Raw receipts:
`.evidence/presented-effects/workspace-badge-recovery.log` and
`termination-formatter-final-executed.log`. Final-source strict/adapter receipts
are recorded separately; no whole-workspace coverage or actual-package/auth pass
is implied by these focused results. The npm296 runtime cutover is independent.


## Final combined source validation

Combined code `1759e0ca0e64c3dcdf452a1bacd9bea35b2d675e` passes strict
workspace all-target Clippy and strict host-internals/test-support all-target
binding Clippy, with zero warnings. Final-source affected runtime checks execute
109 tests with zero failures: termination17, formatter1, presented83, snapshot1,
transition7. Formatting, diff and configured Rust/JS complexity checks pass.
Receipts: `.evidence/presented-effects/FINAL-HOST-PIPELINE.json`, its named logs,
and `clippy-badge-recovery-final.log`. Whole-workspace3248 is attributed above to
the combined implementation before the separately tested diagnostic guard, not
misrepresented as an exact-source rerun after it. An exact final workspace rerun
and refreshed binding coverage continue independently; the last410 binding
measurement97.61% is not claimed for this changed source. Layout/Core production
source is unchanged by these two fixes; existing floor receipts remain identified
separately. No tag, publication or main merge occurs.


## Completed exact workspace and binding floor

The exact final ad55 source subsequently completes the full default workspace
with **3250 reported passes, zero failures and13 ignores**, exit0, across243
summaries (`workspace-ad55-exact.log`, `WORKSPACE-AD55-RESULT.json`).

The first changed-source binding measurement is retained RED at23211/23800
(97.53%). Test-only checkpoint `c10b61dc472dd325d5a7c83f5420180caa5b62ff`
adds four real boundary controls without production edits. Its focused87/87,
strict all-target Clippy, fmt and scopedCC9 pass. Exact LLVM tarpaulin then passes
the unchanged97.6% floor: **23231/23800 =97.61%**, exit0. Twenty net lines are
covered; no impossible OOM input or fabricated geometry was introduced. Hashed
receipts and raw reports: `.worktrees/animation-token-coverage-087/.evidence/token-coverage/`
(`RECEIPT.json`, `NEW-COVERED.json`, `COVERAGE.log`, `coverage/tarpaulin-report.json`).
The test-only follow-up does not require a consumer dependency repin.

Surf's actual optimized ad55 addon has SHA256
`d8d3e23ae678b8f179afe2d9f3f32e11c5fa1865bfd82f5f706452716b416bd9`.
Build exit0/sourceUnchangedtrue; normalized421 discovery returns8/8 in729/33ms,
images pass18/18, and original+retarget Badge geometry matches all10 frozen
Chromium rectangles. Actual auth acceptance is separate: awake smoke currently
shows a backend403/portal-legal state under investigation, with no producer cause
attributed. The sleep-interrupted earlier attempt is explicitly invalidated.
No authentication/full-E2E pass is claimed.

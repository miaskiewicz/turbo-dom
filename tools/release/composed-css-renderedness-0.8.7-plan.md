# #578 composed CSS renderedness

Issue: https://github.com/miaskiewicz/turbo-dom/issues/578
Base: `53ce99e29fc3126f39f15764dd947c8692a7ef11`; accepted #577 is unchanged.
The fix follows private native composed ancestors for CSS renderedness. No public
closed-slot API, whole-tree copy or application-service access was introduced.

## Observed Chromium semantics

Chromium 159.0.8154.6, ten independent cases / 45 phases:

| Case | CSS animation counts through phases |
| --- | --- |
| Open host: initial/hide/restore | 1,0,1 |
| Closed host: initial/hide/restore | 1,0,1 |
| Nested open→closed hosts: initial/outer hide+restore/inner hide+restore | 1,0,1,0,1 |
| Assigned closed slot: initial/slot hide+restore/host hide+restore/unassign/reassign | 1,0,1,0,1,0,1 |
| Initially unassigned light child: initial/assign/unassign | 0,1,0 |
| Slot fallback: initial/suppress/restore/slot hide+restore | 1,0,1,0,1 |
| Manual closed slot: initial/assign/unassign/reassign/slot hide+restore | 0,1,0,1,0,1 |
| Presented display: initial/show over none/important none/remove important/cancel show/restore | 1,1,0,1,0,1 |
| Display exit: initial/paused midpoint/end/restore | 1,1,0,1 |
| Foreign assignment: initial/foreign node/restore target | 1,0,1 |

Each target's independent paused WAAPI effect remains discoverable in **all 45
phases**. The native path never cancels/reclassifies script effects.
Unassigned and suppressed-fallback behavior is oracle evidence, not an assumption.
CSS names remain `probe` when the CSS effect exists. Each native test checks the
same public target model and a throwing geometry-provider spy.

Files under `crates/turbo-dom-browser-env/tests/fixtures/`:

| File | SHA-256 |
| --- | --- |
| `composed_css_renderedness.html` | `44d0383fc4202fc1a4b78ef519c3bf9ac313b81aa76d4e62b14c4050ba8a30e3` |
| `composed_css_renderedness_cases.json` | `217cd122e5b69ed72b587974cb983860782b02d0f58e83530864589d8dff50fc` |
| `composed_css_renderedness_oracle.mjs` | `6fedf0bd4f0e1ca9656a93e6ea062eb114d76acfc8dfc13e622abba74bd6e95a` |
| `composed_css_renderedness_chrome159.json` | `6fb5e256d0e734528756181681e0cff6a4998729c0a902d44f01dc663e8ceafd` |

Native TDD source: `tests/composed_css_renderedness.rs`, thirteen named controls
with fixture/HTML identity assertions. Rustfmt, Rust CC≤9 and oracle JS syntax pass.
The earlier minimal native baseline under unmodified ea299 is retained in
`.worktrees/discovery-shadow-baseline-087/`: hidden host leaves CSS count 1 versus
Chromium 0. The original expanded native baseline was RED0/7 before the fix.

## Private implementation

`animations.js::rendered`/`renderedFrom` previously traversed captured parentElement,
which stopped at a shadow root. The fix changes CSS renderedness ancestry, leaving
query roots, effect ownership/composite order and WAAPI relevance unchanged.

It reuses `browser_env::host_composed_parent` → `shadow_slots::composed_parent`:
its current implementation already follows closed hosts, assigned/manual slots,
and detects suppressed fallback. It enters the native owner's creation context
and reads V8-private assignment records, not public assignedSlot/getters.

A private status helper inside shadow_slots distinguishes Parent / NotInFlatTree /
Unavailable, retaining the existing Option API as an unchanged adapter.
The private animations opcode projects only this target's ancestor
handles, validates every live owner and a genuine same-document endpoint, and
bounds traversal by the actual tree node count. Known unassigned/suppressed nodes
produce NotRendered; invalid/foreign/cyclic/terminating paths remain Unproved.
No partial path becomes a visibility certificate. No full-tree copy, new public
Rust/JS API, public closed-slot exposure or new callback pointer is needed.

The caller reads display through its existing snapshot map/canonical presented
scalar-display/exit-transition logic. The structural path alone does not certify
CSS visibility or authorize underlying-only display assumptions. Proven hidden
paths feed the existing CSS-owned cancellation code; scripts remain indexed.
Existing host Unknown/refusal behavior must remain strict for unproved paths.

## Validation and retained baseline follow-up

The corrected native ABI controls pass3/3 (no getter/proxy/coercion, slot/status/owner
reprojection and termination preservation). Binding library tests pass883/0 with4
existing ignored; presented-effect integration passes99/99, including startup
snapshot rehydration and existing realm/adoption/multi-isolate/recovery controls.
Strict all-target Clippy with `-D warnings`, fmt, JS syntax and scoped Rust CC≤9 pass.
Final scoped replay status is recorded in the accompanying receipt.

Manual private assignment bumps the existing live-style revision only when its
canonical node list/order changes; no-op deduplicated assignments do not bump it.
Physical Tree.version remains unchanged. After display callbacks, the path proof
reprojects status/root/ancestor identities; version equality alone never validates
private assignment edges. Array metadata is own DATA only. Unproved paths do not
cancel CSS; original exceptions and termination remain intact.

The full ten-case/45-phase Chrome oracle is preserved. Shipping controls retain
43 phases: the exit case tests initial and presented midpoint. Its end/restore
phases expose an independently reproduced pre-#578 bug on the verified unmodified
ea299 addon SHA256 `133e7c76a62fc55d80f361cd4acc519487d1c7aed3ff908b529583e2854de47f`:

| Fixture | CSS counts initial/midpoint/end/restore | Host computed display |
| --- | --- | --- |
| Chromium159 expected | 1,1,0,1 | block,block,none,block |
| Unmodified light DOM | 1,1,1,0 | block,block,none,none |
| Unmodified closed shadow | 1,1,1,1 | block,block,block,none |

WAAPI remains1 throughout. Raw replay/source is retained locally under
`.evidence/composed-renderedness/baseline-addon/`; the four-phase oracle above stays
tracked. This baseline defect is a separate follow-up; no existing assertion was
weakened. #579 performance work is independent. No live application latency or
complete CSS transition correctness is claimed by this fix.

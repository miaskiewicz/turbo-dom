# #578 source-only renderedness controls and proposed seam

Issue: https://github.com/miaskiewicz/turbo-dom/issues/578
Base: `53ce99e29fc3126f39f15764dd947c8692a7ef11`; accepted #577 is unchanged.
This checkpoint contains public fixtures/oracles and uncompiled native test source,
not a production fix. No Rust compiler or application service was used.

## Observed Chromium semantics

Chromium 159.0.8154.6, seven independent cases / 32 phases:

| Case | CSS animation counts through phases |
| --- | --- |
| Open host: initial/hide/restore | 1,0,1 |
| Closed host: initial/hide/restore | 1,0,1 |
| Nested open→closed hosts: initial/outer hide+restore/inner hide+restore | 1,0,1,0,1 |
| Assigned closed slot: initial/slot hide+restore/host hide+restore/unassign/reassign | 1,0,1,0,1,0,1 |
| Initially unassigned light child: initial/assign/unassign | 0,1,0 |
| Slot fallback: initial/suppress/restore/slot hide+restore | 1,0,1,0,1 |
| Manual closed slot: initial/assign/unassign/reassign/slot hide+restore | 0,1,0,1,0,1 |

Each target's independent paused WAAPI effect remains discoverable in **all 32
phases**. Thus the proposed native path must not cancel/reclassify script effects.
Unassigned and suppressed-fallback behavior is oracle evidence, not an assumption.
CSS names remain `probe` when the CSS effect exists. Each native test checks the
same public target model and a throwing geometry-provider spy; tests are not run yet.

Files under `crates/turbo-dom-browser-env/tests/fixtures/`:

| File | SHA-256 |
| --- | --- |
| `composed_css_renderedness.html` | `44d0383fc4202fc1a4b78ef519c3bf9ac313b81aa76d4e62b14c4050ba8a30e3` |
| `composed_css_renderedness_cases.json` | `b5198c499ae3687685c3dffb36edfea8f3d521b19861b6054d98a405c4aabbc7` |
| `composed_css_renderedness_oracle.mjs` | `6fedf0bd4f0e1ca9656a93e6ea062eb114d76acfc8dfc13e622abba74bd6e95a` |
| `composed_css_renderedness_chrome159.json` | `96b70a5db2e3e4329682fa75201b9b6ddf0450e6c9132560bfd3f8f812da46b6` |

Native TDD source: `tests/composed_css_renderedness.rs`, seven named tests with
fixture/HTML identity assertions. Rustfmt, Rust CC≤9 and oracle JS syntax pass.
The earlier minimal native baseline under unmodified ea299 is retained in
`.worktrees/discovery-shadow-baseline-087/`: hidden host leaves CSS count 1 versus
Chromium 0. That single baseline case was run; this expanded native matrix was not.

## Smallest proposed existing-seam reuse (not implemented)

`animations.js::rendered`/`renderedFrom` currently traverse captured parentElement,
which stops at a shadow root. Change only CSS renderedness ancestry, leaving
query roots, effect ownership/composite order and WAAPI relevance unchanged.

Reuse `browser_env::host_composed_parent` → `shadow_slots::composed_parent`:
its current implementation already follows closed hosts, assigned/manual slots,
and detects suppressed fallback. It enters the native owner's creation context
and reads V8-private assignment records, not public assignedSlot/getters.

The existing Option conflates known exclusion with unavailable evidence. A small
PRIVATE status helper inside shadow_slots can distinguish Parent / NotInFlatTree /
Unavailable, while retaining the existing Option API as an unchanged adapter.
The proposed private animations opcode then projects only this target's ancestor
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

Before implementation ships: native RED→GREEN for this matrix; dynamic display,
importance/exit, owner/adoption, exception/termination and startup rehydration
controls; no-provider and single-clock controls. No production change is approved
by this source-only document, and no live application outcome is claimed.

# Display-transition lifecycle cache refresh

Base: `e6e8c519c41b9a2f4504732f1f5bbf5e846b6ecf` (release/0.8.7).

A CSS animation/transition update could reuse its underlying-input cache after
presented `display` reached an exit. Its stale rendered state retained child CSS
animations and a before-change style; showing the element then incorrectly
created a `none`→`block` transition. Independent paused WAAPI effects remain
valid while hidden and must remain discoverable.

The existing canonical `transitionRendering` calculation precedes cache reuse
when rendering may have changed. Cached equality includes its base and hidden
state. Unchanged rendering still skips CSS lifecycle updates. A host-owned,
ordinary-path no-display-effect inverse retains the original dependency bound. No interpolation, timeline clock,
provider, global cache, version guard, CSS priority or Unknown policy changed.
Unknown rendering is never cached as hidden. The full #578 exit/end/restore oracle
is restored to the shipping test (45 phases).

## Causal controls

The unchanged producer `ea29941bd66ce302343abb18d5834b5483ffd4cb` was independently
replayed using its verified addon SHA256
`133e7c76a62fc55d80f361cd4acc519487d1c7aed3ff908b529583e2854de47f`.
Light-DOM child CSS counts were `[1,1,1,0]` and closed-shadow counts `[1,1,1,1]`
versus Chromium159 `[1,1,0,1]`; WAAPI remained1. No application services were used.

The pinned release native baseline passes4/7 new semantic tests: all four WAAPI
exact-start/pre-delay inverses pass, while both CSS exit/restoration cases and the
closed-shadow end/restore case fail. A test-only private input-cache disabling
control passes7/7; it is not included in the production fix. The narrow fix passes7/7; final coverage expands to15 controls including steady
inputs, effect removal/retarget/frame rewriting, clock callbacks, adoption and
foreign effects.

Fresh Chromium159.0.8154.6 supplies six source-owned boundary cases /27 samples.
It proves WAAPI `none`→`block` remains `none` at progress0, including exact active
start, and flips immediately afterward. The existing interpolation boundary is
correct and unchanged. CSS restoration instead has no new transition and reads
`block` immediately. Progress comparisons allow only1e-12 arithmetic tolerance;
display, model counts and timestamps remain exact.

## Stable work comparison

Five unchanged public discovery reads,16 CSS-opacity targets under4 shared
section ancestors (plus body/html):

| Case | Version | Cascade runs | Scalar ancestor reads | CSS updates | Used-size provider |
| --- | --- | ---: | ---: | ---: | ---: |
| No display effect | Baseline | 120 | 0 | 0 | 0 |
| No display effect | Fixed | 120 | 480 | 0 | 0 |
| Paused ancestor display effect | Baseline | 200 | 0 | 0 | 0 |
| Paused ancestor display effect | Fixed | 200 | 480 | 0 | 0 |

The additional scalar reads are exactly `5 ×16 ×(4+body+html)`. They reuse the
same per-read style map: no extra cascade or native used-size work was observed.
These are deterministic operation counts, not a wall-time or live-app claim.
Public/legacy discovery conservatively performs the scalar checks. Native host
sampling instead transports its existing ownership proof through the same private
weak batch, with a document world stamp captured before selection/clock callbacks.
The shortcut also requires exact document/path, unchanged ordinary-tree model
style-generation and no indexed display/custom-property frames. Uncertain,
foreign, shadow/slot or changed-world paths use fresh scalar checks. No dependency
import authority was changed. The inherited-depth guard remains12/18/30 validation
visits at depths2/4/8; the host opacity inverse performs0 scalar reads.

Existing style-generation scope0 follows physical own/ancestor model changes and
ignores sibling-only model creation. Values outside the safe integer range refuse
the shortcut. Model changes during rendering refuse reuse; model removal,
retargeting and frame rewriting are tested alongside cancellation. Host ownership
is checked again after callbacks before returning a snapshot, so a clock callback
creating a foreign effect cannot reuse the entry proof. This is two linear native
ownership scans per host sample, not a per-target registry scan. The shortcut
still performs native path/model guards; these counters do not claim zero metadata
cost or a live-app speedup.

## Validation

Final gate counts/source hashes are recorded in the adjacent JSON receipt.
Raw baseline, causal intervention, steady-work and fixed logs are retained locally
under `.evidence/display-boundary/`. Fixtures and Chrome results are tracked under
`crates/turbo-dom-browser-env/tests/fixtures/display_boundary_*`.

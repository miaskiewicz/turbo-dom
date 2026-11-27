# Presented target membership — #576

Issue: https://github.com/miaskiewicz/turbo-dom/issues/576

`preparePresentedTargets` rediscovered every selected target after each root,
then linearly searched the selected array for each rediscovered target. With N
static candidates, explicit identity comparisons were exactly N²(N+1)/2 even
though cascade and validation work were linear in the measured controls.

The fix keeps one private WeakSet per synchronous batch, using the constructor
and has/add intrinsics captured at trusted bootstrap. After each fresh root it
rediscovers only canonical indexed effect targets; previously selected candidates
already retain their original order. Newly discovered targets append in canonical
first-seen order. No global cache, clock change, style validation waiver, frame
advance or ownership relaxation is added.

## Deterministic public controls

The cfg(test)-only captured counter measures explicit identity comparisons and
set operations separately. Its native callback is absent in production. Counts
below use four shared static ancestors and no effects:

| Candidates | Before: identity comparisons | After: identity comparisons | After: set operations | Fresh cascades, unchanged | Validations, unchanged |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 16 | 2,176 | 0 | 16 | 22 | 90 |
| 32 | 16,896 | 0 | 32 | 38 | 186 |
| 64 | 133,120 | 0 | 64 | 70 | 378 |
| 128 | 1,056,768 | 0 | 128 | 134 | 762 |
| 451 | not run in baseline | 0 | 451 | 457 | 2,700 |

At N16 and depths8/12/16, original identity comparisons remained2,176;
corrected set operations remain16. Cascades26/30/34 and validations150/210/270
are unchanged. These controls did not prove pathological dependency-DAG
validation, so that path is untouched. Instrumented debug wall times are not an
optimized performance floor or a live application success claim.

Validation: causal scaling test RED→GREEN; presented-effects97/97 and CSS-live
93/93 pass. Controls preserve duplicate candidate normalization, dynamically
created ancestor target discovery, exact first-seen order, one canonical clock,
zero provider calls, foreign/adopted ownership, simultaneous isolates, exceptions
and termination. Strict all-target browser-env Clippy (-Dwarnings), fmt, JS syntax
and scoped changed-helper/test CC≤9 pass on Rust1.98. Whole css_live_tests.rs still
contains unrelated preexisting `run_in_mode` CC12; this change does not modify it.

## Actual-scene qualification

The read-only Surf scene hash was verified:
`4d01b37728ec8a9cf825db2593c96e0fb1c876b376487e88c7c6f4cf154c9816`
(`SCENE-74631.json`, seven sheets/874 rules). No private app HTML, CSS, URLs or
identifiers are tracked here. Root's separately normalized static replay has451
elements and seven active effect targets; active effect count is not sampler
candidate count. This patch fixes the proven public producer complexity defect;
actual live candidate count and modal latency attribution remain consumer work.

Raw source/count receipts are retained locally under
`.worktrees/presented-membership-scaling-087/.evidence/selection-scaling/`.

## Integrated and actual consumer outcomes

Production source `ea29941bd66ce302343abb18d5834b5483ffd4cb` completed the
default Rust1.98 workspace gate: 3,281 reported passes, zero failures and 13
ignored across 286 summaries. Full workspace all-target Clippy (`-D warnings`)
and fmt pass. The first Clippy attempt was interrupted during the shared disk
outage; the resumed full command exited0. Local receipts: `WORKSPACE-MEMBERSHIP-576.json`,
`workspace-membership-576.log`, `clippy-membership-576-resumed.log`.

Surf private clean controls passed30/30. The optimized actual Git build exited0
with source unchanged in747.67 seconds: native SHA256
`133e7c76a62fc55d80f361cd4acc519487d1c7aed3ff908b529583e2854de47f`,
MCP SHA256 `b4a2b4a075e4f3cdc20a2d4a34cb8ec2f04df1d4d978a93136ea17be619d76e6`.
Actual images18/18, Badge ten rectangles,421-element discovery eight records on
both reads (836.36ms first/32.95ms second) and smoke2/2 passed.

The first affected FluxSuper case still failed the original15-second
wait-for-hidden assertion (0/1,106 seconds overall); #576 does not resolve that
application failure. The frozen451-node static scene discovery1025.59ms and
rectangle332.27ms are single observations, not a stable performance floor.
Actual candidate count remains unknown; the874-rule frozen CSSOM has an
approximation of138 candidates, not a substitute for the host scoped list.
The separate discovery/reference-transform issue is tracked as#577.

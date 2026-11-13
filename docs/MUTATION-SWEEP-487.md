# Completing mutation sweep #487

The scope is `event_handlers.rs`, `worker_scope.rs`, `trusted_types.rs`,
`paint_png.rs` and `paint_svg.rs`. The earlier multicol sweep and the shipped
`record.target` regression are separate completed work. No full receipts for
these five files were found in the previous release artifacts; the identities
of the five historical Trusted Types survivors were not retained in the issue.
Consequently the current Trusted Types inventory is included in full.

## Reviewable pilot, before the full run

`scripts/mutation-sweep-plan.json` now selects **full** mode after the reviewed pilot. The
standalone workflow runs on pushes only to `codex/084-mutation-487`; it does not
change ordinary CI, release gates, floors or complexity limits. It uses Rust
1.98.0, cargo-mutants 27.1.0 and immutable commits resolved from the repository's
existing action versions. Its token has read-only contents permission, checkout
credentials are not persisted, and it neither publishes nor uses extra secrets.

The pilot has five jobs, one per file, with at most five mutants in each. The
installed tool's deterministic slice sharding produces 5/4/5/5/5 mutants for the
current inventories (24 total). Maximum parallel jobs is four. A sweep step is
bounded to 32 minutes inside a 45-minute job so the always-upload step has room
after a step timeout. A timeout is unresolved evidence, never a passing result.

Before authorizing the full matrix, read the pilot's actual baseline times and
per-mutant Build/Test durations from `outcomes.json`. No measured runtime is
claimed before that pilot. The original reviewed matrix used 1,479 mutants. After the validated large-coordinate
PNG/SVG fixes, a fresh inventory at `655404321e2ae05a93e25aad8752726800246944` is:

| Target | Current mutants | Proposed shards |
|---|---:|---:|
| event_handlers | 40 | 4 |
| worker_scope | 11 | 1 |
| trusted_types | 98 | 2 |
| paint_png | 922 | 12 |
| paint_svg | 439 | 6 |

That is **1,510 mutants** across the same 25 jobs, at most four concurrent.
The plan records the source reference and reviewed per-file counts; an actual
inventory with extra, missing or duplicate mutants stops before any baseline or
mutation. Re-enumerate and review after further production changes rather than
reusing the old 906/424 painter totals. Adjust the shard
counts from observed pilot cost before changing the tracked plan to `full`.
The plan generator rejects more than 12 shards per target or 32 total jobs.
This is a ceiling, not authorization to consume the ceiling without review.

The narrow push trigger starts the reviewed branch-only pilot without creating
a PR or changing main. GitHub documents that the manual UI button depends on the
default branch, while a workflow that has run can be dispatched against a branch
through the API/CLI; see [workflow_dispatch](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_dispatch).
A later approved plan-mode commit or dispatch can start the full sweep after
its shard counts are reviewed. Do not push this workflow before parent review.

## Evidence and fail-closed execution

Each job records the exact commit/tree, source and test hashes, lockfile and
effective Cargo metadata, compiler/tool versions and the V8 archive hash. The
browser jobs use the exact locked Rust-only fork and its pinned upstream
official archive, as ordinary CI does; no untracked local path override enters
the graph. Current mutation targets contain no production feature-gated blocks;
browser jobs enable host-internals/test-support and raster jobs use defaults.

1. Build the unmodified package's **all-targets** tests with `--no-run`, then run
   them in a separate timed command. Build/Test argv, logs and elapsed durations
   are separate; compile time is never included in the fallback timeout model.
   No mutation starts if that baseline fails. Warning capping applies equally to baseline and mutants only;
   ordinary strict Clippy remains independent and unchanged.
2. Run the deterministic file/shard using `--in-place --no-shuffle --timeout 300
   --build-timeout 300 --baseline skip`. In-place execution is inherently serial;
   version 27.1.0 rejects an explicit `--jobs` together with `--in-place`, even
   when its value is one. `-C --lib` restricts both build
   and test phases; passing `--lib` after the tool's `--` would restrict only the
   test phase in cargo-mutants 27.1.0 and waste relinking integration binaries.
   Browser primary runs filter to `handler`, `worker_scope::` or `trusted_types::`
   respectively, after an additional exact filtered unmutated baseline that must
   execute at least one passing test (zero selection fails before mutating). The pilot
   showed the unrelated full browser lib suite alone can take 170–355 seconds,
   so repeating it for every relevant helper mutation would dominate the sweep
   and can exceed the fixed 300-second mutant timeout even without a mutation.
   Raster primary runs retain the complete library suite and add the existing
   `multicol_rule_chrome` control and public rule-progress integration targets;
   PNG also includes `paint_memory` allocation assertions. No primary survivor is
   accepted merely because it is outside these filters: step 3 still runs it
   against all targets without any filter.
3. Retry every lib-suite survivor against the full all-targets tests, selected by
   its exact escaped name. All-targets baseline was already verified in step 1.
   Only this unfiltered fallback's test budget is `max(300, ceil(2*T + 30))`,
   where `T` is the observed successful unmutated **Test** command duration after
   the completed no-run build. Missing/malformed/zero/nonfinite or combined-phase
   timing is rejected. A computed budget above 900 seconds fails for review
   instead of silently clamping. Primary mutant test timeouts remain 300 seconds;
   neither production/test-source timeouts nor any quality floors are raised.
4. Require one outcome and nonempty raw diff/log for every selected mutant. A
   caught mutant passes. Unviable mutants retain compiler evidence separately.
   Misses, timeouts, unexpected successes, unknown outcomes, missing records or
   source-restoration failures fail. There are no equivalence allowlists.
5. Always upload raw inventories, baselines, diffs and outcomes, including misses.
   Aggregation rejects overlapping/missing shards, mixed revisions and partial
   full-file coverage. A pilot's result explicitly says `complete_scope: false`.

Any equivalent mutant needs an individual source proof and review; this runner
does not silently exempt it. Any genuine survivor needs a behavior assertion or
implementation fix and a retained red/green receipt. Then rerun its exact mutant.
The issue stays open until all five full inventories are accounted for on the
final source graph. The original sweep used release commit
`26272a83e8f8c1f335d9c615c942674b1de8ec1a` plus mutation tooling; its
[43483416004 run](https://github.com/miaskiewicz/turbo-dom/actions/runs/43483416004)
retains both complete and deadline-limited partial artifacts. Recovery tooling
and regression tests are rebased onto release candidate
`46b1dcc939a033d09fb5708eb517fde49af56771`, including #509/#511. New acceptance
runs retain separate source/dependency receipts on that graph. Old spans are
reused only after proving the corresponding production source byte-identical.
The separate #504 feature remains excluded. Any subsequent production or
dependency change requires explicit provenance review.

## Commands

```sh
python3 -m unittest discover -s scripts -p test_mutation_sweep.py
python3 scripts/mutation_sweep.py matrix --mode pilot
python3 scripts/mutation_sweep.py matrix --mode full

# After the parent grants the native build lane; use a committed clean worktree.
CARGO_ENCODED_RUSTFLAGS=--cap-lints=warn \
  python3 scripts/mutation_sweep.py run --target worker_scope --mode pilot \
  --shard 0 --shards 1 --output /private/tmp/mutation-487-worker-pilot
```

The pilot orchestration has fourteen fail-closed Python tests and the installed
tool's exact-name regex selection was checked without building Rust. No mutation
outcomes or timings are claimed from that source-only preparation.

## Exact replay selection and bounded rechecks

cargo-mutants 27.1.0's `StructField` visitor pushes those mutants directly rather
than applying its usual name-regex filter. A real PNG replay requested 25 names
but `--re` alone selected 37, including 12 unrelated struct-field deletions.
The runner therefore adds a source-matching `--in-diff` line mask, which the tool
applies to every mutation genre after discovery. This mask is explicitly a
selection aid, not a claimed Git change, and does not edit production source.

Before compiling a replay, `--list --json` must return exactly the requested name
set, with no duplicates, omissions or extras. The actual pinned tool selected
exactly 25/25 in the PNG reproduction after adding the mask. If a same-line
collision still prevents exact selection, the runner stops for review; it never
silently drops a mutation genre or accepts unexpected outcomes. The full primary
inventory remains unchanged. Python tests cover exact selection failures and
source-matching line-mask generation.

`workflow_dispatch` can select one `target` (default `all`) with `mode: full`.
It runs that file's complete existing shard plan, without restarting unrelated
files. A per-file aggregate validates its explicitly requested complete inventory but
keeps `complete_scope: false`; the default all-target aggregate still requires
all five inventories on the same acceptance graph. Existing matrix
ceilings, deadlines, unfiltered all-targets fallback and quality floors remain.

Local native validation must use a target directory owned by its worktree.
Parallel worktrees sharing a Cargo target directory can overwrite identically
named package binaries; a Cargo build lock does not establish attribution of a
later test invocation. Recovery records source and binary hashes, uses its own
APFS-cloned cache, and cleans only that private target's workspace artifacts
before validating. Never clean another worktree's or a shared target directory.

A completed primary shard now writes `source-after-primary.json` and
`primary-summary.json` before any expensive full fallback. Source restoration
must match exactly at that boundary. These receipts preserve which primary
mutants were caught or compiler-unviable even if the later bounded fallback is
interrupted. Primary misses/timeouts are not acceptance: the ordinary summary
and project aggregate remain failing until every selected mutant is resolved.

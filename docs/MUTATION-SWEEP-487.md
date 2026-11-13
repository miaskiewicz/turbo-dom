# Completing mutation sweep #487

The scope is `event_handlers.rs`, `worker_scope.rs`, `trusted_types.rs`,
`paint_png.rs` and `paint_svg.rs`. The earlier multicol sweep and the shipped
`record.target` regression are separate completed work. No full receipts for
these five files were found in the previous release artifacts; the identities
of the five historical Trusted Types survivors were not retained in the issue.
Consequently the current Trusted Types inventory is included in full.

## Reviewable pilot, before the full run

`scripts/mutation-sweep-plan.json` deliberately starts in **pilot** mode. The
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
claimed before that pilot. The proposed full matrix is currently:

| Target | Current mutants | Proposed shards |
|---|---:|---:|
| event_handlers | 40 | 1 |
| worker_scope | 11 | 1 |
| trusted_types | 98 | 1 |
| paint_png | 906 | 6 |
| paint_svg | 424 | 4 |

That is 1,479 mutants across 13 jobs, at most four concurrent. Adjust the shard
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

1. Run the unmodified package's **all-targets** tests. No mutation starts if that
   baseline fails. Warning capping applies equally to baseline and mutants only;
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
   Raster primary runs retain the complete library suite. No primary survivor is
   accepted merely because it is outside these filters: step 3 still runs it
   against all targets without any filter.
3. Retry every lib-suite survivor against the full all-targets tests, selected by
   its exact escaped name. All-targets baseline was already verified in step 1.
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
final source graph. The #504 branch is still changing; pilot costs may be learned
now, but the full acceptance run must use the integrated release graph, or prove
unchanged production source and monotonic test/dependency behavior explicitly.

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

The pilot orchestration has nine fail-closed Python tests and the installed
tool's exact-name regex selection was checked without building Rust. No mutation
outcomes or timings are claimed from that source-only preparation.

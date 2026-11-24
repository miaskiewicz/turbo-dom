# Live sibling selector ranks, v0.8.7 (#547)

## Reproduction and boundary

The baseline is release/0.8.7 at
`ffbba11985b3d3e2026ffcc7549d8ebe4d4caf81`. A public `Tree::matches_scoped`
sweep over dense span siblings repeatedly builds and filters the same sibling
vector in the default `Navigator::type_position`. Live and scoped navigators
inherited that implementation. The selector parser was already warm.

The deterministic isolated regression failed before implementation: 512 siblings,
four warmed structural selectors, 2,048 checks produced **16,384 allocations /
4,194,304 bytes**. The candidate produces **zero allocations / zero bytes** with
the same 1,024 positive matches. This gate is ignored during parallel tests
because its allocator counts the entire process; run it separately:

```sh
cargo +1.98.0 test -p turbo-dom --lib sibling_rank_alloc_gate -- --ignored --nocapture
```

A real Surf failing-stage CPU sample motivated the reduction. These results prove
producer repeated work, not that this alone resolves the consumer's navigation
timeout. Surf owns its independent candidate filtering and actual rebuilt-addon
integration. No Surf source, pins, frozen artifact, services or fixtures were
changed here. Consumers must repin the DOM-family crates coherently and rebuild.

## Implementation and semantics

`query_ranks.rs` lazily records child/type indices and totals per raw sibling
parent. The table belongs to the Tree and is invalidated by its full mutation
version, including mutations in detached trees. Only queried parents are stored;
old parent tables are dropped on version change. No nodes/tree are copied, no
selector verdicts are cached, and an unused rank cache allocates nothing.

Element groups preserve the existing exact namespace/local-name identity,
including Other namespace URIs, ignored prefixes and unpaired UTF-16 units.
Borrowed scalar names are used only while building a table; retained metadata
contains handles and numeric ranks. Parentless and non-element navigator behavior
is preserved. ScopedNavigator delegates raw ranks to LiveNavigator. The default
Navigator methods and custom/composed layout rank implementations are unchanged.
Filtered `nth-child(... of S)` and live state matching still use their existing
paths.

Four new integration tests and an uncached-reference unit test cover sibling order and totals, insertion/reparenting/
detachment, fragments/shadow roots, namespace identity, prefixes, exact case,
non-scalar names/URIs, live class/checked predicates, and navigator edge cases.
Existing Chrome selector controls remain unchanged.

## Benchmark

Run the same public API benchmark on each revision with the harness from this
change:

```sh
cargo +1.98.0 run -p turbo-dom --release --example sibling_ranks
```

Each fixture is parsed before timing, the selector/rank lookup is warmed, and
seven complete sibling sweeps are measured. Every sweep asserts the same exact
positive match count. Timings describe warmed repeated matching, not parsing,
first-use rank construction, full layout, or application navigation.

The initial same-host medians (nanoseconds per full sweep) were:

| Siblings | Baseline | Candidate |
| --- | ---: | ---: |
| 256 | 630,916 | 3,916 |
| 512 | 2,430,291 | 7,833 |
| 1,024 | 9,033,375 | 16,041 |
| 2,048 | 35,095,500 | 32,125 |
| 4,096 | 138,527,125 | 64,292 |

The old path scales quadratically; the candidate's warmed sweeps scale linearly.
Both release binaries and repeated samples are retained locally under
`.evidence/sibling-ranks/`, alongside the failing allocation test and gate logs.
The committed example and tests are the portable reproductions. Timing is not
used as a flaky pass/fail test.

## Validation

- Full workspace tests with browser-env `host-internals,test-support`: **3,183 passed,
  0 failed, 13 ignored**, across 226 summaries. The additional ignored test is the
  isolated counting-allocator gate. The subsequently added uncached-reference unit
  test also passed, alongside the final four integration tests and the core coverage
  run. It compares cached ranks against the retained default Navigator algorithm
  across mixed exact element identities, detached nodes and reordering.
- Strict workspace/all-target clippy with the same features and `-D warnings`: passed.
- Workspace formatting, Rust complexity at the unchanged maximum 9, version
  agreement, one-V8 dependency graph and diff checks: passed.
- Core LLVM tarpaulin, offline/locked, CI source filter and 600-second timeout:
  **100.00%, 13,433/13,433 lines**, unchanged `--fail-under 100`.
  The rank module itself is 42/42. The first coverage run exposed previously
  covered default-navigation paths now bypassed by the optimization; the independent
  uncached-reference test restored coverage without exclusions or floor changes.
- Final isolated allocation regression: **0 allocations / 0 bytes**.
- Existing isolated churn gate: **372 allocations/op, 82,270 bytes/op**, under
  the unchanged 380/85,000 ceilings.

No coverage floor, assertion, complexity limit, execution budget or resource limit
was relaxed. Toolchain: Rust 1.98.0, at most two compilation jobs, unchanged
one-V8 graph (turbo-dom-v8 150.4.2 at 03839d09).

Six paired ABBA/BAAB blocks (two runs of each binary per block, seven timed
sweeps per run) reproduced the scaling result after the builds/gates finished.
Median paired speedup at 4,096 siblings was 2232.5x. This is a synthetic warmed
selector result, not an end-to-end application speedup. Binary SHA-256:

- baseline: `0c251156270a2c901b57fc9b32fa23759f0894ca2925d28081e60a2aa672bc7f`
- fixed: `52eac90fe8763defdf267fd545f487f87ffcf1f15916efe9d509ab16d102bf47`

Implementation checkpoint:
`8647e7e6bdc1ada1f701abdee036f8912f4a653f` on
`codex/fix-087-sibling-rank-cache`, based on
`ffbba11985b3d3e2026ffcc7549d8ebe4d4caf81`. Integration uses a normal merge
into `release/0.8.7`; no tag, publication or main merge is part of this fix.

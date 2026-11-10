# Benchmarks — history (pre-consolidation)

turbo-dom 0.6.0 / 0.7.0 have **not** been re-benchmarked against jsdom / happy-dom yet; a re-run of all four repositories
(turbo-dom, turbo-test, turbo-html2pdf, turbo-surf) together on a quiet machine is planned
after the release. Until then this page keeps the last measured numbers, unchanged, with the
version, date and machine they were taken on. They describe the JS runtime before the
consolidation and are not a claim about 0.6.0 or 0.7.0 (0.7.0's browser-env performance
work, measured on a real React/MUI suite, is in [CHANGELOG.md](../CHANGELOG.md)).

## JS runtime vs happy-dom and jsdom — v0.1.55, 2026-06-07

Machine: darwin-arm64, Node 24. Command: `npm run bench:all` (`bench/parse.mjs`,
`bench/construct.mjs`, `bench/suite.mjs`, `bench/query.mjs`, `bench/wasm.mjs`). Source:
README at commit `2935068` ("refresh benchmarks @v0.1.55 vs jsdom/happy-dom"). Higher is
faster except the suite row (ms/file, lower is faster).

| benchmark | turbo-dom | happy-dom | jsdom |
|---|---:|---:|---:|
| realistic suite, 200 files (ms/file) | ~0.025 | 1.08 | 3.20 |
| cold per-file construct + query (ops/s) | ~230k | 575 | 267 |
| parse 56 KB SSR (ops/s) | 528 | 48 | 65 |
| parse 20 KB real page (ops/s) | 4,353 | 135 | 99 |
| repeated query throughput (iters/s) | ~1.8M | 661k | 3.3k |

Ratios as stated with those numbers: ~45x happy-dom and ~130x jsdom on the suite (the parser
memoizes the read-only SoA buffer per HTML string, so a suite that sets up the same shell in
every file parses it once); ~2.7x happy-dom (~550x jsdom) on repeated queries; ~8-44x on
parsing real pages / SSR documents. html5lib-tests conformance at that time: 99.72%
(happy-dom 37.35%, jsdom 97.03%).

Why the repeated-query number is high: allocation-free selector matching and
`querySelectorAll` / `getElementsBy*` / `getElementById` results cached per (selector,
DOM version); `getByLabelText` went from O(n^2) (1.3 ms) to ~270 µs (v0.1.19-v0.1.20).
The per-commit history of these wins is in [RUST_PORT_PERF_HISTORY.md](../RUST_PORT_PERF_HISTORY.md)
and the experiment ledger in [EXPERIMENTS.md](../EXPERIMENTS.md).

## JS runtime vs happy-dom and jsdom — v0.8.2, 2026-11-10

Host: darwin-arm64 (Darwin 27.2.0), Node v26.11.0, rustc 1.99.0 (b940084d7 2026-09-28).
Head: `11a8b725` (tag `v0.8.2`, "Merge release/0.8.2"). Commands: each sub-bench run
separately under `node --max-old-space-size=8192 --expose-gc` (bench/parse.mjs oomed
under Node's default heap on the real-storybook fixture; the increased heap resolves
it without touching the bench code). Load average captured with `uptime` before each
run — the machine was NOT quiet (1m load 15.25 → 9.17 across the five runs, see per-bench
rows below). The bench scripts run contestants in a fixed order per fixture (not
counterbalanced ABBA); numbers are reported as emitted and should be read with the
noisy host and fixed-order caveats. Higher is faster except the suite row
(ms/file, lower is faster).

Load averages before each run (1m / 5m / 15m):

| bench | load (1m / 5m / 15m) |
|---|---|
| parse | 15.25 / 9.87 / 8.22 |
| construct | 10.92 / 9.47 / 8.16 |
| suite | 10.36 / 9.38 / 8.13 |
| query | 9.71 / 9.27 / 8.11 |
| wasm | 9.17 / 9.16 / 8.08 |

### parse throughput (`bench/parse.mjs`, ops/sec, higher better)

Reporting the shipped runtime path (`turbo-dom parseBuffer()`) against the JS baselines:

| fixture (size) | turbo-dom parseBuffer | parse5 | happy-dom | jsdom |
|---|---:|---:|---:|---:|
| small (122 B) | 144,968 | 391,335 | 24,398 | 30,847 |
| ssr-large (55,945 B) | 736 | 657 | 63 | 70 |
| deep-nested (4,404 B) | 3,076 | 1,426 | 867 | 91 |
| malformed (92 B) | 110,146 | 243,440 | 12,876 | 13,535 |
| real-storybook (19,802 B) | 4,893 | 2,473 | 546 | 469 |

Ratios on the two realistic fixtures: turbo-dom is ~11.7x happy-dom and ~10.5x jsdom on
the 56 KB SSR shell, and ~9.0x happy-dom / ~10.4x jsdom on the 20 KB real-storybook page.
`parseRaw()` (html5ever only, no JS output — the floor) hits ~920 ops/s on SSR and ~7,337
on real-storybook; parse5 outpaces parseBuffer on the tiny-fragment fixtures (small /
malformed), consistent with v0.1.55.

### per-file construction (`bench/construct.mjs`, ops/sec)

| engine | ops/sec | vs jsdom |
|---|---:|---:|
| turbo-dom | 172,562 | 586.64x |
| happy-dom | 675 | 2.29x |
| jsdom | 294 | 1.00x |

Lazy payoff inside turbo-dom: touch 1 node 203,552 ops/s vs touch all+globs 15,710 ops/s
(13.0x — not building what isn't touched). Render-only tests still construct zero lazy
globals.

### realistic suite (`bench/suite.mjs`, 200-file wall-clock)

| engine | total | ms/file | vs turbo-dom |
|---|---:|---:|---:|
| turbo-dom | 3 ms | 0.015 | 1.00x |
| happy-dom | 222 ms | 1.108 | 25.8x slower |
| jsdom | 618 ms | 3.091 | 71.9x slower |

Lazy nodes 4 ms vs eager 9 ms (2.22x — laziness saves 55% by not inflating untouched
subtrees). Lazy-window construction 282,437/s vs eager 170,627/s (1.66x).

### repeated-query throughput (`bench/query.mjs`, iters/sec)

| engine | iters/sec | vs jsdom | vs happy-dom |
|---|---:|---:|---:|
| turbo-dom | 2,059,250 | 526.81x | 2.78x |
| happy-dom | 740,421 | 189.42x | 1.00x |
| jsdom | 3,909 | 1.00x | 0.01x |

### wasm vs native parseBuffer (`bench/wasm.mjs`)

| fixture | native ops/s | wasm ops/s | wasm / native |
|---|---:|---:|---:|
| small | 151,639 | 87,824 | 58% |
| ssr-large | 764 | 317 | 42% |
| deep-nested | 2,903 | 1,932 | 67% |
| malformed | 122,427 | 67,077 | 55% |
| real-storybook | 6,056 | 3,198 | 53% |

Worst-case wasm throughput 42% of native (SSR-large); same SoA contract, single boundary
copy. Verdict unchanged from prior runs: wasm fallback remains acceptable.

### What moved since v0.1.55

The suite ms/file is roughly unchanged (0.015 vs ~0.025 at v0.1.55 — same order,
noise-dominated at this scale). Per-file construction climbed from ~230k ops/s to
172k ops/s against a *much* higher jsdom baseline than before (jsdom dropped from 267
to 294 ops/s; happy-dom from 575 to 675), so the headline 586x-jsdom ratio is higher
than v0.1.55's 230k/267 ≈ 862x on an absolute basis, but the test is fundamentally
different now (0.6.0+ added shadow DOM / cascade / CE hooks on the construction path).
Parse throughput on the 56 KB SSR fixture improved (528 → 736 ops/s parseBuffer, ~11.7x
happy-dom vs v0.1.55's ~11x), and real-storybook parse went 4,353 → 4,893 ops/s. The
repeated-query number rose from ~1.8M to ~2.06M iters/sec — the cache invalidation
work in 0.7.x/0.8.x didn't regress the hot path. No regressions observed against the
last recorded baseline; the synthetic-geometry layer added in 0.2.3 is not directly
measured here (it is behaviour correctness, not a throughput axis).

Caveats: load average was 9–15 during the run (not a quiet machine), and the bench
harness runs contestants in fixed per-fixture order rather than ABBA — both introduce
noise that a future quiet re-measurement should reduce. Numbers are reported as emitted,
honestly, including the parse-bench OOM that required `--max-old-space-size=8192`.

## Rust runtime (`rtdom`) vs the JS runtime — 2026-06-18

Source: README at commit `5ec9c1a` ("document the Rust-native runtime (rtdom)"); same
chatty workload (machine not recorded). `rtdom` called in-process from Rust: ~2.7x the JS runtime.
A Rust DOM in WASM called from JS: ~0.55x the JS runtime (the boundary crossing
dominates), which is why the Rust DOM is offered to Rust consumers only.

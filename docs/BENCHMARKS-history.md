# Benchmarks — history (pre-consolidation)

turbo-dom 0.6.0 has **not** been re-benchmarked yet; a re-run of all four repositories
(turbo-dom, turbo-test, turbo-html2pdf, turbo-surf) together on a quiet machine is planned
after the release. Until then this page keeps the last measured numbers, unchanged, with the
version, date and machine they were taken on. They describe the JS runtime before the
consolidation and are not a claim about 0.6.0.

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

## Rust runtime (`rtdom`) vs the JS runtime — 2026-06-18

Source: README at commit `5ec9c1a` ("document the Rust-native runtime (rtdom)"); same
chatty workload (machine not recorded). `rtdom` called in-process from Rust: ~2.7x the JS runtime.
A Rust DOM in WASM called from JS: ~0.55x the JS runtime (the boundary crossing
dominates), which is why the Rust DOM is offered to Rust consumers only.

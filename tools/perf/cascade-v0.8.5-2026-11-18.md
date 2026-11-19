# Cascade allocation experiment, 2026-11-18

Baseline: published v0.8.4 source `c49fd0b1`. Candidate: v0.8.5 local allocation
changes; the adjacent JSON records the exact candidate source hash, toolchain,
binary hashes, output hashes and every sample. Harness commit: `3b8ec2a1`.

Nine alternating ABBA/BAAB blocks, 30 iterations per run. Each matched-override
iteration contains 100 resolutions; fixture preparation is outside timing.

| Pre-matched cascade workload | Median paired wall-time improvement |
| --- | ---: |
| Override one inherited 64-byte custom property | 3.90% |
| Override one inherited 16 KiB custom property | 44.12% |
| Override eight inherited 64-byte theme properties | 14.91% |

All styled-tree and fragment/glyph hashes match. The deterministic resource tests
also cover moving inline strings and avoiding a discarded 1 MiB parent-value copy.

Ordinary report/card/article layout results vary around the noise floor across
repeats. They do **not** establish a general layout or PDF speedup. The retained
change targets avoidable cascade allocations; do not extrapolate these isolated
numbers to whole documents or downstream consumer benchmarks.

A broader borrowed-candidate experiment and a map-reuse variant were rejected
after mixed full-layout results. The retained inheritance path preserves the
original property insertion order. No coverage floor, complexity limit, oracle,
or benchmark accuracy check was weakened.

Reproduce with `scripts/benchmark-cascade-refs.py` after committing the candidate;
see `docs/CASCADE-PERFORMANCE.md`. Use an idle host and review sample spread.

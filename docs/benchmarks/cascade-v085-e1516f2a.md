# v0.8.5 performance checkpoint: e1516f2a

**Assessment: failed healthy-layout performance checkpoint.** All 15 output hashes match v0.8.4. Definite flex layout is 3.04% slower in every paired block; definite grid is 2.17% slower in 14 of 15 blocks. The 100-row report is approximately flat. Styling cards and large inherited-variable overrides improve. These results require investigation of the healthy flex/grid overhead before a performance claim.

Measured runtime candidate: `e1516f2a0bf86e9fa64efc23af5d951da626981e`. Baseline v0.8.4: `c49fd0b10be0d44764f57215f3ca716b75c22c93`. Later release changes are outside this measured revision.

Machine: Apple M5 Max, Mac17,7, 18 physical/logical cores, macOS 27.3 arm64. Toolchain: `rustc 1.98.0 (88d9e12ae 2026-08-18)`; Python 3.15.2. Both builds use default features, identical repository release profiles, no Rust flag overrides, the same build source path, and the same harness (SHA-256 `084fa88e8d148468253d0ea24b606bac8dd1f325e7d2758277a0f62bf1f140d5`). The freshly rebuilt baseline binary is byte-identical to the earlier baseline.

The coordinator parked owned builds, tests and browser jobs. The pre-run process snapshot showed a background Docker VM using about 61.9% of one core and other processes below 6% each. User services were left running. This was a controlled own-workload quiet window, not absolute host idleness; small changes and block spread must be interpreted conservatively.

Run: 2026-11-18T21:49:05.299796+00:00 to 2026-11-18T21:51:08.189737+00:00 (122.89 seconds). Fifteen alternating ABBA/BAAB blocks, 100 iterations per process sample, 60 measured process invocations. Every invocation checks the complete styled-tree and fragment/glyph hashes for all 15 workloads before timing. Separate one-iteration preflight timings are excluded.

Each workload parses its HTML and constructs fonts outside its timing loop, hashes one untimed output, then performs three untimed warmups. Styling/layout and output destruction remain timed. Table medians are medians of 15 block values, each averaging the two process samples for that binary. The paired percentage is the median of candidate/baseline block ratios, so it need not equal the ratio of the two separately reported medians. Positive percentages mean faster; negative percentages mean slower.

| Native workload | Baseline block median (ms/op) | Candidate block median (ms/op) | Paired improvement | Slower blocks |
|---|---:|---:|---:|---:|
| `style_report100` | 1.402 | 1.389 | +1.66% | 3/15 |
| `layout_report100` | 5.158 | 5.173 | +0.19% | 6/15 |
| `style_report10` | 0.183 | 0.179 | +1.40% | 1/15 |
| `layout_report10` | 0.554 | 0.554 | -0.22% | 9/15 |
| `style_cards100` | 1.071 | 1.022 | +3.97% | 0/15 |
| `layout_cards100` | 4.077 | 4.081 | -0.24% | 10/15 |
| `style_article100` | 0.370 | 0.363 | +1.98% | 3/15 |
| `layout_article100` | 2.220 | 2.215 | +0.88% | 5/15 |
| `style_definite_flex100` | 0.565 | 0.553 | +1.49% | 0/15 |
| `layout_definite_flex100` | 1.543 | 1.592 | -3.04% | 15/15 |
| `style_definite_grid100` | 0.578 | 0.563 | +2.01% | 2/15 |
| `layout_definite_grid100` | 1.555 | 1.594 | -2.17% | 14/15 |

The flex layout paired range is −6.44% to −0.69%; the grid range is −6.07% to +0.17%. These are consistent regressions in the two healthy fixtures. Report100 layout ranges from −2.09% to +2.46%, with a +0.19% paired median; this run supports no meaningful report speedup.

The following isolated cascade cases each perform **100 resolutions per iteration**. Their units differ from the document workloads, and their gains are not full-layout speedups.

| Matched-cascade workload | Baseline block median (ms/100 resolutions) | Candidate block median | Paired improvement | Slower blocks |
|---|---:|---:|---:|---:|
| `matched_override_small` | 0.031 | 0.029 | +6.05% | 1/15 |
| `matched_override_payload` | 0.050 | 0.029 | +41.36% | 0/15 |
| `matched_override_theme` | 0.169 | 0.140 | +17.13% | 0/15 |

Whole-process child CPU has a +0.71% paired median improvement. It includes startup, warmup and output hashing and weights all cases together; it does not negate the healthy-layout regressions.

The [admitted JSON](cascade-v085-e1516f2a.json) retains every paired block value and ratio used in these statistics, CPU block values, output hashes, binary fingerprints, build/harness metadata, machine details and the timing-session receipt. Block values average two process samples; individual render durations and individual process samples are not retained by this harness. These measurements cover native styling/layout, rather than PDF generation or jsdom/jsPDF comparisons. Runtime correctness gates remain separate from this fixture-specific output admission.

Reproduce from a checkout containing this harness:

```sh
python3 scripts/benchmark-cascade-refs.py \
  c49fd0b10be0d44764f57215f3ca716b75c22c93 \
  e1516f2a0bf86e9fa64efc23af5d951da626981e \
  --iterations 100 --rounds 15 --jobs 2 \
  --out /private/tmp/cascade-v085-e1516f2a --accuracy-gates
```

The output directory must be new. Use the same idle-workload coordination and inspect all paired samples. No unchanged-code rerun was used to replace this checkpoint.

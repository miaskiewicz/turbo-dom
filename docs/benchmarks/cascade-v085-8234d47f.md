# v0.8.5 experimental performance checkpoint: 8234d47f

**Assessment: failed healthy-layout performance checkpoint.** The experimental single-resolution/early-row-guard change does not establish a speed improvement. Relative to v0.8.4, healthy flex is 3.89% slower (13/15 paired blocks), grid 1.93% slower (12/15), and article layout 2.03% slower (12/15). Do not admit this feature as a measured speedup. All 15 output hashes match.

Candidate `8234d47f1864210e7e0842af02a990005db7ff34` is an unmerged experiment. Baseline is v0.8.4 `c49fd0b10be0d44764f57215f3ca716b75c22c93`. The earlier [e151 checkpoint](cascade-v085-e1516f2a.md) remains unchanged. Different timing sessions are not an isolated paired comparison between these two candidates.

Both binaries use the identical controlled source path, default features, repository release profile, Rust 1.98.0, jobs 2, no Rust flag overrides, and identical harness. The baseline binary remains byte-identical (SHA-256 `0b2f86c89804eb8a8a8cf9c4878410838fb8770ec230adfdf72ce0a02f989933`); candidate SHA-256 is `b3751ce0f86504e1902a9cec15674b592066cc954de3cbfe49a81ed72e7f0197`. Host: Apple M5 Max, 18 cores, macOS 27.3 arm64, Python 3.15.2.

The coordinator parked owned workloads. Surf cargo parent processes 35988 and 44604 were explicitly paused after their children finished; other agents were idle. The coordinator was notified immediately on benchmark completion to resume these parents. User background services were retained; no fresh numerical background CPU snapshot was supplied for this run. This is controlled own-workload quiet, not absolute host idleness.

Run: 2026-11-18T22:17:20.611722+00:00 to 2026-11-18T22:19:30.596275+00:00 (129.98 seconds), 15 alternating ABBA/BAAB blocks ×100 iterations, 60 process invocations. Every invocation verifies all 15 complete output digests before timing. The separate one-iteration preflight is excluded.

HTML parsing and font construction are outside timed loops. Each process hashes one untimed output per workload and performs three warmups; layout/styling and destruction are timed. Each block averages two process samples per binary. Positive paired percentages mean faster; they use the median candidate/baseline block ratio, rather than the ratio of separately reported medians. Individual process and render durations are not retained.

| Workload | Baseline median ms | Candidate median ms | Paired improvement | Slower blocks |
|---|---:|---:|---:|---:|
| `style_report100` | 1.4664 | 1.4368 | +1.74% | 3/15 |
| `layout_report100` | 5.4935 | 5.4553 | +0.50% | 7/15 |
| `style_report10` | 0.1918 | 0.1863 | +1.50% | 3/15 |
| `layout_report10` | 0.5776 | 0.5792 | +0.27% | 6/15 |
| `style_cards100` | 1.1272 | 1.0766 | +3.99% | 2/15 |
| `layout_cards100` | 4.3326 | 4.3425 | -0.66% | 10/15 |
| `style_article100` | 0.3864 | 0.3797 | +3.05% | 4/15 |
| `layout_article100` | 2.3251 | 2.3665 | -2.03% | 12/15 |
| `style_definite_flex100` | 0.5836 | 0.5723 | +1.29% | 4/15 |
| `layout_definite_flex100` | 1.6235 | 1.6739 | -3.89% | 13/15 |
| `style_definite_grid100` | 0.5977 | 0.5893 | +0.79% | 6/15 |
| `layout_definite_grid100` | 1.6257 | 1.6823 | -1.93% | 12/15 |
| `matched_override_small` | 0.0323 | 0.0297 | +7.19% | 0/15 |
| `matched_override_payload` | 0.0519 | 0.0303 | +41.07% | 0/15 |
| `matched_override_theme` | 0.1772 | 0.1471 | +18.06% | 0/15 |

Document workloads use ms/document. Matched-override workloads use **ms per 100 resolutions**, so their microbenchmark gains are not full-layout gains.

Whole-process child CPU paired improvement is -0.21%; this includes startup, hashing and warmups. Report100 layout is approximately flat. Flex/grid regressions remain unresolved; source simplification alone is insufficient evidence of faster execution.

The [admitted JSON](cascade-v085-8234d47f.json) retains all block averages/ratios, hashes, build metadata and the timing receipt. Native layout results do not measure PDF generation, jsPDF or jsdom. No outliers were removed, thresholds changed, or unchanged-code reruns used. Functional correctness gates remain separate.

Reproduce using the identical harness at both refs:

```sh
python3 scripts/benchmark-cascade-refs.py \
  c49fd0b10be0d44764f57215f3ca716b75c22c93 \
  8234d47f1864210e7e0842af02a990005db7ff34 \
  --iterations 100 --rounds 15 --jobs 2 \
  --out /private/tmp/cascade-v085-8234d47f --accuracy-gates
```

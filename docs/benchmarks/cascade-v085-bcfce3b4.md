# v0.8.5 typed-edge performance checkpoint: bcfce3b4

**Assessment: modest measured candidate-v0.8.4 gains, with healthy flex approximately flat.** Report100 layout improves 1.16%, report10 1.38%, and cards 1.59%; each is faster in 14 of 15 paired blocks. Article improves 0.86% (10/15). Healthy flex is +0.03% (8/15), supporting no meaningful flex speedup; grid is +0.53% (11/15). All 15 output hashes match.

Candidate: `bcfce3b4f31824e8b18293d7c795c4d11f8605a9`. Baseline v0.8.4: `c49fd0b10be0d44764f57215f3ca716b75c22c93`. This candidate was measured before integration. The failed [e151](cascade-v085-e1516f2a.md) and [8234](cascade-v085-8234d47f.md) checkpoints remain unchanged. This new candidate-versus-baseline run does not isolate the typed-edge effect from other release changes; cross-session comparisons do not establish causality.

Both revisions were freshly built in the same controlled absolute source path with the exact current 1dadf820/6ee harness (SHA-256 `5d1434af05fee8281bfda2c6cbbcab6a25a732b0cb64921d429b229c80450c3b`), default features, matching release profiles, Rust 1.98.0, jobs 2, and no Rust flag overrides. The old harness baseline was not reused. New baseline binary SHA-256: `9c3993975d4ec98e073cf5b4d300791f54dcabd5bd314eab141390e6c5d4a0a3`; candidate: `3736f64e7680ee51adc3afff4986479ae1cc4e1ed4490ea549345fc400178550`. All 15 default fixture hashes are unchanged by the optional selector, which was omitted for this official run.

Host: Apple M5 Max, Mac17,7, 18 physical/logical cores, macOS 27.3 arm64; Python 3.15.2. The coordinator reverified and paused only its owned Surf Cargo coordinator PID35988 after its child completed normally. That parent was stopped at zero CPU and other agents were idle. User background services were retained; no new numeric background CPU snapshot was supplied. The coordinator was notified immediately at process completion to resume PID35988. This is a controlled own-workload quiet window, not absolute host idleness.

Run: 2026-11-18T22:43:24.634009+00:00 to 2026-11-18T22:45:31.162408+00:00 (126.53 seconds); 15 alternating ABBA/BAAB blocks ×100 iterations, 60 measured process invocations. Every invocation verifies all 15 complete output hashes before timing. Separate one-iteration preflight values are excluded.

Parsing and font construction are outside timed loops. Every process hashes one untimed result per workload and performs three untimed warmups; styling/layout and output destruction remain timed. Each block averages two process samples per binary. Paired percentages use the median candidate/baseline block ratio; they need not equal the ratio of separately reported medians. Positive means faster. Individual process and render durations are not retained.

| Native workload | Baseline block median ms | Candidate block median ms | Paired improvement | Slower blocks |
|---|---:|---:|---:|---:|
| `style_report100` | 1.4619 | 1.4095 | +4.05% | 0/15 |
| `layout_report100` | 5.3542 | 5.3078 | +1.16% | 1/15 |
| `style_report10` | 0.1890 | 0.1861 | +1.47% | 3/15 |
| `layout_report10` | 0.5741 | 0.5676 | +1.38% | 1/15 |
| `style_cards100` | 1.1097 | 1.0652 | +3.94% | 0/15 |
| `layout_cards100` | 4.2787 | 4.2002 | +1.59% | 1/15 |
| `style_article100` | 0.3865 | 0.3704 | +4.58% | 0/15 |
| `layout_article100` | 2.2867 | 2.2718 | +0.85% | 5/15 |
| `style_definite_flex100` | 0.5826 | 0.5682 | +1.44% | 2/15 |
| `layout_definite_flex100` | 1.5917 | 1.5944 | +0.03% | 7/15 |
| `style_definite_grid100` | 0.5928 | 0.5816 | +2.13% | 2/15 |
| `layout_definite_grid100` | 1.5930 | 1.5895 | +0.53% | 4/15 |
| `matched_override_small` | 0.0316 | 0.0296 | +6.59% | 0/15 |
| `matched_override_payload` | 0.0510 | 0.0296 | +41.82% | 0/15 |
| `matched_override_theme` | 0.1727 | 0.1443 | +16.02% | 0/15 |

Document cases use ms/document; matched-override cases use **ms per 100 resolutions**. Their isolated cascade gains are not document-layout gains.

`layout_report100` paired block range: -0.40% to +2.65%.
`layout_definite_flex100` paired block range: -1.94% to +3.24%.
`layout_definite_grid100` paired block range: -2.69% to +6.80%.

Whole-child CPU paired improvement is +1.52%. It includes startup, hash verification and warmups. Small gains require conservative precision; the evidence supports modest report/card improvement and no consistent healthy-layout regression in these fixtures, not a broad or guaranteed speedup.

The [admitted JSON](cascade-v085-bcfce3b4.json) retains all block averages/ratios, build profiles and hashes, host details and timing receipt. No filtering, outlier removal, threshold changes or unchanged-code reruns were used. Functional correctness gates are separate. This measures native styling/layout, not PDFs, jsPDF or jsdom.

Reproduce from the current harness revision:

```sh
python3 scripts/benchmark-cascade-refs.py \
  c49fd0b10be0d44764f57215f3ca716b75c22c93 \
  bcfce3b4f31824e8b18293d7c795c4d11f8605a9 \
  --iterations 100 --rounds 15 --jobs 2 \
  --out /private/tmp/cascade-v085-bcfce3b4 --accuracy-gates
```

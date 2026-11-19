# v0.8.5 healthy flex/grid audit

Baseline: `c49fd0b10be0d44764f57215f3ca716b75c22c93` (v0.8.4).
Candidate: `50ab570db2da89a0a671cfcb7ec510baab4d5d00`.
Harness commit: `2834c6550bc1dc02a22b60bfeb4a597f13ed45a7`.
Toolchain: `rustc 1.98.0 (88d9e12ae 2026-08-18)`; default features, no Rust flag overrides.

100 iterations per sample, 15 alternating ABBA/BAAB blocks. Two samples per binary per block are averaged; improvement is the median paired block ratio. Positive means faster. Styled trees and complete fragment/glyph outputs agree for all 15 cases in every sample. Parsing/font setup and digest generation are outside per-case wall loops; whole-process CPU includes startup/warmup/hash preparation.

Healthy definite flex/grid are approximately neutral: +0.32%/+0.65% paired medians fall inside both positive and negative block spread. This run establishes no consistent multi-percent regression in these fixtures; it does not prove an engine-wide bound or a PDF/jsdom speedup. Other small changes are likewise inconclusive. Large pre-matched variable-override improvements are isolated microbenchmarks, not full-layout gains.

| Case | Baseline median ms | Candidate median ms | Paired improvement | Slower blocks |
|---|---:|---:|---:|---:|
| style_report100 | 1.433936 | 1.433212 | -0.77% | 9/15 |
| layout_report100 | 5.386452 | 5.362544 | +1.11% | 4/15 |
| style_report10 | 0.188694 | 0.188708 | +1.46% | 4/15 |
| layout_report10 | 0.578994 | 0.576749 | -0.53% | 8/15 |
| style_cards100 | 1.127808 | 1.072964 | +4.44% | 1/15 |
| layout_cards100 | 4.299983 | 4.259723 | +0.66% | 6/15 |
| style_article100 | 0.383107 | 0.377615 | +1.19% | 4/15 |
| layout_article100 | 2.293331 | 2.298365 | -0.31% | 11/15 |
| style_definite_flex100 | 0.583544 | 0.572262 | +1.87% | 2/15 |
| layout_definite_flex100 | 1.604911 | 1.585877 | +0.32% | 6/15 |
| style_definite_grid100 | 0.597168 | 0.586302 | +1.55% | 4/15 |
| layout_definite_grid100 | 1.623981 | 1.612535 | +0.65% | 3/15 |
| matched_override_small | 0.031898 | 0.030824 | +3.15% | 0/15 |
| matched_override_payload | 0.051129 | 0.030697 | +39.44% | 0/15 |
| matched_override_theme | 0.173445 | 0.150242 | +13.79% | 0/15 |

## Reproduction

```sh
python3 scripts/benchmark-cascade.py --baseline target/paired-085/baseline --candidate target/paired-085/candidate --iterations 100 --rounds 15 --out target/paired-085/comparison.json
```

`comparison.json` retains paired ratios/raw block samples, binary fingerprints, revisions, platform and harness hashes. `harness/` preserves the exact source used. Build/preflight logs and JSON are retained alongside. The preflight one-iteration samples were discarded and never used in these claims.

# Healthy flex/grid CPU sampling

Four 10-second macOS `sample` captures compare v0.8.4 c49fd0b1 and v0.8.5 e1516f2a,
using identical release-build harness sources and fixed 100-item flex/grid inputs.
Each selected loop performs 100000 iterations; sampling starts one second after
launch, then the owned process is terminated after capture. Other repository
builds may run concurrently. **No elapsed time from this exercise is admitted.**
The original paired wall-time checkpoints remain the performance evidence.

All 15 default output digests match the preceding harness and both refs. Every
one of the 15 exact selectors produces its matching single digest; malformed
arguments and unknown selections fail. The default fixture order is unchanged.

| e151 hotspot | Flex self | Grid self | Flex inclusive | Grid inclusive |
|---|---:|---:|---:|---:|
| `edge_lengths` | 103/6862 (1.50%) | 122/6896 (1.77%) | 392/6862 (5.71%) | 408/6896 (5.92%) |
| `frames::style` | inline/unattributed | inline/unattributed | 164/6862 (2.39%) | 156/6896 (2.26%) |

Inclusive rows overlap; they must not be added. The baseline inlines edge parsing
differently, so its missing `edge_lengths` symbol does not demonstrate zero cost.
Allocator operations, string comparisons and text work dominate self stacks in
both refs. These single captures identify concrete setup work, but do not prove
which change explains the paired regressions or predict an attainable speedup.

A next bounded candidate could preserve typed padding/margin edges during existing
BoxStyle resolution and reuse them for solver conversion. The safe invariant is:
for unchanged ComputedStyle and resolved font size, typed LengthPct edges are
independent of containing-block width. Percent/calc must remain typed until actual
layout, Auto must survive, and logical/physical/shorthand/UA precedence must stay
identical. Font-dependent styles require the current resolution context. This is
a proposal, not an implemented or admitted optimization. The unsuccessful 8234
clone-removal experiment remains unmerged.

`summary.json` retains counts, raw hashes, binary/harness fingerprints and capture
limitations. Compressed `.sample.txt.gz` files preserve full original symbolized
captures; `sample-receipts.json` records successful sampler exits. Reproduce using
the documented optional selector, then `sample PID 10 1 -file profile.txt`.
Terminate only the owned long-running loop after capture.

The rejected clone-removal experiment is preserved as [rejected-8234d47f.patch](rejected-8234d47f.patch), including its parent identity, so it can be reconstructed without an unpublished local branch. It remains unmerged.

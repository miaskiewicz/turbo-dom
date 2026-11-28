# Packed Rust CSS hotspots after projection and envelope integration

The main remaining packed-suite cost is the JavaScript preparation around the
Rust engine. Native Rust still offers useful CPU reductions, but optimizing only
the N-API string conversion is unlikely to close the packed gap.

This profile uses production commit `b2eed4ed7bd2a4bb6c4d7c6afa7fb71d7e287298`
and optimized N-API artifact
`4471a11648079ec38bb340f958862431c6d8d4d13ce0397a2ffd080aaf52d848`.
Documentation-only commit `7abf9f36` has identical production code.
The [machine-readable receipt](css-hotspots-accepted-b2-2026-11-28.json) records
identities, samples, diagnostic counts and limitations.

## Measurements

There are three fresh CPU-profiled packed runs for each of JavaScript, ordinary
Rust wire, Rust template, and Rust native-state modes. Every run passed the same
29 tests. Profiles include the CLI and all seven test workers, sampled at 500 µs.
One run passed its tests but did not write the inputs worker profile; it was
preserved, excluded, and repeated. Four unprofiled controls also passed.

Owned competing builds were paused during sampling. The machine still ran desktop
and VM services. These profiles locate work; they are not speed-acceptance results.
Weighted sample milliseconds are estimates, not exact CPU timers. CLI and worker
idle time must not be treated as application CPU.

### Packed test path

Median inclusive sampled time per complete suite:

| Path | Ordinary Rust wire | Rust native-state |
| --- | ---: | ---: |
| `getComputedStyle` | 144.48 ms | 162.88 ms |
| Source capture and preparation | 43.16 ms | 46.98 ms |
| Matching entry construction/reuse | 35.12 ms | 42.95 ms |
| Contextual matching collection, inside matching entry | 27.43 ms | 32.27 ms |
| Shadow/context selector program construction, nested within collection | 14.22 ms | 16.74 ms |
| Recursive own-data wire encoding | 24.33 ms | 24.57 ms |
| Descriptor collection, nested within encoding | 11.89 ms | 9.41 ms |

**Do not sum these rows:** their call paths overlap. Descriptor collection remains
significant, but is not the sole explanation. The earlier 46.5 ms descriptor
measurement is not substituted for these fresh results.

Input tests remain the largest file-level delta: median profiled callbacks were
53.71 ms for JS versus 105.47 ms for ordinary Rust and 114.35 ms for native-state.
Forms measured 29.21 ms versus 55.71/55.90 ms. These are instrumented diagnostics,
not a new benchmark ranking. Testing Library pointer-event checks account for
75.85 ms of inclusive native-state `getComputedStyle` ancestry; changing the
fixture to omit those checks would not be a valid optimization.

### Existing native-call boundaries in the actual packed tests

A separate private hook timed existing calls and counted primitive string lengths;
it performed no additional native calls. It ran after the quiet sampling window,
so its durations are diagnostic and are not used to compare protocols.
All three modes again passed 29/29. The exact native-state run recorded:

| Existing native operation | Calls | Aggregate elapsed time |
| --- | ---: | ---: |
| Compile models | 6 | 4.501 ms |
| Compiled metadata | 6 | 2.672 ms |
| Prepare models | 6 | 0.163 ms |
| Prepared metadata | 6 | 0.230 ms |
| Install matching answers | 14 | 1.191 ms |
| Resolve and project native style state | 118 | 7.756 ms |
| Runtime policy and computed preferences | 12 | 1.093 ms |
| **All measured calls** | **168** | **17.605 ms** |

These timers include N-API conversion and the invoked Rust work. They exclude
argument preparation, recording overhead after the timestamp, and native work
performed before the setup hook (including module initialization). Do not subtract
this separate run from sampled milliseconds to derive an exact exclusive JS cost.

Compiled metadata emitted 279,738 UTF-16 code units across six calls, once per
compiled model. A metadata *cache* would therefore miss the important first-use
cost in this workload; direct serialization is the better first experiment.
The state route emitted 12,194 code units across its 118 resolution calls.

The template hook recorded 14 installations, 93 compact resolutions, and 11
ordinary resolutions. The initial hook had two incorrect template export names;
that incomplete diagnostic is preserved and excluded from the final accounting.

### Native Rust function sample

A six-second macOS native sample used the unchanged optimized addon while
replaying the captured six-model/118-request corpus. The replay includes
compile, metadata, prepare, matching installation, resolve, and disposal. It
excludes DOM capture and JavaScript request encoding. Each of 2,000 iterations
checks its output checksum; exact output and negative controls run outside the
measured loop. Thus this is a mixed cold/warm native workload, not a warm-read-only
profile or an estimate of its share of the entire packed suite.

Of 4,127 main-thread samples, 1,840 were under native style resolution, 1,183 under
model construction, and 759 under compiled metadata serialization. Within the
1,840 style-resolution samples, exclusive ancestry classification found:

| Native phase | Samples | Share of style-resolution samples |
| --- | ---: | ---: |
| Declaration selection/admission | 592 | 32.17% |
| Computed values | 494 | 26.85% |
| Rule matching | 477 | 25.92% |
| Other target resolution | 104 | 5.65% |
| Projection and projection validation | 80 | 4.35% |
| JSON work outside the phases above | 48 | 2.61% |
| Other native work / renderer defaults | 45 | 2.45% |

Allocator self frames accounted for 1,215/4,127 samples (29.44%) across the replay;
these occur *within* the phases above and must not be added to them. The largest
named non-allocator self hotspot was the mechanical-answer `BTreeMap::get`
instantiation at 207 samples. `CssSource::new` had 127 self samples.
Input/output N-API string conversion appeared in 35/4,127 ancestry samples
(0.85%); optimized/inlined symbols limit attribution. This does **not** make the
whole JS/Rust boundary cheap: capture and wire encoding are measured separately.

## Next experiments, grounded in this profile

Each candidate gets an isolated implementation and causal counter test before
packed ABBA validation. Keep a CPU reduction with flat packed timing only when
correctness and existing coverage/complexity gates pass. Do not merge on sampled
percentages alone, weaken limits, or switch the default.

1. **Batch selector dependency validation inside a closed operation.**
   `compileSharedShadowFacts` and contextual matching spend visible time preparing
   programs, including repeated `checkFactsArrayDependencies`/shape checks.
   Try one validation receipt only for a private, synchronous, callout-free
   primitive compilation/matching batch. Continue validating each public entry
   and after any possible guest callout. First count guard invocations per batch;
   verify unchanged selector work and answers, poisoning between operations,
   numeric prototype setters, nested scopes, shadow boundaries, and reentry.
   No cross-read cached permission or blanket intrinsic exemption.

2. **Construction-owned request encoding.**
   Avoid building private primitive request fields and then reflecting them again
   in generic `descriptors`/`ownData`. Try emitting bounded wire fragments alongside
   construction, initially for the closed inheritance-target/context records only.
   Keep every fresh DOM/descriptor/state validation in capture, and retain the
   generic encoder for foreign/custom/proxy-bearing inputs. Require exact wire,
   Unicode/lone-surrogate, error-order, size-limit and mutation parity. Measure
   descriptor visits, allocations, and encoder CPU before packed timing. This is
   different from reusing a stale descriptor map.

3. **Direct Rust metadata serialization.**
   Replace temporary `serde_json::Value`/B-tree assembly in `compiled_metadata`
   with borrowed `Serialize` views. Start with the outer envelope; iterate through
   selector/policy/group rows if allocation counts still dominate. Preserve exact
   field ordering, nulls, escapes, invalid-selector certificates and disposed-handle
   errors. Gate on byte-identical metadata and native allocation/CPU counts.
   Six cold metadata calls cost 2.672 ms in the boundary run, so expect a modest
   packed effect even if this substantially improves its local CPU cost.

4. **Index immutable matching answers for the common route.**
   Investigate a prepared-owner/matching-token-bound index for the common unscoped
   context, avoiding repeated tuple-key B-tree searches. Keep existing scoped,
   slotted and arbitrary-context lookup as the fallback. Preserve exact answer
   domains, missing-answer errors, target membership, decision-budget charges,
   token replacement, disposal and payload caps. First prove fewer searches with
   identical logical decisions; then measure the native 118-request corpus.

5. **Reuse declaration admission evidence, not computed values.**
   `computed_property_tokens` substitutes, constructs `CssSource`, and admits the
   winning value again. Investigate an immutable compiled certificate only for
   unchanged, variable-free, context-independent ordinary declarations that have
   already passed the same admission grammar. Continue computing inherited values
   and units fresh. Pending shorthands, variables, custom properties, contextual
   values, and altered limits retain the ordinary path. Charge current budgets and
   preserve invalid-at-computed-value/error behavior. Count parser/source
   constructions before attempting a cache or changing representation.

Experiments 1–2 address the larger packed-side costs; 3–5 address actual Rust CPU
hotspots and may also help other consumers of the shared core. No implementation
of these five experiments is included in this report.

## Reproduction and durable evidence

Local evidence lives at
`.evidence/css-perf-088/parity-program-18/accepted-hotspots/` in the release
experiment worktree. `run.py`/`run-initial.py`, `analyze.py`, `native-sample.py`,
`analyze-native.py`, `run-boundary.py`, and the private hook/config are preserved.
The receipt records file hashes. Raw CPU profiles and native stacks remain local;
production sources, accepted package bytes and addon were unchanged.

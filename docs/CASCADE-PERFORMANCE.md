# Cascade performance and output parity

The Rust `cascade_perf` example measures styling and full native layout for a
100-row report, a 10-row report, 100 cards with inline styles, and 100 paragraphs.
It parses each document and constructs the font registry outside the timed loops.
Each workload warms its caches before measuring; allocation, output destruction,
cascade and layout work remain inside the relevant measured loop.
Three additional pre-matched cascade cases override inherited CSS variables:
one 64-byte value, one 16 KiB payload, and eight 64-byte theme values. Each sample
iteration performs 100 resolutions; parent/declaration construction is excluded.
These isolate allocation costs and are reported separately from full layout.

Run a comparison of committed revisions from a clean or dirty checkout:

```sh
python3 scripts/benchmark-cascade-refs.py v0.8.4 release/0.8.5 \
  --out /private/tmp/cascade-comparison-085 --accuracy-gates
```

The output directory must be new. The command builds both revisions sequentially
in one disposable worktree, using the same harness copied from this checkout.
That also supports older baseline revisions which do not contain the harness.
It leaves the caller's checkout alone and removes its scratch worktree afterward.
The external `target/`, both binaries, build/test logs and JSON remain under `--out`;
delete that target directory after review when its build cache is no longer useful.
`--target-dir /path/to/cache` optionally reuses an existing Cargo cache instead;
avoid sharing it with a concurrently running build or benchmark.
Rust 1.98.0, Python 3 and a working Cargo dependency cache/network are required.
The timing wrapper uses POSIX child CPU accounting (macOS/Linux).

`--accuracy-gates` runs the candidate's complete all-feature layout test suite,
including its Chrome controls, before timing. Without that flag, full output hash
equality is still mandatory. Neither path edits golden controls or accepts a
changed expected output just to make a benchmark pass.

For existing release binaries of the example:

```sh
python3 scripts/benchmark-cascade.py --baseline /path/to/baseline \
  --candidate /path/to/candidate --iterations 100 --rounds 15 \
  --out /private/tmp/cascade-paired.json
```

Every sample must have identical workload names and SHA-256 hashes of the complete
styled trees and fragment/glyph outputs. Any mismatch fails the command. This
checks these fixtures, not all DOM/browser behavior; retain the regular crate and
Chrome-oracle gates for an actual change.

Each block executes ABBA or BAAB, alternating blocks. It averages each binary's
two samples, then reports the median paired improvement across blocks. Positive
means faster; negative means slower. The report retains all wall-time samples,
child-process CPU samples, binary fingerprints and output hashes. The ref runner
also records source revisions, harness revision/file hashes and Rust toolchain.
Child CPU samples cover the whole benchmark process, including startup, warmup
and output hashing; per-workload wall timings exclude those preparation steps.

Use an idle machine, keep dependency/features/font inputs fixed, and inspect the
spread of paired samples. Small changes inside host noise are inconclusive. A
faster styling phase does not establish faster full layout; review every workload
and repeat meaningful regressions before integrating a runtime change. These are
native layout measurements, not PDF generation or comparisons with jsPDF/jsdom.

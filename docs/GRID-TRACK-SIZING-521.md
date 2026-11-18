# Percentage grid track sizing (#521)

Taffy 0.8.3 resolves a percentage preferred width against the entire grid while
computing an item's minimum contribution. For a 1152px three-column grid with
40px gaps and full-width cards capped at 374px, this produces 1152px track minima
and card positions 0/1192/2384. The native measurement callbacks themselves
reported a 56px intrinsic width. Released Taffy 0.9.2 and 0.10.1 retain this
minimum-contribution implementation; an API-breaking upgrade does not fix it.

The adapter first solves intrinsic tracks with cyclic percentage widths
indefinite, then freezes the actual used columns and gutters. It restores the
original area-relative item sizes for final placement and row-height measurement.
No clipping or zero automatic track minima hide overflow. Explicit placements
retain spans and leading implicit tracks; distributed content spacing is retained.

Validation on Chromium 159.0.8154.6 at widths 1440/768/390:

- The text-free `nested-grid-tracks` component has zero geometry differences and
  zero changed pixels at every width in fresh independent captures.
- The full `resources` fixture has zero selected geometry differences at every
  width. It still has text/image rendering differences: changed pixel fractions
  are 3.14%, 8.13%, and 8.85%. This is not full screenshot parity.
- The source-hashed focused control contains 630 independent cases, all compared
  by default. The original 264 cases retain their independent inputs and geometry.
  Coverage includes auto-fit/auto-fill, weighted fr, fixed/min/max functions,
  implicit tracks, spans, all content alignments and collapsed absolute anchors.
- Auto-repeat counting uses the CSS definite maximum floored by the minimum;
  repeated zero lengths receive a 1px counting floor. Actual sizing separately
  retains the original minimum/maximum/fr functions. Gutters are represented as
  fixed sizing tracks, with collapsed gutters overlapping once and outer gutters
  excluded from the sizing budget. There is no fixed-width compensation.
- Final column placement preserves Chromium's distinct collapsed internal and
  outer boundary anchors, including leading alignment and trailing line offsets.
  Both row and column topology are retained through intermediate solves.
- Grid automatic minima respect absent/auto versus explicit zero. An offline,
  hash-pinned Liberation Serif control distinguishes wrapping min-content from
  max-content and keeps nowrap and explicit-zero cases green.
- Final percentage frames resolve against the actual grid area, including a
  spanning area's positioned gutters, before row contributions are measured.

Regenerate controls with `scripts/grid-track-contributions-chrome-control.mjs`;
run the component harness per `tools/component-parity/README.md`. The harness
retains its independent oracles and assertion thresholds.

The auto-fit gutter correction is tracked in [#525](https://github.com/miaskiewicz/turbo-dom/issues/525); its original six controls now pass in the default gate.

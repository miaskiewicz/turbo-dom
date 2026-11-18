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
- The source-hashed focused control contains 102 independent flex/block, sizing,
  span, implicit-track, auto-repeat and content-alignment cases. The default test
  pins all descendants of the original full-width capped-card reduction (six
  cases). The broader matrix also exposes two separate existing problems:
  `auto-fit` with gaps retains one extra trailing 40px gutter at desktop/tablet;
  explicit minimum widths can be reapplied inside a used grid item and widen its
  descendants. Neither residual is described as passing.

Regenerate controls with `scripts/grid-track-contributions-chrome-control.mjs`;
run the component harness per `tools/component-parity/README.md`. The harness
retains its independent oracles and assertion thresholds.

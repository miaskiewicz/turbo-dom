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

## Native grid capacity

The native adapter supports at most **1000 logical tracks per axis**. This is an
explicit memory limit of this implementation, not a claim about Chromium's much
larger grid limit. [CSS Grid §5.4](https://www.w3.org/TR/css-grid-1/#overlarge-grids)
permits a UA-defined limited grid and specifies how to truncate or reposition
areas outside it. Its suggested `[-10000, 10000]` line range exceeds this native
budget; the specification itself also illustrates a 1000-track implementation.

Counted and automatic repetitions are bounded before allocation or conversion to
Taffy's narrower coordinates. An axis retains its explicit tracks, reserves only
the required negative implicit space within the limit, and uses the remaining
capacity for positive implicit tracks. An area crossing the boundary is clipped;
an area wholly outside is moved to the last track on that side with a span of
one. Children remain in the layout, including when automatic placement fills the
limited grid. Ordinary grids within the capacity retain the existing placement
path. Oversized automatic placements use a bounded occupancy prepass instead of
letting their spans grow Taffy's matrix unchecked.

The overflow prepass uses a 128 kB (125 KiB) bitset. Taffy's logical placement matrix is at
most 1000×1000 cells; the internal gutter-expanded solve has at most 2001 columns
and 2001 rows when both axes require gutter expansion. This bounds both native coordinates and the per-grid occupancy
allocation, including zero-sized auto repeats and extreme author counts/spans.
Native tests exercise large repeated spans, signed out-of-range lines, saturated
counted repetitions, and more automatic items than available rows. These stress
checks test the documented native capacity; the existing Chromium controls keep
their original expected geometry and tolerance.

## Grid height and automatic flow (#534)

The capacity review exposed a separate public mapping omission: the native root
previously discarded definite grid height and `grid-auto-flow`. Additional
offline Chromium controls pin content-box and border-box 100px heights with
padding/borders and auto-fill row gaps, plus sparse/dense row and column flow
(including either keyword order). They join the existing intrinsic-size control
corpus without changing previous expectations.

Grid roots now reuse the flex root's block-owned content-height boundary,
including an actual known containing-block height and converted min/max-height
constraints. Row auto-repeat minimum constraints round upward; definite and
maximum constraints round downward. Row auto-fit collapse and content alignment
reuse the original track functions after column areas and frames are resolved.
The root's frame stays owned by native block layout; it is not
subtracted twice. Automatic-flow keywords reach Taffy's corresponding row,
column, row-dense, or column-dense enum. Oversized column placement transposes
the bounded prepass and restores the original axes afterward.

# Flex row automatic minimums

A font-free 60px row containing an auto-width item with a 100px child must keep
that item's 100px automatic content minimum, overflowing the row. The adapter
previously treated ordinary row `min-width:auto` as zero and shrank it to60px.
The baseline Chromium control also exposes `overflow:clip` and small flex bases.

Restore Taffy's content minimum for non-scrollable row items, retaining explicit
minimums and the zero automatic minimum of scrollable overflow. Preserve the
preferred width in solver style: the automatic minimum must still be capped by
the specified-size suggestion and max-width.

Explicit content flex bases cannot fall back to the preferred width. Their
intrinsic basis is computed independently, including actual typed frames and
replaced-image intrinsic dimensions. A definite height can transfer through an
aspect ratio; the content minimum still applies to a non-replaced item. Numeric
and percentage bases retain their existing semantics.

Forty offline, text-free Chromium159.0.8154.6 controls pin visible/clip/scrollable
overflow, explicit minimums, preferred/max-width caps, content basis, percentages,
frames, ratio transfer, reverse/wrap and an embedded PNG image. The same public
layout test passes without default features. Restoring the old row-minimum policy
causes16 geometry mismatches; restoring the fix passes.

Validation also includes the full layout suite with svg/perf/test-support,
strict all-target Clippy, formatting and changed-source complexity <=9.

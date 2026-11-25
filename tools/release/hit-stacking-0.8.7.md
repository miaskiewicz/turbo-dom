# Context-owned native paint and hit order

Base: `974c0d423e69058e591b9a3422229c939de408f2`.

A relative positioned ancestor with `z-index:auto` previously trapped its whole
fragment subtree below a later outside `z-index:1` box. Its own absolute child
at `z-index:100` should paint above that outside box. The public native reduction
failed hit order while every original rectangle agreed with Chromium159.

The shared `layout::paint::PaintNode` schedule owns paint order separately from
fragment storage/parent indices. Positioned descendants can participate in their
nearest real stacking context. Real contexts stay atomic. Promoted entries replay
the original non-context `clip:rect()` and square/rounded overflow clips in both
PNG and SVG. Geometry ancestors, source paths, transforms, containing blocks,
scroll and text provenance remain in the existing native fragment tree.

## Native API

- `Fragment.establishes_stacking_context: bool` is explicit context evidence beyond
  ordinary positioned-z/opacity/transform classification; `Fragment::new` defaults
  it to false. Rust consumers constructing `Fragment` with a struct literal must
  initialize this added field. It is separate from `is_positioned` (which also
  affects layout/baselines). Flex/grid item z-index is not a positioning sentinel.
- `ComputedStyle::{with_stacking_context,has_stacking_context}` carries native
  per-element metadata outside CSS declarations. Inheritance/anonymous styles do
  not propagate it. Any contributing Animation/Transition origin that establishes
  a context marks it even if an important declaration wins the property value.
- `LayoutSnapshot::paint_order()` returns context-owned back-to-front fragment
  indices. `fragments()` and their parent/source indices retain native storage.
- `PresentedTarget.establishes_stacking_context` is read in the same pinned batch
  from canonical native effect ownership/frames/relevance. Missing wire metadata
  fails decoding instead of becoming a false certificate. No second interpolation,
  geometry callback, lifecycle advancement or guest getters are introduced.
- `live::snapshot_with_stacking_contexts(style,capture,options,&[SourceId],hook,
  inputs,diagnostics)` accepts host-verified context sources, validates their native
  captured document/version/element identity, and applies metadata after the hook
  so a CSS-unit/zoom reconstruction cannot erase it. The host preserves native
  world/realm/tree ownership and supplies only sources in that composed capture.
  The existing `snapshot` API delegates an empty context list.

The canonical flag matters even with no sampled property pairs. Chromium159
creates a context during a non-filling opacity delay for CSS and WAAPI; an applied
CSS animation retains its context after pause/finish/cancel while its CSS rule
remains applied. Script effects follow canonical relevance/fill/replacement.
The host must sample before accepting geometry-cache hits and include this flag
in effect admission/cache identity, or conservatively avoid reuse while it is true.
Unknown presentation still follows the existing strict unsupported path.

## Evidence and verification

- Public baseline native hit RED; z0/opacity-half/identity-transform inverses GREEN.
- Source-pinned `hit_stacking_probe.js` / `hit_stacking_chrome159.json`: sixteen
  Chromium159 controls including plain-vs-constant opacity, important declarations,
  fixed/sticky, isolation, containment, static z and width-only animation.
- Source-pinned `presented_context_phases_probe.js` / corresponding Chromium159
  control: fourteen settled CSS/WAAPI phase observations, consumed by host tests.
- Direct PNG and independently rasterized SVG pixel controls cover the public
  inverse, true context, square/rounded/legacy clipping and empty clip scopes.
- Extra Rust1.99 full layout+raster run: 1,786 passed, zero failed, 166 binaries.
- Extra Rust1.99 canonical host presentation suite: 31 passed, zero failed, including delay,
  ownership, policy, important, one-clock, foreign/adoption and startup snapshot.
- Normative Rust1.98 strict all-target Clippy: green for layout, raster and
  browser-env (`host-internals,test-support`), zero warnings; fmt/CC9 green.
- Normative focused native runtime: 47 passed, zero failed (5 public hit controls,
  3 typed live-context transport controls, 2 origin/identity controls, 5 dual
  painter controls, 31 canonical host controls, 1 strict wire-format control).
  Actual binary hashes and source/toolchain manifests are preserved in
  `.evidence/hit-stacking/198-FOCUSED.json` and `SOURCE-MANIFEST.json`.
- Normative Rust1.98 full workspace and coverage: running/pending at this checkpoint.
  Every normative invocation pins both `RUSTUP_TOOLCHAIN=1.98.0` and
  `cargo +1.98.0`; the shell default is1.99. Its newer preexisting raster test
  lint failure was retained as extra evidence without changing the test/flags.

No app services, fixtures, source or consumer package pins were changed here.
These controls establish producer behavior; they do not claim an application E2E
success or resolve independent unsupported rendering features.

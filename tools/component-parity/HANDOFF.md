# Fix handoff: component layout and raster parity

The standalone current-main **turbo-dom 0.8.4 source build** reproduces these
failures without a turbo-surf addon or browser binding. It was compared directly
with **Chromium 159.0.8154.6** at widths 1440/768/390, height 1000, DPR1. Exact
native/compiler inputs are identified by the executable and Cargo.lock hashes in
[evidence/observed-2026-11-18.json](evidence/observed-2026-11-18.json).

Start with the layout tickets in the order below. The proposed source boundaries
are investigation pointers; where the exact mistake is unproven, do not treat the
hypothesis as a reviewed fix. This PR only supplies test and evidence files.
[README.md](README.md) has build commands, runtime dependencies, fixture
normalization, expected-red semantics and output descriptions.

## Tickets (milestone v0.8.5)

- [layout: parse MUI Grid multiply/divide and nested calc widths](https://github.com/miaskiewicz/turbo-dom/issues/518)
- [layout: preserve pixel offsets in mixed calc flex dimensions](https://github.com/miaskiewicz/turbo-dom/issues/519)
- [layout: nested flex grow counts border-box padding as available content height](https://github.com/miaskiewicz/turbo-dom/issues/520)
- [layout: nested full-width CSS grid gives each fr track the container width](https://github.com/miaskiewicz/turbo-dom/issues/521)
- [layout/raster: partner logo intrinsic sizing and wrapping differ from Chromium](https://github.com/miaskiewicz/turbo-dom/issues/522)
- [browser-env: ImageData function is not constructible and blocks dotLottie playback](https://github.com/miaskiewicz/turbo-dom/issues/523)

## Working loop for another agent

1. Build `cargo build -p turbo-dom-raster --example component_parity`.
2. Run one case: `node tools/component-parity/run.mjs --native target/debug/examples/component_parity --native-only --assert --case nested-flex-padding --widths 1440`.
3. Read the saved request, native box list and `results.json`; inspect the committed desktop PNG triplet.
4. Make a focused change in the owning layout module. Add permanent small Rust regression assertions against the independent Chromium geometry/pixels; keep this harness useful for the full components.
5. Rebuild the example before rerunning. Run fresh Chromium mode for the fixed case, then the full 24-case matrix. The width-parser, mixed-dimension, nested-height and grid cases are independent; one green case does not clear the rest.
6. Follow the repository's normal test/clippy/fmt/coverage/complexity gates. Never update Chromium numbers from native output, relax the floors, clip incorrect layout to hide it, or add website-specific class-name logic.

Geometry comes from `screenshot_element_rects` on the same native static input as
PNG painting, not from a separate JS live-layout consumer. Geometry tolerance is
1px; pixels have a per-channel threshold of 16. The text-free reductions remove
font-antialiasing ambiguity. Fix all descendants, not only matching outer cards.

## Priority 1: MUI calculated widths

The deployed feature/product cards use this author width, with a different item
column count at each breakpoint:

```css
width: calc(100% * 4 / var(--Grid-parent-columns)
  - (var(--Grid-parent-columns) - 4)
  * (var(--Grid-parent-columnSpacing) / var(--Grid-parent-columns)));
```

The isolated `mui-width-expression` fixture expects three 362.65625px-wide items
in a 1152px container with 32px column gaps. Native raster geometry instead gives the
empty-content reductions 104px item widths. Real text changes the fallback's
content sizes, explaining why reproductions need both the reduction and the
actual component markup.

**Owner: turbo-dom-layout.** In `src/layout/value.rs`, `eval_calc_length` tokenizes
an additive sequence of raw length terms; multiplication, division and nested
parenthesized terms are unsupported. Its existing tests explicitly reject
`calc(100% * 2)`. This is a source-confirmed limitation, not a flex-wrap hypothesis.

## Priority 2: mixed percentage/pixel flex dimensions

Replacing the expression with the equivalent
`calc(33.3333333333% - 21.3333333333px)` does not restore parity. Native raster geometry uses
383.9996px rather than Chromium's 362.65625px and wraps the third item to another
row. The missing amount is the pixel offset.

**Owner: turbo-dom-layout, `src/layout/flex.rs::dim` (and `edge`).** The source
explicitly converts `LengthPct::Calc { pct, .. }` to a percentage, discarding its
pixel term. Supporting richer calc parsing alone will therefore leave this bug.

## Priority 3: definite border-box height in nested column flex

`nested-flex-padding` removes all width arithmetic. At 1440px, the outer items and
cards agree with Chromium within the 1px tolerance. Each nested `.body`, however,
is **120px high instead of 56px**; it extends below the 180px card. The difference
is exactly the parent's 64px vertical padding.

**Owner: turbo-dom-layout.** The likely conversion point is
`src/layout/flex.rs::container_style`: it passes the resolved `bs.height` to the
padding-free taffy container size, while `block.rs::definite_content_height`
explicitly subtracts padding/border for border-box heights. `place_one` is the
other boundary to inspect when carrying solved heights into nested layout. This
pinpoints a conversion path for follow-up; no implementation change has been
made or validated here.

The literal-width fixture remains red deliberately. A width-only fix must not
mistake matching card outlines for matching descendant layout.

## Priority 4: nested grid track sizing

The resource section is a CSS grid inside centered column-flex ancestors. Its
isolated `nested-grid-tracks` fixture keeps `width:100%`, `repeat(3,1fr)`, 40px gaps,
and nested full-width column-flex cards capped at 374px. It removes all text,
images, remote fonts and JS.

At desktop width Chromium starts the three cards at **0, 397.328, 794.656px** with
widths about **357.33px**. Native raster geometry starts them at **0, 1192, 2384px**, capped
at **374px**. The native column stride is the entire 1152px container plus its
40px gap. This directly reproduces resource cards escaping off the page.

**Owner: turbo-dom-layout.** Inspect `layout_grid`, `build_grid_leaves`,
`measure_item`/`measure_width`, and the intrinsic constraints fed into taffy's
track solve in `src/layout/flex.rs`. The reduction proves a track-sizing failure;
the exact internal mistake is not yet established. It does not prove a z-index
or stacking-context defect.

## Other component evidence

The frozen partner section reproduces different logo wrapping/intrinsic sizing.
The direct `native.screenshotCapture` path also disagrees with Chromium for the
layout cases, bypassing the shim's screenshot resource planning. This places the
core layout problems below the turbo-surf wrapper. Some logo/resource painting
differences may still involve asset handling and need independent follow-up.

The full page also contains sticky/scroll-driven content and different static vs
hydrated states. Do not attribute all full-page differences to the four isolated
bugs, and do not file a separate stacking bug without a stacking-specific repro.

## Evidence to open first

For each prefix below, open `-chromium.png`, `-native.png`, and `-diff.png` under
[evidence/](evidence/). Pink in the diff is a changed pixel; white is within the
channel threshold.

- `mui-width-expression-1440`: incorrect intrinsic fallback when arithmetic is rejected.
- `mui-width-additive-1440`: lost offset makes the third card wrap.
- `nested-flex-padding-1440`: inner lavender block overflows while the outer card matches.
- `nested-grid-tracks-1440`: later columns lie beyond the viewport.

The deployed component fixtures remain necessary: text, SVG ratios, aspect-ratio
and responsive rules can interact with the reduced bug. Actual partner logo
intrinsics/wrapping are tracked separately and should be reassessed after P1 fixes.

No stacking-context/z-index bug has been independently proven. Wrong track
placement and overflowing descendants already explain several apparent stacking
symptoms. Do not expand a fix into an unrelated paint-order rewrite without a
separate Chromium control.

## Secondary animation scope

The ImageData/Lottie capability issue is tracked separately in v0.8.5. There are
no animation dependencies or assets in this PR. Prioritize flex/grid fixes and
harnesses; do not make this work depend on completing animation playback.

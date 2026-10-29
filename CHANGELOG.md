# Changelog

All notable changes to `@miaskiewicz/turbo-dom` — a native (html5ever + Rust) DOM/window
environment for vitest/jest. Format based on [Keep a Changelog](https://keepachangelog.com/).
Early versions were released as lightweight tags / version-stamped commits (no per-release notes
at the time); this file reconstructs them from history.

## [Unreleased]

## [0.6.0] — turbo-dom consolidation

turbo-dom becomes the home of the shared browser engine of turbo-html2pdf, turbo-test
and turbo-surf: five published crates instead of copies in three repositories (see
[docs/CONSOLIDATION.md](docs/CONSOLIDATION.md) for the architecture, dependency graph
and publish order).

| crate | version | from |
|---|---|---|
| `turbo-dom` (+ npm `@miaskiewicz/turbo-dom`) | 0.6.0 | this repository |
| `turbo-dom-v8` | 150.4.0 | rusty_v8 150.4.0 + turbo-surf's reviewed fixes (generated, not vendored) |
| `turbo-dom-layout` | 0.6.0 | turbo-html2pdf-core 0.3.9's layout half, turbo-surf's CSSOM-View modules |
| `turbo-dom-raster` | 0.6.0 | turbo-surf-raster 0.5.5 |
| `turbo-dom-browser-env` | 0.6.0 | turbo-test 0.4.8's rtdom <-> V8 binding |

The moves themselves were behaviour-preserving (consumer corpora byte-identical) except
the two approved Chrome-matching changes: one Chrome 156-correct selector engine for the
DOM APIs and the layout cascade, and turbo-surf static rendering of `t:*` elements as
unknown inline elements. Every later output change is a bug fix backed by a Chrome 156
control and declared in `tools/parity/expected/` (the entries below). MSRV: 1.89 for
turbo-dom, -layout and -raster; 1.91 for turbo-dom-v8 and -browser-env.

### CI — gates for 0.6.0
- Coverage is blocking: tarpaulin `--fail-under` per crate (turbo-dom 100, turbo-dom-parser
  `src/core.rs` 100, turbo-dom-layout 99.8, turbo-dom-raster 100 in its default / avif /
  jxl configs, turbo-dom-browser-env 97.6).
- Cyclomatic complexity (<= 15 per function; Rust via cc-check, JS via oxlint over `src/`
  and `crates/turbo-dom-browser-env/src`) runs report-only (`continue-on-error`) until the
  post-release refactor clears the backlog.
- PR and push CI run on Linux and Windows; the macOS (aarch64-apple-darwin) builds and
  jobs run only in the release workflow (`ci.yml` called with `darwin: true`).
### Fixed — JS-runtime `Range` boundary points (miaskiewicz/turbo-dom#207, Chrome 156 controls)
The `./runtime` Range now follows the DOM boundary-point algorithms instead of only the
single-container case. `extractContents`, `cloneContents`, `deleteContents` and `toString`
handle ranges across containers and inside Text/Comment nodes. `insertNode` splits a Text
start and moves the end offset. `surroundContents` throws `InvalidStateError` on a partially
selected element. `setStart`/`setEnd` throw `IndexSizeError`/`InvalidNodeTypeError` and
reorder the range, and `compareBoundaryPoints`, `comparePoint`, `isPointInRange`,
`intersectsNode` and the `START_TO_START`... constants are added. The range is still not
live: mutations made outside its own methods do not move its boundary points.

### Fixed — layout grid auto-repetition, forced page breaks and `opacity` (miaskiewicz/turbo-dom#201/#204/#206, Chrome 156 controls)

- `grid-template-columns/-rows: repeat(auto-fill | auto-fit, ...)` creates as many tracks as
  fit (`auto-fit` collapses the empty ones); it was one `auto` track (#201).
- `break-before/-after: left | right | recto | verso` force a page break (they were dropped),
  the legacy `page-break-before/-after` (`always | left | right | avoid`) and
  `page-break-inside: avoid` are read when the standard property is unset, and
  `break-inside: avoid-page` avoids a break. Keywords are case-insensitive (#204).
- CSS `opacity` reaches the galley (`Fragment::opacity`, `BoxStyle::opacity`). A box with
  `opacity < 1` or a `transform` paints in its stacking context's `z-index: auto` layer
  (`Fragment::stacks_as_positioned`), so it lands over an earlier `position: relative`
  sibling. turbo-dom-raster paints the box and its subtree as one transparency group in
  both the PNG and the SVG painter (#206). `Fragment` gains the public `opacity` field.

### Fixed — view-transition pseudo-classes are valid selectors (miaskiewicz/turbo-surf#117)
`:active-view-transition`, `:active-view-transition-type(<ident>#)` and `:only-child` after
the named `::view-transition-*` pseudo-elements now parse as in Chrome 156 (never matching:
no view transition runs). Google's consent stylesheet used them, so `matches()` /
`querySelector*` threw "unknown pseudo-class" and turbo-surf's live layout failed.

### Fixed — browser-env setup survives a termination during install (miaskiewicz/turbo-dom#184)
A `TerminateExecution` (an embedder's watchdog) landing while the document is being installed
no longer panics at the internal-namespace lookup (`range_contents`). The new
`browser_env::try_install_with_html` returns `Err(SetupTerminated)` when setup was cut short,
so the embedder can discard the incomplete context.

### Fixed — oversized DOM strings throw instead of aborting (miaskiewicz/turbo-dom#183)
A DOM string past V8's length limit (`unit_string`, and page-derived UTF-8 strings such as
href components, `document.domain`, serialized CSS and XPath data) now throws
`RangeError: Invalid string length`, V8's own error, instead of panicking the process.

### Fixed — layout cascade expands the `font` shorthand (miaskiewicz/turbo-dom#176)
The layout cascade now expands `font` (via turbo-dom's shorthand grammar) into its
longhands at the shorthand's cascade position, so `font: 16px/20px serif` lays lines out
20px apart like Chrome 156 (it was ~19.4px: size, line-height and family were ignored).
An invalid `font` value contributes nothing.

### Fixed — browser-env engine JS ignores page-replaced builtins on the insertion path (miaskiewicz/turbo-dom#172)
- The bootstrap's document helpers, element creation, traversal, event construction, event
  handler IDL attributes and invoker activation use intrinsics captured at install time,
  so a page that replaces `Object.defineProperty`, `Function.prototype.call`,
  `Array.prototype.includes`, `RegExp.prototype.exec`, … still creates, inserts and
  dispatches like Chrome 156. Inline handler compilation sets `on*` through the setters
  captured at install (`captured_intrinsics.js`). The CSSOM sheet parser (`<style>`
  insertion) dispatches through uncurried captured members and grows its lists through own
  definitions (miaskiewicz/turbo-dom#175).

### Fixed — browser-env tasks of an inert document run on its creating window (miaskiewicz/turbo-dom#171)
- `<details open>` parsed into template contents queues its toggle task like Chrome 156.

### Fixed — browser-env interface constructors called without `new` throw Blink's messages (miaskiewicz/turbo-dom#173)
- "Illegal constructor" for interfaces without a constructor, "Failed to construct 'X':
  Please use the 'new' operator, …" for the others (was V8's "Constructor requires 'new'").

### Fixed — browser-env caret moves step through table edges and cells (miaskiewicz/turbo-dom#144)
- `Selection.modify('move', …, 'character')` stops on the positions before and after a table
  (`BODY:1`, `BODY:2`) and in its cells like Chrome 156, instead of sticking at the text
  next to the table.

### Fixed — layout white-space preserves spaces and newlines like Chrome 156 (miaskiewicz/turbo-dom#120, #121, #122, #166)
- A preserved newline (`pre` / `pre-wrap` / `break-spaces` / `pre-line`) is a forced line
  break; `pre-wrap` / `break-spaces` spaces no longer collapse (a space-only line is a line
  box); a `pre-line` newline between blocks is a line; a white-space-only `nowrap` run
  renders nothing. `InlineRun` gains `preserve_spaces` / `forced_newlines`, and the UA
  sheet gives `pre, listing, xmp, plaintext` `white-space: pre`. Control:
  `scripts/html-document-chrome-control.mjs` cases `ws-*`.

### Fixed — layout cssom_view caret and viewport extents follow Chrome 156 (miaskiewicz/turbo-dom#127, #147)
- `caret` descends from the hit element with untransformed positions, a point in the gap
  below a block lands on the next block's first line, and an element without text takes
  the caret itself (offset 0, empty rect).
- `viewport_metrics` ignores a block's (or float's) inline-end margin and a transformed
  box's untransformed rect; `scroll_metrics` ignores an absolute box's untransformed rect.

### Fixed — layout table rows carry `break-inside: avoid` in their break hints (miaskiewicz/turbo-dom#158)

### Fixed — layout cascade: `@supports` <general-enclosed> is false (miaskiewicz/turbo-dom#123, Chrome 156 control)
- A parenthesised `@supports` condition that is neither a `property: value` declaration nor a
  nested condition (`(not foo)`, `((not foo))`, `(not)`, `(foo)`, `(not foo: bar)`) is
  <general-enclosed> and evaluates false, so `not (not foo)` / `not (foo)` now hold; text that
  is not one group (`(not a) (b)`) does not parse and drops the rule. Control:
  `scripts/html-document-chrome-control.mjs` case `supports-general-enclosed`.
### Fixed — browser-env reflections, ::slotted() admission, deferred zoom, inert sheets (miaskiewicz/turbo-dom#118, #143, #148, #162, #163, #165, #168, #169, #174)
- `HTMLBaseElement.target`, HTMLMediaElement's booleans (`controls`, `autoplay`, `loop`, `defaultMuted`,
  `disableRemotePlayback`, video `playsInline`) and `name` on img / embed / object / iframe / map /
  fieldset / a / details reflect their content attributes; `muted` is media state; new
  `HTMLParamElement`; `Text.prototype.wholeText`.
- After `::slotted()` only Chrome 156's pseudo-elements are admitted, and no pseudo-class.
- `zoom: var(--z)` / `env()` declarations are kept; a `<style>` in a document without a browsing
  context gets a sheet and `Document.prototype.styleSheets` lists it.

### Fixed — browser-env getKeyframes() spells a shorthand member from its longhands (miaskiewicz/turbo-dom#167)
- `animations.rs keyframe_value` serializes a shorthand through `rtdom::css_shorthand` the way
  Chrome 156 does (`flex: '0'` is `0 1 0%`, `margin: '0'` is `0px`); var() values keep their text.

### Fixed — CSSOM shorthand / longhand spelling of math functions and grammars follows Chrome 156 (miaskiewicz/turbo-dom#117)
- `rtdom::css_shorthand` and its npm port `css-shorthand.mjs` (in lockstep): `abs()` / `sign()` /
  `min()` / `max()` / `clamp()` (`none` bounds) simplify when Chrome resolves their units, single
  times / angles resolve to `s` / `deg`, math functions are typed (`calc(min(1px) * min(2px))`
  and `min(1px 2px)` are invalid) and keep their arguments as sums (`min(1px, 1em + 1px)`).
- `cubic-bezier()` takes four numbers, `linear()` resolves math stops; `hsl()` rejects a hue that
  is not a number / angle; relative colors check their channels; `text-shadow` puts the color
  first; `grid-template-areas` tokenizes its strings (trash tokens, `.` runs, rectangles);
  `place-content: baseline <justify>`; `offset` / `offset-rotate` reject a unitless `0` angle and
  bad `path()` data; `columns` ignores `column-wrap`; `timeline-trigger` lists of different
  lengths have no shorthand spelling. The binding picks the longhand grammars up from rtdom
  (`transform: scale(sign(-1px))` there is turbo-dom#170).

### Fixed — browser-env manual slots re-entering a shadow root signal like Chrome 156 (miaskiewicz/turbo-dom#133)
- A slot inserted into a manual-assignment root no longer fires slotchange (Chrome signals only
  the removal, and named roots still signal an inserted slot); a slot leaving a manual root
  signals whenever assign() recorded nodes for it. Chrome's stale `assignedSlot` of a removed
  manual slot (Blink lazy recalc, order-dependent) stays a documented KNOWN_GAP.

### Fixed — browser-env Document pre-insert validity counts the current children (miaskiewicz/turbo-dom#132, Chrome 156 control)
- `dom_mutations` validated a Document parent against the simulated final child list, so
  `d.append(d.documentElement)`, `d.replaceChildren(d.doctype)` or `d.replaceChildren(element)`
  succeeded. As in Chrome 156 (Blink `Document::CanAcceptChild`), the current children count: a
  single node that is already a child counts twice, replaceChildren keeps the children it will
  remove, and only the replaced child is left out (replaceWith / replaceChild). `moveBefore`'s
  pre-move check counts the moved element too.

### Fixed — browser-env CSSOM at-rule cssText follows Chrome 156 (miaskiewicz/turbo-dom#134)
- `@keyframes` (also `@-webkit-keyframes`, now type 7): one keyframe per line, normalized keys
  (`from` → `0%`, `50.0%` → `50%`), a string name serialized as an identifier; keyframe blocks
  drop `!important` and the `animation-*` declarations Blink ignores there.
- Grouping rules (`@media`, `@supports`, `@container`, `@scope`, `@starting-style`, `@layer`
  blocks) serialize one child rule per line (`{\n}` when empty); empty descriptor blocks are `{ }`.
- `@font-face` `font-feature-settings` / `font-variation-settings` validate and serialize like
  Chrome; `@counter-style` rejects reserved names; `@layer a, b;` statements are rules.

### Fixed — turbo-dom-layout CSSOM-View scroll/caret reads follow Chrome 156 (miaskiewicz/turbo-dom#124, #125, #126)
- `cssom_view::scroll::overflow` keeps `clip` beside a scrollable axis (`overflow:auto;
  overflow-y:clip` computes auto/clip and the clip axis does not scroll; it was `hidden`).
- `cssom_view::scroll::scroll_metrics`: that clip axis of a scroll container still counts
  in-flow end margins and the scroller's end padding in scrollWidth/Height (#180).
- `cssom_view::scroll::expose`: `scrollIntoView({block/inline: 'center'})` centers a target
  larger than the scrollport (it kept Chromium's old no-scroll special case).
- `cssom_view::text::caret` lands in `visibility: hidden` text like `caretPositionFromPoint`.
- New Chrome 156 control `scripts/cssom-view-chrome-control.mjs`
  (`crates/turbo-dom-layout/tests/fixtures/cssom_view_chrome156.json`); its remaining caret
  gaps (transformed lines, carets in textless elements) are miaskiewicz/turbo-dom#127.

### Fixed — a `var()` nested in `calc()` is a pending substitution (P5-cov, miaskiewicz/turbo-dom#115, Chrome 156 control)
- `rtdom::css_shorthand::longhand_value` / shorthand expansion rejected a value whose `var()` /
  `env()` / `attr()` sits inside another function or a parenthesis block
  (`width: calc(var(--a) + 1px)`: `""`, Chrome keeps `calc(var(--a) + 1px)`). The substitution
  scan now reads every nested block to its end.

### Tests — turbo-dom line coverage back to 100% (P5-cov step 4)
- New CSSOM control rows (`scripts/make-css-{shorthand,longhand}-cases.py`, Chrome 156; the
  probe also reads a shorthand's `getPropertyPriority`), `tests/css_shorthand_api.rs`
  (`is_shorthand` / `shorthands_of` against Chrome's table, `shorthand_value` of a longhand,
  embedder-admitted values), and `css_shorthand`'s provably dead branches removed or
  `unreachable!()`: 98.98% → 100% (11907/11907). Rows Chrome spells differently in both
  `rtdom::css_shorthand` and `src/runtime/css-shorthand.mjs` are `KNOWN_GAPS` under
  miaskiewicz/turbo-dom#117.

### Fixed — elementsFromPoint on overflowing text (C2, miaskiewicz/turbo-dom#146)
- `cssom_view::hit::elements_at_point`: text hits its inline ancestors only (passing over a
  `display: contents` one) up to the first block-level ancestor, which only its own box hits,
  so text overflowing its block no longer hits the block (Chrome 156 control
  `scripts/cssom-metrics-chrome-control.mjs`, case `hits-overflowing-text`).

### Tests — turbo-dom-layout coverage (C2)
- `scripts/cssom-metrics-chrome-control.mjs` -> `crates/turbo-dom-layout/tests/fixtures/
  cssom_metrics_chrome156.json`: offsetLeft/Top/Parent, offset/client sizes, client rects and
  `elementsFromPoint` asserted through `cssom_view::{metrics, inline, hit}`; the `Composition`
  scope graph; turbo-dom-layout's CI coverage floor rises from 90 to the measured value
  (open differences held out as miaskiewicz/turbo-dom#128, #142).

### Tests — parity checkpoint `b-fixed`
- `tools/parity/manifests/b-fixed`: every corpus (html2pdf, html2pdf-svg, html2pdf-extra, surf,
  surf-extra) after the bug-fix round, at turbo-dom f860fea / turbo-html2pdf 55f623a /
  turbo-surf 27ed802. It equals `b-main` plus exactly the 883 declared changes of
  `tools/parity/expected/*.json` (compare: 0 failures), so later work compares against it directly.

### Changed — the workspace is rustfmt-clean, and CI keeps it so (miaskiewicz/turbo-dom#19)
- `cargo fmt --all` over the workspace (formatting only; no code change), and a `cargo fmt --all
  --check` step in CI's guards job. `scripts/verify-copied.sh current` lists the turbo-dom#112 /
  turbo-test#125 / turbo-dom#113 edits of `dom_ownership.rs` and `css_live_tests.rs` and the two
  new ownership test modules, which the bug-fix rounds had left unlisted.

### Fixed — getComputedStyle reuse follows CSSOM edits made in another realm (L1, miaskiewicz/turbo-dom#113)
- The CSSOM epoch the turbo-test#125 reuse key reads was kept per realm, so a live declaration
  one realm returned for another realm's element (a parent reading an iframe's element, or the
  reverse) stayed stale after that document's sheet was edited: the edit runs in the realm
  owning the sheet and bumped only that realm's epoch (turbo-surf
  `computed-style-owner.test.mjs`: `61px` instead of Chrome's `67px`). The epoch is now native
  (`__cssomEpoch` / `__bumpCssomEpoch`) and shared by every realm on the thread. The other key
  inputs were already realm-independent: the tree version, top layer and font context are read
  natively from the element, and ancestor styles are re-checked through the same key. Repeated
  reads of an unchanged element still scan no sheets (`css_live_tests`
  `computed_style_reuse_follows_cssom_edits_in_another_realm`).
- tools/parity: conformance `58-object-fit-cover` png/svg are no longer declared by turbo-dom#59
  and turbo-dom#107 — #107 returns them to the `b2-surf` bytes, so `compare` reported both as
  stale.

### Changed — getComputedStyle reuses an element's computed style until an input changes (K1, miaskiewicz/turbo-test#125)
- turbo-dom-browser-env's `getComputedStyle` recomputed the whole cascade — every sheet's
  rules matched against the element and, for inheritance, every ancestor — on each call
  and on each property read of a returned declaration (five reads of an unchanged element
  rescanned the document's sheets 315 times). An element's computed style is now reused
  until something it was computed from changes, keyed on the element's tree version (a new
  `__computedStyleVersion` native: any DOM change), a CSSOM epoch (every stylesheet, rule,
  declaration, disabled-sheet, linked / imported sheet and `adoptedStyleSheets` change),
  its owner document, its top-layer state and font context, and the ancestor styles it
  read. A computation that depended on something outside that key is never reused: a rule
  whose selector reads live form / popover / dialog / view-transition / interest state and
  could match, an animated element, the fullscreen element (viewport-sized) and an
  `@import` with a media query. Every call still returns its own declaration. Values are
  unchanged (all Chrome 156 controls pass); `css_live_tests` / `computed_defaults_tests`
  pin the reuse (work count) and each invalidation.

### Fixed — `pin_callback_owners` no longer allocates per native call (miaskiewicz/turbo-test#127)
- turbo-dom-browser-env's `pin_callback_owners`, which runs before every native DOM method and
  setter to keep the call's owner documents alive, built a `HashSet` and a `Vec` each time and
  showed up high in profiles of a React suite. It now deduplicates owners in a fixed array and
  pins each document by localizing it in the call's handle scope: no heap allocation (test:
  `dom_ownership_alloc_tests`, a per-thread counting allocator; 200 -> 0 allocations per 100
  calls), the same documents pinned.

### Fixed — DOM ownership bookkeeping is linear on large trees (miaskiewicz/turbo-dom#112)
- turbo-dom-browser-env reconciled DOM wrapper ownership after every mutating native call,
  walking every live wrapper (and rebuilding a V8 Map per ownership component), and swept every
  registered listener and the document list each time, so building or mutating an n-node tree
  was O(n^2) (a React suite spent 94% of a worker in `reconcile_all`). Past 256 live wrappers the
  reconcile now runs once per `wrappers / 32` mutating calls, with the deferred calls'
  receivers and arguments pinned so an attached wrapper (and its expandos) cannot be collected
  before it; hosts settle the rest with the new `flush_ownership(scope)` at their event-loop
  turn. Smaller trees keep the exact per-call path. The listener/document sweeps run only after
  a GC (`install` registers a GC epilogue per isolate). A reconcile now localizes wrappers in its
  own handle scope, so one run from host code no longer keeps detached wrappers alive. Approach
  from miaskiewicz/turbo-test#105 (ethan); tests count wrapper visits and sweeps
  (`dom_ownership_scale_tests`).

### Fixed — undecodable images paint nothing, truncated AVIFs are broken (P5b-G1, miaskiewicz/turbo-dom#107, Chrome 156 controls)
- An image whose pixels do not decode paints nothing (miaskiewicz/turbo-dom#107): an
  image Chrome 156 loads and sizes but cannot paint — a frame-corrupt AVIF, a PNG cut
  inside `IDAT`, a GIF with an empty LZW stream — painted turbo-dom-raster's grey
  (220,220,220) placeholder; both painters now leave the box empty (the backdrop shows),
  as Chrome does. The placeholder stays for an image the caller did not supply and for an
  AVIF / JPEG XL built without its decoder feature.
- A truncated AVIF is broken (miaskiewicz/turbo-dom#107): `image::probe` (so layout, the
  PDF and the raster painter) and `image::probe_natural` sized an AVIF from its `ispe`
  alone, so one cut inside `mdat` laid out at its full size. An AVIF now loads once its
  primary item's `iloc` extents lie within the data — Chrome errors (naturalWidth 0)
  before that, and loads while only the alpha item is incomplete. Controls: the AVIF
  prefixes (`formats/q.avif`, `formats/q-alpha.avif`) of
  `scripts/image-truncation-chrome-control.mjs`; the `undecodable-*` cases of
  `scripts/raster-img-chrome-control.mjs`.

### Fixed — turbo-dom-layout intrinsic widths, floats, inline-blocks, column flex, the initial containing block and the fullscreen element (P5b-W7, Chrome 156 controls)
Each fix is backed by a Chrome for Testing 156 control and lists the corpus outputs it
changes in `tools/parity/expected/p5b-w7-turbo-dom-*.json`.
- Intrinsic widths count margins (miaskiewicz/turbo-dom#79): a box's min-/max-content
  width left out its children's horizontal margins, so a float, a `width: auto`
  absolutely positioned or inline-block box and an auto table cell were narrower than
  Chrome's by the child's `margin-left + margin-right` (the child overflowed). Each in-flow
  child (and each atomic inline) now contributes its margin box: signed (a negative
  margin shrinks it, the box never below 0), `%` and `auto` margins counting 0. Every
  layout path. Control: `intrinsic-margins-*` in `scripts/html-document-chrome-control.mjs`.
- Inline-blocks are formatting-context roots (miaskiewicz/turbo-dom#80): an
  `inline-block` dropped its first / last child's vertical margins (it was as tall as the
  child's border box, the child at its top). It now lays its content out as a block
  formatting context (CSS 2.2 §8.3.1), so those margins stay inside it, as for a float.
  Every layout path. Control: `inline-block-margins-*` in `html-document` and
  `live-geometry`.
- The initial containing block on the static path (miaskiewicz/turbo-dom#81): on the
  screenshot / `layoutBoxes` path a `position: fixed` box, and an absolutely positioned
  box whose containing block is the initial containing block, resolved `bottom` (and `%`
  insets and a `%` height) as if that block had no height — it landed at its static
  position (y = 0). New entries `layout_in_viewport` and `layout_html_tree_in_viewport`
  take the viewport `(width, height)`: the initial containing block has the viewport's
  height, so such a box anchors to the viewport's bottom edge however tall the document
  (a fixed box ignores positioned ancestors). `layout_boxes` lays out in its
  `cb_width x cb_height` viewport and turbo-dom-raster's screenshots in their canvas
  size. `layout_with_options` / `layout_html_tree` (and so turbo-html2pdf's paged
  layout, whose initial containing block would be the page box) keep an unknown height
  and lay out as before. The live path already used the viewport. Control: `icb-*` in
  `html-document` and in the pixel control `scripts/paint-chrome-control.mjs`.
- Floats are shrink-to-fit (miaskiewicz/turbo-dom#83, found while fixing #79): a
  `width: auto` float took `min(max-content, container)` — squeezed below its
  min-content in a narrow container, its `min-width` / `max-width` ignored and its own
  margins counted as available space. It is now `min(max(min-content, available),
  max-content)` with `available` net of its margins, then `min-width` / `max-width`
  (CSS 2.2 §10.3.5). Control: `float-shrink-*` in `html-document`.
- Out-of-flow children add nothing to intrinsic widths (miaskiewicz/turbo-dom#84, found
  while fixing #79): an absolutely positioned or fixed child widened its parent's min- /
  max-content (a float, an inline-block, an auto table cell or a row flex container
  was as wide as it). Control: `intrinsic-oof-*` in `html-document`.
- `top` + `bottom` stretch an auto height on the static path (miaskiewicz/turbo-dom#89,
  found while fixing #81): an absolutely positioned or fixed box with `height: auto`
  and both insets took its content height (0 when empty); it is now the containing
  block's height less the insets, its margins and (content-box) its padding and border,
  then `min-height` / `max-height` (CSS 2.2 §10.6.4), as on the live path. Control:
  `abs-stretch-*` in `html-document`.
- Floats side by side in a max-content width (miaskiewicz/turbo-dom#85, found while
  fixing #79): a block container's max-content took its widest float, so a
  shrink-to-fit container (a float, an inline-block, an auto table cell) was too narrow
  and its floats wrapped onto separate rows. Consecutive floats now add up (per side,
  a `clear` closing that side's run); an in-flow block ends the run, and one that avoids
  floats (a formatting-context root, a replaced box) counts beside it. Control:
  `float-sum-*` in `html-document`.
- The fullscreen element (miaskiewicz/turbo-dom#82): since #58 the binding reports the
  fullscreen stack to the tree, but layout applied none of Chrome's UA rules for the
  fullscreen element, so its rect and `elementFromPoint` over it differed. While a tree's
  fullscreen stack is non-empty the cascade adds Chrome 156's `:not(:root):fullscreen`
  rules (`position: fixed; inset: 0; width / height: 100%; margin: 0; box-sizing:
  border-box; min / max sizes reset; transform: none`, all `!important`, and `object-fit:
  contain`): the HTML drive adds them itself (`Cascade::with_fullscreen`); a live-DOM
  embedder appends `style::fullscreen_user_agent_css()` to `user_agent_css()`. The
  element's box is in the top layer: it moves to the end of the initial containing
  block's content after its `::backdrop` (a black viewport-filling fixed box), above
  every other box, out of reach of its ancestors' offsets, transforms, clips and
  scrolling (its old place collapses). A document without fullscreen pays nothing.
  Control: `fullscreen-*` in `live-geometry` (a trusted click runs `requestFullscreen()`),
  asserted on the live and the static path.
- `layout_recorded_in_viewport` (for miaskiewicz/turbo-html2pdf#39): `layout_recorded` with
  an initial containing block of a given height — a paged layout passes its page area,
  the print initial containing block, so fixed boxes and absolutely positioned boxes it
  contains resolve `%` / `bottom` insets and `%` heights against it (as #81 does for
  screenshots). `layout_recorded` keeps the height unknown.
- Positioned boxes against their containing block on the static path
  (miaskiewicz/turbo-dom#100, found while fixing #89): an absolutely positioned / fixed
  box resolved a `%` width (and `%` padding / margins) against its parent's content width
  and was placed against its positioned ancestor's content box; it now uses its
  containing block, the ancestor's padding box (the viewport for `fixed`), as Chrome and
  the live path do. A definite zero-height ancestor's padding box keeps its padding (the
  `height: 0; padding-bottom: N%` aspect-ratio box). Control: `abs-percent-*` /
  `abs-padding-box-*` in `html-document`. P5b-W6 fixed #100's insets / margins side too
  (`positioned_style`) and turbo-dom#101 (a block inside an inline splits it on the
  generic policy); the merge keeps both, and W6's #102 control
  (`positioned-padding-box-*`) passes with this padding-box fix.
- Also found and fixed on this branch (P5b-W7): turbo-dom#86 (column flex items) and #88
  (`display: flow-root`) — P5b-W6 fixed both too; the merge keeps W6's implementations
  (see its section below) and both branches' Chrome 156 cases (`column-flex-*`,
  `flow-root-*`).

### Fixed — CSSOM shorthands expand and serialize like Chrome (P5b-X8, miaskiewicz/turbo-dom#90, Chrome 156 control)
- `element.style` (and `CSSStyleRule.style`) stored a shorthand verbatim: `style.flex = '1'`
  read `1` with empty `flex-grow` / `flex-shrink` / `flex-basis` (Chrome `1 1 0%`), and no
  shorthand set its longhands. turbo-dom's new `rtdom::css_shorthand` implements Chrome 156's
  129 shorthands (plus 15 shorthand aliases): every longhand set in Chrome's order (omitted
  ones at their initial value, `var()` values pending on the shorthand), `getPropertyValue`
  / `cssText` serialized from the longhands with Blink's rules and shorthand preference,
  `removeProperty(shorthand)`, `cssText` parsing order and `!important`, 101 longhand aliases
  (`word-wrap` is `overflow-wrap`). Used by turbo-dom-browser-env (`__cssShorthandNative`) and,
  as the lockstep port `src/runtime/css-shorthand.mjs`, by the npm runtime's `el.style`.
- turbo-dom-browser-env's computed cascade expands UA / Web Animations shorthands with the
  same engine (the old token-shape heuristics are gone) and drops a UA shorthand entry a later
  longhand outdates.
- npm runtime behaviour changes: `cssText` / the `style` attribute end each declaration with
  `;`; multi-token shorthands set their longhands (the single-token read fallback is gone);
  an invalid shorthand value is ignored.
- Control: `crates/turbo-dom/tests/fixtures/css-shorthand-{probe.js,cases.json}` →
  `css_shorthand_chrome156.json` (3110 scenarios, `scripts/make-css-shorthand-cases.py`; round
  five — cases a mutation run showed unpinned — also fixed `calc()` type checks and nested /
  distributed simplification (`calc(2 * (1px + 1em))` → `calc(2em + 2px)`, `pi`, `e`), `min()` /
  `clamp()` arity, `rgb()` / `hsl()` arity, `none` and `calc()` channels, `linear()` stop
  clamping, `steps()` / `cubic-bezier()` ranges, family names and quoted animation names spelled
  like Chrome, `oblique` angle range, three-value positions with `center`, grid track sign checks,
  `timeline-trigger` lists and omitted parts, layered shorthands with uneven longhand lists, and
  `font-variant` alternates anywhere).

### Fixed — CSSStyleDeclaration longhand spelling (P5b-X8, miaskiewicz/turbo-dom#97, Chrome 156 control)
- A longhand set directly kept its authored text (`width: 0` → `0`, Chrome `0px`; `1PX`,
  `url(a)`, single quotes) and some valid values were rejected (`column-rule-color: red`,
  `position-try-fallbacks: --a`, `interest-delay-start: inherit`, `animation-duration: auto`,
  `text-align: -webkit-center`). `rtdom::css_shorthand::longhand_value` spells (and, for the
  longhands it has a grammar for, admits or rejects) values like Chrome; the binding and the npm
  runtime store that spelling. The binding's computed cascade keeps reading the stylesheet
  grammar's text (full precision).
- Control: `crates/turbo-dom/tests/fixtures/css-longhand-{probe.js,cases.json}` →
  `css_longhand_chrome156.json` (2566 pairs, `scripts/make-css-longhand-cases.py`; round two
  added grammars for `transform-origin`, `perspective-origin`, `object-position`, `offset-anchor`
  / `-position`, `border-image-slice` / `-width` / `-outset`, `justify-items`, `animation-trigger`).
- Typed OM (`StylePropertyMap.set` / `append`, the map's reads) keeps the value as parsed —
  Chrome stores the `CSSStyleValue` — instead of setProperty's specified-value spelling
  (`__ns.setDeclarationAuthored` / `__ns.declarationAuthoredValue`).

### Fixed — SVG and MathML elements: style, dataset and the HTMLOrSVGElement mixins (P5b-X8, miaskiewicz/turbo-dom#87, Chrome 156 control)
- turbo-dom-browser-env: `svg.style` was `undefined` (React components with an inline-styled
  SVG icon threw, miaskiewicz/turbo-test#79), and SVG / MathML elements had no `nonce`,
  `tabIndex`, `autofocus`, `focus()`, `blur()` or `on*` handlers; MathML elements were plain
  `Element`s. `SVGElement` and the new `MathMLElement` carry those members like Chrome, each
  brand-checked (HTMLElement's `dataset`, `focus()`, `blur()`, `click()` no longer accept SVG /
  MathML receivers); SVG / MathML script focus follows Chrome's measured rules.
- npm runtime: MathML-namespace elements are `MathMLElement`s (`window.MathMLElement`).
- Control: `crates/turbo-dom-browser-env/src/svg_mathml_element_{probe.js,cases.json,chrome156.json}`.

### Fixed — HTMLTrackElement constants in deferred key order (P5b-X8, miaskiewicz/turbo-dom#99)
- turbo-dom-browser-env: `NONE` / `LOADING` / `LOADED` / `ERROR` were created
  non-configurable, so the key-order step could not move them after attributes installed
  later (turbo-surf's child frames listed them before `src`); they are now configurable until
  the key order freezes them.
- A second install into an already arranged realm (turbo-surf's initial about:blank child
  frame, `<iframe></iframe>`, installs its document twice) re-ran `html_track.js` over the
  frozen constants: it moved `src` behind them and then threw redefining them, and the next key
  order could not move `src` back. The re-run now leaves `src` and the frozen constants in
  place (P5b-H1); turbo-surf can drop its `interface_key_order.rs` pin.

### Fixed — turbo-dom-browser-env canvas fillStyle / strokeStyle / shadowColor (P5b-G2, miaskiewicz/turbo-dom#108, Chrome 156 control)
- The 2D context's `fillStyle` / `strokeStyle` / `shadowColor` are accessors over turbo-dom's
  CSS colour parser (`rtdom::color::parse_color`, through the binding's `__canvasColorNative`)
  instead of plain data properties: reads come back in Chrome's serialization (`#rrggbb`, or
  `rgba(r, g, b, a)` with Chrome's 8-bit alpha digits — `hsl(0,100%,50%)` reads `#ff0000`,
  `transparent` `rgba(0, 0, 0, 0)`), a value the canvas cannot parse (or a non-string that does
  not stringify to a colour) is ignored and the old value stays, gradients / patterns are kept by
  identity, and `currentcolor` (any case) resolves once, at assignment: Chrome 156 uses the
  connected canvas's own inline `color` only (an inherited or stylesheet colour, an inline
  `inherit`, or a detached canvas give black) and later colour changes do not follow. The op
  log records the resolved colour, so the raster replay paints it (it painted the previous
  fill for `currentcolor`). New `rtdom::color::Color::canvas_serialization`. Control:
  `crates/turbo-dom-browser-env/src/canvas_style_probe.js` -> `canvas_style_chrome156.json`
  (`scripts/invokers-chrome-control.mjs`), asserted by `canvas_style_tests.rs`.

### Fixed — turbo-dom-browser-env canvas save() / restore() (P5b-H1, miaskiewicz/turbo-dom#110, #111, Chrome 156 control)
- The 2D context keeps a drawing-state stack: `save()` snapshots `fillStyle`, `strokeStyle`,
  `shadowColor`, `globalAlpha`, `globalCompositeOperation`, the line, shadow, text and
  image-smoothing attributes and the line dash; `restore()` brings the last snapshot back (an
  empty stack is a no-op). They only logged their call, so reads after `restore()` (and the
  paint state later op-log tuples record) kept the latest assignment. `setLineDash` stores
  the dash (an odd count doubled, a negative or non-finite entry ignored) instead of being a
  no-op, and the initial `direction` is `inherit` (#111; it was `ltr`). Control:
  `crates/turbo-dom-browser-env/src/canvas_state_probe.js` -> `canvas_state_chrome156.json`
  (`scripts/invokers-chrome-control.mjs`), asserted by `canvas_style_tests.rs`.

### Fixed — turbo-dom-raster screenshots and canvas (P5-cov, miaskiewicz/turbo-dom#94/#95, Chrome 156 controls)
- A corrupt AVIF is a broken image instead of aborting the process (#94): rav1d 1.1.0 with one
  frame context panics inside its `extern "C"` API on a decode error (upstream
  memorysafety/rav1d#1496, fixed after 1.1.0; 1.1.1 is unpublished). The decoder now runs with
  two frame contexts, where the error comes back as a result; AV1 output is bit-exact, so pixels
  are unchanged. A child-process fuzz test over every AVIF fixture (truncations, bit flips) and
  over their AV1 streams fed to rav1d directly (truncations, the second half overwritten with 0xFF,
  every byte flipped; P5b-R1) gates it. Still open: Chrome paints nothing for such an image and
  rejects a truncated AVIF (miaskiewicz/turbo-dom#107).
- AVIF identity (GBR, matrix 0) samples convert exactly, every plane on the luma scale like libavif
  (they were half a level low, limited range on the chroma scale) (#95).
- SVG screenshots stroke a rounded box's border along its radius, like the PNG painter (#95).
- Blurred box-shadows of any size paint: only the visible part is blurred, in exact tiles (a
  shadow whose scratch passed 8192px was skipped); the blur radius is capped at 256px like Chrome's (#95).
- Canvas op-log colours parse through turbo-dom's new `rtdom::color::parse_color` (hsl()/hsla(),
  percentage rgb() channels and alpha, the full named set; channels rounded like Chrome) instead of a
  local subset; `canonicalize_color`'s output is unchanged (#95).
  `currentcolor` is resolved by the recorder at assignment (in Chrome and, since
  miaskiewicz/turbo-dom#108, turbo-dom-browser-env's context: the canvas's inline colour).
- Canvas `lineTo`/`arc` after `closePath` (or `rect`) starts at the closed subpath's first point, as in
  Chrome (found by the P5-cov mutation pass).

### Tests — turbo-dom-raster at 100% line coverage (P5-cov)
- turbo-dom-raster reaches 100% line coverage (tarpaulin, LLVM engine) by default and with
  `--features avif` / `--features jxl` (79.02% / 79.63% / 79.31% before); the CI `coverage`
  job gates each configuration at 100. New Chrome 156 controls back the pixel tests:
  `scripts/canvas-ops-chrome-control.mjs` (the 2D-canvas op log),
  `scripts/raster-avif-chrome-control.mjs` (every AVIF chroma layout, matrix family and
  range, fixtures from `scripts/make_raster_avif_fixtures.py`) and new `paint-*` cases in
  `scripts/paint-chrome-control.mjs` (rounded borders and gradients, shadows, translucency;
  the SVG screenshot is checked against the same probes through resvg). Every Chrome comparison
  is exact (no tolerances). Tests were strengthened against `cargo mutants` (see
  `docs/CONSOLIDATION.md` "Coverage").
- No behaviour change from the coverage refactors. Dead defensive branches were restructured or removed with their
  invariant documented: `glyph::outline_face` (a `FontFace` always re-parses at its index),
  the bundled faces always resolve (`measure_text`, canvas text), clip layers and shadow
  scratch buffers within their bounds always allocate, the box blur's empty guards (a
  `Pixmap` is never empty), the AVIF / JPEG XL size-only fallback is one branch, rav1d's
  open / get-picture results share their exits, and JPEG XL's unreachable empty-sample arm.

### Fixed — `HTMLTrackElement` and `TextTrack` (P5b-X6, miaskiewicz/turbo-dom#74, Chrome 156 control)
- turbo-dom-browser-env: `<track>` had no interface, so `track.src`, `kind`, `srclang`,
  `label`, `default`, `readyState`, `track` and `NONE` / `LOADING` / `LOADED` / `ERROR` were
  `undefined`. `HTMLTrackElement` now exists with Chrome's members and key order:
  - `kind` is the enumerated reflection (missing → `subtitles`, invalid → `metadata`);
  - `srclang` / `label` / `default` reflect;
  - `readyState` is `NONE` (the binding never loads tracks);
  - `track` is one `TextTrack` per element (an `EventTarget`). Its `kind` / `label` /
    `language` / `id` follow the element, `mode` is enumerated (`disabled` by default, invalid
    values ignored), `cues` / `activeCues` are empty `TextTrackCueList`s once enabled,
    `oncuechange` works, and `addCue` / `removeCue` reject non-cues.
- Every HTML element constructor now throws Chrome's `Failed to construct 'X': Illegal
  constructor` and keeps its interface constants.
- `track.src` uses the shared reflected-URL getter, so `<base href>` resolution comes with
  turbo-dom#73 (P5b-X7). Its four control rows are listed as pending on #73.
- Control: `src/html_track_{probe.js,cases.json,chrome156.json}`.

### Fixed — events across shadow trees (P5b-X6, miaskiewicz/turbo-dom#66, Chrome 156 control)
- turbo-dom-browser-env: `dispatchEvent` used to build its path from DOM parents only, so an
  event never left a shadow tree, even with `composed: true`, and its target was never
  retargeted. It now follows the DOM dispatch algorithm:
  - a composed event crosses each shadow root into the host; an uncomposed one stops at the
    root of the target's tree;
  - a slotted node passes through its assigned slot;
  - every listener sees `target` / `relatedTarget` retargeted against its own tree, and
    hosts are AT_TARGET;
  - `composedPath()` hides closed trees the listener is outside of;
  - propagation stops where the retargeted `relatedTarget` is the host, and nothing is
    dispatched when it equals the target;
  - after dispatch `target` / `relatedTarget` are cleared when they stayed inside a shadow
    tree.
- Paths that cross no shadow root are unchanged. `MouseEvent` / `FocusEvent` `relatedTarget`
  reads the dispatch's retargeted value.
- Control: `src/shadow_events_{probe.js,cases.json,chrome156.json}`. The fullscreen control's
  shadow `fullscreenchange` row and `reference_target_tests.rs`' invoker known gap now match
  Chrome.

### Changed — self-contained binding tests (P5b-X6)
- The DOM adoption / custom element reactions control moved from the npm tree
  (`test/fixtures/dom_adoption_reactions_{probe.js,cases.json,chrome156.json}`) into
  `crates/turbo-dom-browser-env/src/`. `src/dom_reactions_tests.rs` used to `include_str!` it
  from outside the crate, so the test could not build from the published package. The npm
  test `test/dom-adoption-reactions-chrome.test.mjs` now reads it from the crate.
- `test_support` exports it, the `dom_reactions_names_*` control and the `shadow_events_*`
  control (`DOM_ADOPTION_REACTIONS_*`, `DOM_REACTIONS_NAMES_*`, `SHADOW_EVENTS_*`,
  `HTML_TRACK_*`; `FILES` has 88 entries).
- A test now asserts that no `include_str!` / `include_bytes!` in the crate's `src/` or
  `tests/` leaves the crate.

### Fixed — document rules no longer style shadow trees (P5b-X6, miaskiewicz/turbo-dom#68, Chrome 156 control)
- turbo-dom-browser-env: `getComputedStyle`'s JS cascade read the owner document's sheets
  for every element, so document rules selected elements inside shadow trees. A shadow tree
  is its own style scope, as in Chrome; its elements now see only inline style and their
  tree's own (native, shadow-scoped) rules. Control: case `fullscreen`, section `shadow`, of
  `src/computed_defaults_chrome156.json` (a document `:fullscreen { opacity: 0.5 }` does not
  reach a fullscreen element in a shadow root).

### Fixed — `getComputedStyle` of a disconnected element (P5b-X6, miaskiewicz/turbo-dom#67, Chrome 156 control)
- turbo-dom-browser-env: a disconnected element blanked only the defaults the binding
  models. Author-rule and inline values (`position: relative`, `transform: scale(2)`, …)
  still came through. Chrome computes no style for it, so every property now reads `""` until
  the element is connected again; the declaration stays live. Typed OM's cascade read is
  unchanged. Control: case `fullscreen`, section `removed`, of
  `src/computed_defaults_chrome156.json`.

### Fixed — fullscreen top layer (P5b-X6, miaskiewicz/turbo-dom#58, Chrome 156 controls)
- turbo-dom: `Tree::set_fullscreen_stack` / `fullscreen_stack` / `fullscreen_element` /
  `is_fullscreen` / `is_fullscreen_ancestor`. The selector engine (DOM APIs, cascade,
  `rtdom::cascade`) now evaluates `:fullscreen` / `:-webkit-full-screen` (the elements in
  the stack) and `:-webkit-full-screen-ancestor` (shadow-including ancestors of the top). They
  used to be never-matching, and `:-webkit-full-screen-ancestor` was rejected although Chrome
  accepts it. A tree without a reported stack matches nothing, so static documents, html2pdf
  and raster are unchanged. npm `selectors.mjs` accepts `:-webkit-full-screen-ancestor` (never
  matching). Control: selector case `fullscreen` of
  `crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json`.
- turbo-dom-browser-env:
  - The fullscreen stack is reported to the tree, and `:modal` also matches the
    fullscreen element.
  - A fullscreen element (not the root) computes Chrome's `:not(:root):fullscreen` UA
    values: `position: fixed`, `inset: 0`, `margin: 0`, `box-sizing: border-box`, min / max
    sizes, `width` / `height: 100%` (reported as the viewport's `innerWidth` /
    `innerHeight` px, as Chrome does) and `transform: none`, all `!important`, plus
    `object-fit: contain`.
  - Its `::backdrop` is fixed at inset 0 and black.
- Control: async case `fullscreen` of `src/computed_defaults_chrome156.json` (user
  activation through the collector's trusted CDP click `__cdpGesture()`).

### Fixed — UA-styled pseudo-elements in `getComputedStyle` (P5b-X6, miaskiewicz/turbo-dom#47, Chrome 156 control)
- turbo-dom-browser-env: `getComputedStyle(el, pseudoElt)` computed only `::before`,
  `::after` and `::backdrop`; the other pseudo-elements read the element's style. Every
  pseudo-element Chrome 156 computes now gets its html.css values, the author rules that
  select it, and inheritance from the element:
  - `::marker` (`inline-block`, or `inline` when inside; `white-space: pre`; marker
    properties only);
  - `::placeholder`, `::file-selector-button`, `::details-content`, `::picker(select)` on
    the elements that have them;
  - `::first-line` / `::first-letter` (only the properties they accept);
  - the highlights (`::selection`, `::highlight()`, `::spelling-error`, …), with highlight
    inheritance once a rule of their type applies;
  - `::checkmark`, `::picker-icon`, `::scroll-marker-group`, `::column`,
    `::-webkit-scrollbar*`, `::scroll-button()`, and the view-transition pseudo-elements
    (inheriting along their tree).
- `::scroll-marker` and invalid arguments read an empty declaration. `::cue` and unknown
  `-webkit-` / `-internal-` names read the element. Author `::-webkit-input-placeholder` /
  `::-webkit-file-upload-button` rules apply as their standard names. Strings in `content`
  serialize double-quoted on every pseudo-element.
- Control: cases `pseudo-ua-forms` / `pseudo-ua` of `src/computed_defaults_chrome156.json`.
  Chrome reuses the first unstyled highlight read's color for later elements, so those
  colors are not compared.

### Fixed — form control fonts in `getComputedStyle` (P5b-X6, miaskiewicz/turbo-dom#61, Chrome 156 control)
- turbo-dom-browser-env: only `<button>` had Chrome's form-control UA font; `<input>`,
  `<select>` and `<textarea>` reported the inherited font (`16px Times`, or the parent's).
  They now compute html.css's `font: -webkit-small-control`: `13.3333px Arial` for every
  input type and `<select>`, `13.3333px monospace` for `<textarea>` and the date / time /
  datetime-local / month / week inputs. `<optgroup>` is `font-weight: bolder` and
  `<option>` `normal`. Author rules (`font: inherit` too) win. Control: case
  `form-control-fonts` of `src/computed_defaults_chrome156.json`.

### Fixed — image size points, `srcset`/`<picture>`, binding DOMExceptions (P5b-X7, Chrome 156 controls)
- The shared image probe follows Blink's load decision for cut-short bodies
  (miaskiewicz/turbo-dom#59). `image::probe` and `image::probe_natural` size a JPEG from
  the end of its first `SOS` header, a GIF from its first image descriptor's position and
  size, a WebP only with its whole declared RIFF payload (a PNG, from its first `IDAT`
  header, already did); before that the image is broken, so layout (and the PDF) does not
  size it and a live `<img>` load errors. `svg::parse`, so every SVG path, rejects a root
  that is not an SVG-namespace `<svg>` (no namespace, another namespace, an unbound
  prefix, `<SVG>`, an `<html>` root) and gzip (`svgz`) streams, and the sniffs accept a
  prefixed `<s:svg>` root, which Chrome renders. turbo-dom-raster decodes a PNG/JPEG/GIF/WebP
  only when the probe sizes it and lays one whose pixels do not decode out at its size (the
  placeholder): Chrome 156 loads the parity inputs `extra/images/png-truncated.png` and
  `jpeg-corrupt.jpg` at 64x40, which the screenshot used to drop. Control:
  `scripts/image-truncation-chrome-control.mjs` (every byte prefix of the
  `scripts/make_truncation_fixtures.py` images).
- turbo-dom-browser-env: DOMExceptions raised by the binding's JS modules are instances of
  the realm's interface (miaskiewicz/turbo-surf#42). Twenty modules captured
  `globalThis.DOMException` when they installed, so after turbo-surf installed its own
  interface `img.decode()`'s `EncodingError` and others were not `instanceof DOMException`.
  Every site now builds its exception when it is raised from the registered interface
  (`register_dom_exception`, which `capture_window_event_interfaces` now also calls with
  the host's installed `DOMException`; else the binding's own), through
  `ts.DOMException` / `ts.realmDOMException()`. The same change stops the throw-time
  `new g.DOMException(…)` sites from following a page script's reassignment of
  `globalThis.DOMException` (miaskiewicz/turbo-dom#72, filed here). Control:
  `src/dom_exception_realm_chrome156.json`.
- turbo-dom-browser-env: `srcset` / `sizes` and `<picture>` source selection
  (miaskiewicz/turbo-dom#71). HTMLImageElement `srcset` / `sizes` and HTMLSourceElement
  `srcset` / `sizes` / `width` / `height` were missing, and source `media` / `type` turned
  `null` into `""`. They now reflect like Chrome. The binding implements Chrome 156's source
  selection: `<picture>` sources by `media` and `type`, `x` and `w` descriptors with the HTML
  srcset parser, and `sizes` with media conditions and `calc()`. It is exposed to hosts as
  `imageHost.selectSource(img, environment)` → `{url, density}` and
  `imageHost.naturalSize(w, h, density)`. Without host state `currentSrc` is the selected
  URL, which is Chrome's answer when nothing loads (before: the `src` attribute). The
  `src`-only `currentSrc` is now parsed against the document base URL by a native resolver
  even without an embedder URL global. Control: `src/html_image_srcset_chrome156.json`.
- turbo-dom-browser-env: the reflected `src` / `href` getters parse the attribute against the
  document base URL (miaskiewicz/turbo-dom#73, filed here). This covers img, script, iframe,
  source, embed, input, video, audio, link, a and area. They used to resolve with the
  embedder's `URL` global against the document URL, so a `<base href>` was ignored, the bare
  binding returned the raw text, and an empty attribute gave `""` (Chrome gives the base URL).
  They now go through the native resolver `ts.resolveDocumentURL` (WHATWG `url` crate).
  `<base>.href` keeps resolving against the document URL. Control:
  `src/reflected_urls_chrome156.json`. Filed, not fixed: turbo-dom#74 (no
  `HTMLTrackElement` interface).
- turbo-dom: a parser-created `is=` element stayed "undefined" (`:not(:defined)`) only
  until native storage collection (miaskiewicz/turbo-surf#41). `Tree::collect_unreachable`
  re-materializes the retained parsed nodes as new ones, and their creation-time state was
  derived from the parse buffer's `is` attribute. turbo-surf collects at the first host
  boundary after a V8 GC, so the first `eval` after `PageSession::open` read `:defined`. The
  collection now records the state first. Reproduced through the binding
  (`collect_native_storage` after `install_html` and a GC, then the
  `custom_elements_defined_chrome156.json` probe).

### Fixed — turbo-dom-layout margins, heights, zero-area clips, empty formatting roots, flex item sizes and blocks in inlines (P5b-W6, miaskiewicz/turbo-dom#75/#76/#77/#78/#88/#93/#100/#101/#102, Chrome 156 controls)
Each fix is backed by Chrome for Testing 156 cases in `scripts/html-document-chrome-control.mjs`
and lists the corpus outputs it changes in `tools/parity/expected/p5b-w6-turbo-dom-*.json`
(new pinned corpus inputs: `tools/parity/inputs/extra/{min-height-margin,
self-collapsing-margins,min-max-height,percent-height,flow-root,column-flex,
flex-item-box-sizing}.html`).
- A last child's bottom margin and `min-height` / `max-height` (#75 item 1): a non-zero
  `min-height` kept the margin inside the parent (`max(min-height, content + margin)`).
  As in Chrome, an auto-height parent lays its content out without it; the margin
  collapses through when the used height is the content height and is dropped (neither
  through nor inside) when `min-height` / `max-height` changed it. Bottom padding /
  border, a definite `height` or an own formatting context still keep it inside.
  Control: `min-height-margin-*`.
- Zero-area `clip: rect()` boxes in `layout_boxes` (#75 item 2): an absolutely positioned
  box whose clip keeps nothing (the sr-only pattern) had no `layoutBoxes` entry. The
  geometry seam (`layout_boxes`, html2pdf / turbo-pdf-napi `layoutBoxes`) now lays it out
  with its empty clip region and reports its border box and its descendants' like
  `getBoundingClientRect()`; renders still leave it out (nothing paints; its text stays
  out of PDF text and tagging). Control: `clip-zero-area-*` / `clip-sr-only*`.
- An empty first child's margins (#76, found while fixing #75): the top and bottom
  margins of an empty (self-collapsing) first child, and the next child's top margin,
  collapse through the parent's top edge (CSS 2.2 §8.3.1) instead of staying inside it.
  Control: `self-collapsing-first-child-*`.
- `min-height` wins over `max-height` (#77, found while fixing #75): the static path
  clamped by `max-height` last. Control: `min-max-height-*`.
- Percentage heights on the static path (#78, found while fixing #75): a `%` `height` /
  `min-height` / `max-height` of an in-flow box resolves against its parent's definite
  height (positioned boxes: also `min-height` / `max-height`); against an indefinite height
  they still behave as `auto` / 0 / `none`. Control: `percent-*`.
- Empty formatting-context roots (#88; P5b-W7 made `display: flow-root` one): a zero-height
  box that establishes a formatting context (flow-root, `overflow`, flex / grid / table) is
  never self-collapsing, so margins no longer run through an empty one. Control:
  `flow-root-*` / `empty-formatting-root-*`; P5b-W6 also adds the `column-flex-item-*`
  cases of #86 (fixed by P5b-W7).
- Content-box flex items (#93, found while working on #86): an item's `width` / `height` /
  `flex-basis` / `min-*` / `max-*` lengths reached taffy (border boxes) unchanged, so a
  padded or bordered `box-sizing: content-box` item lost its frame. They are now
  border-box lengths. Control: `flex-item-*`.
- Blocks inside inlines (#101): on the generic / PDF policy a block-level box inside an
  inline element stayed an atomic inline (shrunk to its content, 0 wide when empty, its
  child's margins not collapsing through it). The inline is now split around it (CSS 2.2
  §9.2.1.1), as the native HTML-metrics flow already did: the runs before and after become
  anonymous line boxes and the block fills the line. Changes four turbo-surf detection
  screenshots (`embedded-font-range-*`, an `<h3>` inside an `<a>`); pinned by
  `tools/parity/inputs/extra/block-in-inline.html`. Control: `block-in-inline-*`.
- Positioned boxes and their containing block (#100, #102; P5b-W7's fix of #100 also
  gives the static path the padding box, #102): P5b-W6 adds the `positioned-percent-*` and
  `positioned-padding-box-*` cases, and an out-of-flow box's `%` horizontal margins and
  padding now resolve against its containing block too where its position is computed.

### Fixed — selector syntax, :scope, RCDATA fragments, quirks and selectedcontent, textarea :dir (P5-cov, miaskiewicz/turbo-dom#91/#92/#96/#98/#103/#104/#106, Chrome 156 controls)
Found by the new `scripts/rtdom-core-chrome-control.mjs` control
(`crates/turbo-dom/tests/fixtures/rtdom_core_chrome156.json`); both engines (rtdom and the
npm `selectors.mjs` / parser crate) unless noted.
- CSS comments in selectors were rejected (`p /* c */`): they are removed as CSS
  tokenizes them, still separating two names (`div/**/p` throws, `[title=a/**/i]` is
  `[title=a i]`); a DOM-API selector's unterminated string / `[` / `(` closes at its end
  (`[title`, `:state(x`, `:is(p`), and an escape at its end is U+FFFD (`[id=x\`) (#91, #98).
- rtdom: `:not()` / `:has()` / `of S` accepted empty items (`:not(p, )`) and items edged
  by a combinator (`:not(p >)`, `:not(> p)`); `:is()` / `:where()` kept such items (#91, #98).
- `:-webkit-any()` never matched in the rtdom DOM APIs and was forgiving in stylesheets: it
  now matches like `:is()` with Chrome's grammar — compound selectors only, unforgiving,
  no pseudo-elements or `:has()` (#91).
- rtdom: `:scope` never matched in the DOM APIs; it is the element `matches()` /
  `closest()` / element-rooted queries are called on (new `Tree::closest`,
  `matches_scoped`, `closest_units`, `matches_units_scoped`), `:root` for document queries
  and stylesheets. npm: `closest()` matched each ancestor as its own scope (#96).
- An+B integers clamp to 32 bits (overflowing digits parsed as 0), an integer glued into
  `n-123` must fit 32 bits, and an A or B outside 31 bits never matches (#98).
- `textarea` / `title` (RCDATA) fragments drop a literal U+0000 like Chrome (they kept it
  as U+FFFD); `&#0;` and every other context are unchanged (#92).
- An empty DOCTYPE system identifier counts as missing for the quirks-mode decision, like
  Chrome (`<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN" "">` is quirks:
  `compatMode` BackCompat, a `<table>` stays in an open `<p>`) (#103).
- `<selectedcontent>` is filled with a clone of the selected option's content (customizable
  select; the last `selected` option, else the first enabled one) — markup5ever_rcdom 0.39's
  clone step never fired. html5lib conformance: rtdom-direct 99.75% → 100%, npm 99.72% →
  99.94% (#104).
- `:dir()` on a `dir=auto` `<textarea>` reads its value — the child text content (direct
  Text children only) — not `textContent`: a Hebrew letter inside an appended `<span>` no
  longer makes it `rtl` (#106).

### Changed — turbo-dom crate coverage 100% (P5-cov-core)
- CI's turbo-dom coverage floor is 100 (was 88). New behaviour tests: the html5lib fixtures
  through the UTF-16 tokenizer fork, the Chrome-156 core control (`tests/rtdom_core_chrome.rs`),
  core / API edge and storage integrity tests.
- The UTF-16 tokenizer fork no longer carries upstream code the crate never runs (the
  `exact_errors` / `discard_bom` / `profile` options, the per-architecture SIMD data-state
  scan — a portable byte scan produces the same chunks; `UPSTREAM.json` lists the changes).

### Fixed — turbo-dom-layout tables, clipping and box sizing (P5b-W5, miaskiewicz/turbo-dom#41/#51/#52/#55/#62/#63/#64/#65/#69/#70, Chrome 156 controls)
Each fix is backed by a Chrome for Testing 156 control and lists the corpus outputs it
changes in `tools/parity/expected/p5b-w5-turbo-dom-*.json`.
- Table captions (#51): a `<caption>` (UA `caption { display: table-caption; text-align:
  -webkit-center }`; `caption-side` is inherited) was laid out as a row group — its block content
  became grid rows, its text was dropped and it had no box. A table with captions is now
  a table wrapper box (CSS Tables 3 §3.1, as Chrome lays it out): the `<table>`'s box (its
  `getBoundingClientRect()` / `layoutBoxes`) holds the `caption-side: top` captions in
  source order, an anonymous table grid box, then the `bottom` captions. The wrapper is
  as wide as the grid or the widest caption's min-content margin box, whichever is wider,
  and the grid stretches to it; each caption is a block formatting context at that width
  with its own box, its margins inside the wrapper. The grid box carries the table's
  border, padding, background, shadow, `height` and rows, so they no longer cover the
  captions; margins, positioning and transforms stay on the wrapper. Every layout path
  (PDF, screenshot, live). `NodeSources::table_grids` lists the grid boxes (no source
  element; html2pdf tags their rows as the table's). Tables without a caption lay out as
  before. Control: `caption-*` in `html-document`, `live-geometry` and the new pixel
  control `scripts/paint-chrome-control.mjs` (`crates/turbo-dom-raster/tests/
  paint_chrome.rs`).
- Partial `clip: rect()` (#52): on the PDF and screenshot paths every element whose `clip`
  started with `rect(` was dropped from layout and paint (it had no box, no
  `layoutBoxes` entry and no pixels), so a partially clipped box — a CSS sprite, a
  clipped badge — vanished. Now only an absolutely positioned box whose clip keeps
  nothing at any size (the sr-only `rect(0,0,0,0)` / `rect(1px,1px,1px,1px)`) is left out
  (its text stays out of PDF text and tagging); any other box lays out (`clip` does not
  apply to a box that is not absolutely positioned, as in Chrome), and an absolutely
  positioned one records its clip region in `Fragment::clip` (`ClipRect`, from the
  border-box corner, `auto` = the edge). turbo-dom-raster paints the box and its
  descendants through it (PNG: a masked layer over the region, in the box's transformed
  space; SVG: a `<clipPath>`), as does turbo-html2pdf's PDF emitter. Control: `clip-*` in
  `scripts/paint-chrome-control.mjs` (pixels + boxes).
- `box-sizing: border-box` widths (#65, found while fixing #51): a `px` width under
  `border-box` is the border box — never narrower than the padding and border — in a
  box's intrinsic widths too (`flex::natural_width` / `min_content_width`: floats,
  inline-blocks, flex items and shrink-to-fit tables around it) and in table column
  widths (a cell's `width`, auto and fixed layout; collapsed: less its half-borders).
  They counted the padding and border twice (a float around a `border-box; width:100px;
  padding:10px` box was 120px wide). Control: `border-box-*` in `html-document`.
- `<table>` is `box-sizing: border-box` (#62, found while fixing #51): the UA `table`
  rule (also `user_agent_css()` for embedders) sets it as Chrome's does, so a table's
  `width` / `min-width` / `max-width` / `height` include its border and padding (a
  `width:120px; border:4px` table was 128px wide; an author `box-sizing` still wins).
  Control: `table-border-box-*` / `table-content-box-author` in `html-document`,
  `caption-narrow` in the paint control.
- `border-spacing` (#63, found while fixing #51): the property is inherited and its
  initial value is 0; the UA `table` rule gives `<table>` its 2px. Every table box used
  to default to 2px, so a `display: table` element that is not a `<table>` (CSS tables,
  a caption test's anonymous table) had 2px gaps around and between its cells, and a
  `display: table` element did not inherit a `border-spacing` from its ancestors.
  Control: `css-table-*` / `table-spacing-not-inherited-past-a-table` in `html-document`.
- Table heights (#64, found while fixing #51): a table whose `height` / `min-height` is
  more than its rows need gives the extra height to its rows, as Chrome 156 does — to
  the body sections (else every section) in proportion to their heights, inside a
  section to its rows without an explicit `height` (else all of them) in proportion,
  equally when they are empty, in 1/64 px units — so cells stretch, and their
  backgrounds, borders and `vertical-align` follow. The table box was tall but its rows
  kept their content height. Separated and collapsed tables, every path. Control:
  `table-height-*` in `html-document`.
- Table column growth (#55): a table wider than its columns' max-content (a definite
  `width`, `min-width`, or stretched) gives the extra width to its auto columns (no `px`
  cell width) in proportion to their max-content, equally when they are all empty, and
  only without auto columns to the `px` ones — as Chrome 156 does. Empty cells of a
  `width:100%` table used to get 0px (no backgrounds or heights painted), and a
  `width:100px` cell beside content grew with it (307px instead of 100px). Control:
  `table-columns-*` in `html-document`.
- Last-child bottom margins (#41): under the generic / PDF layout policy a block dropped
  its last in-flow child's bottom margin — it was neither kept inside a parent with a
  bottom border or padding, a definite height or its own formatting context, nor
  collapsed through an auto-height parent into the parent's margin. It now follows CSS
  2.2 §8.3.1 on every policy (the live / HTML-metrics policy already did), and a box
  laid out as its own formatting context by its parent (a table cell, a flex or grid
  item) keeps its children's top and bottom margins inside. Changes the galley of every
  PDF whose blocks end with a margined child (and paginated breaks: turbo-html2pdf#32's
  `exact-fill-child-margin`). Control: `last-margin-*` in `html-document`.

- Legacy alignment (#69, found while fixing #51): a `text-align: center` / `right`
  container centred / right-aligned its block-level children; in Chrome only the legacy
  `-webkit-center` / `-webkit-right` / `-webkit-left` values do (plain values align inline
  content only). Those values are now parsed (inline content aligns as `center` /
  `right` / `left`; `BoxStyle::legacy_align` moves the blocks), the UA `center` and
  `caption` rules use `-webkit-center`, and the `align` attribute of `div`, `p`, `h1`-`h6`,
  `td`, `th` and `tr` is a presentational hint (`center`/`middle` → `-webkit-center`,
  `left`/`right` → `-webkit-left`/`-webkit-right`, `justify`; also in
  `presentational_declarations` for live layouts). Control: `legacy-align-*` in
  `html-document`.

- Header cells (#70, found while fixing #51): the UA stylesheet gains Chrome's
  `th { font-weight: bold; text-align: center }` (`center` aligns the cell's inline
  content, not its blocks, as Chrome's `-internal-center`); `<th>` text was regular weight
  and left-aligned in PDFs and screenshots. Control: `th-*` in `html-document`.

### Fixed — raster `<img>` handling, AVIF orientation (P5b-W4, Chrome 156 controls)
- `turbo-dom-raster` paints an `<img src="data:…">` from its own URL
  (miaskiewicz/turbo-dom#27 item 1). The screenshot decodes every `data:` image of the
  parsed document through `image::decode_data_url` (base64 or percent-encoded, any case of
  the scheme); before, it painted only caller-supplied bytes.
- `image_urls` and the lazy-image fill walk the parsed document's HTML `<img>` elements,
  skipping template content (miaskiewicz/turbo-dom#27 item 2). `<img` text in an
  attribute value, a `<script>` string or a comment is no longer fetched or rewritten:
  the old text rewrite could end a quoted attribute early and replace the element's own
  style. New `delazy_tree(&mut Tree)` is what the screenshot uses. `delazy_images` returns
  the filled document re-serialized (doctype identifiers kept) when an image changes,
  else the input. Control: `scripts/raster-img-chrome-control.mjs` (shared PNG sampling in
  `scripts/lib/png-samples.mjs`).
- AVIF `irot` / `imir` transform properties orient the image
  (miaskiewicz/turbo-dom#45 item 1). The probe composes them, in association order, into
  `Intrinsic::orientation` (new `Orientation::then`), so the natural size, the box
  (stored under `image-orientation: none`), the raster pixels and the PDF placement match
  Chrome. The fixtures are derived from `q.avif` by `scripts/make_format_fixtures.py`, and
  the control `scripts/image-formats-chrome-control.mjs` gains seven cases.
- JPEG XL images (miaskiewicz/turbo-dom#45 item 2). Chrome 156 decodes them (jxl-rs). The
  shared probe now reads a JPEG XL's `SizeHeader` and orientation in every build: bare
  codestream or container, new `Format::Jxl`. turbo-dom-raster decodes it under the new
  off-by-default `jxl` feature with `jxl-oxide` (MIT OR Apache-2.0, pure Rust, no thread
  pool; CI raster job). The fixtures are three jxl-rs test files with their BSD-3-Clause
  license (`crates/turbo-dom-layout/tests/fixtures/images/jxl/`), and the control gains
  three cases. The PDF does not embed JPEG XL.
  Corpus changes: `tools/parity/expected/p5b-w4-turbo-dom-27-*.json`,
  `-turbo-dom-45-1.json`.
### Fixed — `createDocument` qualified names (P5b-X5, miaskiewicz/turbo-dom#57, Chrome 156 control)
- turbo-dom-browser-env: `document.implementation.createDocument` checked the qualified
  name against the XML `Name` production and rejected every name with two colons. It now
  validates and extracts it like `createElementNS`, as Chrome does: `a:b:c` gives a
  document element with prefix `a` and local name `b`. `a::b`, `:a:b` and `a:1b:c` throw
  `InvalidCharacterError`. Control: section `documentNames` of
  `src/dom_reactions_names_chrome156.json`.

### Fixed — qualified names with several colons (P5b-X5, miaskiewicz/turbo-dom#44, Chrome 156 controls)
- turbo-dom-browser-env: `createElementNS`, `setAttributeNS` and `createAttributeNS` now
  split the qualified name strictly on `:` (DOM "validate and extract"), as Chrome 156 does.
  The prefix is the first part and the local name the second: `a:b:c` / `a:b:c:d` /
  `a:b:` create prefix `a`, local name `b`, qualified name `a:b`. An empty second part
  (`a::b`) throws `InvalidCharacterError`. The binding used to keep `b:c` as the local name.
- Controls: `nsTwoColons` in `src/dom_adoption_reactions_chrome156.json` and
  section `names` of `src/dom_reactions_names_chrome156.json`.

### Fixed — custom element upgrades and reaction order (P5b-X5, miaskiewicz/turbo-dom#42, #43, Chrome 156 controls)
- turbo-dom-browser-env, #42: `customElements.upgrade(root)` used to upgrade template
  contents, whose inert document has no registry. It now skips every candidate the
  registry does not govern. Inserting into a disconnected tree no longer upgrades either:
  only connected elements upgrade on insertion. So template contents appended to a
  detached element adopt without constructing, and upgrade once when connected. An element
  is never constructed twice.
- #43: an element's upgrade is now queued in its own reaction queue inside a `[CEReactions]`
  call. Its constructor and the `attributeChangedCallback` / `connectedCallback` it queues
  run before the next element's reactions (`importNode`: `ctor u1, attr u1, ctor u2`).
- #43: setters (`innerHTML`, `outerHTML`, reflected attributes, …) are `[CEReactions]`
  scopes like methods. The HTML fragment parser (`innerHTML`, `outerHTML`,
  `insertAdjacentHTML`, `setHTMLUnsafe`, `createContextualFragment`) queues its elements'
  upgrades before the setter's removals queue `disconnectedCallback`s
  (`ctor r4, conn r4, disc r1, …`).
- Controls: `src/dom_adoption_reactions_chrome156.json` (shared with the npm runtime) and the new
  `src/dom_reactions_names_{probe.js,cases.json,chrome156.json}` (fragment-parser setters).

### Fixed — custom element reactions in independent documents (P5b-X5, miaskiewicz/turbo-dom#54, Chrome 156 control)
- turbo-dom-browser-env: in a document installed with `install_independent_html`,
  `removeChild()` / `remove()` (and every other call that mutates the tree directly)
  used to run the `disconnectedCallback`s they queued during the *next* DOM call, or never
  when the removal was the last call. The mutation flush checked only the current document;
  it now also delivers another (independent) document's recorded mutations before the call
  returns, as in Chrome. Control: section `removal` of
  `src/dom_reactions_names_chrome156.json`.

### Fixed — `getComputedStyle(el, pseudoElt)` (P5b-X5, miaskiewicz/turbo-dom#47, Chrome 156 control)
- turbo-dom-browser-env: `pseudoElt` used to be ignored, so every read returned the element's
  style. `::before` / `::after` (also `:before` / `:after` and a bare `before` / `after`,
  ASCII case-insensitive, escapes and comments allowed) now compute the pseudo-element:
  - its own rules apply, and `color`, `visibility` and fonts inherit from the element;
  - `display` is `inline`, blockified for floats, absolute positioning and when the
    element is a flex / grid container;
  - `content` is `none` unless set, and strings serialize double-quoted.
- An invalid pseudo-element returns an empty declaration (`::foo`, `:backdrop`,
  `:marker`, `::before:hover`, `x:before`, a trailing space, `::part(x)`). A string
  without a leading colon reads the element, as do `null` / `undefined`.
- An element's `content` computes to `normal`.
- Still the element's style (open in #47): `::marker`, `::placeholder`, `::first-line` /
  `::first-letter`, `::selection`, `::file-selector-button`, `::view-transition*` and
  the other UA-styled pseudo-elements.
- Control: case `pseudo-elements` in `src/computed_defaults_chrome156.json`; no corpus
  output changes (`tools/parity/expected/p5b-x5-47.json`).

### Fixed — `getComputedStyle` for modal dialogs, the top layer and `::backdrop` (P5b-X5, miaskiewicz/turbo-dom#30, Chrome 156 control)
- turbo-dom-browser-env: a `<dialog>` opened with `showModal()` (it matches `:modal`) now
  computes Chrome's `dialog:modal` values: `position: fixed`, `top` / `bottom: 0px`,
  `max-width` / `max-height: calc(100% - 6px - 2em)` at the dialog's font size (16px →
  `calc(100% - 38px)`), `overflow: auto` and `visibility: visible`. Author and inline
  declarations still win. A dialog opened with `show()` stays `position: absolute`;
  `close()` and removal from the document leave the top layer.
- A top-layer element (modal dialog, open popover) whose `position` is `static`,
  `relative` or `sticky` computes to `absolute` (CSS Position 4).
- `getComputedStyle(el, '::backdrop')` reads the backdrop instead of the element. It is
  `display: block`. Outside the top layer it is `position: absolute` with `auto` insets.
  In the top layer it is `position: fixed` at inset `0px`, with `rgba(0, 0, 0, 0.1)`
  behind a modal dialog (popovers: transparent). Author `::backdrop` rules apply, and
  `visibility`, `color` and fonts inherit from the element. A disconnected element's
  backdrop reports `""`. The returned declaration stays live like the element's.
- Control: new case `dialog-top-layer` in `src/computed_defaults_{probe.js,cases.json,chrome156.json}`
  (show / showModal / close, open attribute removed while modal, removal, stacked modals,
  hidden ancestor, author overrides, popovers, `::backdrop`); the earlier sections are
  unchanged. No corpus output changes (`tools/parity/expected/p5b-x5-30.json`).

### Fixed — `getComputedStyle` defaults and no injected `<title>` (P5b-X3, miaskiewicz/turbo-surf#36 items 1–2, Chrome 156 control)
- turbo-dom-browser-env: `getComputedStyle` used to return `""` for `display`, `position`
  and `visibility` when no author rule applied; only `<button>` had UA values. It now
  reports Chrome 156's computed values:
  - `display` is the element's html.css display: block / inline / none / table-* /
    list-item / inline-block / contents / ruby. `[hidden]`, a closed `dialog` or popover,
    `input[type=hidden]` and `audio` without controls are `none`. MathML gives `math` /
    `block math`.
  - `display` is then blockified for floats, absolutely positioned elements and flex /
    grid items, as in Chrome.
  - `position` is `static`, except `dialog` (`absolute`) and `[popover]` (`fixed`).
  - `visibility` is inherited and `visible` at the root.
  - The CSS-wide keywords and shadow-scoped (`:host`) declarations resolve.
  - Disconnected elements and children of `<video>` / `<audio>` report `""`, like Chrome.
- turbo-dom-browser-env: the document no longer gets an empty `<title>` injected into
  `<head>` at setup (`<!doctype html><body>` has an empty head, as in Chrome). Setting
  `document.title` still creates the `<title>`.
- Control: `src/computed_defaults_{probe.js,cases.json,chrome156.json}`, also exported as
  `test_support::COMPUTED_DEFAULTS_*` (76 files) so turbo-surf can run it through
  `PageSession`. Corpus: every surf `live-*` dump changes
  (`tools/parity/expected/p5b-x3-surf-36.json`).

### Fixed — npm runtime namespaces, adoption, template contents, reaction order (P5b-X4, miaskiewicz/turbo-dom#29, Chrome 156 control)
- `createElementNS` creates the element in the namespace it is given: any namespace other
  than HTML / SVG / MathML (and the null namespace — `null`, `undefined` or `''`) gives a
  plain, case-preserving `Element` (`namespaceURI`, `localName`, attribute names and
  `tagName` keep their case; it is never a custom element). It used to create an HTML
  element. Prefixes are kept (`prefix`, `tagName` = the qualified name, e.g. `svg:rect`;
  they were dropped) and the qualified name is validated like Chrome 156
  (`InvalidCharacterError` / `NamespaceError`; Chrome keeps the local name up to a second
  colon). `createElement` validates its name too (`createElement('a b')` threw nothing;
  turbo-dom#31). `getElementsByTagName` matches non-HTML elements by their exact qualified
  name (`getElementsByTagName('foreignObject')` found nothing; turbo-dom#32), and the
  serializer writes other namespaces' qualified names and treats only HTML elements as
  void / `<template>` (turbo-dom#33).
- Inserting a node from another document (`insertBefore` / `appendChild` / `append` /
  `replaceChild`, fragments' children too) adopts it, as `adoptNode` does: its subtree reads
  its own attributes and children instead of the new document's buffer (an un-inflated
  subtree used to read garbage — wrong attributes, other documents' nodes), the old document
  stops reporting it (its version is bumped), and `adoptedCallback` runs. DOM versions now
  come from one process-wide counter, so an adopted node can never hit a cache entry keyed
  on its old document's version. A custom element moved into a document without a registry
  keeps its connected / disconnected callbacks there.
- `insertBefore(node, node)` and `replaceChild(node, node)` no longer throw `NotFoundError`
  after detaching the node (it was lost): the reference child is checked before the tree
  changes and becomes the node's next sibling, as in Chrome; `replaceChild` removes the old
  child before inserting the new one (MutationObserver records in that order;
  turbo-dom#34). `DocumentType.cloneNode()` keeps `name` / `publicId` / `systemId`
  (turbo-dom#35).
- `<template>` contents live in the document's inert template contents owner document
  (`content.ownerDocument !== document`, no `defaultView`, no custom element registry),
  so `template.content.cloneNode(true)` no longer upgrades custom elements right away:
  they upgrade once inserted into the document (`document.importNode(content, true)`
  clones into the document and upgrades there). Every template now has its contents
  (`document.createElement('template').content` was `null`), `template.innerHTML` parses
  into them (it replaced the template's children), and `cloneNode(true)` clones them
  (turbo-dom#36). Moving parsed contents in and out of the document stays lazy (the
  inert document shares the buffer). `element.content` is an accessor: `<meta>` reflects
  its attribute (it was `null`; turbo-dom#37), other elements have no such property.
  Adoption reaches shadow roots (a shadow host's root stayed in the old document;
  turbo-dom#38), and `document.cloneNode(true)` clones the children into the new document.
- Custom element reactions are queued on each element and run when the DOM call returns,
  element by element, like Chrome's `[CEReactions]`: callbacks see the finished operation
  (`append(a, b)`: `a`'s `connectedCallback` sees `b` inserted; `innerHTML` /
  `replaceChildren` / `textContent`: `disconnectedCallback` sees the new children; clone /
  `importNode` constructors see the cloned children; `innerHTML` queues the new elements'
  upgrades before the old children's disconnections), and DOM calls made inside a callback
  run their own reactions first. `attributeChangedCallback` runs after the mutation record
  is queued. `insertAdjacentHTML` parses with the element or its parent as the context
  instead of a `createElement(localName)` scratch element (a custom element of that name was
  constructed; SVG context lost; positions are case-insensitive; `NoModificationAllowedError`
  / `SyntaxError` like Chrome; turbo-dom#39). `replaceWith(…)` listing the node itself keeps
  it, and the node is removed before its replacements go in (turbo-dom#40).
  Control: `crates/turbo-dom-browser-env/src/dom_adoption_reactions_chrome156.json` (probe +
  cases beside it; under `test/fixtures/` until P5b-X6), recorded with `crates/turbo-dom-browser-env/scripts/select-element-chrome-control.mjs`),
  asserted by `test/dom-adoption-reactions-chrome.test.mjs` (the one deviation: the runtime's
  `HTMLElement` is `Element`, so `instanceof HTMLElement` is true for every element).

### Changed — npm `:defined` and custom elements (P5b-X3, miaskiewicz/turbo-dom#26 item 2, Chrome 156 control)
- The JS runtime keeps each element's HTML custom element state in lockstep with the Rust
  tree (`src/runtime/custom-elements.mjs`): the creation-time rule (a valid custom element
  name, or an `is` value from the parser, the fragment parser or `createElement(…, {is})`,
  is "undefined"; later `is` edits never change it) and upgrades / constructions on top.
  `:defined` in `matches` / `querySelector*` / `getComputedStyle` reads it instead of
  looking the current name up in `window.customElements`, so a later `is` edit no longer
  flips it and a detached element created before `define()` stays undefined (Chrome).
- `window.customElements` is a working registry instead of a stub: `define()` upgrades the
  document's candidates in shadow-including tree order (constructors run on the existing
  element, `this` is the element, prototypes become the class's), insertion into the
  document, `customElements.upgrade()`, `cloneNode` / `importNode` and `innerHTML` upgrade
  too, `createElement` / `createElementNS` / `new C()` construct (customized built-ins
  via `{ extends }` and `HTMLButtonElement` & co.), a throwing constructor leaves a failed
  element (reported through `window.reportError`, not thrown), and `connectedCallback` /
  `disconnectedCallback` / `attributeChangedCallback` (`observedAttributes`) /
  `adoptedCallback` run. BREAKING for tests that relied on the stub: defined custom elements
  now run their constructors and callbacks; an autonomous definition no longer makes a
  `<button is=…>` of the same name match `:defined`. Documents without a touched
  registry run every DOM path unchanged. New exports: `customElementState`,
  `setCustomElementState`. Also `DocumentFragment` gains `firstElementChild` /
  `lastElementChild` / `childElementCount` (template content traversal, used by the
  control). Control: `crates/turbo-dom-browser-env/src/custom_elements_defined_chrome156.json`,
  asserted by `test/custom-elements-chrome.test.mjs` (every section; the failed element is a
  plain element, not an `HTMLUnknownElement`).

### Added — `HTMLImageElement` `width` / `height` / `x` / `y` (P5b-X3, miaskiewicz/turbo-dom#26 item 1, Chrome 156 control)
- turbo-dom-browser-env: `HTMLImageElement.prototype` gains the `width` / `height`
  reflecting IDL attributes (HTML §4.8.4.3; `img.width` was `undefined`) and the CSSOM
  View `x` / `y`, as Chrome's prototype accessors ("Illegal invocation" for other
  receivers). The getters return the rendered box while the image is being rendered, which
  the host answers through the new private hook `record.imageHost.setLayoutQuery(query)`
  (`query(img)` → `{width, height, x, y}` or `null`); otherwise, as Blink, the attribute
  when it parses as a non-negative integer (≤ 4294967295), else the natural size from
  `imageHost.setState` (`0` without one), and `x` / `y` are `0`. The setters write the
  attribute as unsigned long (ToUint32, above 2147483647 → `0`; Symbol / BigInt rejected
  with Blink's "Failed to set the 'width' property …" TypeError). Without a query (turbo-test)
  no image is being rendered. Control: `src/html_image_probe.js` gains a `dimensions` section
  and `src/html_image_cases.json` rendered / `display: none` images with their layout
  boxes (`layout`), recaptured on Chrome for Testing 156.0.8078.4 (earlier sections
  unchanged).

### Fixed — turbo-dom-layout bug fixes (P5b-W1, Chrome 156 controls)
Each fix is backed by a Chrome for Testing 156 control and lists the corpus outputs it
changes in `tools/parity/expected/p5b-w1-*.json`.
- `@media` / `@supports` rule order (miaskiewicz/turbo-dom#24 item 1): the rules of a
  conditional group rule whose condition holds take the block's own source position in the
  cascade (recursively for nested blocks), instead of following every top-level rule of the
  sheet — `@media screen{div{width:10px}} div{width:20px}` is 20px wide, as in Chrome.
  `Stylesheet::at_rule_position` exposes where each at-rule sat among the qualified rules.
  Control: `scripts/html-document-chrome-control.mjs` (`media-*` / `supports-*` cases).
- Elements the HTML drive does not lay out still count for selectors
  (miaskiewicz/turbo-dom#25 item 2): a stripped `<style>`, `<script>`, `<noscript>`,
  `<template>`, `<title>`, `<meta>` or `<link>` is a child, a sibling and a `:has()`
  descendant in the layout cascade, as in Chrome — `<div><style></style></div>` is not
  `:empty`, the `<p>` of `<style></style><p>` is not `:first-child`, `script + p` matches.
  Control: `scripts/html-document-chrome-control.mjs` (`*-empty`, `*-sibling*`,
  `has-style-child`, `meta-link-title-in-body-count` cases).
- Positioned inlines and unsupported inline constructs in live snapshots
  (miaskiewicz/turbo-surf#31): `position: absolute` / `fixed` blockify the box
  (`display_of`, CSS 2.1 §9.7, every layout path), and such a box inside inline content
  takes a zero-size place at its static position in the line — it no longer splits its
  inline ancestor or the surrounding text into anonymous blocks, and no longer counts
  towards the run's intrinsic widths. The native HTML snapshot no longer fails as a whole
  on an inline construct its inline producer does not support (top/bottom/middle
  `vertical-align`, vertical/RTL writing, `sticky`, a displaced block-in-inline split):
  `LayoutSnapshot::unsupported_inline_geometry` names that element's subtree and
  `cssom_view::metrics::element_geometry` fails those reads with the same message, while
  every other element answers (a visually-hidden `.sr-only` span made every read on the
  page throw). Control: `scripts/live-geometry-chrome-control.mjs`
  (`crates/turbo-dom-layout/src/live/chrome_tests.rs`).
- Table rows (miaskiewicz/turbo-surf#31): a row box runs from its first cell's left edge
  to its last cell's right edge — inside the border-spacing, or between the outer
  grid-line centres of a collapsed table — instead of over the table's whole content
  width, and in live geometry it records used box metrics, so `getBoundingClientRect()`
  on a `<tr>` answers (`[10,10,9.109375,20]` for a one-cell table in Chrome) instead of
  failing with "used box metrics are missing". Control: `live-geometry` `table-width-*`.
- Table row groups in live geometry (miaskiewicz/turbo-surf#31): a `<tbody>` / `<thead>` /
  `<tfoot>` gets a transparent box — the union of its rows, which it now contains — with
  used box metrics, so its CSSOM reads answer (`[10,10,9.109375,20]` for a one-cell
  table) instead of failing through the inline fallback ("decorated inline border
  production is pending" / "empty inline placement is pending"). PDF and screenshot
  layouts keep the rows flat (unchanged output). Control: `live-geometry`
  `table-sections`, `table-two-bodies`, `table-empty-cell`, `table-decorated-collapsed`.
- Auto-width tables in live geometry (miaskiewicz/turbo-surf#36 item 3): a `width: auto`
  table is shrink-to-fit — no wider than its columns' max-content plus border-spacing,
  padding and borders (`min-width`/`max-width` still apply, and it never drops below its
  min-content) — so a one-cell table is `[8,8,13.109375,24]` as in Chrome, not the full
  1264px line, and `margin: 0 auto` centres it. The PDF / screenshot paths
  follow in P5b-W3 (miaskiewicz/turbo-dom#28 item 7, below). Control: `live-geometry` `table-auto-*`.
- `<template>` is `display: none` in the user-agent sheet (miaskiewicz/turbo-surf#36 item
  4), as in Chrome: a live `<template>` (also one inside a table cell) has the all-zero
  rect of a boxless element instead of an empty box at its flow position. Embedders that
  match `user_agent_css()` (turbo-surf) get the rule too; the HTML drive already stripped
  `<template>`. Control: `live-geometry` `template-in-*`.

### Fixed — turbo-dom-layout bug fixes (P5b-W3, miaskiewicz/turbo-dom#28, Chrome 156 controls)
Each fix is backed by a Chrome for Testing 156 control and lists the corpus outputs it
changes in `tools/parity/expected/p5b-w3-turbo-dom-28-*.json`.
- `clip: rect()` in hit testing (item 1): `cssom_view::hit::elements_at_point` (surf's
  `elementFromPoint` / `elementsFromPoint`) and the caret query skip the parts of an
  absolutely positioned box — and of its descendants and text — that its
  `clip: rect(top, right, bottom, left)` cuts away (offsets from the border-box corner,
  `auto` = the border-box edge), as Chrome does: a point over a `.sr-only` span hits
  the element underneath instead of the span. `clip` on a box that is not absolutely
  positioned is ignored. Control: `live-geometry` `clip-rect-*` cases (`hits`).
- Out-of-flow margins (item 2): an absolutely positioned box's own margins are added
  once. Its static position is the margin edge of its hypothetical box — the flow's
  content edge and the margins collapsed before it — so `margin-left` is no longer
  doubled (in `place_special_kid` and the deferred pass) and its `margin-top` no longer
  also joins the preceding margin run; a `right`-anchored box keeps its `margin-right`
  inside the inset instead of being shifted by `margin-left`. Every layout path.
  Control: `abs-static-position-*` / `abs-inset-margins` in `live-geometry` and
  `html-document`.
- Anonymous block style (item 3): the anonymous block box wrapping an inline run next to
  block-level siblings takes only its parent's inherited properties (CSS 2.1 §9.2.1.1) on
  every path. The PDF / screenshot path copied the parent's whole style, so the run was
  padded, bordered, sized (`height`, `width`) and offset by the parent's margins a second
  time — `<div style="padding:10px">x<p>…</p></div>` pushed the `<p>` down by 20px. Control:
  `anonymous-block-*` in `html-document` and `live-geometry`.
- Floats and `inline-table` (item 4): `display_of` blockifies a float like an absolutely
  positioned box (CSS 2.1 §9.7) — a floated `<span>` is a shrink-to-fit block float
  instead of an inline laid out in the line — and `inline-table` is its own
  `Display::InlineTable`, an atomic inline-level table (`table` once blockified) instead of
  a block. A float met inside line content (after inline content, or inside an inline
  element) no longer splits the line or its inline ancestor: it takes a place in the run
  and is placed at the top of the line it lands on, the lines beside it narrowed (the
  paragraph is re-laid until the floats settle), as in Chrome; in live snapshots such a
  float is supported instead of failing as a "positioned/floating block-in-inline
  split". A table's intrinsic widths (`flex::natural_width` / `min_content_width`:
  inline tables, floats, flex items, inline-blocks around tables) are its columns plus
  border-spacing (`table::max_content_width` / `min_content_width`), not its widest
  row. Control: `float-*` / `inline-table-*` in `live-geometry` and `html-document`.
- Empty row groups in live geometry (item 5): a `<tbody></tbody>` (`<thead>`, `<tfoot>`)
  gets a zero-height box as wide as the table's rows, at the bottom of the rows before it
  (the first row edge when none precede it; zero-size at the content-box origin of a
  table without columns), as in Chrome, instead of no box (its reads failed with "native
  empty inline placement is pending"). Control: `live-geometry` `empty-row-group-*`.
- Zero-height lines (item 6): a line box holding no text, preserved white space, framed
  inline (horizontal margin/border/padding), in-flow atom or `<br>` — only the places of
  absolutely positioned or floated boxes, collapsible spaces and empty inlines — is
  zero-height (CSS 2.1 §9.4.2), on every path, instead of one strut tall:
  `<div><span><span style="position:absolute"></span></span></div>` is 0px high as in
  Chrome (was 18px live / 19.36px PDF). `layout::inline::InlineAtom` gains `placeholder`
  (set for those places). Control: `abs-only-line-*` / `float-only-line-in-inline` in
  `live-geometry` and `html-document`.
- Auto-width tables on every path (item 7): a `width: auto` table is shrink-to-fit in
  PDFs and screenshots too (turbo-surf#36 item 3 did it for live geometry only) — its
  columns' max-content plus border-spacing, padding and borders, `min-width` /
  `max-width` applied, never below its min-content — so the table box (background,
  borders, `layoutBoxes`) hugs its columns as in Chrome instead of spanning the
  containing block, and `margin: 0 auto` centres it. `table-layout: fixed` applies only
  to a table with a definite width (a `width: auto` one uses the automatic algorithm, as
  in Chrome), and the columns of an auto table fill its box (a `min-width` or stretched
  table no longer leaves them at their natural widths). Changes many html2pdf PDFs: the
  exact list is `tools/parity/expected/p5b-w3-turbo-dom-28-item7.json`. Control:
  `table-auto-*` in `html-document` and `live-geometry`.
- `inline-flex` / `inline-grid` (miaskiewicz/turbo-dom#53, found while fixing #28 item 4):
  `Display::InlineFlex` / `InlineGrid` are atomic inline-level flex / grid containers that
  sit in the line and shrink to fit (`flex` / `grid` once blockified), instead of
  block-level boxes filling the line; a grid's intrinsic width is its `px` column tracks
  plus gaps (else its widest item). Control: `inline-flex-*` / `inline-grid-*` in
  `live-geometry` and `html-document`.
- Absolutely positioned `width: auto` boxes (miaskiewicz/turbo-dom#48, found while fixing
  #28 item 2): a non-replaced one is shrink-to-fit (CSS 2.1 §10.3.7) — `min(max(min-content,
  available), max-content)`, available from its set inset or static position to the far
  edge of the containing block, then `min-width` / `max-width` — on every path, and one
  with both `left` and `right` set stretches between them on the PDF / screenshot path
  too (live already did); a `right`-anchored box is placed by that width. It used to take
  the whole containing block (`<div style="position:absolute;right:0">` of a 30px child
  was 1280px wide at x=0). Control: `abs-auto-width-*` / `fixed-auto-width-shrinks` in
  `live-geometry` and `html-document`.
- Percentage columns (miaskiewicz/turbo-dom#50, found while fixing #28 item 7): in the
  automatic table layout a column whose cell has `width: N%` (or `width="N%"`) takes that
  share of the table (never below its min-content; the other columns share the rest), and
  an auto table's max-content grows so it can — the largest of the plain sum, each
  percentage column's max-content over its share and the other columns' over theirs;
  unbounded (the table fills the line) at 100% or more beside other content — as in
  Chrome. Control: `table-percent-*` in `live-geometry` and `html-document`.
- Auto-layout collapsed tables (miaskiewicz/turbo-dom#49, found while fixing #28 item 5):
  a `border-collapse: collapse` table with the automatic layout (and a `width: auto`
  `table-layout: fixed` one) now merges shared borders like a fixed-layout one — one
  border per grid line at the widest abutting width, half on each side, the table's own
  border as the outer line — with content+padding column tracks from the automatic
  algorithm (percentages honoured; a spanning cell widens its columns in proportion).
  Cells, rows and row groups sit at the grid-line centres as in Chrome instead of each
  cell keeping its full borders (`68x20` for two 2px-bordered cells, was `70x20`), and a
  collapsed table with its own border no longer counts it twice (also fixed layout).
  Control: `table-collapse-auto-*` in `live-geometry` and `html-document`.
- Spanning cells in separated tables (miaskiewicz/turbo-dom#56, found while fixing #49):
  a `colspan` cell wider than its columns spreads the excess over them in proportion to
  their widths (evenly when empty) instead of widening only the last one, the
  border-spacing between them counting towards the span — for the max-content and the
  min-content — and the cell is laid out over that spacing too, as in Chrome.
  Control: `table-colspan-*` in `live-geometry` and `html-document`.

### Changed — selector follow-ups (P5b-X2, miaskiewicz/turbo-dom#24, Chrome 156 controls)
- `rtdom::cascade` (`getComputedStyle`) and the npm `cascade.mjs` rank rules by the
  Selectors 4 specificity of each `parse_for_cascade` item — the layout cascade's — instead
  of a per-token count: `:where()` adds nothing, `:is()` / `:not()` / `:has()` count their
  most specific argument once, `:nth-child(… of S)` is a pseudo-class plus S, an escaped
  `\:` or a `#` inside an attribute value is not a pseudo-class / id, and a type selector
  inside a pseudo-class argument counts. A later rule of equal specificity now wins
  (`:where(#a) p` vs `p`). npm: `_internal.specificity` / `compareSpecificity`.
- `:defined` after `customElements.define()` (miaskiewicz/turbo-dom#24 item 3): the tree
  keeps each element's HTML custom element state (`Tree::custom_element_state` /
  `set_custom_element_state`, `rtdom::tree::CustomElementState`). turbo-dom-browser-env
  reports upgrades, constructions and failed constructors to it (and carries it through
  clones and adoption), so an upgraded element matches `:defined` in `matches` /
  `querySelector*` / stylesheets / `getComputedStyle` instead of `:not(:defined)`, and a
  throwing constructor leaves it undefined. Without a report the state is the
  creation-time one: the `is` attribute counts only as the parser saw it (setting `is`
  on a created element no longer makes it undefined; removing it from a parsed one no
  longer defines it), `createElement(…, {is})` is undefined, as Chrome 156
  (`turbo-dom-browser-env/src/custom_elements_defined_chrome156.json`).
- BREAKING (Rust API): the `s` (case-sensitive) attribute selector modifier is rejected
  like Chrome 156 does — `[a=b s]` is a `SelectorError` in `query_selector*_checked` /
  `matches_checked` / `parse_for_cascade`, never matches through the lenient APIs, and drops
  its stylesheet rule. It was a documented Rust extension; the npm engine already threw.

### Changed — `Image()` and `decode()` (P5b-X2, miaskiewicz/turbo-dom#23, Chrome 156 control)
- turbo-dom-browser-env: `Image(width, height)` is the native HTML legacy factory (like
  `Option()`): new-only with Blink's call-without-new TypeError, length 0, name `Image`,
  `Image.prototype === HTMLImageElement.prototype`, NewTarget's prototype honoured
  (subclasses, `Reflect.construct`), trailing `undefined` arguments absent, `width` /
  `height` converted as unsigned long and reflected into the attributes (above 2147483647
  → 0), Symbol / BigInt rejected with Blink's messages. BREAKING for embedders that relied
  on the old JS fallback: images made by `Image()` no longer have an own `src` and no
  longer fire a fake `onload` after `src` is set — `src` reflects through the prototype and
  load events are the host's (`record.imageHost`). turbo-surf replaces its own
  `globalThis.Image` with this one.
- turbo-dom-browser-env: `HTMLImageElement.prototype.decode()` (it was missing): resolves
  when the host reports the image decoded (complete with a natural size), rejects with an
  `EncodingError` DOMException otherwise; a call during a host-reported load settles at
  the host's next state. Bad receivers reject with Blink's "Illegal invocation" TypeError.
  Control: `src/html_image_probe.js` gains `factory`, `own` and `decode` sections,
  recaptured on Chrome for Testing 156.0.8078.4 (the earlier sections unchanged).

### Fixed — selector engine and cascade order (P5b-X, Chrome 156 controls)
Each fix is backed by `crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json`
(regenerated by `scripts/cascade-selectors-chrome-control.mjs`) and lists the corpus
outputs it changes in `tools/parity/expected/p5b-*.json`.
- `turbo-dom-layout` cascade order (miaskiewicz/turbo-dom#21): declarations are ranked by
  importance, origin, specificity, then the rule's source order and only then the
  declaration's index in its rule — a later rule of equal specificity now wins over an
  earlier rule with more declarations (`p{margin:0;color:red} p{color:green}` is green), and
  an `!important` user-agent declaration beats an `!important` author one.
- `:disabled` / `:enabled` (miaskiewicz/turbo-dom#18): HTML "actually disabled" — a
  control inside a disabled `<fieldset>` (outside its first `<legend>`) is disabled, an
  `option` follows its `optgroup` and owner `select`, and only `button`, `input`,
  `select`, `textarea`, `fieldset`, `optgroup` and `option` match either (`div[disabled]`
  is neither).
- Malformed selectors (miaskiewicz/turbo-dom#18): `query_selector_all_checked` /
  `query_selector_checked` / `matches_checked` accept exactly what Chrome's
  `querySelector` accepts (the `parse_for_cascade` grammar). Newly rejected, as in Chrome:
  a type selector after another simple selector (`[a]div`, `.x*`), a non-identifier name
  (`#1a`, `.1a`, `p:`), a simple selector after a pseudo-element (`::before.x`), an invalid
  `An+B` (`p:nth-child(foo)`, `:nth-of-type(1 of p)`), a plain pseudo-class with an
  argument (`:first-child(1)`) or a functional one without, unknown pseudos, `:selected`,
  empty lists/items and a trailing combinator (`""`, `a,,b`, `p >`). Stylesheets
  (`parse_for_cascade`) drop the same rules, and a pseudo-class after a pseudo-element is
  allowed only where Chrome allows it (`::-webkit-scrollbar:hover`, `::part(x):hover`,
  `::selection:window-inactive`; not `::before:hover`). The lenient APIs never match an
  invalid nth argument or wrong-arity pseudo. Attribute selectors follow Chrome's grammar
  too: the name and an unquoted value are identifiers (`[1a]`, `[a=1]`, `[a=]`,
  `[href=a.html]` are invalid), `[*|a]` matches the attribute in any namespace and `[|a]`
  in none (an undeclared `[ns|a]` is invalid), and `:has()` cannot nest. (The `s`
  modifier stayed a Rust extension here; P5b-X2 above rejects it like Chrome 156.)
- `:lang()` (miaskiewicz/turbo-dom#18) is evaluated by the DOM APIs too (it was
  stylesheet-only and never matched in `matches` / `querySelector*`). As in Chrome 156 it
  takes exactly one identifier (`:lang("de")` and `:lang(en, de)` are invalid, also in
  stylesheets), matches the range or a `-` subtag extension, and reads the nearest
  `xml:lang` or `lang` attribute on any element (SVG `lang` counts; `lang=""` matches
  nothing).
- Pseudo-class names are ASCII case-insensitive in the DOM APIs too (`P:FIRST-CHILD`,
  `:Not(.x)`, `:LANG(en)`; miaskiewicz/turbo-dom#18) — stylesheets already were.
- `:defined` (miaskiewicz/turbo-dom#18) is evaluated by the DOM APIs too (it never
  matched there): every element except an HTML autonomous custom element or one parsed
  with an `is` attribute (`<button is="x-b">`, also `is=""`), whose custom element state is
  "undefined" in a document without definitions. The stylesheet side gains the `is` rule.
  (Elements upgraded by a script-defined custom element stayed `:not(:defined)` here;
  P5b-X2 above fixes that.)
- `:dir(ltr|rtl)` (miaskiewicz/turbo-dom#18) is evaluated — DOM APIs and stylesheets; it
  never matched. HTML directionality as Chrome 156 computes it: a valid `dir` attribute
  (ASCII case-insensitive), `dir=auto` / `<bdi>` by the first strong character of the text
  (bidi classes from the new `unicode-bidi` dependency; text inputs and textareas by their
  value; `bdi`, `script`, `style`, `textarea` and elements with a valid `dir` skipped),
  `input type=tel` `ltr`, otherwise the parent's (shadow content: the host's). One
  identifier argument; `:dir(auto)` / `:dir(foo)` are valid and never match.
- `<template>` content (miaskiewicz/turbo-dom#18): `query_selector_all` / `query_selector`
  (string and UTF-16), `get_element_by_id(_units)`, `get_elements_by_tag_name` and `:has()`
  no longer descend into a template's content fragment, and content nodes no longer match
  across it (`template p` is false for a `<p>` in the content), as in Chrome. Querying from
  the content fragment itself still finds its nodes. Consequence: a `<style>` inside
  template content no longer styles the document in `rtdom::cascade`.
- `:root` (miaskiewicz/turbo-dom#18) matches the document element only — no longer the
  top element of a document fragment, a shadow tree, template content or a detached
  subtree (Chrome).
- `rtdom::cascade` (`getComputedStyle`; miaskiewicz/turbo-dom#18) parses rule preludes with
  `parse_for_cascade`: lists split at top-level commas only (`:is(.a, .b)`, `:not(.x, .y)`,
  `:nth-child(2 of .a, .b)` and `[title="a,b"]` stay whole), an invalid list drops the whole
  rule (`.c, :bogus {}`), and items match with stylesheet semantics (`:scope` is `:root`).
  Specificity kept the rough per-item count here (P5b-X2 above switches to Selectors 4).
- npm `@miaskiewicz/turbo-dom` JS engine (`src/runtime/selectors.mjs`, miaskiewicz/turbo-dom#18):
  rewritten in lockstep with the Rust engine and asserted against the same Chrome 156
  control (`test/selectors-chrome.test.mjs`: every row's validity, `matches`,
  `querySelectorAll`, the cascade sentinel, and the template / `:root` probes). New:
  CSS escapes and non-ASCII identifiers, `*|` / `|` prefixes (types and attributes), the
  `i` attribute modifier, `:nth-child(An+B of S)`, `:link` / `:any-link`, `:lang()`,
  `:dir()`, `:defined` (reads the window's `customElements` registry), `:scope`,
  `:-webkit-any()`, the valid never-matching pseudo-classes (`:hover`, `:focus`, …) and
  pseudo-elements (never match); Chrome's `:empty`, `:root`, `:read-only` / `:read-write`
  and `:disabled` rules. **Breaking for lenient callers**: every selector Chrome rejects
  now throws `SyntaxError` — empty lists and items (`""`, `a,,b`), a trailing combinator
  (`a >`), a type after another simple selector (`div[id=x]y`, `.foo*`), empty or
  non-identifier names (`#`, `.1a`), unquoted non-identifier attribute values
  (`[href=a.html]`, `[a=]`), unknown pseudo-classes and pseudo-elements, `:selected` (use
  `:checked`), wrong arity / invalid An+B (`:nth-child()`), nested `:has()`, the `s`
  modifier. `cascade.mjs` splits rule selector lists at top-level commas and drops invalid
  rules, like `rtdom::cascade`.

### Fixed — browser binding (P5b-Z, Chrome 156 controls)
- `turbo-dom-browser-env`: the `Option(text, value, defaultSelected, selected)` legacy
  factory function exists (turbo-dom#20): a `new`-only native constructor sharing
  `HTMLOptionElement.prototype` that honours `NewTarget`, matching Chrome 156
  (`src/option_factory_chrome156.json`).
- `turbo-dom-browser-env`: `Reflect.defineProperty(select.options, i, {get(){}})` returns
  `true` and changes nothing, as in Chrome, instead of throwing; `Object.defineProperty`
  still throws, now with Blink's message (turbo-dom#20). Named-object definers receive
  V8's should-throw mode.
- `turbo-dom-browser-env`: `HTMLImageElement.prototype` has Chrome's `complete`,
  `naturalWidth`, `naturalHeight` and `currentSrc` accessors (turbo-surf#32). Without
  host state they report a not-loading image (`complete` true, 0×0, resolved `src`);
  loading hosts publish load state through the document record's
  `imageHost.setState(img, state)`. `test_support` exports the
  `HTML_IMAGE_*` probe/cases/control (70 files).
- `turbo-dom-browser-env/scripts/dom-implementation-chrome-control.mjs` labels its
  control `fixture: "dom_implementation_probe.js"` and writes `probeSha256`, like the
  committed `dom_implementation_chrome155.json` (it wrote `dom-implementation-probe.js`
  and no hash); a Chrome 156 recollection matches the committed result.

### Fixed — screenshot inline SVG, CI (P5b-W2, Chrome 156 controls)
- `turbo-dom-raster` inline `<svg>` (miaskiewicz/turbo-dom#25 item 1): the screenshot
  converts the parsed document's `<svg>` elements, not `<svg` source text. A raw-UTF-8 SVG
  `data:` URL in an `<img src>`, a `<script>` string, an attribute value or `<noscript>`
  text is left alone (it used to be cut out and rasterized, swallowing whatever lay
  between it and the next `</svg>`). The element is re-serialized as namespace-well-formed
  XML, so `xlink:href` without an `xmlns:xlink`, HTML character references (`&nbsp;`) and
  lowercase `viewbox`/`lineargradient` (adjusted by the parser) now paint. Control:
  `scripts/inline-svg-chrome-control.mjs` → `crates/turbo-dom-raster/tests/fixtures/inline_svg_chrome156.json`;
  `tests/image_formats_chrome.rs` now also compares the `svg-utf8` source.
- `turbo-dom-raster` `delazy_images`: a placeholder `src` (any `data:` URI, spacer names)
  with no lazy URL is no longer rewritten with its own unescaped value (that cut a
  `&quot;`-quoted SVG `data:` URL at its first `"`); inserted lazy URLs are
  attribute-escaped.
- CI: the raster job runs clippy and the tests with `--features avif` too.
- Corpus outputs: none change (`tools/parity/expected/p5b-w2-turbo-dom-25.json`).

### Bug fixes (P5b — intended, Chrome 156-controlled behaviour changes)
- turbo-dom-layout HTML drive (miaskiewicz/turbo-html2pdf#29): the whole document is
  styled as Chrome does (`Roots::Document` replaces the match-only `Roots::Shell`):
  `<head>` `<style>` sheets apply (all `<style>`s in document order, ahead of the caller's
  extra CSS; `HtmlOptions::linked_css` for fetched `<link>` sheets), `<html>`/`<body>` are
  real boxes (UA `body { margin: 8px }`, `head { display: none }`, root margins never
  collapse), `rem` resolves against the root element's font size (`root_font_scope`), and
  the root's (else `<body>`'s) background colour is propagated to the canvas (the galley
  root's background). New `Cascade::for_document`, `document_root_font_px`,
  `html_layout::propagate_canvas_background`. turbo-dom-raster no longer scrapes `<style>`
  blocks out of the raw HTML (`style_extract::collect_style_blocks` and the text-level
  script/noscript/template strips are gone) and fills the canvas from the propagated
  background instead of guessing it from the first full-width box. Control:
  `scripts/html-document-chrome-control.mjs` -> `crates/turbo-dom-layout/tests/fixtures/
  html_document_chrome156.json`; corpus diffs: `tools/parity/expected/p5b-turbo-html2pdf-29.json`.
- turbo-dom-layout image layer (miaskiewicz/turbo-html2pdf#30): EXIF orientation. The
  probe reads it from a JPEG `APP1` Exif segment or a PNG `eXIf` chunk (`Orientation`,
  `Intrinsic::orientation`; WebP `EXIF` is ignored, as in Chrome 156); boxes lay out at
  the displayed size (5-8 swap width/height) unless `image-orientation: none` (now
  inherited); `ImagePlacement::orientation` carries the transform to the painters —
  turbo-dom-raster turns the decoded pixels (PNG and SVG screenshots), turbo-html2pdf
  rotates the image `cm`. Breaking for struct literals: `Intrinsic` and `ImagePlacement`
  gained a field (`Intrinsic::new`). Control: `scripts/image-orientation-chrome-control.mjs`
  -> `crates/turbo-dom-layout/tests/fixtures/image_orientation_chrome156.json`; corpus diffs:
  `tools/parity/expected/p5b-turbo-html2pdf-30.json`.
- Image formats (miaskiewicz/turbo-surf#30, layout half): the shared probe sizes BMP,
  ICO/CUR (the largest entry) and AVIF (`ispe`; `image::avif_payload` demux) — new
  `Format::{Bmp, Ico, Avif}` (breaking for exhaustive matches); `image::probe_natural`
  (browser natural size incl. SVG at 1x with Chrome's default-object rules,
  `svg::natural_size`) and `image::decode_data_url` (base64 + percent-encoded `data:`) for
  turbo-surf's live image loads; the HTML drive's `data:` `<img>` decode now takes
  percent-encoded URLs too. turbo-dom-raster decodes BMP and ICO/CUR (`image` crate
  `bmp`/`ico`, PNG icon entries through the shared PNG decode) and, under the new `avif`
  feature, AVIF via `rav1d` (pure Rust, no assembly); TIFF stays unsupported as in
  Chrome. Control: `scripts/image-formats-chrome-control.mjs` ->
  `crates/turbo-dom-layout/tests/fixtures/image_formats_chrome156.json`; corpus diffs:
  `tools/parity/expected/p5b-turbo-surf-30.json`.

### Release (consolidation S5b)
- Versions: turbo-dom, turbo-dom-parser and npm 0.6.0; intra-workspace path
  dependencies carry `version = "0.6.0"` so the crates publish.
- `.github/workflows/publish.yml`: on a `v*` tag, after the full CI gate, publishes
  turbo-dom-v8 -> turbo-dom -> turbo-dom-layout -> turbo-dom-raster ->
  turbo-dom-browser-env sequentially (already-published guards, each waits for the
  crates.io index) and the npm package; `scripts/check-no-selector-dual.sh` runs first.
- CI per crate (base, layout, raster, V8 + browser-env matrix, MSRV 1.89, guards,
  verify-copied checks, parity harness self-test, per-crate coverage floors).
- `turbo-dom-browser-env`: the 18 custom-element lifecycle probe/control constants in
  `test_support` (turbo-surf's `tests/custom_elements.rs`); the Chrome-control
  collectors and `docs/live-css-host.md` moved from turbo-test into the crate.

### Added
- `turbo-dom-browser-env` 0.6.0: turbo-test's rtdom <-> V8 browser binding moved into
  this workspace (from miaskiewicz/turbo-test@cb5c64e), on V8 150.4, with
  `install_independent_srcdoc_units`, `install_html`, `document_html`, the
  `css_snapshot` JSON contract and the `host-internals`, `test-support` and
  `native-sdk-frame-read` features.
- `turbo_dom::rtdom::css_values`: the Chrome 156 `zoom` grammar (`zoom_value`) and
  Blink number serialization (`css_number`), moved from the binding so every consumer
  shares one grammar. turbo-dom now depends on `cssparser` 0.37.
- `turbo-dom-v8` 150.4.0: turbo-dom's rusty_v8 fork (interceptor `ShouldThrowOnError`
  fix, isolate weak-teardown fix, `UPSTREAM_V8_VERSION` download pin,
  `v8::FORK_SHOULD_THROW_FIX` marker), generated by `crates/turbo-dom-v8/scripts/generate.py`
  from pinned inputs; regression tests for interceptors and weak teardown.

- turbo-dom-layout `live` (consolidation S4): the live-DOM input ("surf mode") — style
  and lay out an embedder-composed live `Tree` without copying it (`live::capture` under
  fixed source budgets with exact errors, `live::snapshot` from pre-matched rules with a
  post-cascade hook; `source_id` mapper). turbo-surf's live-tree copier is gone.
- turbo-dom-layout `cssom_view`: turbo-surf's pure CSSOM-View geometry modules (metrics,
  hit testing, scroll, visibility, inline, text/caret, Range, the `Composition` scope
  graph), moved from turbo-surf-render; optional `serde` feature for its records;
  `set_media_viewport_width`.
- `rtdom::query::parse_for_supports`: the CSS `selector()` support condition (one
  unforgiving complex selector), used by turbo-surf's `@supports selector()` /
  `CSS.supports` (Chrome 156 control in turbo-surf).
- `scripts/cascade-selectors-chrome-control.mjs` and the Chrome-156 selector control
  `crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json` (S3a selector cases
  plus the S3b stylesheet cascade cases), asserted by turbo-dom and, end to end through
  the HTML path, by turbo-dom-layout ([docs/selector-engine.md](docs/selector-engine.md)).
- Stylesheet selectors on the one engine (`rtdom::query`, consolidation S3b):
  `parse_for_cascade` (Chrome stylesheet validity, pseudo-elements, Selectors-4
  `CascadeSelector::specificity`), `match_for_cascade` over a `Navigator` (the trait the
  matcher is now generic over; `LiveNavigator` keeps `matches`/`querySelector*` exactly
  as they were) and the cascade-mode `PositionCache`. `SelectorError::Invalid` is
  reported by `parse_for_cascade` only.
- turbo-dom-layout's cascade parses and matches with that engine instead of html2pdf's
  own selector parser and matcher (html2pdf PDFs, turbo-dom-raster static screenshots);
  `style::RuleSelectors`, `html_layout::style_html_tree`. Output changes only where the
  old engine disagreed with Chrome 156 (escapes, namespace prefixes, namespaced
  attributes, `:has()`, `:read-*`, `:required`/`:optional`, `:nth-last-*`, `of S`,
  unknown/case-folded pseudo-classes, pseudo-elements, specificity), each covered by the
  Chrome control; no release-corpus output changed.

- turbo-dom-layout's shared image layer (consolidation S5): `ImageResolver::intrinsic`
  (layout sizes every image through it; default = the header `probe`), `PngFrame` /
  `PngBudget` / `decode_png_rgba` (one PNG decode: PDF native samples and
  `image::to_rgba8`-identical RGBA), `Rgba8Image`, and under the new `svg-rasterize`
  feature `svg::rasterize(bytes, scale, Oversize)` / `css_size` / `pixel_size` (one SVG
  rasterizer: PDF 2x + clamp, raster 1x + reject). turbo-dom-raster decodes PNG and SVG
  through it and no longer re-encodes images to PNG for layout sizing. No output changes
  (differential tests over the parity image inputs).
- `FontRegistry::system(SystemFontPolicy)` / `system_cell` / `load_system`: one
  process-wide installed-font load per policy; turbo-dom-raster's `system_fonts` renders
  use it.
- turbo-dom-layout `test_support` (`test-support` feature): font fixtures from
  `assets/fonts`, `ComputedStyle` pairs and test-only tree literals, shared with
  turbo-html2pdf-core's tests (which no longer keep a copy of the fonts).

### Changed
- The root `turbo-dom-parser` crate is marked `publish = false` (it ships to npm only).
- turbo-dom-layout: the legacy owned-`Node` input is removed (`node` / `markup` modules,
  `Node`, `Element`, `NodeTree`, `tree_from_nodes`, `TreeView::to_nodes`, `style_tree*`,
  `html_layout::{parse_html, parse_html_with_roots, strip_non_visual, collect_style_css}`);
  `Tag` / `Attr` / `MarkerKind` / `MarkerClassifier` move to `tag` (root re-exports
  unchanged); `layout_snapshot::snapshot_{,html_}live_tree*` take a `StyleInput`;
  `style::presentational_declarations(local, attrs)`. html5ever / markup5ever_rcdom are no
  longer dependencies of turbo-dom-layout.
- `[workspace.lints]`: turbo-dom, turbo-dom-layout and turbo-dom-raster share the
  pedantic clippy gate (layout and raster newly pedantic-clean; `#[must_use]` added to
  their pure functions).

### Fixed
- Selector engine (Rust `rtdom::query`, both the string and the UTF-16 API) now matches
  Chrome 156 for the bug list in
  [miaskiewicz/turbo-dom#18](https://github.com/miaskiewicz/turbo-dom/issues/18):
  - CSS escapes (`.md\:flex`, `#\61`, `\70`) are decoded by the string API, and
    `*_checked` accepts escapes and non-ASCII identifiers (`.café`, `.日本`);
  - type selectors accept the `*|` (any namespace) and `|` (no namespace) prefixes;
  - `:empty` ignores comments, processing instructions, empty text nodes and a
    `<template>`'s content;
  - `:read-only` / `:read-write` follow the HTML read-write rule (mutable text-like
    `input`, mutable `textarea`, editable content; fieldset-disabled controls are not
    mutable; non-HTML elements match neither);
  - `:nth-child(An+B of S)` and `:nth-last-child(An+B of S)` are supported;
  - `:link` / `:any-link` match HTML `a`/`area` and SVG `a` with an `href`.
  For page JS (via turbo-dom-browser-env) the `*|p`, `:empty`, `of S` and `:link` fixes
  are visible; escapes already went through the UTF-16 parser, selector validity is the
  binding's own admission, and `:read-only`/`:read-write` are rewritten by the binding,
  so those reach Rust callers and CSS consumers (e.g. `rtdom::cascade`) only.
  Still open in #18: `:lang()`/`:dir()`, `:defined`, querySelectorAll/getElementById
  descending into `<template>` content, `:root` on fragments/detached nodes, malformed
  selectors accepted by `*_checked`, and porting the fixes to the npm `selectors.mjs`.

## [0.5.8] — 2026-10-25 — iframe srcdoc no-quirks parsing

### Added
- `Tree::parse_srcdoc_utf16` and `parse_srcdoc_utf16_with_source_positions`, which
  parse with html5ever's `iframe_srcdoc` flag so a `srcdoc` document without a
  doctype is no-quirks (`CSS1Compat`), matching Chrome. Consumed by turbo-surf's
  Chrome-parity work (turbo-surf#28); verified there against a Chrome 156 control.

## [0.5.7] — 2026-10-21

### Fixed
- Parse CSS attribute case modifiers separately from quoted/unquoted values in
  scalar and original-unit selectors. Apply ASCII-only `i/I` matching consistently
  across all six comparison operators, preserving escapes, comments, literal
  suffixes and lone UTF-16 units. Public Rust `s/S` remains a standards-defined
  exact-match policy, separate from Chrome155 browser admission.
- Normalize HTML attribute names during selector parsing and apply the measured
  HTML `input`, `button` and `ol` type-value defaults, preserving foreign and
  no-namespace attribute names and unrelated data values. Preserve explicit
  sensitive overrides; XHTML-in-XML document-mode policy is outside these controls.
- Keep invalid modifiers distinct from literal operands and retain forgiving
  selector-list behavior. Add an independently captured, source-pinned Chrome155
  oracle and public Rust regressions for the native hidden-search-field defect.

### Added

- Add `Tree::create_contextual_fragment_utf16(context, units, scripting_enabled)`
  for detached HTML parsing against a real element, including namespace,
  integration-point attributes, nearest form ancestor and document quirks mode.
  Preserve original UTF-16, existing context children and pending mutation
  records. Script/noscript tokenization follows the explicit scripting flag;
  the parser never executes scripts or invokes custom-element callbacks.

- Expose live form-state override reads to native embeddings; reject invalid
  setter handles and avoid cache invalidation for repeated identical writes.
  Restrict `:checked` to checkbox/radio inputs and selected options rather than
  treating arbitrary checked attributes as browser state.

- Store custom element states as exact UTF-16 keys independently of attributes.
  Add public set, membership, clear and enumeration APIs for native embeddings;
  real membership changes invalidate selector and computed-style caches without
  emitting attribute mutation records.
- Match `:state()` CSS identifiers with escaping, comments, case-insensitive
  pseudo names and CSS input replacement, preserving exact DOM-string state
  identity. Add fresh Chrome155 controls for arbitrary keys and clone/import/
  adoption semantics. Native adoption can enumerate and reapply states; cloning
  and import do not copy private state.
- Charge custom state payload to storage collection budgets and reclaim it with
  unreachable nodes while keeping externally pinned states intact.

### Validation
- Rust 1.99.0 passes 342 backend contracts and seven parser tests, all three
  strict Clippy variants, changed-file formatting, rebuilt native and Node/Web
  WASM packages, 548 JS tests, 29 vitest tests and two Jest tests.
- Fresh Chrome155 controls cover 57 comparable attribute-selector cases and five
  XML cases. Seven separate Rust `s/S` controls preserve the public standard
  policy; Chrome155 browser admission of that modifier is deliberately distinct.
- The isolated allocation gate remains 367 allocations and 81,606 bytes per
  operation (ceilings 380/85,000). html5lib retains five known failures
  (1778/1783 evaluated; eight skipped).

## [0.5.6] — 2026-10-15

### Added
- Public native `Tree::take_template_contents` extracts only genuine parsed HTML
  template fragments while preserving ordinary light children, mutation records
  and collection ownership. Retain extracted components with explicit collection
  pins. Keep nine reviewed extraction controls, including imported UTF-16, cache
  invalidation, shadow roots, failure behavior and eventual storage reclamation.
- Optional original-input parser provenance through `Tree::parse_with_source_positions`
  and `parse_utf16_with_source_positions`. Token-time lexical node positions and
  attribute installation context preserve CRLF, non-BMP and lone UTF-16 units,
  distinguish duplicate sources and later html/body attributes, and keep
  reconstructed/synthetic node spans absent. Source metadata follows stable
  handles and is reclaimed by bounded native storage collection.
- Fresh normal Chrome155.79 / V8.23 controls cover13 mainframe parser source
  cases; an independent inert-template control is retained for the browser
  binding's owner-document reporting semantics.

### Validation
- Rust 1.99.0 passes 326 backend contracts and seven parser tests, all three
  strict Clippy variants, changed-file formatting, rebuilt native and Node/Web
  WASM packages, 548 JS tests, 29 vitest tests and two Jest tests.
- The isolated allocation gate remains 367 allocations and 81,606 bytes per
  operation (ceilings 380/85,000). html5lib retains its five known failures
  (1778/1783 evaluated; eight skipped).

## [0.5.5] — lossless native DOM strings

### Added
- Native UTF-16 payload, name, namespace, doctype and processing-instruction APIs
  preserve lone surrogates through editing, cloning, serialization, selectors and
  mutation records. Existing scalar getters remain compatibility views.
- Original-unit HTML document and fragment parsing uses a source-pinned,
  license-preserved html5ever tokenizer adaptation. Native code immediately
  decodes its collision-free internal transport into original unit storage.

### Fixed
- Resolve ID, class and unqualified CSS attribute selectors through exact
  null-namespace attributes; namespaced lone-unit values cannot hide ordinary
  IDs or alias literal replacement characters.
- Charge original-unit doctype fields and processing-instruction targets before
  collection commits; insufficient payload budgets reject atomically.
- Preserve SVG/MathML fragment contexts and integration-point attributes in both
  scalar and unit parsing, including adjacent insertion and BOMs.
- Consume CRLF as one CSS hexadecimal-escape whitespace terminator.
- Preserve template content and interior BOMs in original-unit DOM parsing.
- Use iterative native parser, import, text and serialization walks; regressions
  cover8192 nested levels on a512KiB host stack.
- Keep ordinary scalar DOM payloads compact. The unchanged mutation-churn gate
  measures367 allocations and81,606 bytes per operation (ceilings380/85,000).

### Tests
- The backend passes 305 contracts and the parser passes seven. All three strict
  Clippy variants, rebuilt native/Node WASM/Web WASM, 548 JS tests, 29 vitest tests
  and two Jest tests pass. The html5lib report retains five known failures
  (1778/1783 evaluated; eight skipped).

## [0.5.4] — 2026-10-14

### Added
- Opt-in native `Tree::collect_unreachable` with explicit external handle pins,
  node/edge/payload budgets, atomic rejection, and retained-storage diagnostics.
  Connected documents, composed detached components and undrained mutation
  events remain live. Embedders must pin all independent native references.
- `Tree::is_live` distinguishes reclaimed identities. Handles are never reused;
  `handles()` enumerates live identities after collection creates gaps.

### Fixed
- Successful collection frees unreachable parsed SoA/string pools, runtime
  node slots and mutation overlays while retaining stable node identities,
  attributes/namespaces, shadows, templates, character data and pending records.
  The dense parse/mutation path remains available until collection is requested.
- Collection does not emit DOM mutations or increment the visible tree version.
  Internal selector/style caches are refreshed after physical storage changes.

## [0.5.3] — 2026-10-14

### Added
- Additive native `MutationEvent` and `Tree::take_mutation_events` surface records
  with mutation-time light-tree ancestry, sibling positions and attribute namespaces
  for embedder observer queues. The existing `MutationRecord` API remains available;
  both APIs drain one opt-in buffer, with no event allocation while recording is off.

### Fixed
- Include generated WASM package metadata in the npm bundle so both parser targets
  retain their release version and module format.
- Native moves emit removal and insertion events with their actual sibling positions;
  invalid-parent removals leave nodes intact and emit no event.
- Legacy native observer filtering uses captured ancestry after later moves/removals.
- Native `innerHTML` reports the complete child replacement; absent attribute removal
  and empty-to-empty child replacement do not emit spurious mutation records.

## [0.5.2] — 2026-10-13

### Added
- Native detached document fragments, including imported template content and
  ordered insertion of fragment children without inserting the fragment itself.
- Namespace-aware element/attribute creation, native Attr identity and ownership,
  document type and processing instruction nodes, and document compatibility mode.
- Host CSS policy filtering and separate effective inline-style state so security
  policy enforcement preserves authored attributes while affecting the cascade.

### Fixed
- Detached element/fragment selectors traverse their own subtree in document order.
- HTML serialization respects element namespaces and retains processing
  instructions, doctypes and namespaced attributes.
- Rust test assertions provide the failed collection value and pass the updated
  stable Clippy gate.

### Performance
- Box rare Attr/doctype/processing-instruction payloads and promote attribute
  lists to namespace records only when needed. Ordinary mutation churn retains
  363 allocations and 81,510 bytes per operation, within the existing gate.

### Release
- Align the npm package, native parser, Rust DOM crate and generated WASM package
  metadata at 0.5.2.

## [0.5.1] — cascade: keep the parsed rule index across DOM mutations

### Performance
- `getComputedStyle` no longer re-parses every `<style>` sheet on each DOM mutation. The parsed
  rule index was keyed on the DOM `version`, which bumps on *any* mutation — so a React re-render
  (which restructures nodes but leaves the injected CSS untouched) threw the index away and
  re-parsed all stylesheets on the next `getComputedStyle`. The index is now keyed on a hash of
  the `<style>` **sources**: it survives version bumps whose CSS is unchanged and is re-parsed
  only on a real stylesheet edit. Per-element resolved maps are still invalidated on every bump
  (selector matching depends on tree shape). Profiling a component suite showed `parse_stylesheet`
  / `parse_decls` recurring under `computed_style` on the re-render path; this removes that repeat
  work. No behavior change — verified by the full cascade suite plus a new reuse/invalidation test.

## [0.5.0] — rtdom internal consistency (stale parent pointers, unset event target)

Fixes the two correctness issues filed in
[#4](https://github.com/miaskiewicz/turbo-dom/issues/4). Both are internal-consistency
bugs, not regressions — neither is reachable through `query_selector*` or serialize.

### Fixed
- **Replaced subtrees are now detached in both runtimes.** `textContent` / `innerHTML` /
  `replaceChildren` overwrite a node's whole child list; they never cleared the OUTGOING
  children's parent pointer, so a retained reference into the replaced subtree kept
  reporting the old parent and walking `parent()`/`parentNode` upward climbed back into the
  live tree. Both runtimes now cut that link (rtdom `Tree::orphan_children`, JS
  `Node.__detachKids`) exactly as `remove_child`/`removeChild` already did on the
  one-at-a-time path. The detached subtree itself stays intact — only its top link is cut.
  No hot-path cost: rtdom moves the existing child overlay out (`SmallVec`, no `Vec`) or
  walks the `SoA` sibling chain in place, and the JS side reads `__kids` directly, so an
  un-inflated buffer-backed child list is never materialized just to null parent pointers.

### Changed (BREAKING — `turbo-dom` crate only)
- **`Event::target` / `Event::current_target` are `Option<Handle>`** (were `Handle`). A
  fresh `Event` defaulted them to `Handle(0)` — the DOCUMENT ROOT — so a pre-dispatch read
  silently claimed the event targeted the document. `dispatch_event` always set `target`
  before any listener ran, so no dispatch behavior changes; this closes the latent footgun
  by modeling "unset" as `None`, matching the sentinel-free parent-pointer modeling adopted
  in 0.4.0. In-process Rust consumers that read `event.target` need `event.target.unwrap()`
  / a `Some(_)` match. The npm package and the napi/wasm parser API are unaffected.

### Added
- Consumer-contract coverage (`crates/turbo-dom/tests/consumer_contract.rs`) for the
  `Dom`/`Event` dispatch surface and for subtree-replacement detachment, so both shapes now
  fail CI before a break can reach turbo-test / turbo-surf.

## [0.4.0] — selector engine rewrite + Rust-native DOM runtime consolidation

Touches BOTH runtimes at parity. The napi/wasm parser API is unchanged; `src/runtime/*.mjs`
gains the new selector engine and a few correctness fixes (below).

### Added (JS runtime + rtdom, mirrored)
- **CSS selector engine rewrite** — a tokenizer + recursive-descent parser producing a non-empty
  `head + tail` AST ("parse, don't validate"), replacing the ad-hoc string-splitting. Correct
  backtracking over mixed child/descendant chains (`.a > .b .c`), and parse-cached. Mirrored Rust ↔ JS.
- **Full combinator + list coverage** — `+`/`~` combinators (closed the Rust gap),
  `:is()`/`:where()`/`:not()` selector lists, and the relational `:has()` — browser-level selectors
  in both runtimes.
- **rtdom live form-state properties** — `Tree::set_form_property`/`clear_form_property` let a Rust
  consumer drive `:checked`/`:selected`/`:disabled`/`:required`/`:read-only` independently of the HTML
  attribute (the Rust analogue of React assigning `el.checked`), matching the JS runtime's live-property
  reads. Parse/no-interaction path is unchanged (attribute-driven).
- **rtdom opt-in strict selector validation** — `query_selector_all_checked`/`query_selector_checked`/
  `matches_checked` return `Err(SelectorError)` for malformed selectors (an unterminated `[...]`, an
  unexpected char, or a stray combinator — the inputs a browser rejects with `SyntaxError`), while the
  infallible `query_selector*` path stays lenient and fast.

### Fixed
- **`:has()` leading combinator** bound the wrong compound: `:has(> .a .b)`, `:has(> .a > .b)`,
  `:has(+ div .x)` returned nothing. The leading combinator now constrains the relative selector's
  HEAD compound, not the whole complex. (both runtimes)
- **rtdom panic** on an unterminated quoted attribute value (`div[a="`) — a lone quote sliced out of
  range; now guarded (`len >= 2`), degrading gracefully.
- **JS `after()`/`replaceWith()`** threw `NotFoundError` when an inserted node was the reference node's
  own next sibling (`a.after(b, c)`); now anchors on a viable-next-sibling (rtdom parity).
- **JS `@import`/`@charset`** statement at-rules no longer swallow the following CSS rule — the
  stylesheet scanner stops at `;` outside a block (rtdom parity).

### Rust-native DOM runtime consolidation (`turbo-dom` crate)
- **`crates/turbo-dom`** — a pure-Rust port of the DOM runtime for in-process **Rust** consumers
  (crawlers/extractors/SSR): lazy COW tree over the SoA, version-cached queries, partial
  `getComputedStyle`, events, shadow DOM, serialize, plus color/cssom/svg/file/canvas/
  custom_elements/location/mutations/node_ref. ~2.7× the JS runtime on chatty access (zero
  boundary); 100% line coverage, 227 tests, a direct html5lib-tests gate at 99.75%. Published to
  crates.io as `turbo-dom` — self-contained (html5ever + rustc-hash only, no napi/wasm),
  vendorable, with a runnable `examples/crawl.rs`.
- **One source of truth.** The engine lives only in the crate: there is no in-repo `src/rtdom`
  copy and no `rust-runtime` cargo feature (an earlier off-by-default feature gated a duplicate
  copy; it was consolidated away — the npm `.node`/wasm artifacts stay lean regardless, as the
  crate is never compiled into them). Build/test from the repo: `npm run build:rtdom`
  (= `cargo build -p turbo-dom --release`), `npm run test:rust` (= `cargo test -p turbo-dom`),
  `npm run conformance:rtdom`.
- **Build/CI hardening.** The shipped napi addon is symbol-stripped (~−169 KB). Both crates adopt
  `clippy::pedantic` (warn-gated, CI-enforced at `-D warnings`), and a deterministic mutation-churn
  allocation gate (`churn_alloc_gate`) runs in CI, locking the create/append/remove path at ≈363
  allocs/op. `mise.toml` pins the Rust + Node toolchain and provisions prebuilt `wasm-pack`/`wasm-tools`.
- Architecture + the per-commit JS-perf-win → Rust mapping: `RUST_PORT_PLAN.md`,
  `RUST_PORT_PERF_HISTORY.md`. (A Phase-1 spike confirmed the spec's thesis: a Rust DOM exposed
  to JS via WASM is ~0.55× the JS runtime — the boundary loses — so rtdom is Rust-only.)

## [0.3.5]

### Fixed
- `rtdom::serialize` no longer escapes regular spaces to `&nbsp;` (a quirk carried over
  from the old JS serializer). It corrupted serialized text into U+00A0 non-breaking
  spaces, so a re-parse / accessible-name / text query saw `"a b"` instead of
  `"a b"` and whitespace-normalized matching (getByRole/getByText/toHaveText on a
  serialized DOM) failed. Spaces now serialize verbatim; only `&`/`<`/`>`/`"` are escaped.

## [0.3.4] — rtdom ChildNode/ParentNode + insertAdjacent + toggleAttribute + getAttributeNS

Crate-only (`crates/turbo-dom`); the npm JS runtime + parser are unchanged. Found via turbo-crawl,
which renders over rtdom — these are the DOM methods server-HTML hydration paths reach that rtdom
lacked, so a consumer had to shim them in JS.

### Added
- **`Tree` ChildNode/ParentNode manipulation** — `before`, `after`, `replace_with`,
  `replace_children` (insert/replace relative to a node or among a parent's children).
- **`insert_adjacent_element` / `insert_adjacent_html`** — position-relative insertion
  (`beforebegin` | `afterbegin` | `beforeend` | `afterend`); the HTML form parses a fragment in the
  appropriate context and imports the nodes.
- **`toggle_attribute`** (with optional `force`) and **`get_attribute_ns`** (namespaced read, matches
  the local name then any `prefix:localName` — covers SVG `xlink:*`).

## [0.3.3] — rtdom CharacterData mutation (`turbo-dom` crate)

Crate-only (`crates/turbo-dom`); the npm JS runtime + parser are unchanged.

### Added
- **`Tree` CharacterData methods** — `set_node_value`, `insert_data`, `delete_data`, `append_data`,
  `replace_data`, `substring_data`, and `split_text`, operating natively on rtdom's text storage
  (offsets/counts in chars; each records a `characterData` mutation so a `MutationObserver` sees the
  edit). These are the spec methods a contenteditable editor (Lexical) + `@testing-library/user-event`
  drive when typing into a contenteditable: the keypress path inserts characters via
  `textNode.insertData()` and a `Range` splits text with `splitText()`. Previously a consumer had to
  shim these in JS over the `data` accessor (string-slicing per edit); native is cleaner and emits the
  mutation record the editor model relies on. `cascade::computed_style` (already inheritance-aware,
  incl. `visibility`) now also backs a consumer's `getComputedStyle` inherited-property resolution.
- `setClock(fn)` (exported from `@miaskiewicz/turbo-dom/runtime`): injectable clock that both
  `performance.now()` and the `requestAnimationFrame` callback timestamp read through; default =
  real host clock. Lets a render tier drive virtual time so time-gated MUI/transition rAF loops
  (`progress=(now-start)/duration`) reach completion instead of spinning.
- `requestAnimationFrame` schedules via the **live** `globalThis.setTimeout` (16ms frame), so a
  tier owning `setTimeout` catches every reschedule; `cancelAnimationFrame` uses live `clearTimeout`.
- `MessageChannel`/`MessagePort` are now a real built-in polyfill (was a host passthrough) routing
  delivery through live `globalThis.setTimeout` — React 19's scheduler runs in the owned/virtual
  queue and a `MessageChannel` exists in the bare V8 isolate.

## [0.2.3] — synthetic geometry
- `getBoundingClientRect`/`offset*`/`client*`/`scroll*` **size** is now a cheap synthetic box model
  (non-zero, stable per DOM version, children fit parents; positions stay 0) — breaks React/MUI's
  measure→setState→re-measure hydration loop. `matchMedia` parses width/height/orientation against
  the viewport; `ResizeObserver`/`IntersectionObserver` fire once with an initial entry.

## [0.2.2] — React 19 attribute nodes
- `Element.removeAttributeNode`/`setAttributeNode` (+ NS aliases) — fixes the React 19
  `releaseSingletonInstance` crash during hydration.

## [0.2.1] — bare-isolate runtime load
- Drop the static `node:perf_hooks` import; `Buffer`-free base64 — runtime loads in a bare V8.

## [0.2.0] — CSSOM fidelity
- `rgb()` color canonicalization, light-DOM style inheritance, `<style>` `textContent` reflects
  `insertRule`.

## [0.1.62] — SVG support
- SVG DOM-property wrappers (`SVGElement`): `el.className.baseVal`, etc.

## [0.1.61]
- Fix CI publish (wasm toolchain) — unblocks npm release.

## [0.1.60]
- Lazy parser registry: WASM/native selection + node-free embedding.

## [0.1.59]
- Minimal CSSOM for CSS-in-JS, host-global load guards, install exports.

## [0.1.58]
- Build + ship the wasm fallback by default; wire native→wasm load.

## [0.1.57]
- Republish: `index.d.ts` regenerated for the HR2 `JsSoa` types.

## [0.1.56] — perf
- Ship SoA string tables as one byte blob — ~9% faster parse.

## [0.1.55] — perf
- Pre-reserve intern-map capacity — ~6% faster parse (realistic doc).

## [0.1.54] — perf
- `FxHashMap` for SoA string interning — ~6–7% faster parse.

## [0.1.53] — perf
- `DocumentFragment.cloneNode` builds the child array directly (+8%).

## [0.1.52] — perf
- Deep `cloneNode` builds the child array directly — ~2× faster.

## [0.1.51] — perf
- `nextElementSibling`/`previousElementSibling` O(n²)→O(n).

## [0.1.50] — perf
- Version-cache `Element`/`ShadowRoot` `getElementsBy*` subtree walks.

## [0.1.49] — perf
- Share the version-cached element-child array across the `child*` getters.

## [0.1.48] — perf
- Version-cache the children element filter — ~58% faster children access.

## [0.1.47] — perf
- Unpack the SoA blob once per cached HTML — ~27% faster `createEnvironment`.

## [0.1.46] — perf
- Lazy `window.origin` — ~18% faster `createEnvironment`.

## [0.1.45] — perf
- Lazy `customElements` registry — ~5% faster `createEnvironment`.

## [0.1.44] — perf
- Read `nodeType` once during child inflation — ~5% faster inflation.

## [0.1.43] — perf
- Read `tagName` once in `__nodeAt` inflation — ~5.5% faster inflation.

## [0.1.42] — perf
- Skip the listener-snapshot slice for single-listener dispatch — ~13% faster.

## [0.1.41] — perf
- Inline `addEventListener` option parsing — ~33% faster listener attach.

## [0.1.40] — perf
- Skip the parse-cache LRU re-insert when the key is already MRU.

## [0.1.39] — perf
- `textContent` fast-path for single-text-child elements — ~28% faster reads.

## [0.1.38] — perf
- Lazy `Event._path` — ~2× faster Event construction.

## [0.1.37] — perf
- Zero-alloc mutations when no `MutationObserver` — ~17% faster.

## [0.1.36] — perf
- Lazy `Document.__mo` — ~6% faster `createEnvironment`.

## [0.1.35] — perf
- Hoist env-independent lazy globals to module scope — ~29% faster window construct.

## [0.1.34] — perf
- Monomorphic `Element` shape — ~7–8% faster real-world suites.

## [0.1.33] — Shadow DOM
- Full Shadow DOM: slots, retargeting, `:host`/`::slotted`, declarative shadow roots; coverage gate.

## [0.1.32]
- Docs: document the partial `getComputedStyle` cascade in the README.

## [0.1.31]
- Normalize `font-family` comma spacing in computed style.

## [0.1.30]
- WHATWG number sanitization; computed-style px-normalize & shorthand expand.

## [0.1.29]
- DOM-correctness batch + partial `getComputedStyle` cascade.

## [0.1.28]
- Fix serializer crash on lazy `__attrs` (buffer-backed elements).

## [0.1.27] — perf
- Packed SoA blob, dict attr values, lazy attr reads.

## [0.1.26] — perf
- Allocation-free `.class` matcher + memoized `HTMLCollection`.

## [0.1.25] — perf
- Lazy `__attrs` allocation (drop the eager `__attrs=[]`).

## [0.1.24] — perf
- Live-collection index fast path + lazy event listeners.

## [0.1.23] — perf
- Parse memoization + lazy attrs — realistic suite ~40× happy-dom.

## [0.1.22] — perf
- Static/dynamic window split — `createWindow` 15µs → 2.5µs.

## [0.1.21] — perf
- Memoize live views + skip listener-less event propagation.

## [0.1.20] — perf
- Per-version query-result cache — repeated queries beat happy-dom.

## [0.1.19] — perf
- Version-keyed `getElementsBy*` cache — `getByLabelText` 4.8× faster.

## [0.1.18] — perf
- Allocation-free selector/match hot paths — query-heavy ~1.8× faster.

## [0.1.17]
- Add `license` field (MIT) so npm shows the license.

## [0.1.16]
- Implicit `<label>` labels only its first control.

## [0.1.15]
- `click`-in-progress flag fixes programmatic `.click()` re-entrancy.

## [0.1.14]
- Proper `document.cookie` jar.

## [0.1.13]
- Fix regression: tag-class prototype `===` `Element.prototype`.

## [0.1.12]
- Tag-specific `HTML*Element` `instanceof`.

## [0.1.11]
- Positional pseudo-classes + `anchor.download`/`rel`/`src` reflection.

## [0.1.10]
- Date/time value sanitization, `:checked` pseudo, `window === globalThis`.

## [0.1.9]
- Focus blur-on-move, File-preserving `FormData`, no `transformMode` warning.

## [0.1.8]
- `window.open`/`scrollTo` + CSS `!important` priority.

## [0.1.7]
- Form-control fidelity: change events for all input types.

## [0.1.6]
- `Event.initEvent` + typed `createEvent` + fix `performance.now`.

## [0.1.5]
- Verify + CI-guard jest support.

## [0.1.4]
- Fix vitest 4 environment adapter + accessible-name (`getByRole`).

## [0.1.3]
- Broaden the DOM/window surface for real-suite compatibility.

## [0.1.2]
- Scope the package as `@miaskiewicz/turbo-dom`.

## [0.1.1]
- First tagged release after the initial publish.

## [0.1.0] — initial release
First public `@miaskiewicz/turbo-dom`. (Developed as fast-dom → gr0gdom → turbodom → turbo-dom.)
### Added
- Native html5ever HTML parser with napi + wasm front-ends (html5ever 0.39, ~99.72% conformance).
- Structure-of-Arrays flat-buffer parser with lazy typed-array inflation; lazy copy-on-write DOM,
  lazy window, honest stubs, fast reset.
- Core DOM surface: `dataset`, `<select>`, real `MutationObserver`, missing globals; attribute
  name/prefix interning.
- vitest + jest environment adapters; npm packaging.
- CI: cross-platform napi prebuilds + Linux test/conformance + tag-triggered publish; platform
  `optionalDependencies`; single bundled package shipping all binaries.
- Benchmarks + conformance deltas vs happy-dom / jsdom / parse5.

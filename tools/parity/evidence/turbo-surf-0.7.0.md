# turbo-surf 0.7.0 parity evidence

This is the Chrome 156 control behind `expected/turbo-surf-0.7.0-release.json` and
`expected/turbo-surf-0.7.0-embedded-font-range.json`. It is committed so that both expected
files point at evidence in the tree.

- Host: macOS arm64, TZ=UTC LANG=C, release napi addons.
- Oracle: the frozen Chrome 156.0.8078.47 headless shell, launched with
  `--use-mock-keychain --password-store=basic`.
- Builds: turbo-surf 0.6.0 (main f03de6e, turbo-dom 0.6.0) against turbo-surf release/0.7.0
  (PR #127, turbo-dom e425311).
- Baseline check: the 0.6.0 build reproduces checkpoint `b-0.6.0` exactly (0 failures).

## What changed

105 outputs changed (surf 86, surf-extra 19), and no image changed size. Of these, 41 are
declared by `expected/turbo-dom-24*.json`, and the two release files declare the other 64.

- Static: text and line boxes move vertically. On most pages the SVG numbers move by at most
  2.6 px. The causes are line-height with no floor (#178), Chrome's line box model (#245) and
  1/64 px advances (#246). Their committed controls are
  `crates/turbo-dom-layout/tests/fixtures/html_document_chrome156.json` and
  `crates/turbo-dom-raster/tests/fixtures/paint_chrome156.json`.
- Moves over 3 px: extra-table-caption (7 px), extra-block-in-inline (3.9 px) and
  extra-clip-rect (12 px clip offsets). embedded-font-range-* moves 8 px, which is a known gap
  (see below).
- extra-block-in-inline compared with Chrome: Chrome puts `A` at y34 h26 and the red `I` at y94.
  surf 0.6.0 gives 35.4 and 98.1, and 0.7.0 gives 36 and 100. The host default font accounts for
  most of the difference. This is a watch item, not a structural regression.
- Structure: the SVGs gain background clipPaths (#248) and the list-marker rects of
  conformance-06 (#128, which Chrome paints). The image order changes in extra-xref-anchors.
  Nothing else in the SVG structure changes.
- Live: unloaded and src-less `<img>` elements now get Chrome's 16x16 inline box, and
  `elementFromPoint` hits them (#241-#243). The coep-dom, csp and lifecycle hits now match
  Chrome's targets. The control is the `img-*` rows of
  `crates/turbo-dom-layout/tests/fixtures/live_geometry_chrome156.json`. The border-cascade
  outputs move by less than 0.07 px.
- One difference was already there before 0.7.0: in coep-dom, the `<sentinel>` is full width
  and the img sits on the next line. Chrome places the img inline at x=64.

No structural regression was found.

## Known gap: embedded-font-range-* (not comparable with Chrome)

These pages load a web font through `@font-face` for their buttons. turbo-surf's static path
loads no `@font-face`, so it lays the button text out in a fallback font, while Chrome uses the
web font.

| button | y | height |
|---|---|---|
| Chrome 156 | 52 | 21 |
| surf 0.6.0 | 50 | 28 |
| surf 0.7.0 | 44 | 26 |

The 0.7.0 move comes from the line-height changes above, applied to a different font. Neither
surf build says anything about Chrome parity for these two pages. They stay a known gap until
surf's static path loads web fonts.

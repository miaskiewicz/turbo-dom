# Selector engine Chrome fixes and the `selector-dual` gate (consolidation S3a)

turbo-dom's `query.rs` (string API) and `query_utf16.rs` (UTF-16 `*_units` API, which the
browser binding uses for selectors containing `\` or invalid UTF-16) are the one selector
engine shared by turbo-dom, turbo-dom-browser-env (turbo-test, turbo-surf live) and, from
S3b, turbo-dom-layout. Consolidation step S3a makes that engine match Chrome 156 for the
bug list in [miaskiewicz/turbo-dom#18](https://github.com/miaskiewicz/turbo-dom/issues/18)
and nothing else. Two checks prove it:

1. **Chrome control**: `scripts/cascade-selectors-chrome-control.mjs` records Chrome for
   Testing 156's answers into `crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json`.
   `crates/turbo-dom/tests/cascade_selectors_chrome.rs` asserts them.
2. **Dual-engine gate**: the non-default `selector-dual` feature runs a frozen copy of
   B-main's engine next to the current one at every engine output. Gate A checks that
   legacy mode reproduces the previous step exactly. Gate B checks that every new-mode
   divergence is explained by a bug-list entry.

## What S3a changed (turbo-dom#18)

| entry | before (B-main, v0.5.8) | now (Chrome 156) |
|---|---|---|
| escapes | `matches_checked(".md\\:flex")` failed with `UnexpectedChar('\\')`; the lenient string API split `.md\:flex` at the `:` | escapes are decoded: a selector containing `\` is parsed by the UTF-16 parser, the same one the binding already used for JS. The strict lexer accepts escapes. |
| non-ASCII | `_checked` rejected `.café` and `.日本` | CSS ident code points accepted |
| `*\|p` | rejected by `_checked`; the lenient API never matched | the `*\|` (any namespace) and `\|` (no namespace) type prefixes work. Other prefixes are undeclared: `_checked` reports `UnexpectedChar('\|')` and the lenient API never matches. |
| `:empty` | false with a comment child, and false on `<template>` (the content fragment counted as a child) | ignores comments, PIs, empty text nodes and template content. Whitespace-only text still counts. |
| `:read-only` / `:read-write` | keyed only on `readonly`, so non-form elements were read-write | follows the HTML rule. Read-write means a mutable `input` of a text-like type, a mutable `textarea`, or editable content (`contenteditable`). A disabled `<fieldset>` disables its controls, except those in the first `<legend>`. Non-HTML elements match neither. |
| `:nth-child(An+B of S)`, `:nth-last-child(An+B of S)` | unsupported | supported (lowercase `of`, as in Chrome) |
| `:link`, `:any-link` | always false | true for HTML `a` and `area` and SVG `a` with `href`; SVG `a` also accepts `xlink:href`. `:visited` is never true. |

The S3 dry run (old html2pdf matcher vs this engine, every element of every corpus input) found
no other live-Tree semantic change. `:defined` is therefore not part of S3a.

These gaps from #18 are still open: `:lang()`/`:dir()`, `:defined`, qsa/getElementById
descending into `<template>` content, `:root` on fragments and detached nodes, and malformed
selectors such as `p:nth-child(foo)` or `[a]div` being accepted by `_checked`. Other open
items are binding-admission error prefixes and porting these fixes to npm `selectors.mjs`.

## Regenerating the Chrome control

```bash
PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core \
node scripts/cascade-selectors-chrome-control.mjs [--chrome "/path/to/Google Chrome for Testing"]
cargo test -p turbo-dom --test cascade_selectors_chrome
```

For each case and selector, the fixture records:

- validity, from `document.querySelector`;
- `el.matches` for every element (document order, template content excluded);
- `querySelectorAll` as element indices;
- the `getComputedStyle(el).order` winner when the case document gets
  `<style>SEL { order: 1 }</style>` as the last `<head>` child.

The test asserts all of these through `matches`, `matches_checked`, `matches_units`,
`query_selector[_all]`, `query_selector[_all]_units` and `rtdom::cascade::computed_style`.

## The `selector-dual` feature

- `crates/turbo-dom/src/rtdom/query_frozen.rs` and `query_frozen_utf16.rs` are B-main
  (`a49793a`) `query.rs` and `query_utf16.rs`, changed only by the scripted rewrites in
  `scripts/verify-query-frozen.rules`:
  - every `impl Tree` fn is renamed `frozen_*`;
  - the engine uses its own `qcache` and `parse_cache`;
  - the nested `#[path]` points at the frozen file;
  - the test module is renamed `frozen_tests`.

  `scripts/verify-query-frozen.sh` re-derives both files from git and fails on any
  difference. `--write` regenerates them.
- The frozen caches live in `FrozenCaches`, on the per-`Tree` cascade cache slot. That
  slot is reset where `Tree.qcache` is (storage collection), and the frozen `qcache` is
  version-keyed like the original. This leaves `tree.rs` untouched.
- `rtdom::selector_dual` wraps every engine output, through the `dual!` macro at the top of
  each public fn in `query.rs` and `query_utf16.rs`:
  - `matches` and `matches_checked`;
  - `query_selector_all`, `query_selector_all_checked`, `query_selector` and `query_selector_checked`;
  - `get_element_by_id` and `get_elements_by_tag_name`;
  - `query_selector_all_units`, `query_selector_all_from_units`, `query_selector_units`,
    `matches_units` and `get_element_by_id_units`.

  Both engines answer every call, and callers get the answer that
  `TURBO_DOM_SELECTOR_ENGINE=legacy|new` selects. A dual build aborts if that variable or
  `TURBO_DOM_SELECTOR_DUAL_LOG_DIR` is unset.
- Each process appends to its own `$TURBO_DOM_SELECTOR_DUAL_LOG_DIR/<pid>-<nonce>.jsonl`:
  - a `header` record with the mode, `frozen_sha`, `build_id` (compiled in from
    `TURBO_DOM_DUAL_BUILD_ID`), `pid` and `artifact` (argv0);
  - `count` records at 1, 2, 4, … compared calls;
  - one `divergence` record per disagreement. It holds the API, the selector, both answers
    and the shape of each element the answers disagree on: tag, namespace, attributes,
    child node kinds and live form state. It also holds element-side `facts` over every
    node the selector can examine from that element.
- Forwarding: `turbo-dom-browser-env/selector-dual`. turbo-test builds with
  `--features turbo-dom-browser-env/selector-dual` until its PR adds a forwarding feature.
  turbo-surf crates must forward `selector-dual` to browser-env and turbo-dom. At S3b,
  turbo-dom-layout, turbo-dom-raster and html2pdf join. Never pass `--all-features` to a
  dual-capable crate.

## Running the gate

```bash
export RUSTY_V8_ARCHIVE=/path/to/v150.4.0-release-aarch64-darwin.a
export TURBO_TEST_DIR=../turbo-test-consolidation      # read only; the driver copies it
# optional: TURBO_SURF_DIR=../turbo-surf-... RUSTY_V8_ARCHIVE_SURF=<150.4 simdutf archive>
scripts/selector-dual-gate.sh S3a legacy     # Gate A run
scripts/selector-dual-gate.sh S3a new        # Gate B run
```

The script's header documents its suites and knobs. It runs these steps:

1. Build every in-scope artifact with the feature, one `TURBO_DOM_DUAL_BUILD_ID` per repo.
   `OUT/manifest.json` records the build ids and the sha256 of every artifact.
2. Install the artifacts where the suites load them from. Each is copied to a temp name
   and moved into place, and originals are restored on exit:
   - turbo-test release binary: `bin/turbo-test-darwin-arm64` and `target/release/turbo-test`
     of a scratch copy of the turbo-test checkout, with its turbo-dom patches pointed at
     this checkout;
   - surf napi `.node`: `TURBO_SURF_NAPI_PATH`;
   - surf `detection_trace`: `TURBO_SURF_DETECTION_BINARY`.
3. Run each suite directly with its own log dir:
   - cargo tests;
   - turbo-test `node --test 'test/*.mjs'` and `fixtures/compat` (JSON reporter);
   - surf `test:shim`, `test:e2e` and `test:detection`.
4. Run `scripts/selector-dual-check.py`. It checks that every header's build id is in the
   manifest, applies the per-suite vacuity guard (at least one header and one compared
   call), and classifies every divergence. One suite is waived from the vacuity guard and
   is compared for Gate A only: `turbo-test-cargo`. Its 29 tests cover the runner, the
   interceptor and V8 teardown, and none of them has called a selector since the binding
   moved to turbo-dom-browser-env.

Gate A (legacy mode) compares every suite's results with the previous step's reference
(the S2 tip for S3a). Results must be equal, except the tests added or changed in the step,
which form the expected-to-fail-in-legacy set:

- `rtdom::query::tests::pseudo_optional_readonly_readwrite_selected` (changed@S3a);
- `cascade_selectors_chrome::selectors_match_chrome156` (new).

Gate B (new mode) requires every divergence record to match one classifier in
`selector-dual-check.py`. Each classifier tests both the selector feature and an
element-side condition on every element the answers disagree on:

| bug-list entry | selector feature | element-side condition |
|---|---|---|
| escapes | contains `\` | the decoded escaped identifier is one of the element's reachable tokens (tag, attribute names and values, class words). For validity, the legacy error is at `\`. |
| non-ASCII | contains non-ASCII text | the non-ASCII identifier is a reachable token, or the legacy validity error is at that character |
| namespace | a `*\|`/`\|` type prefix outside `[...]` | the named type is a reachable tag, or the legacy validity error is at `\|` |
| `:empty` | contains `:empty` | a reachable element with no element or non-empty text child but at least one comment, PI, empty text or template content child |
| `:read-only`/`:read-write` | contains either | a reachable element on which the two engines' bare `:read-write`/`:read-only` verdicts differ |
| `:nth-child(of S)` | `:nth-[last-]child(… of …)` | the element has a parent |
| `:link`/`:any-link` | contains either | a reachable HTML `a`/`area` or SVG `a` with `href` |

Any divergence that matches no classifier fails the gate.
`python3 -m unittest discover -s scripts -p 'test_selector_dual_check.py'` tests the
classifiers, including their negative cases.

## Removal

The last S3b commit deletes the following; every publish workflow then guards that
`selector-dual` and `query_frozen` no longer exist:

- the `selector-dual` features;
- `query_frozen*.rs` and `selector_dual.rs`;
- `verify-query-frozen.*`, `selector-dual-*` and the `CssCache.dual` slot;
- the old html2pdf matcher.

# turbo-dom's selector engine: Chrome 156 fixes (S3a, P5b), the cascade on the engine (S3b), and the retired `selector-dual` gate

turbo-dom's `rtdom::query` is the ONE selector engine of the consolidated stack:

- `query.rs` (string API) and `query_utf16.rs` (UTF-16 `*_units` API, which the browser
  binding uses for selectors containing `\` or invalid UTF-16; the string API hands it any
  selector containing `\` too) serve turbo-dom's `matches` / `querySelector*`, and through
  turbo-dom-browser-env turbo-test and turbo-surf live;
- `query_cascade.rs` serves stylesheets: `parse_for_cascade` / `match_for_cascade`, used by
  turbo-dom-layout's cascade — html2pdf's PDFs and turbo-dom-raster's static screenshots
  (turbo-surf's static `screenshot` / `screenshotSvg`) — since S3b.

Consolidation step S3a made the engine match Chrome 156 for the bug list in
[miaskiewicz/turbo-dom#18](https://github.com/miaskiewicz/turbo-dom/issues/18); S3b moved
the layout cascade off html2pdf's own selector parser and matcher onto it. Both steps
were allowed to change output only where a selector engine disagreed with Chrome, and
two checks proved it:

1. **Chrome control**: `scripts/cascade-selectors-chrome-control.mjs` records Chrome for
   Testing 156's answers into `crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json`.
   `crates/turbo-dom/tests/cascade_selectors_chrome.rs` asserts them through the DOM APIs
   and the stylesheet API; `crates/turbo-dom-layout/tests/cascade_selectors_chrome.rs`
   asserts the computed winners end to end through the layout HTML path.
2. **Dual-engine gate** (retired at the end of S3b, see the last section): the non-default
   `selector-dual` feature ran a frozen copy of the previous engine next to the current
   one at every engine output. Gate A checked that legacy mode reproduced the previous
   step exactly. Gate B checked that every new-mode divergence was explained by a
   Chrome-controlled class.

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

S3a left these gaps from #18 open: `:lang()`/`:dir()`, `:defined`, qsa/getElementById
descending into `<template>` content, `:root` on fragments and detached nodes, malformed
selectors such as `p:nth-child(foo)` or `[a]div` being accepted by `_checked`, and the port
to npm `selectors.mjs`. P5b closed them (see "What P5b changed" below); the
binding-admission error prefixes remain open.

## What S3b changed: the layout cascade on the engine

turbo-dom-layout's cascade — html2pdf's PDF path and turbo-dom-raster's static
screenshots (`layout_html`), i.e. turbo-surf's static `screenshot` / `screenshotSvg` —
used html2pdf's own selector parser and matcher (`style/selector.rs` and the matching half
of `style/cascade.rs`). Since S3b it uses this engine:

- `rtdom::query::parse_for_cascade(&str) -> Result<SelectorList, SelectorError>`
  (`query_cascade.rs`) parses a rule's selector list with the DOM APIs' grammar plus
  stylesheet validity, pseudo-elements and Selectors-4 `specificity()` (see the module
  docs). `match_for_cascade(nav, element, selector)` matches one selector through any
  `Navigator`.
- Matching is generic over `rtdom::query::Navigator` (scoped parent, element siblings,
  positions, type name, `:root`, `:empty`, `:has()` descendants, slot assignment). The
  live-Tree `LiveNavigator` keeps the raw-node semantics exactly, so `matches` /
  `querySelector*` did not change (the live-Tree dual differential against the frozen S3a
  engine: zero divergences). turbo-dom-layout's `style::Navigator` (a `TreeView` scope)
  implements it: kept nodes only, the `Roots::Detached` cutoff, the `Roots::Document`
  scope (real `<html>`/`<head>`/`<body>`; the S3b-era `Roots::Shell` match-only shells
  were replaced in P5b, turbo-html2pdf#29),
  `:root` = an element named `html`, marker type keys; `PositionCache` builds each sibling
  list's positions once.
- Pseudo-classes the DOM engine does not evaluate are valid and never match in a
  stylesheet when Chrome knows them (`:hover`, `:focus`, `:visited`, `:default`,
  `:placeholder-shown`, `:valid`, …; `:dir()` until P5b). `:scope` (= `:root`), `:defined`,
  `:lang()` and `:-webkit-any()` were evaluated in stylesheets only at S3b (P5b moved
  `:defined` and `:lang()` into the DOM APIs too). `::slotted()` matches only through a navigator that reports slot assignment
  (a scaffold: turbo-surf's composed cascade still walks `:host()`/`::slotted()` with its
  own matcher, plan §1.4 follow-up).

Output changes only where the old html2pdf engine disagreed with Chrome. Each quirk class
has a Chrome-156 control in `cascade_selectors_chrome156.json` (the S3a `cases` and the
S3b `cascade_cases`, whose sheets make rules compete so validity and specificity show in
the winning `order`), asserted by `crates/turbo-dom/tests/cascade_selectors_chrome.rs`
(live navigator: validity, matches, a minimal cascade with specificity) and end to end
through the HTML path by `crates/turbo-dom-layout/tests/cascade_selectors_chrome.rs`
(`style_html_tree`, the styling step of `layout_html`):

| quirk class | old html2pdf engine | now (Chrome 156) | control |
|---|---|---|---|
| escapes | `\` was a separator (`.md\:flex` = `.md` + a dropped pseudo, `\70` = `*`) | decoded | `cases/escapes` |
| namespace | `*\|p`, `\|p` misread; `svg\|rect` accepted | `*\|` any, `\|` none; undeclared prefix drops the rule | `cases/namespace` |
| attribute-namespace | `[href]` matched a namespaced `xlink:href` (local-name lookup) | no-namespace attributes only | `cascade_cases/attribute-namespace` |
| `:read-only`/`:read-write`, `:required`/`:optional`, `:nth-last-*`, `:has()` | unknown: dropped, so the compound matched as if it were absent | evaluated | `cases/read-only-read-write`, `cases/guards`, `cases/nth-of-s`, `cascade_cases/unsupported-pseudo-class` |
| `of S` | `An+B of S` parsed as a garbled An+B | supported | `cases/nth-of-s` |
| unknown-pseudo | unknown pseudo-classes (and other-case names: `:FIRST-CHILD`) dropped; `:matches()`/`:-moz-any()` = `:is()`; `:selected`, `:lang()`, `:defined`, `:scope` dropped | an unknown or rejected name drops the rule; names are case-insensitive; `:lang()`, `:defined`, `:scope` evaluated | `cascade_cases/unknown-pseudo-class`, `cascade_cases/cascade-only-pseudos` |
| pseudo-element | unknown pseudo-elements (`::-webkit-scrollbar`) dropped, so they styled the element | never match an element | `cascade_cases/specificity-pseudo-element` |
| specificity | every pseudo-class one class-weight, `:where()`/`:is()`/`:not()` included; pseudo-elements as classes | `:where()` 0; `:is()`/`:not()`/`:has()` their most specific argument; `of S` adds S; pseudo-elements are types | `cascade_cases/specificity-where`, `cascade_cases/specificity-is-not-has` |

No rendered output of the parity corpora changed (`tools/parity/expected/s3b-*.json`: one
file per class, each listing no change): html2pdf, html2pdf-svg and html2pdf-extra are
byte-identical to `b-main`, and turbo-dom-raster's static screenshots of the surf static
inputs are byte-identical to S2. The scoping contexts with no Chrome equivalent —
`Roots::Detached` (template body, notes, regions), the `Roots::Shell` html/body shells
(since replaced by the browser-like `Roots::Document`, P5b turbo-html2pdf#29),
markers, and `:root` there — keep the old engine's answers by construction (the navigator
reproduces the old match contexts), and the dual gate saw no divergence there.

Known cascade gaps after S3b (each matched what the old engine did): `:dir()` and the other
valid-but-unevaluated pseudo-classes never matched; simple selectors after a pseudo-element
(`::before.x`) and the malformed forms the DOM engine accepted (`[a]div`,
`p:nth-child(foo)`) did not invalidate a rule; `#a#b` keeps only the last id. The
pre-existing cascade ordering bug (candidates ordered by `rule order + declaration index`,
miaskiewicz/turbo-dom#21) was kept. P5b fixed all but `#a#b`.

## S4: `selector()` support conditions on the engine

`rtdom::query::parse_for_supports(&str) -> Result<CascadeSelector, SelectorError>` is the
`selector(…)` condition of `@supports` and `CSS.supports`: one complex selector (a list is
unsupported), valid like a stylesheet item (pseudo-elements and Chrome's valid
never-matching pseudo-classes included), parsed **unforgivingly** — an invalid item of
`:is()` / `:where()` at any depth makes it unsupported instead of being dropped.
turbo-surf's native `@supports selector(…)` and `CSS.supports("selector(…)")`
(`live_layout_supports.rs`) use it; before S4 they parsed with lightningcss and admitted
only a fixed component allow-list (so `selector(*|p)`, `:hover`, `::before`, `|p`,
`[*|title]`, `:link`, `:empty`, `:host`, `::slotted()` and `:nth-child(… of S)` were
unsupported, unlike Chrome). Control: turbo-surf
`harness/detection/fixtures/css-supports-chrome156.json` (`css-supports-probe.js`, 22
new `selector()` cases incl. escapes), asserted by turbo-surf's
`tests/shadow_layout.rs` and by `rtdom::query::cascade::tests::supports_selectors_follow_chrome`.

## What P5b changed (turbo-dom#18 remainder, turbo-dom#21)

Each item has its own commit, Chrome-156 control rows in `cascade_selectors_chrome156.json`
(new `cases`, `cascade_cases` and the scripted `probes`) and a parity expected-diff file
`tools/parity/expected/p5b-*.json`. No parity-corpus output changed: html2pdf,
html2pdf-svg and html2pdf-extra are byte-identical to `b-main`, and turbo-dom-raster's
static screenshots of the surf static inputs to S5.

| item | before | now (Chrome 156) | control |
|---|---|---|---|
| cascade order (#21, turbo-dom-layout) | candidates ranked by (importance, level, specificity, rule order + declaration index): `p{margin:0;color:red} p{color:green}` was red | ranked by importance, origin (an `!important` UA declaration beats `!important` author ones), specificity, the rule's source order, then the declaration index | `cascade_cases/declaration-order`, `importance`, `inline-importance` |
| `:disabled` / `:enabled` | the element's own `disabled`, on any element | HTML "actually disabled": disabled `<fieldset>` ancestors outside the first `<legend>`, `option` via `optgroup` / owner `select`; only form controls match either | `cases/disabled-fieldset` |
| malformed selectors | `_checked` validated a lexical subset (`[a]div`, `p:nth-child(foo)`, `::before.x` accepted; `""`, `a,,b`, `p >` tolerated) | `_checked` = the `parse_for_cascade` grammar = Chrome's `querySelector` on every control row: type selector first, identifier names, nothing but allowed pseudo-classes after a pseudo-element (per pseudo-element, as Chrome), valid `An+B`, pseudo arity, attribute grammar (unquoted values are identifiers, undeclared prefixes invalid; `[*\|a]` any namespace), no nested `:has()` (a forgiving `:is()` drops such items). The `s` attribute modifier was a Rust extension until P5b-X2 (see below). | `cases/malformed` |
| pseudo-class case | DOM APIs case-sensitive (`P:FIRST-CHILD` never matched) | ASCII case-insensitive | `cases/malformed`, `cases/lang` |
| `:lang()` | stylesheet-only; strings and lists accepted | DOM APIs too; one identifier; `lang` on any element (SVG) | `cases/lang` |
| `:defined` | stylesheet-only | DOM APIs too; `is=` elements are undefined | `cases/defined` |
| `:dir()` | valid, never matched | HTML directionality (`dir`, `dir=auto` / `<bdi>` first strong character via `unicode-bidi`, `type=tel`, inheritance across shadow roots) | `cases/dir` |
| template content | queries, `:has()` and ancestor walks entered the content fragment | content is a separate fragment (`Tree::for_each_light_child`, `LiveNavigator::parent`) | `cases/template`, `probes/template-traversal` |
| `:root` | any element without an element parent | the document element only (the binding's internal selector paths anchor with `:not(* > *)`) | `probes/root` |
| `rtdom::cascade` lists | split on every `,`; invalid rules applied their valid half | `parse_for_cascade` items; an invalid list drops the rule; stylesheet matching (`:scope` = `:root`) | every row's sentinel `winner`, `cases/cascade-comma` |
| npm `selectors.mjs` | pre-S3a engine, lenient | the same Chrome grammar and matching (`test/selectors-chrome.test.mjs` asserts the shared fixture: validity, `matches`, `querySelectorAll`, cascade sentinel, probes); `cascade.mjs` splits lists like `rtdom::cascade`; the `s` modifier throws | all `cases` + `probes` |

Known limits (both fixed by P5b-X2 below): through the V8 binding a custom element upgraded
by `customElements.define()` still matched `:not(:defined)`; `rtdom::cascade` kept its rough
per-item specificity count.

## What P5b-X2 changed (turbo-dom#24)

Follow-ups of P5b found while fixing turbo-dom#18 / #21. Each item has its own commit, a
Chrome 156 control and a parity expected-diff file `tools/parity/expected/p5b-x2-24-*.json`.

| item | before | now (Chrome 156) | control |
|---|---|---|---|
| `rtdom::cascade` specificity (#24 item 2) | a per-token count of each selector item's text (`a*10000 + b*100 + c` over `#…`, `.…` / `[…]` / `:…`, type names after a combinator): `:where(#a) p` outranked `p`, `:not(.x)` and `.md\:x` counted two classes, `[title="#t"]` an id, `div :is(i)` a class | the `parse_for_cascade` item's Selectors 4 `specificity()` — the layout cascade's: `:where()` 0, `:is()` / `:not()` / `:has()` their most specific argument, `:nth-child(… of S)` a pseudo-class plus S, pseudo-elements as types; npm `cascade.mjs` the same (`selectors.mjs` `specificity()` / `compareSpecificity()`) | `cascade_cases/specificity-*` (new `specificity-arguments`), `cascade-only-pseudos` — now also asserted through `rtdom::cascade::computed_style` and the npm `getComputedStyle` |
| `:defined` after `define()` (#24 item 3) | the tree could not see the binding's registry: an upgraded custom element still matched `:not(:defined)`; the `is` rule read the current attribute | `Tree::custom_element_state` (`rtdom/custom_element_state.rs`): the binding reports upgrade / construction / failure (and carries the state through clones and adoption); otherwise the creation-time state (custom element names and the parser's `is` attribute are undefined; later `is` edits never change it; `createElement(…, {is})` is undefined) | `turbo-dom-browser-env/src/custom_elements_defined_chrome156.json` (binding test `defined_follows_upgrades_matching_chrome156`), `tests/custom_element_state.rs` |
| `s` attribute modifier (#24 item 4) | a documented Rust extension of the lenient / `_checked` / stylesheet parsers (`[a=b s]` compared case-sensitively); the Chrome rows were skipped (`RUST_CONTRACT_ROWS`) | a SyntaxError in the checked APIs, never a match in the lenient ones, an invalid stylesheet rule — like Chrome and the npm engine | `cases/malformed` (`[a=b s]`, `[a=b S]`, `[a="1" s]`), `tests/fixtures/attribute-case` (7 `s` cases) |

## What P5b-X3 changed (turbo-dom#26 item 2)

| item | before | now (Chrome 156) | control |
|---|---|---|---|
| npm `:defined` | `selectors.mjs` looked the name up in `window.customElements` and read the current `is` attribute: a later `is` edit changed `:defined`, a defined name matched before any upgrade (detached elements too), and the stub registry never upgraded or constructed anything | `src/runtime/custom-elements.mjs` keeps each element's custom element state with the Rust tree's creation-time rule (custom names and the parser's / `createElement(…, {is})` / fragment parser's `is` value are undefined; later edits never change it); the registry upgrades connected candidates on `define()`, on insertion, `customElements.upgrade()`, clones and `innerHTML`, constructs on `createElement` / `new C()`, leaves a throwing constructor's element failed, and runs the lifecycle callbacks; `customElementState` / `setCustomElementState` are exported | `turbo-dom-browser-env/src/custom_elements_defined_chrome156.json` (`test/custom-elements-chrome.test.mjs`; every section but the failed element's interface name), `test/custom-elements.test.mjs` |

## Regenerating the Chrome control

```bash
PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core \
node scripts/cascade-selectors-chrome-control.mjs [--chrome "/path/to/Google Chrome for Testing"]
cargo test -p turbo-dom --test cascade_selectors_chrome
cargo test -p turbo-dom-layout --test cascade_selectors_chrome
```

For each S3a case (`cases`) and selector, the fixture records:

- validity, from `document.querySelector`;
- `el.matches` for every element (document order, template content excluded);
- `querySelectorAll` as element indices;
- the `getComputedStyle(el).order` winner when the case document gets
  `<style>SEL { order: 1 }</style>` as the last `<head>` child.

For each S3b cascade case (`cascade_cases`, one per quirk class of the old html2pdf
engine, plus P5b's declaration-order cases) it records the case's sheet and
`getComputedStyle(el).order` for every element when that sheet is the last `<head>` child:
several rules compete, so rule validity, specificity and source order show in the winner.
P5b adds `sheet_valid` per selector row (stylesheet validity; it agrees with
`querySelector` on every row) and `probes`: scripted DOM checks (template content
traversal, `:root` outside the document element) whose named values the Rust test rebuilds
through the Tree API and `test/selectors-chrome.test.mjs` through the JS DOM.

`cargo test -p turbo-dom --test cascade_selectors_chrome` lists the items whose fix has
not landed in `OPEN` (empty since P5b) and asserts every row (P5b-X2 removed the
`RUST_CONTRACT_ROWS` skip of the `s` attribute modifier). `node --test
test/selectors-chrome.test.mjs` asserts the same fixture against the npm JS engine.

turbo-dom's test asserts the S3a rows through `matches`, `matches_checked`,
`matches_units`, `query_selector[_all]`, `query_selector[_all]_units`,
`rtdom::cascade::computed_style`, `parse_for_cascade` and `match_for_cascade` (live
navigator), and each cascade case through a minimal cascade over `parse_for_cascade`,
`match_for_cascade` and `specificity()` and (all but the `cascade-order` class: importance and
origins are the layout cascade's) through `rtdom::cascade::computed_style`;
`test/selectors-chrome.test.mjs` checks the same cascade cases through the npm
`getComputedStyle`. turbo-dom-layout's test styles every S3a selector
and every cascade case through `style_html_tree` (the styling step of `layout_html`) and
compares each styled element's `order` with Chrome.

## The `selector-dual` gate (retired at the end of S3b)

The gate existed only while S3a and S3b ran. Its code — the `selector-dual` cargo features
(turbo-dom, turbo-dom-browser-env, turbo-dom-layout, turbo-dom-raster and turbo-html2pdf's
crates), the frozen engine copies `rtdom/query_frozen.rs` + `query_frozen_utf16.rs`, the
dual layers `rtdom/selector_dual.rs` and turbo-dom-layout's `style/selector_dual.rs`, the
`CssCache.dual` slot, html2pdf's old `style/selector.rs` parser and matcher, and the
scripts `selector-dual-gate.sh`, `selector-dual-check.py` (+ its tests) and
`verify-query-frozen.sh` (+ rules) — was deleted in the last S3b commit.
`scripts/check-no-selector-dual.sh [DIR]` fails if any of it comes back (a `selector-dual`
feature, a frozen copy or dual layer, or a code/build/script/workflow file naming them);
publish workflows run it.

How it worked: a dual build compiled the previous engine next to the current one and
answered every engine output with both. `TURBO_DOM_SELECTOR_ENGINE=legacy|new` picked the
answer callers got; each process appended a header (mode, frozen-engine SHA, a build id
compiled in from `TURBO_DOM_DUAL_BUILD_ID`), periodic compared-call counts and one record
per disagreement (API, selector, both answers, element shape) to
`$TURBO_DOM_SELECTOR_DUAL_LOG_DIR/<pid>-<nonce>.jsonl`. A driver built every in-scope
artifact with the feature (recording each artifact's sha256 against its build id),
installed the artifacts where the suites load them, ran the suites in one mode and checked
the logs: every header's build id in the manifest, a per-suite vacuity guard (at least one
header and one compared call), and in new mode every divergence matched by a classifier
that tested both the selector feature and the element-side condition.

- **S3a** (live Tree): the frozen engine was B-main's (v0.5.8) `query.rs` +
  `query_utf16.rs`, regenerated by a scripted rewrite. Gate A (legacy): every turbo-dom,
  browser-env and turbo-test result equal to S0 except three listed tests; turbo-test
  cargo 29 / npm 63 / compat 109-8 identical. Gate B (new): 1,180 divergences over the
  turbo-dom suites, all in the seven bug-list classes; turbo-test npm + compat 0.
- **S3b** (cascade): the live-Tree frozen engine was re-frozen at the S3a tip, so the live
  Tree had to agree with it exactly, and turbo-dom-layout's cascade ran the old html2pdf
  parser + matcher (over match contexts rebuilt from the scope navigator exactly as S2
  built them) as the legacy engine, comparing rule validity, the selector list, every
  item's specificity and every (selector, element) match. Suites: turbo-dom +
  browser-env; turbo-dom-layout (+ raster) in the default, svg, no-default and
  every-feature configurations; turbo-html2pdf workspace and core every-feature /
  no-default / svg; the html2pdf parity corpora (default + svg napi addons); turbo-dom-raster
  static screenshots of the surf static inputs.
  - Gate A (legacy): every result equal to S2 except the expected-to-fail-in-legacy set
    (`turbo-dom-layout cascade_selectors_chrome::html_path_cascade_matches_chrome156`,
    new; `turbo-html2pdf style::unknown_pseudo_class_is_ignored`, changed@S3b); corpora
    byte-identical to `b-main`, static screenshots to S2.
  - Gate B (new): 0 live-Tree divergences (turbo-dom suite, >= 39,946 compared calls);
    every cascade divergence classified — per layout configuration 325 (`:read-only` /
    `:read-write` 136, `of S` 33, namespace 31, unknown-pseudo 27, `:nth-last-*` 26,
    escapes 25, `:required`/`:optional` 22, pseudo-element 11, specificity 7,
    attribute-namespace 4, `:has()` 3, nearly all from the Chrome-control documents),
    per html2pdf configuration 14 (pseudo-element 6, unknown-pseudo 5, specificity 3),
    html2pdf corpora 6 and static screenshots 3 (pseudo-element specificity only, inert);
    results equal to the plain build; corpora and screenshots unchanged.

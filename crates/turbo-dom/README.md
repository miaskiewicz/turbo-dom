# turbo-dom

A **pure-Rust DOM**: a lazy, copy-on-write tree built over an
[html5ever](https://github.com/servo/html5ever) Structure-of-Arrays parse buffer, with
turbo-dom's one Chrome 156-checked selector engine, a CSSOM, a partial `getComputedStyle`,
events, shadow DOM, mutation records and UTF-16-exact parsing. It is a plain Rust API with no
JS boundary. On a chatty per-node workload it measured ~2.7x turbo-dom's JS runtime, while a JS
caller going through WASM measured ~0.55x, so this crate serves Rust consumers only (numbers
from before 0.6.0; see [BENCHMARKS-history](../../docs/BENCHMARKS-history.md)).

## Where it sits

The base of the turbo-dom 0.6.0 crate family
([docs/CONSOLIDATION.md](../../docs/CONSOLIDATION.md)):

```
turbo-dom ──► turbo-dom-layout ──► turbo-dom-raster
    └───────► turbo-dom-browser-env (+ turbo-dom-v8)
```

Consumers: **turbo-test** and **turbo-surf** (through turbo-dom-browser-env and directly),
**turbo-html2pdf** (through turbo-dom-layout's cascade). The npm package
`@miaskiewicz/turbo-dom` is the separate JS-consumer path; it does not use this crate.

## Install

```bash
cargo add turbo-dom        # turbo-dom = "0.6"; MSRV 1.89
```

Dependencies: html5ever / markup5ever_rcdom 0.39, cssparser 0.37, rustc-hash, smol_str,
smallvec, unicode-bidi, log. No napi, no wasm-bindgen.

## Features

None.

## API

Crate root: `Dom`, `Tree`, `Handle`, `Event`, `ParseOptions`, `NodeRef`, `DocumentExt`; the
parser core is `turbo_dom::core` (`parse_html_document`, `parse_html_soa`,
`parse_html_soa_with_compat_mode`, `Soa`, `Node`).

```rust
use turbo_dom::{Dom, DocumentExt, Event};
use turbo_dom::rtdom::{cascade, serialize};

let mut dom = Dom::parse("<main class=grid><div class=card id=hero>hi</div></main>");

// query (version-cached), read attrs/text — zero boundary
let cards = dom.tree.query_selector_all("div.card");
let id = dom.tree.get_attribute(cards[0], "id");          // Some("hero")
let text = dom.tree.text_content(cards[0]);               // "hi"

// partial, honest getComputedStyle (specificity, inheritance, color canon)
let style = cascade::computed_style(&dom.tree, cards[0]);
let color = cascade::get_property_value(&style, "color");

// ergonomic NodeRef facade (descendant-scoped)
let title = dom.tree.query("div.card");

// event dispatch; a handler may mutate the tree mid-dispatch
dom.add_event_listener(cards[0], "click", false, false, Box::new(|tree, _ev| {
    let span = tree.create_element("span");
    tree.append_child(tree.root(), span);
}));
dom.dispatch_event(cards[0], &mut Event::new("click", true, true));

// serialize back to HTML
let html = serialize::serialize_inner(&dom.tree, dom.tree.root());
```

Worked example: `cargo run --release -p turbo-dom --example crawl`.

Modules (`turbo_dom::rtdom::*`):

| module | role |
|---|---|
| `tree` | COW tree over the immutable SoA + `version` counter; lazy attrs; mutations; `set_inner_html`; shadow DOM; mutation recording; custom element state (`CustomElementState`, read by `:defined`; embedders report upgrades) |
| `query` | selectors (tag incl. `*\|`/`\|` prefixes/`.class`/`#id`/`[attr]` with `^=`/`$=`/`*=`/`~=`/`\|=`, CSS escapes, all four combinators, comma, structural/form/link pseudo-classes incl. `:nth-child(An+B of S)`, `:is`/`:where`/`:not`/`:has`), checked against a Chrome 156 control; version-cached `querySelectorAll` / `getElementById` |
| `cascade` | partial `getComputedStyle` — inline + matched `<style>` rules with Selectors 4 specificity / source order, curated inheritance, `:host`/`::slotted` shadow scoping, color canonicalization; memoized per version |
| `events` | `Dom` (owns the tree + listeners); capture/target/bubble dispatch, listener-less fast-skip, `stopPropagation`/`preventDefault`, tree-mutating handlers |
| `serialize` | inner/outerHTML (void + raw-text, single-pass escaping) |
| `node_ref` | ergonomic `NodeRef` + `DocumentExt` façade over handles |
| `color` | CSS color → `rgb()`/`rgba()` (named/hex/hsl) |
| `css_shorthand` | CSSOM shorthands as Chrome 156 implements them: the shorthand / alias tables, `expand` (a shorthand value → its longhands), `shorthand_value` / `block_text` (getPropertyValue / cssText serialization), `longhand_value` (a longhand's Chrome spelling and admission), `DeclarationBlock` (the reference CSSOM block); checked against `tests/fixtures/css_{shorthand,longhand}_chrome156.json` |
| `css_values` | Chrome-compatible CSS value grammars: `zoom_value` (Chrome 156 standardized `zoom`, incl. `calc()`), `css_number` (Blink number serialization) |
| `cssom` | `CssStyleSheet`/`CssStyleRule` + insert/deleteRule + parse |
| `svg` | SVG length/string/viewBox accessors |
| `file` | Blob/File + sync FileReader (text / data-URL base64 / array-buffer) |
| `custom_elements` | name→definition registry |
| `location` | Location (URL parse) + History stack |
| `stubs` | Storage, media-query eval, observer entries |
| `mutations` | `MutationObserver` — gated record buffer, sync `take_records`, type/target/subtree/old-value filtering; additive `MutationEvent` includes mutation-time ancestry/siblings/namespace for embedder queues |

## Status

- Line coverage is gated in CI at **100%** (tarpaulin LLVM over `src/`, blocking),
  with behaviour-asserting tests: Chrome 156 controls for what a page can observe
  (`tests/rtdom_core_chrome.rs`, `tests/cascade_selectors_chrome.rs`, …) and the html5lib
  tree-construction fixtures through both parsers (upstream html5ever and the UTF-16
  tokenizer fork).
- The parser core (`turbo_dom::core`) shares the parser crate's html5ever options and
  passes html5lib-tests tree-construction conformance at **99.94%** there (the rtdom-direct
  gate: 100%).
- CI: Pull-request CI runs this crate's jobs on Linux; macOS arm64 is added in the release run only (`publish.yml` calls `ci.yml` with `darwin: true`). Complexity (<= 15 per function) is report-only until the post-release refactor.
- Honest by design: `getComputedStyle` only returns values a real rule/inline/
  inherited declaration set (no invented initial values, no layout/`@media`/state).

Embedder entry points beyond the browser-facing API:

- **Parsing:** `Tree::parse_with(html, ParseOptions)` (scripting flag);
  `Tree::parse_utf16` and `parse_srcdoc_utf16` keep the original UTF-16 units. `srcdoc` is
  no-quirks, as in Chrome. `parse_with_source_positions` and
  `parse_utf16_with_source_positions` record token positions, read with
  `source_position` / `source_attribute_position`.
- **Fragments:** `Tree::create_contextual_fragment_utf16(context, units, scripting_enabled)`
  parses against a real context element (Range `createContextualFragment`).
- **Selectors for other engines:** `rtdom::query::{parse_for_cascade, match_for_cascade,
  parse_for_supports}`, generic over the `rtdom::query::Navigator` trait. turbo-dom-layout's
  cascade uses these, so the DOM APIs and layout share one engine
  ([docs/selector-engine.md](../../docs/selector-engine.md)).
- **Mutation events:** `Tree::start_recording()`, then `take_mutation_events()` (records plus
  ancestry and sibling positions for an embedder's observer queues) or
  `take_mutation_records()`.
- **Storage:** `Tree::collect_unreachable(&pins, budget)` reclaims detached storage at a
  quiescent point. Handles are never reused: check `is_live()`, and iterate `handles()`
  rather than `0..node_count()`. `storage_stats()` reports usage
  ([docs/native-storage-collection.md](../../docs/native-storage-collection.md)).

## Gates

- Line coverage **100%**, blocking (CI `coverage` job: `cargo tarpaulin --engine llvm -p
  turbo-dom --include-files 'crates/turbo-dom/src/*' --fail-under 100`). Dead defensive lines
  are removed or `unreachable!()` with a proof, never excluded.
- html5lib-tests tree construction through the rtdom tree: **100%**
  (`npm run conformance:rtdom`). The perf gate `churn_alloc_gate` (≤ 380 allocs/op) runs alone:
  `cargo test -p turbo-dom churn_alloc_gate -- --ignored`.
- clippy pedantic `-D warnings`, rustfmt, MSRV 1.89 check.
- Cyclomatic complexity ≤ 15 per function is **report-only** until the post-release refactor.
- CI runs on Linux for pull requests; macOS arm64 is added only in the release run.

## Testing

```bash
cargo test -p turbo-dom
cargo test -p turbo-dom --test cascade_selectors_chrome   # selector engine vs Chrome 156
cargo test -p turbo-dom --test rtdom_core_chrome          # tokenizer / fragments / DOM edges vs Chrome 156
```

Web-observable behaviour is asserted against **Chrome 156 controls**: JSON fixtures recorded
by the collectors in the repository's `scripts/`, stored in `tests/fixtures/` and
`tests/controls/`. Examples: `cascade_selectors_chrome156.json`,
`rtdom_core_chrome156.json` and `css_{shorthand,longhand}_chrome156.json`. A behaviour
change needs a new control row. The npm runtime asserts the same selector and shorthand
fixtures, so the two engines stay in lockstep.

## Links

[turbo-dom repository](https://github.com/miaskiewicz/turbo-dom) ·
[CHANGELOG](../../CHANGELOG.md) · [UTF16.md](UTF16.md) ·
[turbo-dom-layout](../turbo-dom-layout/README.md) ·
[turbo-dom-browser-env](../turbo-dom-browser-env/README.md)

License: MIT.

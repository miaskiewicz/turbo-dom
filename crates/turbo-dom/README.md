# turbo-dom

A **pure-Rust DOM runtime** for in-process Rust consumers (crawlers, extractors,
SSR, scraping). A lazy, copy-on-write DOM tree built directly over an
[`html5ever`](https://github.com/servo/html5ever) Structure-of-Arrays parse
buffer, with a **native Rust API and zero JS boundary**.

This is the Rust-native runtime of the [turbo-dom](https://github.com/miaskiewicz/turbo-dom)
project (the npm package `@miaskiewicz/turbo-dom` is the JS-consumer path). No napi,
no wasm-bindgen, no JavaScript — those exist only on the JS-consumer path (where an
in-process JS runtime beats a WASM/napi boundary anyway). This crate is the path for
a **Rust** consumer, where the DOM lives in-process and every read / mutation is a
plain Rust call.

## Why

On the chatty per-node workload (querySelectorAll + getAttribute + tagName +
parent-walk, 300-card fixture), the native Rust runtime measured **~2.7× the
optimized JS runtime** — because there's no boundary to cross. (A JS caller going
through WASM/napi measured ~0.55×, i.e. *slower* than the JS runtime — which is
exactly why this is Rust-only.)

## Install

```bash
cargo add turbo-dom        # turbo-dom = "0.6"; MSRV 1.89
```

turbo-dom is the base of the turbo-dom crate family (see the repository's
`docs/CONSOLIDATION.md`): `turbo-dom-layout` lays a `Tree` out, `turbo-dom-raster` paints
it, `turbo-dom-browser-env` binds it to V8 (`turbo-dom-v8`).

## Use

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

Run the worked example: `cargo run --release --example crawl`.

## What's inside (`turbo_dom::rtdom::*`)

| module | role |
|---|---|
| `tree` | COW tree over the immutable SoA + `version` counter; lazy attrs; mutations; `set_inner_html`; shadow DOM; mutation recording; custom element state (`CustomElementState`, read by `:defined`; embedders report upgrades) |
| `query` | selectors (tag incl. `*\|`/`\|` prefixes/`.class`/`#id`/`[attr]` with `^=`/`$=`/`*=`/`~=`/`\|=`, CSS escapes, all four combinators, comma, structural/form/link pseudo-classes incl. `:nth-child(An+B of S)`, `:is`/`:where`/`:not`/`:has`), checked against a Chrome 156 control; version-cached `querySelectorAll` / `getElementById` |
| `cascade` | partial `getComputedStyle` — inline + matched `<style>` rules with Selectors 4 specificity / source order, curated inheritance, `:host`/`::slotted` shadow scoping, color canonicalization; memoized per version |
| `events` | `Dom` (owns the tree + listeners); capture/target/bubble dispatch, listener-less fast-skip, `stopPropagation`/`preventDefault`, tree-mutating handlers |
| `serialize` | inner/outerHTML (void + raw-text, single-pass escaping) |
| `node_ref` | ergonomic `NodeRef` + `DocumentExt` façade over handles |
| `color` | CSS color → `rgb()`/`rgba()` (named/hex/hsl) |
| `css_values` | Chrome-compatible CSS value grammars: `zoom_value` (Chrome 156 standardized `zoom`, incl. `calc()`), `css_number` (Blink number serialization) |
| `cssom` | `CssStyleSheet`/`CssStyleRule` + insert/deleteRule + parse |
| `svg` | SVG length/string/viewBox accessors |
| `file` | Blob/File + sync FileReader (text / data-URL base64 / array-buffer) |
| `custom_elements` | name→definition registry |
| `location` | Location (URL parse) + History stack |
| `stubs` | Storage, media-query eval, observer entries |
| `mutations` | `MutationObserver` — gated record buffer, sync `take_records`, type/target/subtree/old-value filtering; additive `MutationEvent` includes mutation-time ancestry/siblings/namespace for embedder queues |

## Status

- Line coverage is gated in CI at **100%** (tarpaulin LLVM over `src/`; 88.33% at 0.6.0),
  with behaviour-asserting tests: Chrome 156 controls for what a page can observe
  (`tests/rtdom_core_chrome.rs`, `tests/cascade_selectors_chrome.rs`, …) and the html5lib
  tree-construction fixtures through both parsers (upstream html5ever and the UTF-16
  tokenizer fork).
- The parser core (`turbo_dom::core`) shares the parser crate's html5ever options and
  passes html5lib-tests tree-construction conformance at **99.94%** there (the rtdom-direct
  gate: 100%).
- Honest by design: `getComputedStyle` only returns values a real rule/inline/
  inherited declaration set (no invented initial values, no layout/`@media`/state).

MIT.

### Native observer event seam (0.5.3)

Call `Tree::start_recording()` and drain `Tree::take_mutation_events()` to fan out
events into an embedder's private observer queues. Each event includes the existing
record plus inclusive light-tree ancestry at mutation time, child-list sibling
positions and the actual attribute namespace. Ancestry stops at shadow roots.
`take_mutation_records()` remains supported and consumes the same buffer. Recording
is opt-in; the ordinary mutation path does not allocate event metadata.

The embedder still implements observer registrations/options, transient removed-tree
registrations, private callback ownership and microtask delivery according to the
[DOM observer algorithms](https://dom.spec.whatwg.org/#mutation-observers). This
additive seam does not establish complete browser MutationObserver parity.

## Native storage collection

Long-lived embedders can call `Tree::collect_unreachable(&pins, budget)` at a
quiescent boundary. Pass every independently retained native handle, including
ranges, iterators, XPath results, queued work and live wrapper components. The
connected document and undrained native mutation events are implicit roots.
A pinned detached descendant retains its ancestors, siblings, shadow trees and
attached Attr nodes. Collection rejects invalid pins or exhausted node/edge/string
budgets before replacing storage. It preserves the DOM version and emits no
mutation record.

`Tree::storage_stats()` reports live nodes, parsed/runtime slots and logical
string payload bytes. Successful collection drops unreachable parsed payloads and
uses sparse native storage for retained identities. New handles are monotonically
minted and never reused. After collection, `node_count()` counts live nodes and
identities can have holes: iterate `handles()` instead of constructing
`0..node_count()` handles. Use `is_live()` before passing an externally retained
handle to a DOM read; omitted pins can be reclaimed. `NodeRef` borrowing protects
concurrent safe Rust mutation but cannot discover external copied handles.

## Parser provenance

`Tree::parse_with_source_positions` and `parse_utf16_with_source_positions` opt
into original-input positions without changing the normal parse path. The
original UTF-16 offset and one-based line/column come from the actual tokenizer
queue at token emission. CRLF counts as one line break and two original units.
Sparse source checkpoints bound position lookup work; metadata does not retain
the input after parsing.

`source_position(node)` identifies the token that created an actual lexical
node. Synthetic and reconstructed nodes return `None`.
`source_attribute_position(node, name)` and its UTF-16 name variant identify the
parser context where an attribute was installed, including later html/body tags
and formatting reconstruction. This differs from the node's lexical provenance.
Mutations do not fabricate new positions; native storage collection prunes
metadata for reclaimed handles.

The independent normal Chrome controls and their pinned probe/case/collector
sources live in `tests/controls/parser_positions`. Recollect with:

```sh
node crates/turbo-dom/tests/controls/parser_positions/csp-document-chrome.mjs \
  crates/turbo-dom/tests/controls/parser_positions/parser-source-position-cases.json \
  crates/turbo-dom/tests/controls/parser_positions/parser-source-position-probe.js \
  crates/turbo-dom/tests/controls/parser_positions/parser-source-position-chrome155.json
```

The backend test compares13 mainframe cases. Inert-template report metadata
requires the browser binding's actual owner-document context; its full14th
control is preserved rather than represented as a backend lexical-position rule.

## Contextual HTML fragments

`Tree::create_contextual_fragment_utf16(context, units, scripting_enabled)`
returns a detached fragment parsed against the actual context element. It uses
its namespace, integration-point attributes, nearest ordinary-tree form ancestor
and owner document quirks mode. Original UTF-16 units survive parsing. Existing
context children and pending mutation events are preserved; no custom-element
constructor or page script runs in the parser.

The scripting flag controls HTML `noscript` tokenization. Embedders still choose
the context element for a Range boundary, handle HTML-root/body fallback and
XML parsing, establish the template-associated owner document, and deliver
custom-element upgrades and script preparation at the correct lifecycle point.
Retain the returned fragment as a native collection pin until it is adopted or
its wrapper ownership is established.

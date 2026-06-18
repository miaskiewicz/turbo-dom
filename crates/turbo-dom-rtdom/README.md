# turbo-dom-rtdom

A **pure-Rust DOM runtime** for in-process Rust consumers (crawlers, extractors,
SSR, scraping). A lazy, copy-on-write DOM tree built directly over an
[`html5ever`](https://github.com/servo/html5ever) Structure-of-Arrays parse
buffer, with a **native Rust API and zero JS boundary**.

Extracted standalone from [turbo-dom](../turbo-dom)'s `rtdom` module. No napi, no
wasm-bindgen, no JavaScript — those exist only on turbo-dom's *JS-consumer* path
(where an in-process JS runtime beats a WASM/napi boundary anyway). This crate is
the path for a **Rust** consumer, where the DOM lives in-process and every read /
mutation is a plain Rust call.

## Why

On the chatty per-node workload (querySelectorAll + getAttribute + tagName +
parent-walk, 300-card fixture), the native Rust runtime measured **~2.7× the
optimized JS runtime** — because there's no boundary to cross. (A JS caller going
through WASM/napi measured ~0.55×, i.e. *slower* than the JS runtime — which is
exactly why this is Rust-only.)

## Install

```toml
[dependencies]
turbo-dom-rtdom = { path = "../turbo-dom-rtdom" }   # or git = "..."
```

## Use

```rust
use turbo_dom_rtdom::{Dom, DocumentExt, Event};
use turbo_dom_rtdom::rtdom::{cascade, serialize};

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

## What's inside (`turbo_dom_rtdom::rtdom::*`)

| module | role |
|---|---|
| `tree` | COW tree over the immutable SoA + `version` counter; lazy attrs; mutations; `set_inner_html`; shadow DOM; mutation recording |
| `query` | selectors (tag/`.class`/`#id`/`[attr]` with `^=`/`$=`/`*=`/`~=`/`\|=`, descendant + child, comma, full pseudo-class set, `:not`); version-cached `querySelectorAll` / `getElementById` |
| `cascade` | partial `getComputedStyle` — inline + matched `<style>` rules with specificity/order, curated inheritance, `:host`/`::slotted` shadow scoping, color canonicalization; memoized per version |
| `events` | `Dom` (owns the tree + listeners); capture/target/bubble dispatch, listener-less fast-skip, `stopPropagation`/`preventDefault`, tree-mutating handlers |
| `serialize` | inner/outerHTML (void + raw-text, single-pass escaping) |
| `node_ref` | ergonomic `NodeRef` + `DocumentExt` façade over handles |
| `color` | CSS color → `rgb()`/`rgba()` (named/hex/hsl) |
| `cssom` | `CssStyleSheet`/`CssStyleRule` + insert/deleteRule + parse |
| `svg` | SVG length/string/viewBox accessors |
| `file` | Blob/File + sync FileReader (text / data-URL base64 / array-buffer) |
| `custom_elements` | name→definition registry |
| `location` | Location (URL parse) + History stack |
| `stubs` | Storage, media-query eval, observer entries |
| `mutations` | `MutationObserver` — gated record buffer, sync `take_records`, type/target/subtree/old-value filtering |

## Status

- **100% line coverage** on every runtime module; **192 unit tests** (`cargo test`).
- The parser core (`turbo_dom_rtdom::core`) is byte-identical to turbo-dom's and
  passes html5lib-tests tree-construction conformance at **99.72%** there.
- Honest by design: `getComputedStyle` only returns values a real rule/inline/
  inherited declaration set (no invented initial values, no layout/`@media`/state).

## Re-syncing from turbo-dom

`core.rs` and `src/rtdom/*` are copied verbatim from the [turbo-dom](../turbo-dom)
repo (sans the `dump`/`conformance` gate harness). To pull upstream changes,
re-copy those files; the public API and module layout are identical.

MIT.

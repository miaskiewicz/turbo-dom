# Changelog

All notable changes to `@miaskiewicz/turbo-dom` — a native (html5ever + Rust) DOM/window
environment for vitest/jest. Format based on [Keep a Changelog](https://keepachangelog.com/).
Early versions were released as lightweight tags / version-stamped commits (no per-release notes
at the time); this file reconstructs them from history.

## [0.3.5]

### Fixed
- `rtdom::serialize` no longer escapes regular spaces to `&nbsp;` (a quirk carried over
  from the old JS serializer). It corrupted serialized text into U+00A0 non-breaking
  spaces, so a re-parse / accessible-name / text query saw `"a b"` instead of
  `"a b"` and whitespace-normalized matching (getByRole/getByText/toHaveText on a
  serialized DOM) failed. Spaces now serialize verbatim; only `&`/`<`/`>`/`"` are escaped.

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

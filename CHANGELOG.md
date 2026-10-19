# Changelog

All notable changes to `@miaskiewicz/turbo-dom` — a native (html5ever + Rust) DOM/window
environment for vitest/jest. Format based on [Keep a Changelog](https://keepachangelog.com/).
Early versions were released as lightweight tags / version-stamped commits (no per-release notes
at the time); this file reconstructs them from history.

### Added

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


## [0.5.7] — unreleased

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

# turbo-dom-browser-env

The native browser environment of turbo-dom. It binds turbo-dom's pure-Rust DOM
(`turbo_dom::rtdom::Tree`) to V8 through native callbacks. Scripts get the browser
surface with **no JavaScript DOM implementation**:

- `window`, `document`, nodes, ranges and selection;
- CSSOM, forms, events;
- custom elements, shadow DOM, view transitions;
- canvas state and text metrics, Trusted Types and the rest of the surface;
- XML documents (`DOMParser` XML types, CDATA sections, `XMLSerializer`), HTML processing
  instructions, XPath (fragment and shadow-root contexts included), CSS animations and
  transitions in `getAnimations()`, the CSSOM grouping / condition rule interfaces
  (`CSSContainerRule`, `CSSMediaRule`, `CSSLayerBlockRule`, …).
- a `getComputedStyle` cascade that orders rules by Selectors 4 specificity, cascade
  layer and `@scope` proximity, and applies the rules nested in `@media`, `@supports`,
  `@layer` and `@scope` (`@container` / `@starting-style` children never apply);
- XML internal DTD subsets: ATTLIST defaults (attributes and namespace declarations),
  with entity expansion bounded like Chrome ("Entity too big").

Behaviour is held to Chrome by committed Chrome 155/156 controls asserted in the crate's
tests. The crate ships its engine JS (`src/*.js`, run in V8) and production JSON with it.

## Where it sits

```
turbo-dom ──┐
            ├──► turbo-dom-browser-env ──► turbo-test (the runner's DOM, per test file)
turbo-dom-v8┘                          └─► turbo-surf (pages; with host-internals)
```

It moved here from turbo-test (miaskiewicz/turbo-test@cb5c64e) in the 0.6.0 consolidation,
so turbo-test and turbo-surf share one published crate. `scripts/verify-copied.sh`
(repository root) re-derives that one-time move for review
([docs/CONSOLIDATION.md](../../docs/CONSOLIDATION.md)).

## Install

```toml
[dependencies]
turbo-dom-browser-env = "0.8"                       # embedders: features = ["host-internals"]
[dev-dependencies]
turbo-dom-browser-env = { version = "0.8", features = ["test-support"] }
```

MSRV 1.91. The crate links `turbo-dom-v8` `~150.4`, which downloads the prebuilt rusty_v8
150.4.0 static library unless `RUSTY_V8_ARCHIVE` points at one. A dependency graph holds
exactly one turbo-dom-v8 (`links = "rusty_v8"`).

## Features

| feature | default | effect |
|---|---|---|
| `host-internals` | off | `turbo_dom_browser_env::host_internals` (doc-hidden): host-only access to the native documents behind DOM wrappers. Never reachable from page JS |
| `test-support` | off | `turbo_dom_browser_env::test_support`: the probe / control / case files as `pub const`s plus `FILES` (name → bytes), for embedders' Chrome-parity tests |
| `native-sdk-frame-read` | off | reads an interceptor's should-throw mode from the V8 frame instead of the fork's `PropertyCallbackArguments::should_throw_on_error`. Only for native V8 SDKs without the fork fix (turbo-surf's engine overlay); otherwise `assert!(v8::FORK_SHOULD_THROW_FIX)` rejects a stock V8 at compile time |

## API

**Model.** There is one native document per V8 context. `window` has Chrome's Window
interface chain (`Window.prototype` -> `WindowProperties` -> `EventTarget.prototype`). A JS node is a V8 object that
carries its `Handle`. Native callbacks read the handle, call `Tree`, and wrap results
back, preserving identity (`el === el`). A callback entered through a node of another
context switches to that context's document and restores the caller's document on
return.

`turbo_dom_browser_env::browser_env`:

| use | items |
|---|---|
| test runners (turbo-test) | `install(scope)` (blank document per test file), `enabled()`, `reset()` between files |
| seeding documents | `install_with_html`, `try_install_with_html` (`Err(SetupTerminated)` when a watchdog terminated setup), `install_html`, `install_with_html_source_positions`, `install_with_utf16_source_positions`, `install_independent_html`, `install_independent_html_units`, `install_independent_utf16_source_positions`, `install_independent_srcdoc_units` (never quirks) |
| serializing | `document_html()` |
| event loop | `flush_ownership(scope)` between tasks (wrapper ownership is reconciled in batches, so building n nodes is O(n)) |
| realms and frames | `register_dom_exception`, `capture_window_event_interfaces`, `retire_custom_element_context`, `set_computed_font_defaults`, `set_canvas_fonts` (canvas `measureText` fonts: the page's loaded `@font-face` faces + system fonts as a turbo-dom-layout `FontRegistry`; bundled faces otherwise) |
| engine namespace | `engine_namespace(scope)` (the realm's internal state object, held in a V8 private: page script cannot reach it), `run_engine_script(scope, source)` (runs `source` as a function body whose `__tsNs` parameter is that namespace; `ENGINE_NAMESPACE_PARAM`). An embedder with a layout sets `layoutUsedSize` on it: `(element) => [borderBoxWidth, borderBoxHeight, contentBoxWidth, contentBoxHeight, fontSizePx?]` in unzoomed CSS px, or `null` for an element without a box; getComputedStyle's `width` / `height` then read the used value of a rendered box like Chrome 156 (turbo-dom#316), and `transform` percentage translates / `em` resolve against the border box and the (optional) fifth fontSize entry (turbo-dom#353). Without it a rendered box reports its declared size (or "") and a transform list with a percentage keeps its declared text. The binding installs no `CSS` namespace: an embedder's `CSS.supports()` calls the namespace's `cssSupports(text)` / `cssSupportsProperty(property, value)` (the grammar of the cascade, `@supports` and `el.style`; turbo-dom#322) and its `CSS.px()`-style factories come from `typedOmFactories()`. `mark_worker_context(scope, context) -> Result<(), WindowAlreadyInstalled>` tags a context as a worker so a later `install*` call skips the DOM template entirely — no `window` / `document` / `Element` on its `globalThis`. Mark it before anything installs into it: a context that already holds a window is refused (`Err(WindowAlreadyInstalled)`, left unmarked), and that includes every context `v8::Context::new` makes on a `snapshot_blob()` isolate (the blob's default context is the bootstrapped window), so worker contexts come from a non-snapshot isolate or the default context of a `snapshot_blob_with(prelude)` isolate (the prelude alone; the window is `SNAPSHOT_WINDOW_CONTEXT`) (minimum opt-out hook for a Worker-shaped global scope that owns its own globals, turbo-dom#385 for turbo-surf #203; the full `install_worker_global_scope` helper — `self`, `importScripts`, structured-clone — lands in 0.8.1). |
| embedding hooks (turbo-surf) | the other `pub fn`s: document response facts (`set_document_last_modified`, `set_document_domain_sandboxed`, `set_document_origin_keyed`; `share_document_domain(scope, document, origin)` joins a document to another's origin — initial `about:blank` / `about:srcdoc` children of one agent cluster — so `document.domain` reads and relaxes one effective domain across them like Chrome 156, turbo-dom#284; without it every document keeps its own), form submission, CSS policy, Trusted Types, inline handlers, trusted events (`mark_event_trusted`, `dispatch_host_event`; `dispatch_host_event_did_listeners_throw(scope, event, target) -> bool` dispatches and also returns the DOM's legacy-output-did-listeners-throw flag — whether a listener of that dispatch threw (nested dispatches don't count) — which IndexedDB aborts a transaction on, turbo-surf#148; every listener exception is reported at the listener's window like Chrome 156, turbo-dom#301), user activation (`ActivationHost` answering `ActivationQuery`: new `ActivationQuery::Sticky` variant — the window has ever been activated — gates media `play()`'s autoplay policy, turbo-dom#323; without a host `Sticky` reports false), document parsing / import (`parse_xml_document_units` parses `DOMParser`'s XML types into an `XMLDocument`, `<parsererror>` documents included, and returns it as `ParsedXmlDocument { document, well_formed }` so an embedder can tell a malformed source without inspecting the tree), node nonces |

**Runtime switches.** Set per window after install; an embedder that never calls them gets
Chrome's behaviour. Embedders select jsdom compatibility explicitly for their test environments.

| switch | default | off / other value |
|---|---|---|
| `set_media_mode(scope, MediaMode)` | `MediaMode::Chrome`: live `matchMedia` / `MediaQueryList`, `@media` / `@import` media lists evaluated against the viewport | `MediaMode::Jsdom` (jsdom 27): no `matchMedia`, a media list applies only with an exact `screen` item |
| `set_css_transitions(scope, enabled)` | `true`: a transitioned property's change creates a `CSSTransition` that animates it and fires `transitionrun` / `transitionstart` / `transitionend` / `transitioncancel` like Chrome 156 | `false` (jsdom 27): no transition runs, the end value applies at once, no transition event fires |
| `set_css_animations(scope, enabled)` | `true`: an `animation-name` naming a `@keyframes` rule creates a `CSSAnimation` that animates the element and fires `animationstart` / `animationiteration` / `animationend` / `animationcancel` like Chrome 156 | `false` (jsdom 27): no `@keyframes` animation runs, no animation event fires; `Element.animate` is unaffected |
| `set_touch_event_mode(scope, TouchEventMode)` | `TouchEventMode::Chrome`: TouchEvent init lists require branded `Touch` entries | `TouchEventMode::Jsdom`: retain entries unchanged in `touches`, `targetTouches`, and `changedTouches`, including touch-like plain objects |
| `set_computed_style_mode(scope, ComputedStyleMode)` | `ComputedStyleMode::Chrome`: Chrome computed-value serialization | `ComputedStyleMode::Jsdom`: preserve the winning declared `text-decoration` shorthand without synthesized computed color; other properties retain Chrome semantics |

The computed-style mode is independent of media, CSS motion and touch-event mode. It
currently scopes compatibility to the declared `text-decoration` shorthand: explicit color
and style tokens remain represented using declared CSSOM serialization, while separately declared longhands do not synthesize a
shorthand. Explicit colors use the same canonical spelling as the declared style (for
example, a hexadecimal color serialized as RGB); equivalent format-only assignments
therefore preserve coherent retained views. Mode changes invalidate computed snapshots,
including retained live views.

The touch-event switch is independent of media and CSS motion. It affects future event
construction in the current realm; existing lists and other realms are unchanged. jsdom
mode preserves entry identity through list indexing, iteration and `item()` (the latter is
an additional TouchList convenience; jsdom 27 itself exposes arrays). A test runner should
select this mode from its effective environment, independently of a Chrome-media opt-in.

The two CSS switches are independent of each other and of the media mode. With both off,
the Web Animations surface jsdom lacks (`Element.prototype.animate` / `getAnimations`,
`Document.prototype.getAnimations` / `timeline`, `ShadowRoot.prototype.getAnimations`,
`Animation`, `AnimationEffect`, `AnimationTimeline`, `DocumentTimeline`, `KeyframeEffect`,
`AnimationPlaybackEvent`, `CSSAnimation`, `CSSTransition`, `AnimationEvent`) is hidden too;
turning either back on restores the same bindings (`TransitionEvent` always stays). CSS
events dispatch to the owner element in the rendering frame after the change, before that
frame's animation-frame callbacks (`css_animation_events_chrome156.json`). A running CSS
animation keeps rendering frames requested while its switch is on (its next iteration is an
event), as Chrome does; script animations without an end still request none.

Also in the crate:
- `css_snapshot::{SheetSnapshot, RuleSnapshot}` is the JSON contract of the private
  `cssHost.exportSheet` ([docs/live-css-host.md](docs/live-css-host.md)).
- `css_values` re-exports `turbo_dom::rtdom::css_values` (`zoom_value`, `css_number`).
- `host_internals` provides:
  - `enter_document_of` / `enter_context_document` (nesting `DocumentScope` guards);
  - `with_tree`, `handle_of`, `wrap`, `is_connected`, `host_composed_children`;
  - `current_document_id`, `document_id_of`, `is_browsing_context`,
    `current_document_context`, `retire_context`;
  - `interface_record`, `cascade_policy`;
  - `css_effect_lifecycle(scope, element)` and `CssEffectSnapshot`: trusted per-element
    CSS/Web Animations lifecycle inspection for native geometry (see below);
  - re-exports of `Tree`, `Handle`, `StylePolicy`, `zoom_value` and the snapshot types.
- `test_support` has 98 constants named after their files (for example
  `KEY_ORDER_PROBE_JS`), plus `FILES` and `frame_read_should_throw`. The files ship in
  the package; consumers reference the constants and never copy the files.

**Native geometry and animations (#507).** With `host-internals`,
`css_effect_lifecycle(scope, element)` enters the wrapper's creation realm and native
owning document, then synchronizes only its underlying CSS cascade and style dependencies.
It reads private canonical animation records, including completed CSS animations no longer
returned by `getAnimations()`. It never calls page-overridable animation getters, flushes the
whole document, composes/interpolates effects, resolves transforms or invokes native layout.
There is no page JavaScript entry point. Normal Chrome animation behavior stays enabled.

The returned snapshot identifies the native document/element, declared names and canonical
effect identities, timing phases, play states, fill, contribution and script overrides.
`allows_underlying_geometry()` proves only that every declared animation has naturally
completed, is finite and non-filling, with no other indexed effects. Active/pending/paused,
infinite, filling, cancelled, replaced, retargeted or script-modified animations, missing
rules, unsupported timelines/composition and disabled CSS kinds retain strict guards.
`None` means unavailable/error/reentry, never an inactive-effect proof. Modified standard builtins/shared
prototypes, indexed prototype properties and promise-assimilation hooks also fail
closed before CSS synchronization can execute guest code. Inspect every element
whose geometry is captured; a target's proof says nothing about its ancestors or descendants.

The host must use explicit microtask checkpoints on the isolate and any separately
attached context queue. The API rejects the isolate's `Auto` policy and never checkpoints
microtasks itself; hosts creating custom queues must configure those queues as `Explicit`.
A snapshot is instantaneous and pins one canonical host-clock sample. Consume it in the same
synchronous native capture without guest JS reentry or advancing the host clock. Requery after
CSSOM/DOM edits, animation methods or frame tasks: private animation changes need not change
the DOM tree epoch. Do not persist these snapshots as a layout-cache permission. The host still
owns the frame clock/animation scheduler and must drive it normally; inspection neither
advances frames nor finishes/cancels animations on the caller's behalf.

**Install phases (turbo-dom#249).** Inside, an install has two parts. The engine bootstrap
(`bootstrap_engine`) does the tree-independent work: templates, natives and the engine
scripts (about 50 `run_engine_script` calls). The document attach (`attach_document`) does
the per-document work: parser scripts, declarative shadow roots, nonces, checkedness,
template contents and inline handlers. The attach runs no engine script. By default every
`install_*` call still runs the original interleaved install. Setting
`TURBO_DOM_ATTACH_PATH=1` makes each install bootstrap the empty shell and then attach the
parsed document.

**Startup snapshot install (turbo-dom#249, opt-in).** `snapshot_blob()` builds, once per
process and from the process's own V8, a startup snapshot whose default context is the
bootstrapped shell, plus the external reference table every isolate made from it needs
(`SnapshotBlob::create_params()`). The blob is never shipped: it is tied to the V8 build
and flags. `install_from_snapshot(html)` creates and owns an isolate from the blob,
rehydrates what the snapshot cannot carry (context slots, isolate keys, the live-range
registry, the window document's native state) and attaches the document. It returns a
`SnapshotWindow` (isolate + context). It runs no engine script, so per install it costs a
deserialization and a parse instead of a bootstrap. The existing `install*` functions are
unchanged. Embedder capabilities (activation host, timers, fonts, …) are registered on
the new context as after any install. Only the window document's wrapper is carried in
the blob: the build releases every other shell node wrapper and checks none is still
reachable. When the build fails (that check, a callback missing from the reference table,
a panic), a line is logged and `snapshot_blob()` / `install_from_snapshot` return `None`,
so the embedder keeps the normal install. Bootstrap options cannot apply to a snapshot
context: a test namespace, deferred key order, an embedder global template, or page script
run before the install. Embedders that use them keep the normal install.

Measured on darwin-arm64, release build, min / median of 30
(`cargo run --release -p turbo-dom-browser-env --example snapshot_install_cost`):

| | min | median |
|---|---|---|
| fresh isolate + `install_with_html` | 21.04 ms | 22.63 ms |
| `install_from_snapshot` (isolate included) | 4.99 ms | 5.38 ms |
| one-time `snapshot_blob()` build | 171 ms | |

An embedder that bakes its own startup context (turbo-test's framework layer) uses
`snapshot_blob_with(prelude)`: the blob's default context ran `prelude` alone (the
embedder's window-free contexts), and context `SNAPSHOT_WINDOW_CONTEXT` ran `prelude`, then
the engine bootstrap (`v8::Context::from_snapshot`). With `set_snapshot_install(true)`, an
install into a window context rehydrates instead of running the engine scripts; installs
into any other context are unchanged. The prelude must hand V8 no native callback.

`TURBO_DOM_SNAPSHOT_PATH=1` makes the test isolates come from the blob and routes every
install into a snapshot context through the same rehydrate + attach. Each such install
asserts that it ran 0 engine scripts. CI runs the whole suite on all three paths
(default, `TURBO_DOM_ATTACH_PATH`, `TURBO_DOM_SNAPSHOT_PATH`). Tests that configure the
bootstrap itself use a plain isolate on every path.

`tests/public_api.rs`, `tests/host_internals_api.rs` and `tests/test_support_api.rs` pin
every item that turbo-test and turbo-surf call, from outside the crate.

## Gates

- Line coverage **97.6%** floor in the `host-internals,test-support` configuration
  (blocking). This is the accepted bar since 0.6.0; the remaining lines are on the
  post-release list. Measurement history is in docs/CONSOLIDATION.md "Coverage".
- CI `v8 + browser-env` job: tests in the default configuration and with
  `host-internals,test-support`, plus clippy with `-D warnings` (new warnings only).
  - Pull requests: linux x64/arm64 and windows x64, each with the default and simdutf V8
    archives.
  - The release run adds macOS arm64.
- The crate is lint-exempt by design (`#![allow(unused, clippy::all)]` in `src/lib.rs`).
- Browser-env is excluded from both complexity scans after build-breaking extractions
  were reverted (#490). The blocking cc ≤ 9 gates cover the other tracked Rust crates
  and the npm runtime (`src/`); they do not certify this binding.
- Native guards reachable by page-controlled values must throw, never abort. Examples:
  oversized strings raise `RangeError` (turbo-dom#183), and a terminated setup returns
  `SetupTerminated` (#184).

## Testing

```sh
export RUSTY_V8_ARCHIVE=/path/to/librusty_v8_release_<target>.a   # v150.4.0
cargo test -p turbo-dom-browser-env
cargo test -p turbo-dom-browser-env --features host-internals,test-support
TURBO_DOM_ATTACH_PATH=1 cargo test -p turbo-dom-browser-env --features host-internals,test-support   # split install path
TURBO_DOM_SNAPSHOT_PATH=1 cargo test -p turbo-dom-browser-env --features host-internals,test-support   # startup snapshot path
```

**Title scaling harness (offline).** The ignored `title_search_scaling` lib test
builds documents with 0, N/10 and N unrelated body rows, checks every title read
and write, and prints native DOM call counts and elapsed milliseconds. For example:

```sh
TITLE_ROWS=5000 TITLE_ITERATIONS=200 cargo test -p turbo-dom-browser-env --lib title_search_scaling -- --ignored --nocapture
```

The regular `title_search_stops_before_unrelated_body_content` regression compares
native call counts, so CI does not depend on wall-clock speed. Document-order,
namespace, insertion and mutation semantics have a separate regression.

**Chrome controls.** Every `src/*_chrome15x.json` file is Chrome's own result for the
probe beside it (`src/*_probe.js`, with cases in `src/*_cases.json`). Each was recorded
with an independently installed Chrome and is pinned to its probe by `probeSha256`.
Leaves that still differ from Chrome are pinned as known gaps, with both values and the
issue number. The collectors live in this crate's `scripts/`; run them from this crate's
directory:

| collector | records |
|---|---|
| `scripts/dom-lifecycle-chrome-control.mjs` | lifecycle probes (custom elements, adoption, templates, form collections, …) |
| `scripts/css-probe-chrome-control.mjs`, `scripts/css-live-chrome-control.mjs` | CSS/CSSOM probes; `css_live_chrome155.json` |
| `scripts/dom-implementation-chrome-control.mjs`, `scripts/xml-factory-elements-chrome-control.mjs` | `dom_implementation_chrome155.json`, `xml_factory_elements_chrome155.json` |
| `scripts/animations-chrome-control.mjs`, `scripts/select-element-chrome-control.mjs`, `scripts/typed-om-chrome-control.mjs` | probe + cases controls (`animations*`, `select_element*`/form pseudo-classes, `option_factory*` (`new Option()`, `HTMLOptionsCollection` definer), `typed_om*`) |
| `scripts/typed-om-registry.mjs` | `typed_om_registry_cases.json` and the production registry `typed_om_properties.json` (from Blink's json5 metadata + the registry control) |
| `scripts/view-transitions-chrome-control.mjs`, `scripts/shadow-html-chrome-control.mjs` | `view_transitions_chrome156.json`; shadow-root / HTML serialization / sanitizer controls |
| `scripts/invokers-chrome-control.mjs`, `scripts/editing-input-chrome-control.mjs`, `scripts/trusted-input-chrome-control.mjs` | invokers/popover, editing, pointer and other trusted-input controls; `canvas_style_chrome156.json` (2D context `fillStyle` / `strokeStyle` / `shadowColor`, `currentcolor`), `canvas_state_chrome156.json` (2D context `save()` / `restore()` drawing-state stack); the edge probes of `edge_probes_tests.rs` (`shadow_slots_edges`, `dom_mutation_validity`, `selector_edges`, `sanitizer_edges`, `document_edges`, `receivers_edges`) |
| `scripts/image-srcset-chrome-control.mjs` | `html_image_srcset_chrome156.json` (Playwright, DPR 1 / 2, local image server; cases from `scripts/make-image-srcset-cases.py`) |
| `scripts/xml-documents-chrome-control.mjs` | `xml_documents_chrome156.json` (XML documents, `<parsererror>`, CDATA sections, `XMLSerializer`; Playwright with `--chrome` the frozen headless shell) |
| `scripts/document-legacy-chrome-control.mjs` | `document_legacy_chrome156.json` (loopback HTTP/HTTPS cases); `document_domain_chrome156.json` (shared `document.domain` across top / about:blank / srcdoc, recorded with the frozen headless shell via `TURBO_SURF_CHROME_PATH`) |
| `scripts/trusted-types-control/` | Trusted Types controls (`trusted_types*_chrome155.json`); `cdp-connection.mjs` is shared with the document-legacy collector |

The `--runner` collectors drive turbo-surf's `harness/detection/chrome-cdp-trace.mjs`.
Pass its absolute path. A refreshed control changes the oracle, so commit it together
with the probe change that needed it.

Layout-dependent reads (geometry) belong to the embedder: this crate lays nothing out.
turbo-surf answers them with turbo-dom-layout, whose tests use fixture fonts. The one
exception is canvas `measureText`, which measures with turbo-dom-layout's text stack
(a dependency since 0.7.0) against the fonts the host passes to `set_canvas_fonts`.

## Links

[turbo-dom](../turbo-dom/README.md) · [turbo-dom-v8](../turbo-dom-v8/README.md) ·
[docs/live-css-host.md](docs/live-css-host.md) · [src/xpath_provider/README.md](src/xpath_provider/README.md) ·
[CHANGELOG](../../CHANGELOG.md)

License: MIT.

### Replacing a live document's contents

`replace_document_content(scope, document, utf16)` is an additive host handoff for
`Page.setContent`. It preserves the existing Window, document wrapper, detached
old nodes, document listeners, URL, compatibility mode and installed host policies.
It synchronously parses and replaces the document's children, then returns script
nodes in parser order. Parser scripts do not execute inside the call. Custom-element
constructors and reactions run synchronously through the normal insertion path
after the replacement content is attached; they can observe and mutate that
content. Their execution is distinct from the host loader completion signal.
Scripts created by those reactions retain their dynamic admission state and are
not returned as newly parsed work; a host-admitted script is never reset.

```rust,ignore
let scripts = replace_document_content(scope, document, &html.encode_utf16().collect::<Vec<_>>())?;
for node in scripts {
    if let Some(script) = prepare_script(scope, node) {
        // Apply the same CSP checks and module/external/inline scheduling as
        // the initial document load, including parser-blocking order.
        page_loader.enqueue(script);
    }
}
// Resolve setContent only when page_loader reaches the requested load state.
```

The call's return means DOM replacement has completed, not that resources have
loaded. The embedder must invalidate its layout/resource captures, update its
ready-state tracking, execute admitted scripts through its existing loader, and
emit DOMContentLoaded/load as that loader completes. `prepare_script` retains its
one-shot admission semantics, parser-inserted flags, UTF-16 source, nonce and
external/module metadata. Inline-handler authorization stays installed and is
applied to the new attributes. Host script policy still belongs to the loader.

This API replaces contents, not the full browser document-stream algorithm:
ordinary guest `document.open/write/close` remains unsupported, compatibility mode
is retained, document listeners are preserved, and parser insertion from nested
`document.write` is not implemented. Embedders should call this host API directly
for content replacement instead of evaluating those stub methods or reopening a
PageSession. In particular turbo-surf must thread the returned scripts into its
existing parser/CSP/resource scheduler and use that scheduler's completion signal;
merely calling the replacement function cannot satisfy a load-state wait.

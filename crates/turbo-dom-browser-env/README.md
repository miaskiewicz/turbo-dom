# turbo-dom-browser-env

The native browser environment over [`turbo-dom`](../turbo-dom): turbo-dom's
pure-Rust DOM (`turbo_dom::rtdom::Tree`) bound to V8 through native callbacks.
Pages and tests get `window`, `document`, nodes, CSSOM, forms, events, ranges,
selection, custom elements, shadow DOM, view transitions and the rest of the
browser surface with **no JavaScript DOM implementation**. Behaviour is held to
Chrome by committed Chrome 155/156 controls (`src/*_chrome15x.json`, recorded by
`src/*_probe.js`) asserted in the crate's tests.

The binding moved here from turbo-test (miaskiewicz/turbo-test@cb5c64e) so that
turbo-test and turbo-surf share one published crate instead of copies.
`scripts/verify-copied.sh` (repository root) re-derives that one-time move for
review; it is not a sync tool and nothing is copied after the move.

## Install

```toml
[dependencies]
turbo-dom-browser-env = "0.6"                     # + features = ["host-internals"] for embedders
[dev-dependencies]
turbo-dom-browser-env = { version = "0.6", features = ["test-support"] }
```

MSRV 1.91 (turbo-dom-v8). The crate links `turbo-dom-v8` `~150.4`, which downloads the
prebuilt rusty_v8 150.4.0 static library unless `RUSTY_V8_ARCHIVE` points at one; a graph
holds exactly one turbo-dom-v8 (`links = "rusty_v8"`).

## Model

One native document lives per V8 context. A JS node is a V8 object carrying its
`Handle`; native callbacks read the handle, call `Tree`, and wrap results back
(identity-preserving, `el === el`). Callbacks entered through a node of another
context switch to that context's document and restore the caller's on return.

## Public API

Everything is in `turbo_dom_browser_env::browser_env` (re-exported pieces noted).

| use | items |
|---|---|
| test runners (turbo-test) | `install(scope)` (blank document per test file), `enabled()`, `reset()` between files |
| seeding documents | `install_with_html`, `install_html` (alias used for page loads), `install_with_html_source_positions`, `install_independent_html`, `install_independent_html_units`, `install_independent_utf16_source_positions`, `install_independent_srcdoc_units` (iframe `srcdoc`, never quirks) |
| serializing | `document_html()` — the current live document as HTML |
| embedding hooks (turbo-surf) | the `pub` functions in `browser_env` (activation host, form submission, CSS policy, trusted types, mutation consumers, key order, storage collection, …); `tests/public_api.rs` pins every item turbo-test and turbo-surf call |
| CSS export contract | `turbo_dom_browser_env::css_snapshot::{SheetSnapshot, RuleSnapshot}` — the JSON the private `cssHost.exportSheet` returns |
| shared CSS grammar | `turbo_dom_browser_env::css_values` = `turbo_dom::rtdom::css_values` (`zoom_value`, `css_number`) |

### Feature `host-internals` (doc-hidden)

`turbo_dom_browser_env::host_internals`: coarse host-only access to the native
documents behind DOM wrappers, for native services written outside the crate
(turbo-surf's native layout capture and compression-dictionary discovery).
Nothing here is reachable from page JavaScript.

- `DocumentScope` guard: `enter_document_of(scope, object)`,
  `enter_context_document(context)`; guards nest like a stack.
- Tree access: `with_tree(|&Tree| ..)` (the closure must not call back into the
  binding or V8), `handle_of`, `wrap`, `is_connected`, `host_composed_children`.
- Document state: `current_document_id`, `document_id_of`, `is_browsing_context`,
  `current_document_context`, `retire_context`.
- Private records: `interface_record` (e.g. a document's `cssHost` and
  `imageHost`, see "Image load state"), `cascade_policy` (`StylePolicy`).
- Re-exports: `Tree`, `Handle`, `StylePolicy`, `zoom_value`, `SheetSnapshot`,
  `RuleSnapshot`.

`tests/host_internals_api.rs` exercises every item from outside the crate.

### Feature `test-support`

`turbo_dom_browser_env::test_support`: the probe/control/case files embedders'
Chrome-parity tests use, as `pub const` (`&str`, or `&[u8]` where a test needs
bytes), named after the file (`KEY_ORDER_PROBE_JS`, `ANIMATIONS_FRAMES_CASES_JSON`,
…), plus `FILES` (file name → bytes) and `frame_read_should_throw` (the
frame-read interceptor source, for accessor agreement tests). The files ship in
the package under `src/`; consumers reference the constants, never copy files.
The 76 files are what turbo-surf's Chrome-parity integration tests read,
including the custom-element lifecycle contract (`tests/custom_elements.rs`:
`CUSTOM_ELEMENTS{,_EXTENDED,_REACTIONS,_IDL,_MOVES,_CALLBACKS,_RETIRED}_*`,
`DOM_PARSER_OBSERVER_LIFECYCLE_*`, `CSS_ADOPTION_LIFECYCLE_*` probe/control pairs)
the image-state control (`HTML_IMAGE_{PROBE_JS,CASES_JSON,CHROME156_JSON}`) an
image-loading host runs through its own loader and layout, the `:defined` control
(`CUSTOM_ELEMENTS_DEFINED_{PROBE_JS,CASES_JSON,CHROME156_JSON}`), and the computed-style
defaults / `<head>` control (`COMPUTED_DEFAULTS_{PROBE_JS,CASES_JSON,CHROME156_JSON}`).
Add a constant (and its `FILES` entry) when another consumer needs a file.
`tests/test_support_api.rs` checks every constant against its file and every
probe/control pair's recorded `probeSha256`.

### DOMException interface

Every DOMException the binding raises — natively (`throw_dom_error`) or from its JS
modules — is built, when it is raised, from the realm's registered interface: the
binding's own `DOMException`, recorded at install, unless the embedder installs its own
and hands it over with `register_dom_exception(scope, constructor)` (turbo-surf's runtime)
or by calling `capture_window_event_interfaces(scope)` after installing it. No module keeps
a constructor captured at install, and page script reassigning `globalThis.DOMException`
changes nothing, as in Chrome (miaskiewicz/turbo-surf#42, miaskiewicz/turbo-dom#72). Module
code uses `new ts.DOMException(message, name)` / `instanceof ts.DOMException` (the
`globalThis[Symbol.for('ts')]` namespace; `ts.realmDOMException()` returns the current
interface). Control: `src/dom_exception_realm_chrome156.json` (27 operations across the
modules, with the interface as installed and after page script replaced it).

### Image load state

The binding never fetches or decodes images. `HTMLImageElement.prototype` carries
`complete`, `naturalWidth`, `naturalHeight` and `currentSrc` as Chrome's read-only
accessors (an `<img>` never has them as own properties); without host state an
image is "not loading": `complete` is `true`, the natural size `0`×`0` and
`currentSrc` the selected source (below; `""` without one), which is Chrome's answer for an
image without a source and for a broken image (`src/html_image_chrome156.json`,
`src/html_image_srcset_chrome156.json` `none1`). A
host that loads images (turbo-surf) publishes each image's state through the
document's private interface record, which page JS cannot reach:

```js
record.imageHost.setState(img, { complete: false });                    // a load started
record.imageHost.setState(img, { complete: true, naturalWidth: 64,
                                 naturalHeight: 40, currentSrc: url }); // it finished
record.imageHost.setState(img, null);                                  // back to the defaults
```

`setState` accepts only an HTML `<img>` of the realm (TypeError otherwise), copies
the state when called (`complete` → boolean, sizes → unsigned long, `currentSrc` →
string, missing → `""`) and keeps it until the host replaces or clears it, so the
host updates it whenever a load starts or ends (including `src` changes).

`width` / `height` are Chrome's unsigned long accessors reflecting the content
attributes (HTML §4.8.4.3; setters write the attribute, above 2147483647 → `0`), and
`x` / `y` the read-only CSSOM View box position. While an image is being rendered the
getters return its layout box; the binding has no layout, so the host answers through
the same record:

```js
record.imageHost.setLayoutQuery((img) =>
  rendered(img) ? { width, height, x, y } : null);       // content-box size, ICB position (CSS px)
record.imageHost.setLayoutQuery(null);                   // no layout: nothing is rendered
```

The query runs on every read (page JS may have changed the DOM since the last one)
with the image as its only argument; `width` / `height` are converted as unsigned long
and `x` / `y` as long. For an image it answers `null` / `undefined` for (or without a
query, as in turbo-test) the getters follow Blink's not-rendered path: the attribute
when it parses as a non-negative integer (up to 4294967295: `"12px"` → 12, `"-1"` and
`"4294967296"` fall through), else the natural size from `setState` (`0` without one);
`x` / `y` are `0`. Control: `src/html_image_chrome156.json` `dimensions` (rendered,
`display: none` and detached images, attribute parsing, setter conversions).

`srcset` / `sizes` (HTMLImageElement and HTMLSourceElement), HTMLSourceElement `media` /
`type` (DOMString reflections) and `width` / `height` (unsigned long) reflect their
attributes. The binding selects an image's source like Chrome 156 (HTML "select an image
source", miaskiewicz/turbo-dom#71): the first `<source>` before the image in its `<picture>`
parent with a non-empty `srcset` whose `media` query list matches and whose `type` Chrome
decodes (JPEG, PNG, APNG, GIF, BMP, ICO, XBM, WebP, AVIF, JPEG XL, SVG; MIME parameters
ignored), else the image's own `srcset` plus `src` as `1x`; `w` descriptors divided by the
`sizes` source size (media conditions with `and` / `or` / `not`, range syntax, `width` /
`height` / `aspect-ratio` / `orientation` / `resolution` / `-webkit-device-pixel-ratio`;
`px`, `em` (16 px), viewport units, `calc()` / `min()` / `max()` / `clamp()`; default
`100vw`), then the first candidate whose density is at least the device pixel ratio, else
the densest. URLs are parsed against the document base URL (the first `<base href>`, else
the document URL) by the native resolver `ts.resolveDocumentURL`, which the reflected `src` /
`href` getters use too (miaskiewicz/turbo-dom#73; no embedder `URL` global needed). A host that loads images asks
for the choice and reports the natural size Chrome derives from it:

```js
const chosen = record.imageHost.selectSource(img, { devicePixelRatio: 2 }); // {url, density} | null
// environment fields default to window.innerWidth / innerHeight / devicePixelRatio
// (1024 x 768 at 1 where the window has none, as in turbo-test)
const { naturalWidth, naturalHeight } = record.imageHost.naturalSize(w, h, chosen.density);
record.imageHost.setState(img, { complete: true, naturalWidth, naturalHeight, currentSrc: chosen.url });
```

`naturalSize` floors the size times the float reciprocal of the density (a 301-px-wide
`2x` image is 150 wide). Control: `src/html_image_srcset_chrome156.json`
(`scripts/image-srcset-chrome-control.mjs` over `scripts/make-image-srcset-cases.py`'s 95
images at DPR 1 and 2, and with nothing loading).

`HTMLImageElement.prototype.decode()` resolves when the host's state is complete
with a natural size and rejects with an `EncodingError` otherwise (no source,
broken, not loading); called while the host reports `complete: false` it settles
at the host's next `setState`. The native legacy factory `Image(width, height)`
(new-only, length 0, `Image.prototype === HTMLImageElement.prototype`, NewTarget's
prototype honoured) creates an ordinary `<img>` with the `width` / `height`
attributes — no own `src`, no fake `onload`: loads (and their `load` / `error`
events) are the host's. Control: `src/html_image_chrome156.json` (`factory`, `own`,
`decode` sections).

### Computed style defaults

`getComputedStyle` (the binding's cascade over document sheets, inline style and
shadow-scoped rules; document rules never select elements inside shadow trees,
turbo-dom#68) reports Chrome 156's computed values where no author rule
applies. `display` is the element's html.css display: block / inline / none /
table-* / list-item / inline-block / contents / ruby, plus `[hidden]`,
`dialog:not([open])`, closed popovers, `input[type=hidden]` and `audio` without
controls → `none`, and MathML `math` / `block math`. It is then blockified for
floats, `position: absolute` / `fixed` and flex / grid items. `position` is
`static` (`dialog` → `absolute`, `[popover]` → `fixed`), and `visibility` is
inherited, `visible` at the root. `inherit` / `initial` / `unset` / `revert`
resolve. Form controls take html.css's `font: -webkit-small-control` (turbo-dom#61):
`<input>` (every type), `<select>` and `<button>` compute `13.3333px Arial`,
`<textarea>` and the date / time inputs `13.3333px monospace`, `<optgroup>` is
`bolder` and `<option>` `normal` weight; author rules win (case `form-control-fonts`).
Disconnected elements report `""` for every property, author and inline values
included (turbo-dom#67; the declaration fills in once the element is connected), and
children of `<video>` / `<audio>` report `""` for the defaults, as in Chrome. Control: `src/computed_defaults_chrome156.json`, which also records that
a document has no `<title>` it did not parse (`document.title = …` creates one).

The top layer is modelled as far as the binding has one (turbo-dom#30): a dialog
shown with `showModal()` matches `:modal` and computes `position: fixed`,
`top` / `bottom: 0px`, `max-width` / `max-height: calc(100% - 6px - 2em)`,
`overflow: auto` and `visibility: visible` (author rules win); a top-layer element
(modal dialog, open popover) with `position: static` / `relative` / `sticky`
computes `absolute`. `getComputedStyle(el, '::backdrop')` computes the backdrop:
`block`, `absolute` with `auto` insets outside the top layer, `fixed` at inset 0
inside it (`rgba(0, 0, 0, 0.1)` behind a modal dialog), author `::backdrop` rules,
inheritance from the element. Control: case `dialog-top-layer` of the same file.

Fullscreen (turbo-dom#58): `requestFullscreen()` with the embedder's user activation
(`set_activation_host`; each request consumes it) puts the element in the document's
fullscreen stack, which the binding reports to the tree (`Tree::set_fullscreen_stack`):
the selector engine matches `:fullscreen` / `:-webkit-full-screen` (every stacked element)
and `:-webkit-full-screen-ancestor` (the top's shadow-including ancestors) everywhere,
stylesheets and the shadow-scoped native cascade included; `:modal` also matches the
fullscreen element. A fullscreen element other than the root computes Chrome's
`:not(:root):fullscreen` values — `position: fixed`, `inset: 0`, `margin: 0`,
`box-sizing: border-box`, `min-*: 0`, `max-*: none`, `width` / `height: 100%` (computed
as the viewport's `innerWidth` / `innerHeight` px), `transform: none`, all `!important`, and `object-fit: contain` — and its
`::backdrop` is fixed at inset 0 with `rgb(0, 0, 0)`. `fullscreenchange` /
`fullscreenerror` fire as before. Control: case `fullscreen` (async; the collector's
`__cdpGesture()` is a trusted CDP click).

### Text tracks

`<track>` is an `HTMLTrackElement` (turbo-dom#74, control `src/html_track_chrome156.json`):
`kind` (missing → `subtitles`, invalid → `metadata`), `srclang`, `label`, `default`, the
shared `src` URL reflection, `readyState` `NONE` (no track is ever loaded) and the
`NONE`..`ERROR` constants; `track` is the element's `TextTrack` (kind / label / language / id
from the element, `mode` `disabled` / `hidden` / `showing`, empty `TextTrackCueList`s for
`cues` / `activeCues` once enabled, `oncuechange`; `addCue` / `removeCue` reject non-cues).

### Inline style: shorthands and longhands

`CSSStyleDeclaration` (`element.style`, `CSSStyleRule.style`) follows Chrome 156's CSSOM
(turbo-dom#90, #97) through turbo-dom's `rtdom::css_shorthand` (`__cssShorthandNative`,
`src/css_shorthand.rs`): setting one of the 129 shorthands Chrome has (and the 15 shorthand
aliases such as `-webkit-flex`) sets its longhands in Chrome's order, omitted ones at their
initial value (`initial` where Blink leaves them implicit) or, for a `var()` value, pending
on the shorthand; `getPropertyValue(shorthand)` and `cssText` serialize back with Blink's
rules and shorthand preference; `removeProperty(shorthand)` removes its longhands and
returns `''`; parsing `cssText` moves a repeated property to the end and lets `!important`
win; `word-wrap` and the other 101 longhand aliases resolve. A longhand set directly is
spelled like Chrome (`0` → `0px`, units / keywords lowercase, `url("...")`, double quotes,
`rgb()`); rtdom's grammar admits or rejects the longhands it knows, the stylesheet grammar the
rest. The computed cascade expands UA / animation shorthands with the same engine and reads
the stylesheet grammar's text of longhands set directly. Typed OM writes and reads
(`StylePropertyMap`) keep the value as parsed, the way Chrome stores the `CSSStyleValue`
(`calc(10px + 5%)` in its order, `calc(3 * 2px)`, `50%`). Controls: turbo-dom's
`crates/turbo-dom/tests/fixtures/css_shorthand_chrome156.json` (3110 scenarios) and
`css_longhand_chrome156.json` (2566 pairs), run by `src/css_shorthand_tests.rs` and
`src/css_longhand_tests.rs`.

### SVG and MathML elements

`SVGElement` and `MathMLElement` include the `ElementCSSInlineStyle`, `HTMLOrSVGElement` and
`GlobalEventHandlers` mixins like Chrome 156 (turbo-dom#87, control
`src/svg_mathml_element_chrome156.json`): `style` (with the `cssText`-forwarding setter),
`dataset`, `nonce`, `tabIndex` (an SVG `<a>` defaults to 0), `autofocus`, `focus()`,
`blur()` and every `on*` handler, each brand-checked against its interface's namespace.
MathML-namespace elements are `MathMLElement`s. An SVG / MathML element takes script focus
when connected, rendered (an SVG element other than `<svg>` needs a non-HTML parent, a
non-SVG element under SVG a `<foreignObject>` parent, nothing under `display: none`) and given
a valid `tabindex` (an SVG `<a href>` needs none). Internal focusing steps (dialogs,
focusgroup navigation) use the unchecked native focus.

### Events across shadow trees

`dispatchEvent` builds the DOM event path through shadow trees (turbo-dom#66): a composed
event crosses each shadow root into its host (an uncomposed one stops at the root of the
target's tree), a slotted node passes through its assigned slot, and every listener sees
`event.target` and `relatedTarget` (MouseEvent / FocusEvent and subclasses) retargeted
against its own tree; hosts receive the event AT_TARGET, `composedPath()` hides closed
trees the listener is outside of, propagation stops where the retargeted `relatedTarget`
is the host it would enter (nothing is dispatched when it equals the target), and after
dispatch `target` / `relatedTarget` are cleared when they stayed inside a shadow tree.
Events whose path crosses no shadow root dispatch as before. Control:
`src/shadow_events_chrome156.json` (exported as `test_support::SHADOW_EVENTS_*`).

`getComputedStyle(el, pseudoElt)` parses `pseudoElt` like Chrome (turbo-dom#47):
`::before` / `::after` (and `:before` / `:after` / bare `before` / `after`) compute
the pseudo-element (own rules, inheritance from the element, `display: inline`
blockified in flex / grid containers, `content: none` unless set), an invalid
pseudo-element returns an empty declaration, and a string without a leading colon
reads the element. Control: case `pseudo-elements`.

The UA-styled pseudo-elements compute too (cases `pseudo-ua-forms` / `pseudo-ua`):
each takes Chrome's html.css values, the author rules that select it — only the
properties it accepts — and inherits from the element:
- `::marker`: `inline-block` (`inline` when `list-style-position` is `inside`, as for
  `summary`), `white-space: pre` (inherited when inside), `tabular-nums`, `isolate`;
  only font / color / content / text properties apply.
- `::placeholder` (text inputs and textarea with a `placeholder`): `block`,
  `rgb(117, 117, 117)`, `pointer-events: none !important`; inputs also `white-space: pre`,
  `overflow: hidden`.
- `::file-selector-button` (file inputs): the button box (`rgb(239, 239, 239)`,
  `1px 6px` padding, `2px outset` border, `margin-inline-end: 4px`, …).
- `::details-content` (details): `block`, `content-visibility: hidden` unless open.
- `::picker(select)`: `none` / `fixed` / `overflow: auto`; `::picker-icon` on select is
  `block`.
- `::first-line` / `::first-letter`: font / color / text / opacity / visibility
  properties (plus float, margins, padding and borders on `::first-letter`).
- highlights (`::selection`, `::highlight()`, `::spelling-error`, `::grammar-error`,
  `::target-text`, `::search-text`): `inline` and `visible`, color / background /
  decoration properties only. A highlight with a rule of its type on the element or an
  ancestor inherits from the parent's highlight; otherwise it takes the element's color.
- `::checkmark` / `::picker-icon` (`content: none`), `::scroll-marker-group` (`block`),
  `::column`, `::-webkit-scrollbar*`, `::scroll-button()`, and the view-transition tree
  (`block`, `absolute`, its insets, inheriting along `::view-transition` > `-group()` >
  `-image-pair()` > `-old()` / `-new()`).

Element-backed pseudo-elements on other elements, and `::checkmark` and the like, are
plain `inline` pseudo-elements; author rules for the element-backed ones do not reach
them. `::scroll-marker` and invalid arguments (`::highlight(*)`, `::picker(foo)`, …)
read an empty declaration. `::cue` and unknown `-webkit-` / `-internal-` names read the
element.

### Custom element state (`:defined`)

`customElements` upgrades, constructions and failed constructors report each
element's HTML custom element state to its turbo-dom `Tree`
(`Tree::set_custom_element_state`), so `:defined` / `:not(:defined)` in `matches`,
`querySelector*`, stylesheets and `getComputedStyle` follow `customElements.define()`
as in Chrome (`src/custom_elements_defined_chrome156.json`: before/after `define()`,
insertion, `customElements.upgrade()`, clones, `innerHTML`, template content, `is`
values from the parser vs `createElement(…, {is})`, a throwing constructor). Clones
and adopted nodes carry the state with them. Without a report the tree derives the
creation-time state (custom element names and parsed `is` attributes are undefined).

### Custom element reactions

DOM methods and setters are `[CEReactions]` scopes: the reactions a call queues run
when it returns, element by element — an element's upgrade (constructor, then the
`attributeChangedCallback` / `connectedCallback` it queues) completes before the
next element's reactions (turbo-dom#43). The HTML fragment parser (`innerHTML`,
`outerHTML`, `insertAdjacentHTML`, `setHTMLUnsafe`, `createContextualFragment`)
queues its elements' upgrades when it creates them, so they run before the old
children's `disconnectedCallback`s. Insertion upgrades connected elements only;
`customElements.upgrade(root)` skips candidates another registry (or none, like
template contents in their inert document) governs; an element is constructed at
most once (turbo-dom#42). Independently installed documents deliver their reactions
before the call returns too (turbo-dom#54). Controls:
`src/dom_adoption_reactions_chrome156.json` (shared with the npm runtime's
`test/dom-adoption-reactions-chrome.test.mjs`, which reads it from this crate) and
`src/dom_reactions_names_chrome156.json`, both exported through `test_support`.

### Feature `native-sdk-frame-read` (off by default)

How setter/definer/deleter interceptors learn whether the caller must throw
(strict `[[Set]]`/`[[Delete]]`, `Object.defineProperty`). With the feature the
binding reads V8's frame; without it, it calls the `turbo-dom-v8` fork's
`PropertyCallbackArguments::should_throw_on_error` and a compile-time
`assert!(v8::FORK_SHOULD_THROW_FIX)` rejects a V8 without the fix. The crate
links the `turbo-dom-v8` fork, so the accessor is the default; only native V8
SDKs that lack the fork fix (turbo-surf's engine-154/155 overlay) enable the
frame read.

## Chrome controls

Every `src/*_chrome15x.json` control is Chrome's own result for the probe next to
it (`src/*_probe.js`, cases in `src/*_cases.json`), recorded with an independently
installed Chrome and pinned to the probe by its `probeSha256`. The collectors that
recorded them live in `scripts/` (moved from miaskiewicz/turbo-test@39b5911 with the
controls' binding; run them from this crate's directory, so `src/` is where they
read probes and write controls):

| collector | records |
|---|---|
| `scripts/dom-lifecycle-chrome-control.mjs` | lifecycle probes (custom elements, adoption, templates, form collections, …) |
| `scripts/css-probe-chrome-control.mjs`, `scripts/css-live-chrome-control.mjs` | CSS/CSSOM probes; `css_live_chrome155.json` |
| `scripts/dom-implementation-chrome-control.mjs`, `scripts/xml-factory-elements-chrome-control.mjs` | `dom_implementation_chrome155.json`, `xml_factory_elements_chrome155.json` |
| `scripts/animations-chrome-control.mjs`, `scripts/select-element-chrome-control.mjs`, `scripts/typed-om-chrome-control.mjs` | probe + cases controls (`animations*`, `select_element*`/form pseudo-classes, `option_factory*` (`new Option()`, `HTMLOptionsCollection` definer), `typed_om*`) |
| `scripts/typed-om-registry.mjs` | `typed_om_registry_cases.json` and the production registry `typed_om_properties.json` (from Blink's json5 metadata + the registry control) |
| `scripts/view-transitions-chrome-control.mjs`, `scripts/shadow-html-chrome-control.mjs` | `view_transitions_chrome156.json`; shadow-root / HTML serialization / sanitizer controls |
| `scripts/invokers-chrome-control.mjs`, `scripts/editing-input-chrome-control.mjs`, `scripts/trusted-input-chrome-control.mjs` | invokers/popover, editing, pointer and other trusted-input controls; `canvas_style_chrome156.json` (2D context `fillStyle` / `strokeStyle` / `shadowColor`, `currentcolor`), `canvas_state_chrome156.json` (2D context `save()` / `restore()` drawing-state stack); the edge probes of `edge_probes_tests.rs` (`shadow_slots_edges`, `dom_mutation_validity`, `selector_edges`, `sanitizer_edges`, `document_edges`) |
| `scripts/image-srcset-chrome-control.mjs` | `html_image_srcset_chrome156.json` (Playwright, DPR 1 / 2, local image server; cases from `scripts/make-image-srcset-cases.py`) |
| `scripts/document-legacy-chrome-control.mjs` | `document_legacy_chrome156.json` (loopback HTTP/HTTPS cases) |
| `scripts/trusted-types-control/` | Trusted Types controls (`trusted_types*_chrome155.json`); `cdp-connection.mjs` is shared with the document-legacy collector |

The `--runner` collectors drive turbo-surf's `harness/detection/chrome-cdp-trace.mjs`
(pass its absolute path); the others speak CDP over `--remote-debugging-pipe`
themselves. A refreshed control is a behaviour change of the oracle: commit it with
the probe change that needed it, and the crate's tests assert it. See
[docs/live-css-host.md](docs/live-css-host.md) for the private live stylesheet
snapshot (`cssHost.exportSheet`) and its control.

### Native DOM lifecycle controls

The binding keeps constructed shadow stylesheet lists during
same-document adoption and disconnect/reconnect, and clears them during
cross-document adoption before custom-element adopted callbacks. Retained
`adoptedStyleSheets` arrays keep their identity. Internal adoption reads private
records instead of page-overridden DOM getters or array methods.

Embedders call `flush_ownership(scope)` whenever control returns to their event
loop (between tasks). On a tree of more than 256 live wrappers, mutating DOM calls
reconcile wrapper ownership only once per `wrappers / 32` calls (keeping the call
wrappers pinned meanwhile), so building n nodes is O(n), not O(n^2); the flush
settles the rest, so a detached subtree becomes collectable before the next task.
`install` counts the isolate's GCs, and the collected-listener/document sweeps run
only after a GC (turbo-dom#112).

Embedders call `retire_custom_element_context(context)` when a frame's browsing
context is retired. This host-only boundary preserves retained registries,
definitions and existing element identity while stopping automatic construction
and callbacks from that definition realm, including reactions already queued.
Ordinary inert documents remain distinct from retired frame contexts.
Text and DocumentFragment constructors create actual native nodes with Web IDL
string conversion, subclass prototypes and constructor-realm document ownership.
The listener-backed EventTarget constructor preserves Node's constructor link.

Independent Chrome155 controls cover these stylesheet transitions, reentrant
custom-element callbacks, contextual fragment ownership, scoped registries and
inert templates. Computed font controls additionally cover actual owner-document
preferences, live inheritance, arithmetic `calc()`, named/manual slots and
retirement. Supported font units and grammar keywords are ASCII case-insensitive;
quoted/custom family names and unresolved custom-property names retain their
original spelling. The host-only `set_computed_font_defaults(context, family, size)`
accepts a nonempty family and finite positive initial size. macOS `Times` is
measured against Chrome155; other-platform `Times New Roman` remains a conventional,
uncertified preference. Metric-dependent `ch`/`ex`, viewport units, `min`/`max`/
`clamp`, custom-property substitution and other per-element heading/form-control
UA font defaults, full `font-variant` combinations and unit-cancelling division
remain unsupported by this bounded resolver. Scalar `calc()` line-height retains
its inherited factor. Unresolved font text is retained through descendants instead
of manufacturing numeric values.
Measured Chrome155 button UA declarations reset its normal Arial 13.3333px font,
spacing and border widths before author CSS is applied. Universal and inline
styles, `font:inherit`, `initial` and measured `revert` behavior are covered by
`src/computed_button_ua_probe.js` and its source-bound Chrome control.
Computed author declarations retain parsed `!important` priority through shorthand
expansion and inline overrides. The source-bound
`src/computed_border_priority_probe.js` control covers nine border cascade cases;
the native-layout/client metrics columns are validated by turbo-surf separately.

Recollect a lifecycle control with turbo-surf's CDP collector and a current Node
version that provides WebSocket (`--chrome /absolute/path/to/Chrome` selects the
executable; use `--probe src/computed_button_ua_probe.js` for the button control):

```bash
node scripts/dom-lifecycle-chrome-control.mjs \
  --runner /absolute/path/to/turbo-surf/harness/detection/chrome-cdp-trace.mjs \
  --probe src/css_adoption_lifecycle_probe.js \
  --out /tmp/css-adoption-control.json
```

These are bounded behavior controls; broader XML parsing and complete browser
lifecycle conformance remain separate work.

## Building and testing

The `v8` dependency links a prebuilt static library. Point `RUSTY_V8_ARCHIVE` at
a local `v150.4.0` archive to avoid the download, then:

```sh
cargo test -p turbo-dom-browser-env
cargo test -p turbo-dom-browser-env --features host-internals,test-support
cargo clippy -p turbo-dom-browser-env --all-targets --features host-internals,test-support -- -D warnings
```

The crate is compiled with `#![allow(unused, clippy::all)]` (see `src/lib.rs`).

## How consumers use it

- **turbo-test** (runner, bundler, transform, coverage, napi host stay there)
  depends on this crate and calls `browser_env::install` / `enabled` / `reset`
  around each test file.
- **turbo-surf** depends on it with `host-internals` for its native layout and
  resource discovery, uses `install_html`, `install_independent_*`,
  `document_html` and the embedding hooks from its Deno runtime, and enables
  `test-support` in dev-dependencies for its Chrome-parity integration tests.

MIT.

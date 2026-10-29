# turbo-dom-browser-env

The native browser environment of turbo-dom. It binds turbo-dom's pure-Rust DOM
(`turbo_dom::rtdom::Tree`) to V8 through native callbacks. Scripts get the browser
surface with **no JavaScript DOM implementation**:

- `window`, `document`, nodes, ranges and selection;
- CSSOM, forms, events;
- custom elements, shadow DOM, view transitions;
- canvas state, Trusted Types and the rest of the surface.

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
turbo-dom-browser-env = "0.6"                       # embedders: features = ["host-internals"]
[dev-dependencies]
turbo-dom-browser-env = { version = "0.6", features = ["test-support"] }
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

**Model.** There is one native document per V8 context. A JS node is a V8 object that
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
| realms and frames | `register_dom_exception`, `capture_window_event_interfaces`, `retire_custom_element_context`, `set_computed_font_defaults` |
| embedding hooks (turbo-surf) | the other `pub fn`s: form submission, CSS policy, Trusted Types, inline handlers, trusted events (`mark_event_trusted`, `dispatch_host_event`), document parsing / import, node nonces |

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
  - re-exports of `Tree`, `Handle`, `StylePolicy`, `zoom_value` and the snapshot types.
- `test_support` has 95 constants named after their files (for example
  `KEY_ORDER_PROBE_JS`), plus `FILES` and `frame_read_should_throw`. The files ship in
  the package; consumers reference the constants and never copy the files.

`tests/public_api.rs`, `tests/host_internals_api.rs` and `tests/test_support_api.rs` pin
every item that turbo-test and turbo-surf call, from outside the crate.

## Gates

- Line coverage **97.6%** floor in the `host-internals,test-support` configuration
  (blocking). This is the accepted bar for 0.6.0; the remaining lines are on the
  post-release list. Measurement history is in docs/CONSOLIDATION.md "Coverage".
- CI `v8 + browser-env` job: tests in the default configuration and with
  `host-internals,test-support`, plus clippy with `-D warnings` (new warnings only).
  - Pull requests: linux x64/arm64 and windows x64, each with the default and simdutf V8
    archives.
  - The release run adds macOS arm64.
- The crate is lint-exempt by design (`#![allow(unused, clippy::all)]` in `src/lib.rs`).
- Cyclomatic complexity ≤ 15 is **report-only** until the post-release refactor, for
  both the Rust code and the shipped JS (`npm run complexity:js` scans `src/`).
- Native guards reachable by page-controlled values must throw, never abort. Examples:
  oversized strings raise `RangeError` (turbo-dom#183), and a terminated setup returns
  `SetupTerminated` (#184).

## Testing

```sh
export RUSTY_V8_ARCHIVE=/path/to/librusty_v8_release_<target>.a   # v150.4.0
cargo test -p turbo-dom-browser-env
cargo test -p turbo-dom-browser-env --features host-internals,test-support
```

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
| `scripts/document-legacy-chrome-control.mjs` | `document_legacy_chrome156.json` (loopback HTTP/HTTPS cases) |
| `scripts/trusted-types-control/` | Trusted Types controls (`trusted_types*_chrome155.json`); `cdp-connection.mjs` is shared with the document-legacy collector |

The `--runner` collectors drive turbo-surf's `harness/detection/chrome-cdp-trace.mjs`.
Pass its absolute path. A refreshed control changes the oracle, so commit it together
with the probe change that needed it.

Layout-dependent reads (geometry, fonts) belong to the embedder: this crate has no
layout. turbo-surf answers them with turbo-dom-layout, whose tests use fixture fonts.

## Links

[turbo-dom](../turbo-dom/README.md) · [turbo-dom-v8](../turbo-dom-v8/README.md) ·
[docs/live-css-host.md](docs/live-css-host.md) · [src/xpath_provider/README.md](src/xpath_provider/README.md) ·
[CHANGELOG](../../CHANGELOG.md)

License: MIT.

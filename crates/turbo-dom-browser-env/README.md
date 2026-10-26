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
- Private records: `interface_record` (e.g. a document's `cssHost`),
  `cascade_policy` (`StylePolicy`).
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
The 67 files are exactly what turbo-surf's Chrome-parity integration tests read,
including the custom-element lifecycle contract (`tests/custom_elements.rs`:
`CUSTOM_ELEMENTS{,_EXTENDED,_REACTIONS,_IDL,_MOVES,_CALLBACKS,_RETIRED}_*`,
`DOM_PARSER_OBSERVER_LIFECYCLE_*`, `CSS_ADOPTION_LIFECYCLE_*` probe/control pairs).
Add a constant (and its `FILES` entry) when another consumer needs a file.
`tests/test_support_api.rs` checks every constant against its file and every
probe/control pair's recorded `probeSha256`.

### Feature `native-sdk-frame-read` (off by default)

How setter/definer/deleter interceptors learn whether the caller must throw
(strict `[[Set]]`/`[[Delete]]`, `Object.defineProperty`). With the feature the
binding reads V8's frame; without it, it calls the `turbo-dom-v8` fork's
`PropertyCallbackArguments::should_throw_on_error` and a compile-time
`assert!(v8::FORK_SHOULD_THROW_FIX)` rejects a V8 without the fix. The crate
links the `turbo-dom-v8` fork, so the accessor is the default; only native V8
SDKs that lack the fork fix (turbo-surf's engine-154/155 overlay) enable the
frame read.

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

//! turbo-dom-browser-env: the native browser environment over turbo-dom.
//!
//! [`browser_env`] binds turbo-dom's pure-Rust DOM (`turbo_dom::rtdom::Tree`) to
//! V8 through native callbacks: `window`, `document`, nodes, CSSOM, forms,
//! events, ranges, custom elements and the rest of the browser surface, with no
//! JavaScript DOM implementation. One native document lives per V8 context.
//!
//! - Test runners (turbo-test) call [`browser_env::install`] per test file,
//!   [`browser_env::enabled`] and [`browser_env::reset`] between files.
//! - Page renderers (turbo-surf) seed fetched documents
//!   ([`browser_env::install_html`], `install_independent_*`), serialize them
//!   back ([`browser_env::document_html`]), and use the doc-hidden
//!   `host_internals` module (feature `host-internals`) for native layout.
//! - [`css_snapshot`] is the JSON contract of the private CSS export.
//! - [`css_values`] re-exports turbo-dom's shared `zoom` grammar.
//! - `test_support` (feature `test-support`) exposes the probe/control data of
//!   the binding's Chrome-parity tests as constants.
//!
//! See the crate README for the feature matrix.

// The binding moved here from turbo-test, which compiled it without lint gates,
// and turbo-surf's copy carried the same two allows on its adapter. Keeping them
// crate-wide makes the move behaviour- and diff-neutral; tightening lints is a
// follow-up that should not ride along with the move. Page-facing callbacks
// keep deliberately unused parameters and test-only helpers, so `unused` stays
// allowed; clippy's default groups are allowed for the same reason.
#![allow(unused, clippy::all)]

pub mod browser_env;
pub mod css_snapshot;

pub use turbo_dom_parser::rtdom::css_values;

#[cfg(feature = "host-internals")]
#[doc(hidden)]
pub use browser_env::host_internals;
#[cfg(feature = "test-support")]
pub use browser_env::test_support;

//! Crate-owned public surface of `turbo-dom-browser-env`, declared as a child of
//! [`browser_env`](super) so it can use the binding's private items without
//! making them public. Everything here is re-exported from `browser_env`.
use super::*;

/// Install an iframe srcdoc document, which the HTML parser never puts in quirks
/// mode, as an independent native document in the current context.
pub fn install_independent_srcdoc_units(
    scope: &mut v8::PinScope,
    units: &[u16],
    source_positions: bool,
) {
    install_independent_tree(
        scope,
        if source_positions {
            Tree::parse_srcdoc_utf16_with_source_positions(units)
        } else {
            Tree::parse_srcdoc_utf16(units)
        },
    );
}

/// Seed a fetched document: [`install_with_html`] under the name embedders
/// (turbo-surf) use for page loads.
pub fn install_html(scope: &mut v8::PinScope, html: &str) {
    install_with_html(scope, html);
}

/// Serialize the current document's live tree back to HTML (the inner HTML of
/// the document node). Empty if no document is installed on this thread.
pub fn document_html() -> String {
    DOM.with(|d| {
        d.borrow().as_ref().map(|s| {
            let root = s.tree.root();
            turbo_dom_parser::rtdom::serialize::serialize_inner(&s.tree, root)
        })
    })
    .unwrap_or_default()
}

#[cfg(feature = "host-internals")]
#[doc(hidden)]
#[path = "host_internals.rs"]
pub mod host_internals;

#[cfg(feature = "test-support")]
#[path = "test_support.rs"]
pub mod test_support;

// Coverage-phase Chrome 156 control tests (crate-owned, not part of the move).
#[cfg(test)]
#[path = "close_watchers_tests.rs"]
mod close_watchers_tests;
#[cfg(test)]
#[path = "edge_probes_tests.rs"]
mod edge_probes_tests;
#[cfg(test)]
#[path = "harness_tests.rs"]
pub(super) mod harness_tests;
#[cfg(test)]
#[path = "host_api_tests.rs"]
mod host_api_tests;
#[cfg(test)]
#[path = "native_guards_tests.rs"]
mod native_guards_tests;
#[cfg(test)]
#[path = "termination_tests.rs"]
mod termination_tests;

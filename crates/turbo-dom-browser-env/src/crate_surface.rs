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
#[path = "binding_termination_tests.rs"]
mod binding_termination_tests;
#[cfg(test)]
#[path = "close_watchers_tests.rs"]
mod close_watchers_tests;
#[cfg(test)]
#[path = "edge_probes_tests.rs"]
mod edge_probes_tests;
#[cfg(test)]
#[path = "guard_edges_tests.rs"]
pub(super) mod guard_edges_tests;
#[cfg(test)]
#[path = "harness_tests.rs"]
pub(super) mod harness_tests;
#[cfg(test)]
#[path = "host_api_tests.rs"]
mod host_api_tests;
#[cfg(test)]
#[path = "host_edges_tests.rs"]
mod host_edges_tests;
#[cfg(test)]
#[path = "member_edges_tests.rs"]
pub(super) mod member_edges_tests;
#[cfg(test)]
#[path = "native_guards_tests.rs"]
mod native_guards_tests;
#[cfg(test)]
#[path = "private_natives_tests.rs"]
mod private_natives_tests;
#[cfg(test)]
#[path = "reflection_regressions_tests.rs"]
mod reflection_regressions_tests;
#[cfg(test)]
#[path = "termination_tests.rs"]
mod termination_tests;
#[cfg(test)]
#[path = "trusted_controls_tests.rs"]
mod trusted_controls_tests;

#[cfg(test)]
#[path = "engine_namespace_tests.rs"]
mod engine_namespace_tests;

/// The parameter name under which [`run_engine_script`] hands an engine script the
/// realm's engine namespace.
pub const ENGINE_NAMESPACE_PARAM: &str = "__tsNs";

/// The realm's engine namespace (the binding's and its embedder's internal state:
/// native factories, captured intrinsics, host hooks), created on first use.
///
/// It lives under a V8 private symbol on the global, which page script cannot name
/// (miaskiewicz/turbo-dom#222): no registered `Symbol.for` key, no property any page
/// object reflects. Only Rust (this function) and the scripts run through
/// [`run_engine_script`] reach it.
pub fn engine_namespace<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Object>> {
    let global = scope.get_current_context().global(scope);
    engine_namespace_of(scope, global)
}

/// [`engine_namespace`] of the realm whose global is `global` (created on first use). A
/// private engine context can [`share_engine_namespace`] another realm's.
pub(super) fn engine_namespace_of<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    global: v8::Local<v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let name = v8::String::new(scope, "turbo-dom.engine-namespace")?;
    let key = v8::Private::for_api(scope, Some(name));
    if let Some(namespace) = global
        .get_private(scope, key)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    {
        return Some(namespace);
    }
    let namespace = v8::Object::new(scope);
    global.set_private(scope, key, namespace.into())?;
    Some(namespace)
}

/// Make `global`'s realm see `namespace` as its engine namespace (an engine-private
/// context that serves another realm, such as the XPath provider).
pub(super) fn share_engine_namespace(
    scope: &mut v8::PinScope,
    global: v8::Local<v8::Object>,
    namespace: v8::Local<v8::Object>,
) {
    let name = v8::String::new(scope, "turbo-dom.engine-namespace").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    global.set_private(scope, key, namespace.into());
}

/// Compile `source` as the body of a sloppy function whose one parameter,
/// [`ENGINE_NAMESPACE_PARAM`], is the realm's [`engine_namespace`], and call it with the
/// global as receiver. Returns the body's `return` value; `None` when the source does not
/// compile or throws (the exception stays on the caller's `TryCatch`).
///
/// The source's first line keeps its line number (the wrapper shares it), so stack
/// positions past line one are unchanged.
pub fn run_engine_script<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let namespace = engine_namespace(scope)?;
    let wrapped = format!("(function({ENGINE_NAMESPACE_PARAM}){{{source}\n}})");
    let code = v8::String::new(scope, &wrapped)?;
    let function = v8::Script::compile(scope, code, None)?.run(scope)?;
    let function = v8::Local::<v8::Function>::try_from(function).ok()?;
    let receiver = scope.get_current_context().global(scope);
    function.call(scope, receiver.into(), &[namespace.into()])
}

/// [`run_engine_script`] for a source that is one expression (an engine module's
/// initializer function): returns its value.
pub(super) fn engine_expression<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let expression = source.trim_end().trim_end_matches(';');
    run_engine_script(scope, &format!("return(\n{expression}\n)"))
}

#[cfg(test)]
thread_local! {
    /// Test hook: the next install publishes the engine namespace as `globalThis.__testNs`
    /// so a test's page script can drive engine internals. Cleared by that install.
    pub(crate) static EXPOSE_TEST_NAMESPACE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn expose_test_namespace(scope: &mut v8::PinScope) {
    if !EXPOSE_TEST_NAMESPACE.with(|expose| expose.replace(false)) {
        return;
    }
    let namespace = engine_namespace(scope).unwrap();
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "__testNs").unwrap();
    global.set(scope, key.into(), namespace.into());
}

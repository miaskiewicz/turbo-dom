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
#[path = "stale_nodes_tests.rs"]
mod stale_nodes_tests;
#[cfg(test)]
#[path = "termination_tests.rs"]
mod termination_tests;
#[cfg(test)]
#[path = "trusted_controls_tests.rs"]
mod trusted_controls_tests;
#[cfg(test)]
#[path = "xpath_contexts_tests.rs"]
mod xpath_contexts_tests;

#[path = "startup_snapshot.rs"]
pub(crate) mod startup_snapshot;

#[cfg(test)]
#[path = "install_split_tests.rs"]
mod install_split_tests;

#[cfg(test)]
#[path = "startup_snapshot_tests.rs"]
mod startup_snapshot_tests;

#[cfg(test)]
#[path = "context_rehydrate_tests.rs"]
mod context_rehydrate_tests;

#[cfg(test)]
#[path = "snapshot_rehydrate_tests.rs"]
mod snapshot_rehydrate_tests;

#[cfg(test)]
#[path = "engine_namespace_tests.rs"]
mod engine_namespace_tests;

#[cfg(test)]
#[path = "worker_context_tests.rs"]
mod worker_context_tests;

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
///
/// Embedder-settable hooks on the namespace (missing ⇒ the binding's fallback):
///
/// - `layoutUsedSize(element)`: `[borderBoxWidth, borderBoxHeight, contentBoxWidth,
///   contentBoxHeight]` in unzoomed CSS px for a rendered box, else `null`. Optional
///   fifth entry `fontSizePx` enables `em` resolution in `getComputedStyle(el).transform`
///   (miaskiewicz/turbo-dom#353): without it, `em` in a transform resolves against the
///   element's own declared `font-size`, so a `translateX(2em)` on an element that
///   inherits its font-size falls back to the initial 16 px (turbo-dom#373).
///   `getComputedStyle`'s `width` / `height` then read the used value of a rendered box
///   like Chrome 156; percentage translates in `transform` resolve against the border box
///   (turbo-dom#316). Without a `layoutUsedSize` a rendered box reports its declared size
///   (or `""`), and a transform list with a percentage keeps its declared text.
/// - `cssSupports(text)` / `cssSupportsProperty(property, value)`: the cascade's
///   declaration grammar, for an embedder's `CSS.supports()`. String coercion of the
///   arguments goes through a private `String` primordial captured at install
///   (miaskiewicz/turbo-dom#322, #370).
/// - `resolveTransform(text, element, fontSizePx)` (internal, set by the binding's
///   engine scripts, not an embedder knob): the pre-resolved `matrix()` /
///   `matrix3d()` / `none` serialization of a `transform` declaration under an element's
///   border box and font size, called from `getComputedStyle` so `transform` reports the
///   resolved value Chrome 156 reports (turbo-dom#353). Returns `null` to keep the
///   declared text.
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
///
/// Embedder sources can differ on every call, so this entry point never touches the
/// process-wide bytecode cache; only the binding's own fixed engine scripts (the install
/// path, [`run_engine_script_cached`]) are cached.
pub fn run_engine_script<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    run_engine_script_with(scope, source, false)
}

/// [`run_engine_script`] for one of the binding's fixed engine scripts: compiled from (and
/// stored to) the process-wide bytecode cache. The set of such sources is fixed by this crate,
/// so the cache stays bounded.
pub(crate) fn run_engine_script_cached<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    run_engine_script_with(scope, source, true)
}

fn run_engine_script_with<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
    cached: bool,
) -> Option<v8::Local<'s, v8::Value>> {
    #[cfg(test)]
    ENGINE_SCRIPTS_RUN.with(|n| n.set(n.get() + 1));
    startup_snapshot::note_engine_source(source);
    let namespace = engine_namespace(scope)?;
    let wrapped = format!("(function({ENGINE_NAMESPACE_PARAM}){{{source}\n}})");
    let code = v8::String::new(scope, &wrapped)?;
    let (script, produce) = if cached {
        compile_engine_script(scope, code, &wrapped)?
    } else {
        (v8::Script::compile(scope, code, None)?, None)
    };
    let function = script.run(scope)?;
    let function = v8::Local::<v8::Function>::try_from(function).ok()?;
    let receiver = scope.get_current_context().global(scope);
    let result = function.call(scope, receiver.into(), &[namespace.into()]);
    // Produced after the call, the cache also holds the functions the install compiled lazily.
    // V8 cannot produce a code cache while it builds a startup snapshot (turbo-dom#249).
    if let Some(key) = produce.filter(|_| !startup_snapshot::building()) {
        engine_code_cache::store(key, &script.get_unbound_script(scope));
    }
    result
}

/// Compile an engine script, consuming this process's bytecode cache for the same source.
/// Every page (each test file's window) runs the same engine scripts; without the cache
/// each install re-parses and re-compiles them from source. Returns the script and, when
/// the cache missed or V8 rejected it, the key to store a fresh cache under.
fn compile_engine_script<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    code: v8::Local<'s, v8::String>,
    wrapped: &str,
) -> Option<(v8::Local<'s, v8::Script>, Option<u64>)> {
    use v8::script_compiler as sc;
    let key = engine_code_cache::key(wrapped);
    if let Some(bytes) = engine_code_cache::get(key) {
        let mut source = sc::Source::new_with_cached_data(code, None, v8::CachedData::new(&bytes));
        let script = sc::compile(
            scope,
            &mut source,
            sc::CompileOptions::ConsumeCodeCache,
            sc::NoCacheReason::NoReason,
        )?;
        let rejected = source.get_cached_data().is_none_or(|data| data.rejected());
        #[cfg(test)]
        engine_code_cache::CONSUMED.with(|consumed| consumed.borrow_mut().push(!rejected));
        return Some((script, rejected.then_some(key)));
    }
    Some((v8::Script::compile(scope, code, None)?, Some(key)))
}

/// The process-wide bytecode cache of engine scripts, keyed by a hash of the wrapped source.
/// V8 validates cached data (version, flags, source hash) and rejects a mismatch, so a stale
/// or colliding entry only costs a normal compile.
pub(crate) mod engine_code_cache {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};

    fn entries() -> &'static Mutex<HashMap<u64, Arc<Vec<u8>>>> {
        static ENTRIES: OnceLock<Mutex<HashMap<u64, Arc<Vec<u8>>>>> = OnceLock::new();
        ENTRIES.get_or_init(Default::default)
    }

    pub(crate) fn key(source: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }

    pub(crate) fn get(key: u64) -> Option<Arc<Vec<u8>>> {
        entries().lock().unwrap().get(&key).cloned()
    }

    pub(crate) fn store(key: u64, script: &v8::Local<v8::UnboundScript>) {
        if let Some(data) = script.create_code_cache() {
            entries()
                .lock()
                .unwrap()
                .insert(key, Arc::new(data.to_vec()));
        }
    }

    #[cfg(test)]
    thread_local! {
        /// Test hook: each cache consume on this thread, `true` when V8 accepted the data.
        pub(crate) static CONSUMED: std::cell::RefCell<Vec<bool>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    #[cfg(test)]
    pub(crate) fn insert(key: u64, data: Vec<u8>) {
        entries().lock().unwrap().insert(key, Arc::new(data));
    }
}

/// [`run_engine_script`] for a source that is one expression (an engine module's
/// initializer function): returns its value.
pub(super) fn engine_expression<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let expression = source.trim_end().trim_end_matches(';');
    run_engine_script_cached(scope, &format!("return(\n{expression}\n)"))
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

/// Replace the contents of a live HTML document without replacing its Window,
/// native document identity, host policies, or existing document listeners.
///
/// This is the host handoff for `setContent`: parsing and DOM replacement finish
/// synchronously; no parser script runs here. Custom-element constructors and
/// reactions run synchronously after the new content is attached and may mutate
/// that content. The returned nodes are in parser
/// order, with fresh parser-inserted flags. Feed them through [`prepare_script`]
/// and the embedder's existing CSP/module/resource scheduler. The embedder owns
/// ready-state transitions and DOMContentLoaded/load completion, as on initial
/// install. This does not implement the guest document.open/write/close stream.
/// Detached old nodes remain usable. The document's URL and compatibility mode
/// remain unchanged. Only live HTML browsing-context documents are accepted.
pub fn replace_document_content<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    units: &[u16],
) -> Result<Vec<v8::Local<'s, v8::Object>>, &'static str> {
    let owner = DomScope::for_object(scope, document);
    let root = handle_of(scope, document).ok_or("Illegal invocation")?;
    let valid = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|state| state.tree.node_type_id(root) == 9 && state.browsing_context)
    });
    if !valid || dom_implementation::is_xml_document(scope, document) {
        return Err("A browsing-context document is required");
    }
    let parsed = parse_html_document_units(scope, units, false);
    let children = {
        let _parsed = DomScope::for_object(scope, parsed);
        let parsed_root = handle_of(scope, parsed).ok_or("Document tree unavailable")?;
        with_tree(|tree| tree.children(parsed_root))
            .unwrap_or_default()
            .into_iter()
            .map(|node| wrap(scope, node))
            .collect::<Vec<_>>()
    };
    let mut incoming = Vec::with_capacity(children.len());
    for child in children {
        incoming.push(dom_mutations::Incoming::Node(
            import_document_node_with_construction(scope, document, child, true, false)?,
        ));
    }
    let remove = with_tree(|tree| tree.children(root)).unwrap_or_default();
    if !dom_mutations::insert_inputs(scope, root, &incoming, None, &remove) {
        return Err("Document replacement failed");
    }
    let scripts = DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let state = dom.as_mut().unwrap();
        let scripts = script_state::parse_order(&state.tree);
        state.scripts.extend(script_state::initial(&state.tree));
        state.parser_scripts = scripts.clone();
        scripts
    });
    let attrs = with_tree(|tree| {
        tree.query_selector_all("*")
            .iter()
            .copied()
            .flat_map(|node| {
                tree.attributes(node)
                    .into_iter()
                    .filter(|(name, _)| name.starts_with("on"))
                    .map(move |(name, _)| (node, name.to_owned()))
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    for (node, name) in attrs {
        event_handlers::record_set_phase(scope, node, &name);
    }
    Ok(scripts.into_iter().map(|node| wrap(scope, node)).collect())
}

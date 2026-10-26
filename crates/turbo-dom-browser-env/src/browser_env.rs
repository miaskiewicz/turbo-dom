//! Browser environment shim (P3) — the V8 BINDING + browser globals over turbo-dom's pure-Rust `rtdom` (the `turbo-dom` crate,
//! imported as `turbo_dom_parser`) to V8 via native rusty-v8 callbacks, so tests get `window`/`document`/Element with
//! NO JS DOM (no `installGlobals`, no `.node` parser, no node_modules turbo-dom).
//!
//! Model: the DOM lives in Rust as one `Tree` per context (handle-based — each node is a `u32`
//! `Handle`). A JS DOM node is a V8 object with the handle in internal field 0; methods/accessors
//! are native callbacks that read the handle, call `Tree`, and wrap result handles back into node
//! objects. A `handle → V8 object` cache preserves identity (`el === el`).
//!
//! This is the P3.2 vertical slice: document/createElement/createTextNode, appendChild/insertBefore/
//! removeChild, get/setAttribute, id/className/tagName/textContent, querySelector(All)/getElementById.
//! Events, cssom/style, classList, forms etc. grew in P3.3 and later. Always on (`enabled()`).

#[path = "animations.rs"]
mod animations;
#[path = "css_selector_syntax.rs"]
mod css_selector_syntax;
#[path = "cssom_parser.rs"]
mod cssom_parser;
#[path = "dom_isolation.rs"]
mod dom_isolation;
#[path = "dom_ownership.rs"]
mod dom_ownership;
#[path = "element_internals.rs"]
mod element_internals;
#[path = "element_members.rs"]
mod element_members;
pub use dom_isolation::{activate_isolate, reset_isolate, NativeDomScope};
#[path = "storage_ownership.rs"]
mod storage_ownership;
pub use storage_ownership::{collect_native_storage, NativeNodePin, NativeStorageReport};

#[path = "collection_records.rs"]
mod collection_records;
#[path = "control_labels.rs"]
mod control_labels;
#[path = "css_policy.rs"]
mod css_policy;
#[path = "custom_elements.rs"]
mod custom_elements;
#[path = "event_brands.rs"]
mod event_brands;
#[path = "event_handlers.rs"]
mod event_handlers;
#[path = "event_listeners.rs"]
mod event_listeners;
#[path = "form_checked_state.rs"]
mod form_checked_state;
#[path = "script_state.rs"]
mod script_state;
#[path = "trusted_types.rs"]
mod trusted_types;
pub use script_state::{
    parser_scripts, prepare_script, script_document, script_state, set_document_resource_active,
    PreparedScript, ScriptState,
};

#[path = "mutation_observers.rs"]
mod mutation_observers;
pub use event_brands::{
    is_native_abort_object, is_native_event, is_native_event_target, native_event_record,
    native_event_target_has_listener, register_native_event_target,
    set_native_event_target_observer, NativeEventTargetObserver,
};
pub use event_listeners::capture_window_event_interfaces;
pub use mutation_observers::{install_mutation_consumer, remove_mutation_consumer};

/// Capture the trusted host timer after the runtime has installed its scheduler.
/// Guest code cannot install or replace this private lifecycle capability.
pub fn capture_popover_scheduler(scope: &mut v8::PinScope) {
    html_popover::capture_scheduler(scope);
}
/// Capture the host's rendering seam for view transitions: `host("frame", step)` runs
/// `step(timestamp)` after the next frame's animation frame callbacks, `host("suppress",
/// flag)` pauses or resumes rendering updates, and `host("hidden")` reports page visibility.
pub fn capture_view_transition_host(scope: &mut v8::PinScope, host: v8::Local<v8::Function>) {
    view_transitions::capture_host(scope, host);
}

/// Capture the embedder's `File` interface once the runtime has installed it, before
/// page script: FileList selections, file form entries and DataTransfer use it.
pub fn capture_file_host(scope: &mut v8::PinScope) {
    file_inputs::capture_host(scope);
}

pub fn clear_context_capabilities(scope: &mut v8::PinScope) {
    let _isolate = activate_isolate(scope);
    let contexts = DOM_CONTEXTS
        .try_with(|contexts| contexts.borrow().values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    for context in contexts {
        if let Some(context) = context.to_local(scope) {
            context.remove_slot::<EventRecordAccessor>();
            context.remove_slot::<HostEventDispatcher>();
            event_listeners::clear_context(context);
            range_contents::clear_context(context);
        }
    }
}
pub fn clear_interface_records(isolate: &mut v8::Isolate) {
    let _native = activate_isolate(isolate);
    storage_ownership::clear(isolate);
    collection_records::clear(isolate);
    range_live::clear(isolate);
    mutation_observers::clear(isolate);
}

/// Access a host-created interface record in the isolate's private ephemeron registry.
pub fn get_interface_record<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Value>> {
    collection_records::get(scope, object)
}
pub fn set_interface_record<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    record: v8::Local<'s, v8::Value>,
) -> bool {
    collection_records::set(scope, object, record)
}

#[path = "document_basics.rs"]
mod document_basics;
#[path = "document_legacy.rs"]
mod document_legacy;
#[path = "interceptor_mode.rs"]
mod interceptor_mode;
pub use document_legacy::{
    set_document_domain_sandboxed, set_document_last_modified, set_document_origin_keyed,
};
#[path = "dom_implementation.rs"]
mod dom_implementation;
pub use dom_implementation::set_initializer as set_inert_document_initializer;
#[path = "document_xpath.rs"]
mod document_xpath;
pub use document_xpath::is_native_xpath_object;
#[path = "dom_mutations.rs"]
mod dom_mutations;
#[path = "editing.rs"]
mod editing;
#[path = "element_relations.rs"]
mod element_relations;
#[path = "file_inputs.rs"]
mod file_inputs;
#[path = "focus_group.rs"]
mod focus_group;
pub use editing::editing_user_input;
#[path = "html_dialog.rs"]
mod html_dialog;
#[path = "html_invokers.rs"]
mod html_invokers;
#[path = "html_popover.rs"]
mod html_popover;
#[path = "pointer_events.rs"]
mod pointer_events;
#[path = "typed_om.rs"]
mod typed_om;
pub use pointer_events::{host_pointer, PointerLockPolicy};
#[path = "form_pseudo.rs"]
mod form_pseudo;
#[path = "html_text.rs"]
mod html_text;
#[path = "hyperlink_utils.rs"]
mod hyperlink_utils;
#[path = "input_members.rs"]
mod input_members;
#[path = "interest_invokers.rs"]
mod interest_invokers;
#[path = "key_order.rs"]
mod key_order;
#[path = "reference_target.rs"]
mod reference_target;
#[path = "select_element.rs"]
mod select_element;
#[path = "shadow_html.rs"]
mod shadow_html;
pub use key_order::{defer as defer_key_order, finish as finish_key_order};
#[path = "shadow_slots.rs"]
mod shadow_slots;
#[path = "user_activation_gates.rs"]
mod user_activation_gates;
pub use user_activation_gates::{
    light_dismiss, modal_backdrop, notify_user_activation, process_close_watchers,
    set_activation_host, ActivationHost, ActivationQuery,
};
#[path = "view_transitions.rs"]
mod view_transitions;

/// Host-only composed identities for native layout. This seam does not expose
/// closed roots or private assignment records to guest JavaScript.
pub fn host_composed_children<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
) -> Option<Vec<v8::Local<'s, v8::Object>>> {
    shadow_slots::composed_children(scope, node)
}
pub fn host_composed_parent<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    shadow_slots::composed_parent(scope, node)
}

pub fn node_nonce(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) -> Option<String> {
    document_basics::nonce(scope, node)
}
pub fn hide_node_nonce(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) {
    let Some(value) = document_basics::nonce_units(scope, node) else {
        return;
    };
    let _dom = DomScope::for_object(scope, node);
    let Some(handle) = handle_of(scope, node) else {
        return;
    };
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .nonces
            .insert(handle, value);
    });
    with_tree_mut(|tree| {
        if tree.get_attribute(handle, "nonce").is_some() {
            tree.set_attribute(handle, "nonce", "");
        }
    });
}
fn initial_nonces(tree: &Tree) -> HashMap<Handle, Vec<u16>> {
    tree.query_selector_all("[nonce]")
        .iter()
        .copied()
        .filter_map(|handle| {
            tree.get_attribute_utf16(handle, "nonce")
                .map(|value| (handle, value))
        })
        .collect()
}
pub fn update_node_nonce(scope: &mut v8::PinScope, node: v8::Local<v8::Object>, value: &str) {
    let _dom = DomScope::for_object(scope, node);
    if let Some(handle) = handle_of(scope, node) {
        DOM.with(|dom| {
            if let Some(state) = dom.borrow_mut().as_mut() {
                state.nonces.insert(handle, value.encode_utf16().collect());
            }
        });
    }
}
pub fn update_node_nonce_units(
    scope: &mut v8::PinScope,
    node: v8::Local<v8::Object>,
    value: &[u16],
) {
    let _dom = DomScope::for_object(scope, node);
    if let Some(handle) = handle_of(scope, node) {
        DOM.with(|dom| {
            if let Some(state) = dom.borrow_mut().as_mut() {
                state.nonces.insert(handle, value.to_vec());
            }
        })
    }
}

#[path = "html_documents.rs"]
mod html_documents;
#[path = "sanitizer.rs"]
mod sanitizer;

use std::cell::RefCell;
use std::collections::HashMap;

use turbo_dom_parser::rtdom::tree::{AttributeDataUtf16, Handle, Tree};
use turbo_dom_parser::rtdom::NodeRef;

/// The all-Rust DOM (rtdom) is now the ONLY DOM environment — the JS turbo-dom bootstrap
/// (installGlobals + the `.node` parser) was removed in v0.3.0. Kept as a function (always true) so
/// the per-file install/reset call sites read intent; `TURBO_RUST_DOM` is no longer consulted.
pub fn enabled() -> bool {
    true
}

/// Whether to RECORD missing-member accesses (debug surface map). The interceptor itself is always
/// installed (for graceful degradation — an unimplemented member returns undefined instead of
/// crashing the file); recording is the only extra cost and is gated here so production pays nothing
/// but a cached bool check + a small fall-through. Cached once (no per-access getenv).
pub fn log_enabled() -> bool {
    use std::sync::OnceLock;
    static LOG: OnceLock<bool> = OnceLock::new();
    *LOG.get_or_init(|| {
        std::env::var("TURBO_RUST_DOM_LOG")
            .map(|v| !v.is_empty() && v != "0")
            .unwrap_or(false)
    })
}

/// Names the native DOM binds (template methods/accessors + document own methods) + JS internals —
/// the interceptor falls through for these and only records the REST as "missing".
const KNOWN: &[&str] = &[
    // element methods + accessors
    "appendChild",
    "removeChild",
    "insertBefore",
    "replaceChild",
    "setAttribute",
    "getAttribute",
    "hasAttribute",
    "removeAttribute",
    "querySelector",
    "querySelectorAll",
    "tagName",
    "parentNode",
    "firstChild",
    "nextSibling",
    "textContent",
    "id",
    "className",
    "style",
    "oninput",
    "onchange",
    "onclick",
    "nodeType",
    "nodeName",
    "childNodes",
    "ownerDocument",
    "addEventListener",
    "removeEventListener",
    "dispatchEvent",
    "matches",
    "contains",
    "innerHTML",
    "outerHTML",
    "children",
    "parentElement",
    "firstElementChild",
    "namespaceURI",
    "value",
    "append",
    "prepend",
    "remove",
    "focus",
    "blur",
    "click",
    "scrollIntoView",
    "getBoundingClientRect",
    "createElementNS",
    "createDocumentFragment",
    "createComment",
    "cloneNode",
    "isConnected",
    "attributes",
    "dataset",
    "createRange",
    "getRootNode",
    "getSelection",
    "closest",
    "data",
    "nodeValue",
    "constructor",
    "documentMode",
    "activeElement",
    "focus",
    "blur",
    "selectionStart",
    "selectionEnd",
    "setSelectionRange",
    "getAttributeNode",
    "TEXT_NODE",
    "ELEMENT_NODE",
    "COMMENT_NODE",
    "DOCUMENT_NODE",
    "DOCUMENT_FRAGMENT_NODE",
    // document own methods/props
    "createElement",
    "createTextNode",
    "getElementById",
    "body",
    "documentElement",
    // JS internals V8 / libs probe constantly — never DOM
    "then",
    "catch",
    "finally",
    "constructor",
    "prototype",
    "toString",
    "valueOf",
    "toJSON",
    "length",
    "name",
    "call",
    "apply",
    "bind",
    "hasOwnProperty",
    "nodeName",
];

thread_local! {
    /// debug logger: unknown property name → access count.
    static MISSING: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

/// Record unresolved property reads while preserving ordinary missing-property semantics.
fn missing_getter(
    scope: &mut v8::PinScope,
    name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) -> v8::Intercepted {
    let _dom = DomScope::for_object(scope, args.holder());
    // V8 inline-cache hazard with NON_MASKING interceptors: once a load site reads a property while
    // it is ABSENT (we intercept → undefined), V8 may cache "this site goes to the interceptor" and
    // keep routing there EVEN AFTER the property is later added as a real own property — masking it
    // (reads return undefined though getOwnPropertyNames/hasOwnProperty see it). React stores
    // `node.__reactFiber$…`/`__reactProps$…` exactly this way: it reads the key (miss → IC poisoned)
    // before precaching the fiber, then later reads it again and gets undefined → it walks up to the
    // wrong ancestor, so delegated onClick/onChange never fire (only after a prior render primed the
    // IC — a brutal Heisenbug). Guard: when the interceptor fires, first look for a REAL property
    // (interceptors bypassed). If one exists, return IT — so a poisoned IC still yields the truth.
    let this = args.holder();
    if let Some(real) = this.get_real_named_property(scope, name) {
        rv.set(real);
        return v8::Intercepted::kYes;
    }
    let key = name.to_rust_string_lossy(scope);
    // symbols (Symbol.toPrimitive, Symbol.iterator, …) + known names → fall through.
    if key.starts_with("Symbol(") || KNOWN.contains(&key.as_str()) {
        return v8::Intercepted::kNo;
    }
    if log_enabled() {
        MISSING.with(|m| *m.borrow_mut().entry(key).or_insert(0) += 1);
    }
    // Reporting an intercepted undefined makes V8's `in` operator claim the
    // absent property exists. Normal lookup already yields undefined on a miss.
    v8::Intercepted::kNo
}

/// Passthrough setter interceptor: NEVER intercept (kNo) → JS property sets create normal own
/// properties. Required so `document.x = …` shims AND React's expando writes on DOM nodes
/// (`node.__reactProps$…`, `_reactListening…`) persist instead of being swallowed by the
/// getter-only interceptor.
fn passthrough_setter<T>(
    _scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    _value: v8::Local<v8::Value>,
    _args: v8::PropertyCallbackArguments,
    _rv: v8::ReturnValue<T>,
) -> v8::Intercepted {
    v8::Intercepted::kNo
}

/// Print + clear the accumulated missing-access log (call between files when debugging).
pub fn dump_missing() {
    let mut items: Vec<(String, u64)> = MISSING
        .try_with(|m| m.borrow_mut().drain().collect())
        .unwrap_or_default();
    if items.is_empty() {
        return;
    }
    items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    eprintln!(
        "[rust-dom] {} missing DOM members (name×accesses):",
        items.len()
    );
    for (name, n) in &items {
        eprintln!("[rust-dom]   {name} ×{n}");
    }
}

#[path = "template_contents.rs"]
mod template_contents;

struct DomState {
    tree: Tree,
    /// handle → JS node object, for identity (`el === el`).
    cache: HashMap<Handle, v8::Weak<v8::Object>>,
    checked_radios: HashMap<Handle, Option<Handle>>,
    dialog: html_dialog::State,
    /// shared element object template (internal_field_count = 2).
    el_template: v8::Global<v8::ObjectTemplate>,
    shadow_modes: HashMap<Handle, String>,
    nonces: HashMap<Handle, Vec<u16>>,
    scripts: HashMap<Handle, ScriptState>,
    /// Main-document parser scripts in parse order (see script_state::parse_order).
    parser_scripts: Vec<Handle>,
    resource_active: bool,
    basics_state: Option<v8::Weak<v8::Object>>,
    control_clone: Option<v8::Weak<v8::Function>>,
    css_policy: css_policy::PolicyState,
    event_handlers: event_handlers::State,
    popover: html_popover::State,
    pointer: pointer_events::State,
    activation: user_activation_gates::State,
    view_transitions: view_transitions::State,
    input_members: input_members::State,
    browsing_context: bool,
    ownership: Option<dom_ownership::Graph>,
    ownership_dirty: bool,
    template_contents: template_contents::State,
    custom_elements_active: bool,
    form_submission_blocked: bool,
    form_submission_handler: Option<v8::Weak<v8::Function>>,
}

thread_local! {
    static DOM: RefCell<Option<DomState>> = const { RefCell::new(None) };
}

// Native callbacks may enter through a node from another same-origin frame.
// Keep each context's tree, wrappers, listeners and focus together, and restore
// the caller's DOM on return (including reentrant event dispatch).
#[derive(Clone, Copy)]
struct DomContextId(u64);
struct FormSubmissionDispatcher(v8::Weak<v8::Function>);
struct FormValueReader(v8::Weak<v8::Function>);
struct FormEntryListReader(v8::Weak<v8::Function>);
struct FormDataEventDispatcher(v8::Weak<v8::Function>);

#[derive(Default)]
struct SavedDom {
    dom: Option<DomState>,
    styles: HashMap<Handle, v8::Weak<v8::Object>>,
    listeners: HashMap<(Handle, String), Vec<event_listeners::Listener>>,
    active: Option<Handle>,
}

thread_local! {
    static DOM_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static NEXT_DOM_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
    static OTHER_DOMS: RefCell<HashMap<u64, SavedDom>> = RefCell::new(HashMap::new());
    static DOM_CONTEXTS: RefCell<HashMap<u64, v8::Weak<v8::Context>>> = RefCell::new(HashMap::new());
}

fn take_dom() -> SavedDom {
    SavedDom {
        dom: DOM.with(|d| d.borrow_mut().take()),
        styles: STYLE.with(|d| std::mem::take(&mut *d.borrow_mut())),
        listeners: LISTENERS.with(|d| std::mem::take(&mut *d.borrow_mut())),
        active: ACTIVE.with(|d| d.borrow_mut().take()),
    }
}

fn put_dom(saved: SavedDom) {
    DOM.with(|d| *d.borrow_mut() = saved.dom);
    STYLE.with(|d| *d.borrow_mut() = saved.styles);
    LISTENERS.with(|d| *d.borrow_mut() = saved.listeners);
    ACTIVE.with(|d| *d.borrow_mut() = saved.active);
}

struct DomScope(u64);
impl DomScope {
    fn enter(id: u64) -> Self {
        let previous = DOM_ID.with(|d| d.replace(id));
        if id != previous {
            let old = take_dom();
            let next = OTHER_DOMS.with(|d| {
                let mut d = d.borrow_mut();
                d.insert(previous, old);
                d.remove(&id).unwrap_or_default()
            });
            put_dom(next);
        }
        Self(previous)
    }

    fn for_object(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> Self {
        Self::enter(dom_id_of(scope, object))
    }
}
impl Drop for DomScope {
    fn drop(&mut self) {
        let current = DOM_ID.with(|d| d.get());
        if current != self.0 {
            let saved = take_dom();
            let previous = OTHER_DOMS.with(|d| {
                let mut d = d.borrow_mut();
                d.insert(current, saved);
                d.remove(&self.0).unwrap_or_default()
            });
            put_dom(previous);
            DOM_ID.with(|d| d.set(self.0));
        }
    }
}

/// Install an independent native document in the current V8 context. The caller's
/// document stays live, and callbacks route by their receiver's creation context.
pub fn install_independent_html(scope: &mut v8::PinScope, html: &str) {
    install_independent_tree(scope, Tree::parse(html));
}
pub fn install_independent_html_units(scope: &mut v8::PinScope, units: &[u16]) {
    install_independent_tree(scope, Tree::parse_utf16(units));
}
pub fn install_independent_utf16_source_positions(scope: &mut v8::PinScope, units: &[u16]) {
    install_independent_tree(scope, Tree::parse_utf16_with_source_positions(units));
}
fn install_independent_tree(scope: &mut v8::PinScope, tree: Tree) {
    dom_isolation::install(scope);
    let context = scope.get_current_context();
    let id = context
        .get_slot::<DomContextId>()
        .map(|id| id.0)
        .unwrap_or_else(|| {
            let id = NEXT_DOM_ID.with(|d| {
                let id = d.get();
                d.set(id + 1);
                id
            });
            context.set_slot(std::rc::Rc::new(DomContextId(id)));
            id
        });
    let _dom = DomScope::enter(id);
    install_with_tree(scope, tree);
}

fn dom_id_of(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> u64 {
    if let Some(value) = object
        .get_internal_field(scope, 1)
        .and_then(|field| v8::Local::<v8::Value>::try_from(field).ok())
        .and_then(|value| v8::Local::<v8::BigInt>::try_from(value).ok())
    {
        return value.u64_value().0;
    }
    object
        .get_creation_context(scope)
        .and_then(|context| context.get_slot::<DomContextId>())
        .map_or(0, |id| id.0)
}

/// Parse an inert document into its own native Tree in the caller's realm.
/// Internal node ownership distinguishes independent documents in one context.
pub fn parse_html_document<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    html: &str,
    sanitize: bool,
) -> v8::Local<'s, v8::Object> {
    parse_html_tree(scope, Tree::parse(html), sanitize)
}
/// Parse an original DOM string into native unit-backed payloads.
pub fn parse_html_document_units<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    units: &[u16],
    sanitize: bool,
) -> v8::Local<'s, v8::Object> {
    parse_html_tree(scope, Tree::parse_utf16(units), sanitize)
}
fn parse_html_tree<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    tree: Tree,
    sanitize: bool,
) -> v8::Local<'s, v8::Object> {
    let sanitizer = sanitize.then(|| (html_documents::default_config(), true));
    parse_html_tree_with(scope, tree, false, sanitizer)
}
/// Build an inert document from a parsed tree. `declarative` attaches
/// `<template shadowrootmode>` roots (`Document.parseHTML[Unsafe]`); a sanitizer
/// configuration (and whether it is safe) then filters the whole tree.
fn parse_html_tree_with<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    mut tree: Tree,
    declarative: bool,
    sanitizer: Option<(&sanitizer::Config, bool)>,
) -> v8::Local<'s, v8::Object> {
    let id = NEXT_DOM_ID.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    });
    let (control_clone, projector) = {
        let current = scope.get_current_context();
        let owner = current
            .get_slot::<DomContextId>()
            .map_or(DOM_ID.with(|id| id.get()), |id| id.0);
        let _owner = DomScope::enter(owner);
        let clone = DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .and_then(|state| state.control_clone.clone())
        });
        (clone, current_node_projector(scope))
    };
    let _dom = DomScope::enter(id);
    let root = tree.root();
    let only = sanitizer.map(|(config, safe)| sanitizer::sanitize(&mut tree, root, config, safe));
    let declared = if declarative {
        shadow_html::declarative(&mut tree, root, only.as_ref())
    } else {
        Vec::new()
    };
    let template = build_el_template(scope);
    let template = v8::Global::new(scope, template);
    DOM.with(|dom| {
        *dom.borrow_mut() = Some(DomState {
            scripts: script_state::initial(&tree),
            parser_scripts: Vec::new(),
            resource_active: false,
            nonces: initial_nonces(&tree),
            tree,
            cache: HashMap::new(),
            checked_radios: Default::default(),
            dialog: Default::default(),
            el_template: template,
            shadow_modes: HashMap::new(),
            basics_state: None,
            control_clone,
            css_policy: Default::default(),
            event_handlers: Default::default(),
            popover: Default::default(),
            pointer: Default::default(),
            activation: Default::default(),
            view_transitions: Default::default(),
            input_members: Default::default(),
            browsing_context: false,
            ownership: None,
            ownership_dirty: true,
            template_contents: Default::default(),
            custom_elements_active: false,
            form_submission_blocked: false,
            form_submission_handler: None,
        })
    });
    script_state::initialize_inert();
    let context = scope.get_current_context();
    let context = v8::Weak::new(scope, context);
    DOM_CONTEXTS.with(|contexts| {
        contexts.borrow_mut().insert(id, context);
    });
    let root = with_tree(|tree| tree.root()).unwrap();
    form_checked_state::parsed(root);
    let document = wrap(scope, root);
    if let Some(function) = projector {
        set_node_projector(scope, document, function);
        let undefined = v8::undefined(scope);
        function.call(scope, undefined.into(), &[document.into()]);
    }
    let roots = shadow_html::register(scope, declared);
    DOM.with(|dom| {
        if let Some(dom) = dom.borrow_mut().as_mut() {
            dom.template_contents.enabled = true
        }
    });
    // Innermost roots first: no raw parser content fragment may enter adoption.
    for root in roots.iter().rev() {
        template_contents::realize(scope, *root);
    }
    template_contents::realize(scope, document);
    document
}

pub fn create_document_element<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    tag: &str,
) -> Option<v8::Local<'s, v8::Object>> {
    let _dom = DomScope::for_object(scope, document);
    let root = handle_of(scope, document)?;
    if with_tree(|tree| tree.node_type_id(root)) != Some(9) {
        return None;
    }
    let xml = dom_implementation::is_xml_document(scope, document);
    let namespace = dom_implementation::default_element_namespace(scope, document);
    let node = with_tree_mut(|tree| {
        if xml {
            let namespace = if namespace.is_some() {
                turbo_dom_parser::rtdom::tree::Namespace::Html
            } else {
                turbo_dom_parser::rtdom::tree::Namespace::Other
            };
            tree.create_element_ns(tag, namespace)
        } else {
            tree.create_element(tag)
        }
    })?;
    Some(wrap(scope, node))
}

/// Copy a native node between independent trees without serializing or evaluating
/// markup. Unsupported tree kinds fail before destination mutation.
pub fn import_document_node<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<v8::Object>,
    node: v8::Local<'s, v8::Object>,
    deep: bool,
) -> Result<v8::Local<'s, v8::Object>, &'static str> {
    let source_node = node;
    struct Snapshot {
        kind: u8,
        name: Vec<u16>,
        value: Vec<u16>,
        public_id: Vec<u16>,
        system_id: Vec<u16>,
        namespace: Option<Vec<u16>>,
        attributes: Vec<AttributeDataUtf16>,
        parent: Option<usize>,
    }
    fn capture(tree: &Tree, node: Handle, deep: bool) -> Result<Vec<Snapshot>, &'static str> {
        let mut records = Vec::new();
        let mut pending = vec![(node, None)];
        while let Some((node, parent)) = pending.pop() {
            let kind = tree.node_type_id(node);
            if !matches!(kind, 1 | 2 | 3 | 7 | 8 | 10 | 11) {
                return Err("Importing this node kind is unsupported");
            }
            let index = records.len();
            records.push(Snapshot {
                kind,
                name: tree
                    .qualified_name_utf16(node)
                    .or_else(|| tree.doctype_name_utf16(node))
                    .or_else(|| tree.processing_instruction_target_utf16(node))
                    .unwrap_or_default(),
                value: tree.node_value_utf16(node).unwrap_or_default(),
                public_id: tree.doctype_public_id_utf16(node).unwrap_or_default(),
                system_id: tree.doctype_system_id_utf16(node).unwrap_or_default(),
                namespace: tree.namespace_uri_utf16(node),
                attributes: tree.attributes_units(node),
                parent,
            });
            if deep {
                for child in tree.children(node).into_iter().rev() {
                    pending.push((child, Some(index)))
                }
            }
        }
        Ok(records)
    }
    fn materialize(tree: &mut Tree, records: Vec<Snapshot>) -> Handle {
        let mut nodes = Vec::with_capacity(records.len());
        for snapshot in records {
            let node = match snapshot.kind {
                2 => tree.create_attribute_units(
                    snapshot.namespace.as_deref(),
                    &snapshot.name,
                    &snapshot.value,
                ),
                3 => tree.create_text_node_utf16(&snapshot.value),
                7 => tree.create_processing_instruction_units(&snapshot.name, &snapshot.value),
                8 => tree.create_comment_utf16(&snapshot.value),
                10 => tree.create_document_type_units(
                    &snapshot.name,
                    &snapshot.public_id,
                    &snapshot.system_id,
                ),
                11 => tree.create_document_fragment(),
                _ => tree
                    .create_element_namespace_units(snapshot.namespace.as_deref(), &snapshot.name),
            };
            for attribute in snapshot.attributes {
                tree.set_attribute_ns_units(
                    node,
                    attribute.namespace.as_deref(),
                    &attribute.qualified_name,
                    &attribute.value,
                );
            }
            if let Some(parent) = snapshot.parent {
                tree.append_child(nodes[parent], node);
            }
            nodes.push(node);
        }
        nodes[0]
    }
    let snapshot = {
        let _source = DomScope::for_object(scope, node);
        let handle = handle_of(scope, node).ok_or("Illegal invocation")?;
        if with_tree(|tree| tree.is_shadow_root(handle)).unwrap_or(false) {
            return Err("Importing a shadow root is unsupported");
        }
        with_tree(|tree| capture(tree, handle, deep)).ok_or("Node tree unavailable")??
    };
    let _target = DomScope::for_object(scope, document);
    let root = handle_of(scope, document).ok_or("Illegal invocation")?;
    if with_tree(|tree| tree.node_type_id(root)) != Some(9) {
        return Err("Illegal invocation");
    }
    let node =
        with_tree_mut(|tree| materialize(tree, snapshot)).ok_or("Document tree unavailable")?;
    let clone = wrap(scope, node);
    copy_control_state(scope, source_node, clone, deep);
    document_basics::copy_nonces(scope, source_node, clone, deep);
    script_state::copy(scope, source_node, clone, deep);
    template_contents::copy_contents(scope, source_node, clone, deep);
    custom_elements::cloned(scope, source_node, clone, deep);
    shadow_html::clone_roots(scope, source_node, clone, deep);
    Ok(clone)
}

fn copy_control_state<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Object>,
    clone: v8::Local<'s, v8::Object>,
    deep: bool,
) {
    input_members::copy(scope, source, clone, deep);
    let function = {
        let _source = DomScope::for_object(scope, source);
        DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .and_then(|state| state.control_clone.clone())
        })
    };
    let Some(function) = function else {
        return;
    };
    let function = dom_ownership::local(scope, function);
    let Some(context) = function.get_creation_context(scope) else {
        return;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let deep = v8::Boolean::new(scope, deep);
    let receiver = v8::undefined(scope).into();
    function.call(scope, receiver, &[source.into(), clone.into(), deep.into()]);
}

/// Read a node object's handle from internal field 0.
fn handle_of(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>) -> Option<Handle> {
    let f = obj.get_internal_field(scope, 0)?;
    let v = v8::Local::<v8::Value>::try_from(f).ok()?;
    // Embedder globals can also have internal fields; undefined and pointers
    // must not be coerced into node handle zero.
    if !v.is_uint32() {
        return None;
    }
    v.uint32_value(scope).map(Handle::from_raw)
}

/// Read an actual native attribute without consulting guest overrides.
pub fn node_attribute(
    scope: &mut v8::PinScope,
    node: v8::Local<v8::Object>,
    name: &str,
) -> Option<String> {
    let _dom = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    with_tree(|tree| tree.get_attribute(handle, name).map(str::to_owned)).flatten()
}

/// Original native parser provenance, unavailable for synthetic lexical nodes.
pub fn node_parser_position(
    scope: &mut v8::PinScope,
    node: v8::Local<v8::Object>,
    attribute: Option<&str>,
) -> Option<turbo_dom_parser::rtdom::SourcePosition> {
    let _dom = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    with_tree(|tree| {
        attribute.map_or_else(
            || tree.source_position(handle),
            |name| tree.source_attribute_position(handle, name),
        )
    })
    .flatten()
}
/// Retire automatic custom-element construction and reactions without clearing
/// retained registries, definitions, instances or author-callable JS functions.
pub fn retire_custom_element_context(context: v8::Local<v8::Context>) {
    custom_elements::retire_context(context);
}

/// Determine actual Window ownership without reading guest document properties.
pub fn node_has_window_owner<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
) -> bool {
    let _dom = DomScope::for_object(scope, node);
    let Some(handle) = handle_of(scope, node) else {
        return false;
    };
    let Some(root) = with_tree(Tree::root) else {
        return false;
    };
    let template = with_tree(|tree| {
        let mut parent = Some(handle);
        while let Some(current) = parent {
            if tree.node_type_id(current) == 11
                && tree
                    .parent(current)
                    .is_some_and(|owner| tree.local_name(owner) == Some("template"))
            {
                return true;
            }
            parent = tree.parent(current);
        }
        false
    })
    .unwrap_or(true);
    if template {
        return false;
    }
    let document = wrap(scope, root);
    let Some(context) = document.get_creation_context(scope) else {
        return false;
    };
    let Some(name) = v8::String::new(scope, "turbo-dom.ownership.window-document") else {
        return false;
    };
    let key = v8::Private::for_api(scope, Some(name));
    context
        .global(scope)
        .get_private(scope, key)
        .is_some_and(|owner| owner.strict_equals(document.into()))
}

/// Native light-document membership, used by hosts that enumerate child navigables.
/// Shadow-tree nodes can be connected without belonging to the document's child list.
pub fn is_light_document_node(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> bool {
    let _dom = DomScope::for_object(scope, object);
    let Some(handle) = handle_of(scope, object) else {
        return false;
    };
    with_tree(|tree| {
        let mut current = Some(handle);
        while let Some(node) = current {
            if node == tree.root() {
                return true;
            }
            current = tree.parent(node);
        }
        false
    })
    .unwrap_or(false)
}

/// Native containment across shadow hosts, for lifecycle work on moved subtrees.
pub fn composed_contains_node(
    scope: &mut v8::PinScope,
    ancestor: v8::Local<v8::Object>,
    node: v8::Local<v8::Object>,
) -> bool {
    let Some(a_context) = ancestor.get_creation_context(scope) else {
        return false;
    };
    let Some(n_context) = node.get_creation_context(scope) else {
        return false;
    };
    if a_context != n_context {
        return false;
    }
    let _dom = DomScope::for_object(scope, node);
    let Some(ancestor) = handle_of(scope, ancestor) else {
        return false;
    };
    let Some(node) = handle_of(scope, node) else {
        return false;
    };
    with_tree(|tree| {
        let mut current = Some(node);
        while let Some(node) = current {
            if node == ancestor {
                return true;
            }
            current = tree.parent(node).or_else(|| tree.shadow_host(node));
        }
        false
    })
    .unwrap_or(false)
}

fn receiver_handle(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    kinds: &[u8],
) -> Option<Handle> {
    let handle = handle_of(scope, args.this());
    let valid = handle
        .and_then(|h| with_tree(|tree| kinds.is_empty() || kinds.contains(&tree.node_type_id(h))))
        .unwrap_or(false);
    if valid {
        return handle;
    }
    let message = v8::String::new(scope, "Illegal invocation").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
    None
}

fn node_projector_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Private>> {
    let name = v8::String::new(scope, "turbo.native.node-projector")?;
    Some(v8::Private::for_api(scope, Some(name)))
}

fn current_node_projector<'s>(
    scope: &mut v8::PinScope<'s, '_>,
) -> Option<v8::Local<'s, v8::Function>> {
    let root = with_tree(Tree::root)?;
    let document = DOM.with(|dom| dom.borrow().as_ref()?.cache.get(&root).cloned())?;
    let document = document.to_local(scope)?;
    let key = node_projector_key(scope)?;
    document
        .get_private(scope, key)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
}

fn set_node_projector(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    function: v8::Local<v8::Function>,
) {
    if let Some(key) = node_projector_key(scope) {
        document.set_private(scope, key, function.into());
    }
}

// Capture only at trusted bootstrap boundaries; a page can replace the public
// namespace and global constructors without intercepting later native wrapping.
fn capture_node_projector(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    let Some(name) = v8::String::new(scope, "ts") else {
        return;
    };
    let symbol = v8::Symbol::for_key(scope, name);
    let Some(namespace) = global
        .get(scope, symbol.into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return;
    };
    let Some(name) = v8::String::new(scope, "projectNativeNode") else {
        return;
    };
    let Some(function) = namespace
        .get(scope, name.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    else {
        return;
    };
    let Some(root) = with_tree(Tree::root) else {
        return;
    };
    let Some(document) = DOM.with(|dom| dom.borrow().as_ref()?.cache.get(&root).cloned()) else {
        return;
    };
    let Some(document) = document.to_local(scope) else {
        return;
    };
    set_node_projector(scope, document, function);
}

/// Wrap a handle as a JS node object (cached for identity).
fn wrap<'s>(scope: &mut v8::PinScope<'s, '_>, handle: Handle) -> v8::Local<'s, v8::Object> {
    let caught = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut caught.init();
    let object = wrap_inner(scope, handle);
    if scope.has_caught() {
        // Weak localization may materialize an interrupt before projection.
        // Never publish the partially initialized wrapper into the identity cache.
        DOM.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state.cache.remove(&handle);
                state.ownership_dirty = true;
                if let Some(graph) = &state.ownership {
                    graph.borrow_mut().invalidate_wrapper(handle.raw());
                }
            }
        });
        scope.rethrow();
    }
    object
}
fn wrap_inner<'s>(scope: &mut v8::PinScope<'s, '_>, handle: Handle) -> v8::Local<'s, v8::Object> {
    script_state::observe(handle);
    if scope.is_execution_terminating() {
        let context = scope.get_current_context();
        return context.global(scope);
    }
    if let Some(object) = DOM.with(|d| {
        d.borrow().as_ref().and_then(|state| {
            state
                .cache
                .get(&handle)
                .and_then(|weak| weak.to_local(scope))
        })
    }) {
        return object;
    }
    user_activation_gates::observe(handle);
    let owner = DOM_CONTEXTS.with(|d| d.borrow().get(&DOM_ID.with(|id| id.get())).cloned());
    let context = owner
        .as_ref()
        .map(|owner| dom_ownership::local(scope, owner))
        .unwrap_or_else(|| scope.get_current_context());
    let scope = &mut v8::ContextScope::new(scope, context);
    let tmpl_g = DOM.with(|d| d.borrow().as_ref().unwrap().el_template.clone());
    let tmpl = dom_ownership::local(scope, &tmpl_g);
    let Some(obj) = tmpl.new_instance(scope) else {
        // Preserve the pending guest exception/termination without a Rust panic.
        let context = scope.get_current_context();
        return context.global(scope);
    };
    let h = v8::Integer::new_from_unsigned(scope, handle.raw());
    obj.set_internal_field(0, h.into());
    let owner = v8::BigInt::new_from_u64(scope, DOM_ID.with(|id| id.get()));
    obj.set_internal_field(1, owner.into());
    let projector = current_node_projector(scope);
    // Raw bootstrap probes run before projection has been installed. Once it
    // exists, the captured projector supplies every interface without guest reads.
    if projector.is_none() {
        // Parser-created controls bypass createElement's JS wrapper. Select their live
        // interface prototype here too, so IDL getters work for query/innerHTML results.
        let tag = with_tree(|t| t.tag_name(handle))
            .flatten()
            .unwrap_or_default();
        let interface = match tag.to_ascii_lowercase().as_str() {
            "input" => Some("HTMLInputElement"),
            "textarea" => Some("HTMLTextAreaElement"),
            "select" => Some("HTMLSelectElement"),
            "option" => Some("HTMLOptionElement"),
            "button" => Some("HTMLButtonElement"),
            "label" => Some("HTMLLabelElement"),
            "canvas" => Some("HTMLCanvasElement"),
            "a" => Some("HTMLAnchorElement"),
            _ => None,
        };
        if let Some(interface) = interface {
            let global = scope.get_current_context().global(scope);
            let key = v8::String::new(scope, interface).unwrap();
            if let Some(ctor) = global
                .get(scope, key.into())
                .and_then(|v| v.to_object(scope))
            {
                let key = v8::String::new(scope, "prototype").unwrap();
                if let Some(proto) = ctor.get(scope, key.into()) {
                    if proto.is_object() {
                        obj.set_prototype(scope, proto);
                    }
                }
            }
        }
    }
    if scope.is_execution_terminating() {
        return obj;
    }
    let g = v8::Weak::new(scope, obj);
    DOM.with(|d| {
        if let Some(s) = d.borrow_mut().as_mut() {
            s.cache.insert(handle, g);
        }
    });
    dom_ownership::observe(scope, handle, obj);
    // Projection runs after caching, since accessors such as ownerDocument and
    // childNodes may synchronously wrap this node again.
    if let Some(function) = projector {
        let undefined = v8::undefined(scope);
        if function
            .call(scope, undefined.into(), &[obj.into()])
            .is_none()
        {
            DOM.with(|dom| {
                if let Some(state) = dom.borrow_mut().as_mut() {
                    state.cache.remove(&handle);
                }
            });
        }
    }
    obj
}

/// Build native node sequences without consulting guest array prototypes.
fn node_array<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    handles: impl IntoIterator<Item = Handle>,
) -> v8::Local<'s, v8::Array> {
    let values = handles
        .into_iter()
        .map(|handle| wrap(scope, handle).into())
        .collect::<Vec<_>>();
    v8::Array::new_with_elements(scope, &values)
}

/// Wrap `Option<Handle>` as object-or-null.
fn wrap_opt<'s>(scope: &mut v8::PinScope<'s, '_>, h: Option<Handle>) -> v8::Local<'s, v8::Value> {
    match h {
        Some(h) => wrap(scope, h).into(),
        None => v8::null(scope).into(),
    }
}

fn arg_str(scope: &mut v8::PinScope, args: &v8::FunctionCallbackArguments, i: i32) -> String {
    args.get(i).to_rust_string_lossy(scope)
}

fn dom_units<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<v8::Value>,
) -> Option<Vec<u16>> {
    if value.is_symbol() {
        let message = v8::String::new(scope, "Cannot convert a Symbol to a string").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return None;
    }
    let value = value.to_string(scope)?;
    Some(string_code_units(scope, value))
}
/// Extract actual V8 DOM string units without a scalar conversion.
pub fn string_code_units(scope: &v8::PinScope, value: v8::Local<v8::String>) -> Vec<u16> {
    let mut units = vec![0; value.length()];
    value.write_v2(scope, 0, &mut units, v8::WriteFlags::empty());
    units
}
fn unit_string<'s>(scope: &mut v8::PinScope<'s, '_>, units: &[u16]) -> v8::Local<'s, v8::String> {
    v8::String::new_from_two_byte(scope, units, v8::NewStringType::Normal).unwrap()
}

/// Resolve argument `i` to a node handle, or `None` for null/undefined/non-node. CRITICAL: guards
/// `to_object()`, which THROWS "Cannot convert undefined or null to object" on null/undefined — and
/// `insertBefore(node, null)` / `removeChild`-style calls pass that constantly (emotion/react-dom).
fn arg_handle(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    i: i32,
) -> Option<Handle> {
    let v = args.get(i);
    if v.is_null_or_undefined() {
        return None;
    }
    let object = v.to_object(scope)?;
    let owner = dom_id_of(scope, object);
    if owner != DOM_ID.with(|id| id.get()) {
        let message = v8::String::new(
            scope,
            "Cross-document node adoption is not supported by the native DOM",
        )
        .unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return None;
    }
    handle_of(scope, object)
}

// ---- with-tree helpers (borrow the thread-local Tree) ----------------------------------------

fn with_tree<R>(f: impl FnOnce(&Tree) -> R) -> Option<R> {
    DOM.with(|d| d.borrow().as_ref().map(|s| f(&s.tree)))
}
fn with_tree_mut<R>(f: impl FnOnce(&mut Tree) -> R) -> Option<R> {
    let recording = mutation_observers::recording();
    DOM.with(|d| {
        d.borrow_mut().as_mut().map(|s| {
            if mutation_observers::suppressed() {
                s.tree.stop_recording();
            } else if recording {
                s.tree.start_recording();
            } else {
                s.tree.stop_recording();
            }
            s.ownership_dirty = true;
            f(&mut s.tree)
        })
    })
}

// ---- element methods -------------------------------------------------------------------------

/// Hosts that install their own DOMException after the binding (turbo-surf's runtime) hand
/// it over here, so natively raised exceptions are instances of the realm's interface.
pub fn register_dom_exception(scope: &mut v8::PinScope, constructor: v8::Local<v8::Function>) {
    // The constructor's own realm, never the (possibly parent) current context.
    let Some(context) = constructor.get_creation_context(scope) else {
        return;
    };
    let global = context.global(scope);
    let key = dom_exception_key(scope);
    global.set_private(scope, key, constructor.into());
}
/// A DOMException of the current realm's registered interface.
fn new_dom_exception<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    name: &str,
    message: &str,
) -> Option<v8::Local<'s, v8::Object>> {
    let text = v8::String::new(scope, message)?;
    let name = v8::String::new(scope, name)?;
    let global = scope.get_current_context().global(scope);
    let key = dom_exception_key(scope);
    let constructor = global
        .get_private(scope, key)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())?;
    constructor.new_instance(scope, &[text.into(), name.into()])
}
fn dom_exception_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Private> {
    let name = v8::String::new(scope, "turbo.native.DOMException").unwrap();
    v8::Private::for_api(scope, Some(name))
}

fn throw_dom_error(scope: &mut v8::PinScope, name: &str, message: &str) {
    let text = v8::String::new(scope, message).unwrap();
    let name_value = v8::String::new(scope, name).unwrap();
    let global = scope.get_current_context().global(scope);
    let key = dom_exception_key(scope);
    if let Some(constructor) = global
        .get_private(scope, key)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(error) = constructor.new_instance(scope, &[text.into(), name_value.into()]) {
            scope.throw_exception(error.into());
        }
        return;
    }
    // Initialization failures can precede installation of the browser constructor.
    let error = v8::Exception::error(scope, text);
    let object = v8::Local::<v8::Object>::try_from(error).unwrap();
    let key = v8::String::new(scope, "name").unwrap();
    object.set(scope, key.into(), name_value.into());
    scope.throw_exception(error);
}

fn el_attach_shadow(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(host) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let Ok(options) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        let text = v8::String::new(scope, "attachShadow requires an options dictionary").unwrap();
        let error = v8::Exception::type_error(scope, text);
        scope.throw_exception(error);
        return;
    };
    let key = v8::String::new(scope, "mode").unwrap();
    let Some(mode) = options.get(scope, key.into()) else {
        return;
    };
    let mode = mode.to_rust_string_lossy(scope);
    if mode != "open" && mode != "closed" {
        let text = v8::String::new(scope, "mode must be open or closed").unwrap();
        let error = v8::Exception::type_error(scope, text);
        scope.throw_exception(error);
        return;
    }
    let key = v8::String::new(scope, "slotAssignment").unwrap();
    let Some(value) = options.get(scope, key.into()) else {
        return;
    };
    let assignment = if value.is_undefined() {
        "named".to_string()
    } else {
        let Some(value) = value.to_string(scope) else {
            return;
        };
        value.to_rust_string_lossy(scope)
    };
    if assignment != "named" && assignment != "manual" {
        let text = v8::String::new(scope, "Invalid slotAssignment").unwrap();
        let error = v8::Exception::type_error(scope, text);
        scope.throw_exception(error);
        return;
    }
    if !with_tree(|tree| shadow_html::valid_host(tree, host)).unwrap_or(false) {
        throw_dom_error(
            scope,
            "NotSupportedError",
            "This element cannot attach another shadow root",
        );
        return;
    }
    if let Some(existing) = with_tree(|tree| tree.shadow_root(host)).flatten() {
        if let Some(root) = shadow_html::reattach(scope, existing, &mode) {
            rv.set(root.into());
        }
        return;
    }
    let root = with_tree_mut(|tree| tree.attach_shadow(host)).unwrap();
    DOM.with(|state| {
        state
            .borrow_mut()
            .as_mut()
            .unwrap()
            .shadow_modes
            .insert(root, mode)
    });
    let node = wrap(scope, root);
    shadow_slots::configure(scope, node, assignment == "manual");
    shadow_html::configure(scope, node, options);
    rv.set(node.into());
}

fn get_shadow_root(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let root = handle_of(scope, args.holder())
        .and_then(|host| with_tree(|tree| tree.shadow_root(host)).flatten());
    let root = root.filter(|root| {
        DOM.with(|state| {
            state
                .borrow()
                .as_ref()
                .and_then(|state| state.shadow_modes.get(root))
                .is_some_and(|mode| mode == "open")
        })
    });
    let value = wrap_opt(scope, root);
    rv.set(value);
}

fn get_shadow_host(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let host = handle_of(scope, args.holder())
        .and_then(|root| with_tree(|tree| tree.shadow_host(root)).flatten());
    if let Some(host) = host {
        let value = wrap(scope, host);
        rv.set(value.into());
    }
}

fn get_shadow_mode(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(root) = handle_of(scope, args.holder()) else {
        return;
    };
    let mode = DOM.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.shadow_modes.get(&root))
            .cloned()
    });
    if let Some(mode) = mode {
        let value = v8::String::new(scope, &mode).unwrap();
        rv.set(value.into());
    }
}

fn get_is_shadow_root(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let is_shadow = handle_of(scope, args.holder())
        .and_then(|h| with_tree(|tree| tree.is_shadow_root(h)))
        .unwrap_or(false);
    rv.set_bool(is_shadow);
}

fn el_append_child<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(parent) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let Some(child) = dom_mutations::incoming_node(scope, &args, 0) else {
        return;
    };
    if dom_mutations::insert_inputs(scope, parent, &[child], None, &[]) {
        rv.set(args.get(0));
    }
}

fn el_remove_child<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(parent) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let Some(child) = dom_mutations::reference_argument(scope, &args, 0) else {
        return;
    };
    if with_tree(|tree| tree.parent(child) != Some(parent)).unwrap_or(true) {
        throw_dom_error(
            scope,
            "NotFoundError",
            "The node is not a child of this parent",
        );
        return;
    }
    html_popover::disconnect(scope, child, true);
    range_live::removing(scope, child);
    with_tree_mut(|tree| tree.remove_child(parent, child));
    form_checked_state::owner_changes();
    rv.set(args.get(0));
}

fn el_replace_child<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(parent) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let Some(child) = dom_mutations::incoming_node(scope, &args, 0) else {
        return;
    };
    let Some(old) = dom_mutations::reference_argument(scope, &args, 1) else {
        return;
    };
    let reference = with_tree(|tree| tree.next_sibling(old)).flatten();
    if dom_mutations::insert_inputs(scope, parent, &[child], reference, &[old]) {
        rv.set(args.get(1));
    }
}

// ---- CharacterData methods (text/comment nodes) ----------------------------------------------
fn arg_usize(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    i: i32,
) -> Option<usize> {
    args.get(i).uint32_value(scope).map(|value| value as usize)
}
fn character_units(node: Handle) -> Vec<u16> {
    with_tree(|tree| tree.node_value_utf16(node).unwrap_or_default()).unwrap_or_default()
}
fn checked_character_offset(scope: &mut v8::PinScope, node: Handle, offset: usize) -> bool {
    if offset > character_units(node).len() {
        throw_dom_error(
            scope,
            "IndexSizeError",
            "CharacterData offset exceeds its length",
        );
        false
    } else {
        true
    }
}
fn replace_character_data(
    scope: &mut v8::PinScope,
    node: Handle,
    offset: usize,
    count: usize,
    data: &[u16],
) {
    let mut units = character_units(node);
    if !checked_character_offset(scope, node, offset) {
        return;
    }
    let count = count.min(units.len() - offset);
    let inserted = data.to_vec();
    range_live::replacing_data(scope, node, offset, count, inserted.len());
    units.splice(offset..offset + count, inserted);
    with_tree_mut(|tree| tree.set_text_content_utf16(node, &units));
}
fn el_insert_data(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[3, 4, 7, 8]) else {
        return;
    };
    let Some(off) = arg_usize(scope, &args, 0) else {
        return;
    };
    let Some(data) = dom_units(scope, args.get(1)) else {
        return;
    };
    replace_character_data(scope, h, off, 0, &data);
}
fn el_delete_data(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[3, 4, 7, 8]) else {
        return;
    };
    let Some(off) = arg_usize(scope, &args, 0) else {
        return;
    };
    let Some(count) = arg_usize(scope, &args, 1) else {
        return;
    };
    replace_character_data(scope, h, off, count, &[]);
}
fn el_append_data(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[3, 4, 7, 8]) else {
        return;
    };
    let Some(data) = dom_units(scope, args.get(0)) else {
        return;
    };
    let offset = character_units(h).len();
    replace_character_data(scope, h, offset, 0, &data);
}
fn el_replace_data(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[3, 4, 7, 8]) else {
        return;
    };
    let Some(off) = arg_usize(scope, &args, 0) else {
        return;
    };
    let Some(count) = arg_usize(scope, &args, 1) else {
        return;
    };
    let Some(data) = dom_units(scope, args.get(2)) else {
        return;
    };
    replace_character_data(scope, h, off, count, &data);
}
fn el_substring_data(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[3, 4, 7, 8]) else {
        return;
    };
    let Some(off) = arg_usize(scope, &args, 0) else {
        return;
    };
    let Some(count) = arg_usize(scope, &args, 1) else {
        return;
    };
    if !checked_character_offset(scope, h, off) {
        return;
    }
    let units = character_units(h);
    let end = off.saturating_add(count).min(units.len());
    rv.set(
        v8::String::new_from_two_byte(scope, &units[off..end], v8::NewStringType::Normal)
            .unwrap()
            .into(),
    );
}
fn el_split_text(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[3, 4]) else {
        return;
    };
    let Some(off) = arg_usize(scope, &args, 0) else {
        return;
    };
    if !checked_character_offset(scope, h, off) {
        return;
    }
    let units = character_units(h);
    let Some(new_h) = with_tree_mut(|tree| {
        tree.set_text_content_utf16(h, &units[..off]);
        let next = tree.create_text_node_utf16(&units[off..]);
        if let Some(parent) = tree.parent(h) {
            let reference = tree.next_sibling(h);
            tree.insert_before(parent, next, reference);
        }
        next
    }) else {
        return;
    };
    range_live::splitting(scope, h, new_h, off);
    let node = wrap(scope, new_h);
    rv.set(node.into());
}

/// Install immutable host handler authorization for the current context's document.
pub fn install_inline_event_handlers(
    scope: &mut v8::PinScope,
    callback: v8::Local<v8::Function>,
    base: &str,
) {
    let id = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map_or(0, |id| id.0);
    let _dom = DomScope::enter(id);
    event_handlers::install(scope, Some(callback), base);
}

/// Install host-only Trusted Types authorization for this Window's native factory.
pub fn install_trusted_types_policy(
    scope: &mut v8::PinScope,
    callback: v8::Local<v8::Function>,
) -> bool {
    trusted_types::install_policy(scope, callback).is_some()
}

/// Convert a host sink input using the target document's native Trusted Types policy.
/// Typed values are unwrapped from private fields, without guest `toString` dispatch.
pub fn trusted_type_string<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    input: v8::Local<'s, v8::Value>,
    kind: &str,
    sink: &str,
    prefix: &str,
) -> Option<v8::Local<'s, v8::String>> {
    let units = trusted_types::units(scope, document, input, kind, sink, prefix, false)?;
    v8::String::new_from_two_byte(scope, &units, v8::NewStringType::Normal)
}

/// Read exact branded script source for the native engine's code-generation callback.
/// Ordinary objects and proxies remain ordinary eval arguments without coercion.
pub fn trusted_script_source<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    input: v8::Local<'s, v8::Value>,
) -> Option<v8::Local<'s, v8::String>> {
    trusted_types::script_source(scope, input)
}

/// Set host sandbox admission for the current document's form default action.
/// The value belongs to the native document, so borrowed methods and adopted
/// controls use their actual target document rather than guest-visible flags.
pub fn set_form_submission_blocked(scope: &mut v8::PinScope, blocked: bool) {
    let id = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map_or(0, |id| id.0);
    let _dom = DomScope::enter(id);
    DOM.with(|dom| {
        if let Some(dom) = dom.borrow_mut().as_mut() {
            dom.form_submission_blocked = blocked;
        }
    });
}

/// First connected document base attribute, read without guest getters.
pub fn form_submission_base_href(
    scope: &mut v8::PinScope,
    form: v8::Local<v8::Object>,
) -> Option<String> {
    let _dom = DomScope::for_object(scope, form);
    with_tree(|tree| {
        tree.query_selector_all("base[href]")
            .iter()
            .find_map(|&node| {
                is_connected(tree, node)
                    .then(|| {
                        tree.get_attribute(node, "href")
                            .map(|value| value.to_string())
                    })
                    .flatten()
            })
    })
    .flatten()
}

/// Construct a native form entry list after FormData's nullable argument conversion.
fn form_data_entries_impl<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    form: v8::Local<'s, v8::Value>,
    submitter: v8::Local<'s, v8::Value>,
    hold: bool,
) -> Option<v8::Local<'s, v8::Array>> {
    let _dom = match v8::Local::<v8::Object>::try_from(form) {
        Ok(form) => DomScope::for_object(scope, form),
        Err(_) => DomScope::enter(
            scope
                .get_current_context()
                .get_slot::<DomContextId>()
                .map_or(0, |id| id.0),
        ),
    };
    let id = DOM_ID.with(std::cell::Cell::get);
    let context = DOM_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .get(&id)
            .and_then(|context| context.to_local(scope))
    })?;
    let callback = context
        .get_slot::<FormEntryListReader>()?
        .0
        .to_local(scope)?;
    let scope = &mut v8::ContextScope::new(scope, context);
    let receiver = v8::undefined(scope);
    let hold = v8::Boolean::new(scope, hold);
    v8::Local::<v8::Array>::try_from(callback.call(
        scope,
        receiver.into(),
        &[form, submitter, hold.into()],
    )?)
    .ok()
}
pub fn form_data_constructor_entries<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    form: v8::Local<'s, v8::Value>,
    submitter: v8::Local<'s, v8::Value>,
) -> Option<v8::Local<'s, v8::Array>> {
    form_data_entries_impl(scope, form, submitter, false)
}
/// Begin a runtime constructor transaction. The host must finish only after success.
pub fn begin_form_data_constructor_entries<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    form: v8::Local<'s, v8::Value>,
    submitter: v8::Local<'s, v8::Value>,
) -> Option<v8::Local<'s, v8::Array>> {
    form_data_entries_impl(scope, form, submitter, true)
}
/// Release a successfully acquired runtime constructor entry-list transaction.
pub fn finish_form_data_constructor(scope: &mut v8::PinScope, form: v8::Local<v8::Value>) {
    let Ok(form) = v8::Local::<v8::Object>::try_from(form) else {
        return;
    };
    let _dom = DomScope::for_object(scope, form);
    let Some(handle) = handle_of(scope, form) else {
        return;
    };
    if !with_tree(|tree| {
        tree.tag_name(handle)
            .is_some_and(|name| name.eq_ignore_ascii_case("form"))
    })
    .unwrap_or(false)
    {
        return;
    }
    let name = v8::String::new(scope, "turbo.forms.private.constructing-entries").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    let value = v8::Boolean::new(scope, false);
    form.set_private(scope, key, value.into());
}
/// Construct the owner-document native form entry list, without submission events.
pub fn form_submission_entries<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    form: v8::Local<'s, v8::Object>,
    submitter: v8::Local<'s, v8::Value>,
) -> Option<v8::Local<'s, v8::Array>> {
    form_data_constructor_entries(scope, form.into(), submitter)
}
/// Read first connected base target directly from the native owner document.
pub fn form_submission_base_target(
    scope: &mut v8::PinScope,
    form: v8::Local<v8::Object>,
) -> Option<String> {
    let _dom = DomScope::for_object(scope, form);
    with_tree(|tree| {
        tree.query_selector_all("base[target]")
            .iter()
            .find_map(|&node| {
                is_connected(tree, node)
                    .then(|| tree.get_attribute(node, "target").map(str::to_owned))
                    .flatten()
            })
    })
    .flatten()
}

/// Dispatch a trusted formdata event with the actual runtime's mutable FormData.
pub fn dispatch_formdata_event<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    form: v8::Local<'s, v8::Object>,
    data: v8::Local<'s, v8::Value>,
) -> bool {
    let _dom = DomScope::for_object(scope, form);
    let id = DOM_ID.with(std::cell::Cell::get);
    let Some(context) = DOM_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .get(&id)
            .and_then(|context| context.to_local(scope))
    }) else {
        return false;
    };
    let Some(callback) = context
        .get_slot::<FormDataEventDispatcher>()
        .and_then(|entry| entry.0.to_local(scope))
    else {
        return false;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let receiver = v8::undefined(scope);
    callback
        .call(scope, receiver.into(), &[form.into(), data])
        .is_some()
}

/// Capture genuine runtime File/FormData snapshot authority for this native document.
pub fn install_form_value_reader(
    scope: &mut v8::PinScope,
    callback: v8::Local<v8::Function>,
) -> bool {
    let Some(context) = callback.get_creation_context(scope) else {
        return false;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let id = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map_or(0, |id| id.0);
    let _dom = DomScope::enter(id);
    let Some(callback) = dom_ownership::helper_function(scope, callback) else {
        return false;
    };
    scope
        .get_current_context()
        .set_slot(std::rc::Rc::new(FormValueReader(callback)));
    true
}

/// Register a host-only submission admission/queue callback for this native document.
/// The document owns the V8 edge; retiring it does not leave a host rooted realm.
pub fn install_form_submission_handler(
    scope: &mut v8::PinScope,
    callback: v8::Local<v8::Function>,
) -> bool {
    let Some(context) = callback.get_creation_context(scope) else {
        return false;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let id = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map_or(0, |id| id.0);
    let _dom = DomScope::enter(id);
    let Some(callback) = dom_ownership::helper_function(scope, callback) else {
        return false;
    };
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let Some(dom) = dom.as_mut() else {
            return false;
        };
        dom.form_submission_handler = Some(callback);
        true
    })
}

/// Install immutable host CSS authorization for the current context's document.
pub fn install_css_policy(scope: &mut v8::PinScope, callback: v8::Local<v8::Function>) {
    let id = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map_or(0, |id| id.0);
    let _dom = DomScope::enter(id);
    css_policy::install(scope, callback);
}

// rtdom's native style cascade (turbo-dom `cascade::computed_style`) resolves a property's computed
// value WITH inheritance for the inherited set (color, visibility, font-*, …). getComputedStyle's
// JS cascade only sees an element's OWN matched rules + inline style, so an inherited property like
// `visibility: hidden` set on an ancestor never reached descendants — testing-library then treated a
// hidden subtree as query-visible. JS getComputedStyle calls this for those inherited properties.
// (The result is version-cached on the Tree, so repeated calls within a render are cheap.)
fn el_cascade_prop(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let name = arg_str(scope, &args, 0);
    // JS inherits document-sheet values from fresh composed-parent snapshots;
    // native text only supplies shadow-scoped rules the JS cascade lacks.
    if !with_tree(|t| css_policy::shadow_scoped(t, h)).unwrap_or(false) {
        rv.set(v8::String::empty(scope).into());
        return;
    }
    let policy = css_policy::shadow_cascade_policy(scope);
    let val = with_tree(|t| {
        let computed = turbo_dom_parser::rtdom::cascade::computed_style_with_policy(t, h, &policy);
        Some(turbo_dom_parser::rtdom::cascade::get_property_value(
            &computed, &name,
        ))
    })
    .flatten()
    .unwrap_or_default();
    rv.set(v8::String::new(scope, &val).unwrap().into());
}

// Convert all WebIDL Node/string arguments before native sequence conversion has effects.
fn resolve_nodes<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
    start: i32,
) -> Option<Vec<dom_mutations::Incoming<'s>>> {
    let mut out = Vec::new();
    for index in start..args.length() {
        let value = args.get(index);
        if value.is_object() {
            let object = value.to_object(scope)?;
            if handle_of(scope, object).is_some() {
                out.push(dom_mutations::Incoming::Node(object));
                continue;
            }
        }
        out.push(dom_mutations::Incoming::Text(dom_units(scope, value)?));
    }
    Some(out)
}

fn el_before<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1, 3, 4, 7, 8, 10]) else {
        return;
    };
    let Some(nodes) = resolve_nodes(scope, &args, 0) else {
        return;
    };
    dom_mutations::sibling_mutation(scope, h, &nodes, false, false);
}
fn el_after<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1, 3, 4, 7, 8, 10]) else {
        return;
    };
    let Some(nodes) = resolve_nodes(scope, &args, 0) else {
        return;
    };
    dom_mutations::sibling_mutation(scope, h, &nodes, true, false);
}
fn el_replace_with<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1, 3, 4, 7, 8, 10]) else {
        return;
    };
    let Some(nodes) = resolve_nodes(scope, &args, 0) else {
        return;
    };
    dom_mutations::sibling_mutation(scope, h, &nodes, true, true);
}
fn el_replace_children<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1, 9, 11]) else {
        return;
    };
    let Some(nodes) = resolve_nodes(scope, &args, 0) else {
        return;
    };
    let Some(nodes) = dom_mutations::convert_variadic(scope, &nodes) else {
        return;
    };
    let removed = with_tree(|tree| tree.children(h)).unwrap_or_default();
    dom_mutations::replace_inputs(scope, h, &nodes, &removed);
}
fn el_insert_adjacent_element<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(target) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(position) = dom_string(scope, args.get(0)) else {
        return;
    };
    let Some(element) = dom_mutations::incoming_node(scope, &args, 1) else {
        return;
    };
    let dom_mutations::Incoming::Node(object) = &element else {
        return;
    };
    let kind = {
        let _source = DomScope::for_object(scope, *object);
        handle_of(scope, *object).and_then(|node| with_tree(|tree| tree.node_type_id(node)))
    };
    if kind != Some(1) {
        let text = v8::String::new(scope, "The argument must be an Element").unwrap();
        let error = v8::Exception::type_error(scope, text);
        scope.throw_exception(error);
        return;
    }
    let position = position.to_ascii_lowercase();
    if !matches!(
        position.as_str(),
        "beforebegin" | "afterend" | "afterbegin" | "beforeend"
    ) {
        throw_dom_error(scope, "SyntaxError", "Invalid adjacent insertion position");
        return;
    }
    let location = with_tree(|tree| match position.as_str() {
        "beforebegin" => tree.parent(target).map(|parent| (parent, Some(target))),
        "afterend" => tree
            .parent(target)
            .map(|parent| (parent, tree.next_sibling(target))),
        "afterbegin" => Some((target, tree.first_child(target))),
        _ => Some((target, None)),
    })
    .flatten();
    let Some((parent, reference)) = location else {
        rv.set(v8::null(scope).into());
        return;
    };
    if dom_mutations::insert_inputs(scope, parent, &[element], reference, &[]) {
        rv.set(args.get(1));
    }
}
fn el_insert_adjacent_html<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(target) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(position) = dom_string(scope, args.get(0)) else {
        return;
    };
    let Some(html) = trusted_types::units(
        scope,
        args.this(),
        args.get(1),
        "TrustedHTML",
        "Element insertAdjacentHTML",
        "Failed to execute 'insertAdjacentHTML' on 'Element': ",
        false,
    ) else {
        return;
    };
    let position = position.to_ascii_lowercase();
    if !matches!(
        position.as_str(),
        "beforebegin" | "afterend" | "afterbegin" | "beforeend"
    ) {
        throw_dom_error(scope, "SyntaxError", "Invalid adjacent insertion position");
        return;
    }
    if matches!(position.as_str(), "beforebegin" | "afterend")
        && with_tree(|tree| {
            tree.parent(target)
                .is_none_or(|parent| tree.node_type_id(parent) == 9)
        })
        .unwrap_or(true)
    {
        throw_dom_error(
            scope,
            "NoModificationAllowedError",
            "No element parent is available for HTML insertion",
        );
        return;
    }
    let location = with_tree(|tree| {
        let parent = match position.as_str() {
            "beforebegin" | "afterend" => tree.parent(target)?,
            _ => target,
        };
        let children = tree.children(parent);
        let index = match position.as_str() {
            "beforebegin" => children.iter().position(|&node| node == target)?,
            "afterend" => children.iter().position(|&node| node == target)? + 1,
            "afterbegin" => 0,
            _ => children.len(),
        };
        Some((parent, index, children.len()))
    })
    .flatten();
    // These nodes are freshly parsed, so they cannot contain an existing ancestor.
    with_tree_mut(|tree| tree.insert_adjacent_html_utf16(target, &position, &html));
    if let Some((parent, _, _)) = location {
        let object = wrap(scope, parent);
        template_contents::realize(scope, object);
    }
    if let Some((parent, index, previous_count)) = location {
        let count = with_tree(|tree| tree.children(parent).len()).unwrap_or(previous_count);
        let added = count.saturating_sub(previous_count);
        for child in with_tree(|tree| {
            tree.children(parent)
                .into_iter()
                .skip(index)
                .take(added)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        {
            let object = wrap(scope, child);
            template_contents::mark_parsed(scope, object, true);
        }
        range_live::inserting(scope, parent, index, added);
    }
}
fn el_toggle_attribute<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let Some(mut units) = dom_units(scope, args.get(0)) else {
        return;
    };
    if with_tree(|tree| tree.namespace_id(h) == 0).unwrap_or(false) {
        lower_ascii_units(&mut units);
    }
    let name = String::from_utf16_lossy(&units);
    let force = if args.length() > 1 && !args.get(1).is_undefined() {
        Some(args.get(1).boolean_value(scope))
    } else {
        None
    };
    let previous = with_tree(|t| t.get_attribute_units(h, &units).is_some()).unwrap_or(false);
    if name == "popover" && previous && force != Some(true) {
        html_popover::attribute_change(scope, h, None);
    }
    let present = with_tree_mut(|t| {
        let present = t.get_attribute_units(h, &units).is_some();
        let want = force.unwrap_or(!present);
        if want && !present {
            t.set_attribute_units(h, &units, &[]);
        } else if !want && present {
            t.remove_attribute_units(h, &units);
        }
        want
    })
    .unwrap_or(false);
    if previous != present {
        element_relations::attribute_changed(scope, args.this(), &name);
    }
    rv.set(v8::Boolean::new(scope, present).into());
}
fn dom_string<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
) -> Option<String> {
    if value.is_symbol() {
        let message = v8::String::new(scope, "Cannot convert a Symbol to a string").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return None;
    }
    value
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
}
fn namespace_units_argument<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: &v8::FunctionCallbackArguments<'s>,
) -> Option<Option<Vec<u16>>> {
    let value = args.get(0);
    if value.is_null_or_undefined() {
        return Some(None);
    }
    dom_units(scope, value).map(|value| (!value.is_empty()).then_some(value))
}
fn lower_ascii_units(units: &mut [u16]) {
    for unit in units {
        if (65..=90).contains(unit) {
            *unit += 32;
        }
    }
}
fn require_arguments(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    count: i32,
) -> bool {
    if args.length() >= count {
        return true;
    }
    let message = v8::String::new(scope, "Not enough arguments").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
    false
}
fn validate_namespace_name(
    namespace: Option<&str>,
    qualified: &str,
    attribute: bool,
) -> Result<(), &'static str> {
    let (prefix, local) = qualified
        .split_once(':')
        .map_or((None, qualified), |(prefix, local)| (Some(prefix), local));
    let forbidden = |value: &str, attribute: bool| {
        value.chars().any(|c| {
            matches!(c, '\0' | '\t' | '\n' | '\x0C' | '\r' | ' ' | '/' | '>')
                || attribute && c == '='
        })
    };
    if prefix.is_some_and(|prefix| prefix.is_empty() || forbidden(prefix, false))
        || local.is_empty()
        || forbidden(local, attribute)
    {
        return Err("InvalidCharacterError");
    }
    if !attribute {
        let first = local.chars().next().unwrap();
        if !first.is_ascii_alphabetic()
            && (!(first == '_' || first == ':' || !first.is_ascii())
                || local.chars().skip(1).any(|c| {
                    c.is_ascii()
                        && !c.is_ascii_alphanumeric()
                        && !matches!(c, '-' | '.' | ':' | '_')
                }))
        {
            return Err("InvalidCharacterError");
        }
    }
    if prefix.is_some() && namespace.is_none()
        || prefix == Some("xml") && namespace != Some("http://www.w3.org/XML/1998/namespace")
        || (qualified == "xmlns" || prefix == Some("xmlns"))
            && namespace != Some("http://www.w3.org/2000/xmlns/")
        || namespace == Some("http://www.w3.org/2000/xmlns/")
            && qualified != "xmlns"
            && prefix != Some("xmlns")
    {
        return Err("NamespaceError");
    }
    Ok(())
}
fn el_get_attribute_ns<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(namespace) = namespace_units_argument(scope, &args) else {
        return;
    };
    let Some(local) = dom_units(scope, args.get(1)) else {
        return;
    };
    match with_tree(|tree| tree.get_attribute_ns_units(node, namespace.as_deref(), &local))
        .flatten()
    {
        Some(value) => rv.set(unit_string(scope, &value).into()),
        None => rv.set(v8::null(scope).into()),
    }
}
fn el_has_attribute_ns<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(namespace) = namespace_units_argument(scope, &args) else {
        return;
    };
    let Some(local) = dom_units(scope, args.get(1)) else {
        return;
    };
    rv.set_bool(
        with_tree(|tree| {
            tree.get_attribute_ns_units(node, namespace.as_deref(), &local)
                .is_some()
        })
        .unwrap_or(false),
    );
}
fn el_set_attribute_ns<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 3) {
        return;
    }
    let Some(namespace) = namespace_units_argument(scope, &args) else {
        return;
    };
    let Some(qualified) = dom_units(scope, args.get(1)) else {
        return;
    };
    let Some(units) = trusted_types::attribute_units(
        scope,
        args.this(),
        args.get(2),
        &String::from_utf16_lossy(&qualified),
        namespace
            .as_deref()
            .map(String::from_utf16_lossy)
            .as_deref(),
        "setAttributeNS",
    ) else {
        return;
    };
    let value = String::from_utf16_lossy(&units);
    if let Err(error) = validate_namespace_name(
        namespace
            .as_deref()
            .map(String::from_utf16_lossy)
            .as_deref(),
        &String::from_utf16_lossy(&qualified),
        true,
    ) {
        throw_dom_error(scope, error, "Invalid namespace or qualified name");
        return;
    }
    if namespace.is_none() && qualified == "popover".encode_utf16().collect::<Vec<_>>() {
        html_popover::attribute_change(scope, node, Some(&value));
    }
    with_tree_mut(|tree| {
        tree.set_attribute_ns_units(node, namespace.as_deref(), &qualified, &units)
    });
    let qualified = String::from_utf16_lossy(&qualified);
    if namespace.is_none() {
        event_handlers::changed(scope, node, &qualified);
        element_relations::attribute_changed(scope, args.this(), &qualified);
    }
    if namespace.is_none() && qualified == "style" {
        css_policy::attribute_changed(scope, node);
    }
    if namespace.is_none() && qualified == "nonce" {
        update_node_nonce_units(scope, args.this(), &units);
    }
}
fn el_remove_attribute_ns<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(namespace) = namespace_units_argument(scope, &args) else {
        return;
    };
    let Some(local) = dom_units(scope, args.get(1)) else {
        return;
    };
    if namespace.is_none() && local == "popover".encode_utf16().collect::<Vec<_>>() {
        html_popover::attribute_change(scope, node, None);
    }
    with_tree_mut(|tree| tree.remove_attribute_ns_units(node, namespace.as_deref(), &local));
    let local = String::from_utf16_lossy(&local);
    if namespace.is_none() {
        event_handlers::changed(scope, node, &local);
        element_relations::attribute_changed(scope, args.this(), &local);
    }
    if namespace.is_none() && local == "style" {
        css_policy::attribute_changed(scope, node);
    }
    if namespace.is_none() && local == "nonce" {
        update_node_nonce(scope, args.this(), "");
    }
}

fn el_insert_before<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(parent) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    if !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(child) = dom_mutations::incoming_node(scope, &args, 0) else {
        return;
    };
    let reference = if args.get(1).is_null_or_undefined() {
        None
    } else {
        let Some(reference) = dom_mutations::reference_argument(scope, &args, 1) else {
            return;
        };
        Some(reference)
    };
    if dom_mutations::insert_inputs(scope, parent, &[child], reference, &[]) {
        rv.set(args.get(0));
    }
}

fn el_set_attribute<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let Some(mut name) = dom_units(scope, args.get(0)) else {
        return;
    };
    let units_input = args.get(1);
    // HTML setAttribute lowercases the qualified name (React calls setAttribute('colSpan', …); jsdom
    // stores 'colspan'). Lowercase for HTML-namespace elements only (ns 0); SVG (ns 1) keeps camelCase
    // attrs like viewBox.
    let is_html = with_tree(|t| t.namespace_id(h) == 0).unwrap_or(true);
    if is_html {
        lower_ascii_units(&mut name);
    }
    let key = String::from_utf16_lossy(&name);
    let Some(units) =
        trusted_types::attribute_units(scope, args.this(), units_input, &key, None, "setAttribute")
    else {
        return;
    };
    let value = String::from_utf16_lossy(&units);
    if key == "popover" {
        html_popover::attribute_change(scope, h, Some(&value));
    }
    with_tree_mut(|t| t.set_attribute_units(h, &name, &units));
    event_handlers::changed(scope, h, &key);
    element_relations::attribute_changed(scope, args.this(), &key);
    if key == "style" {
        css_policy::attribute_changed(scope, h);
    }
    if key == "nonce" {
        DOM.with(|dom| {
            dom.borrow_mut()
                .as_mut()
                .unwrap()
                .nonces
                .insert(h, units.clone());
        });
    }

    // When an <input> becomes a checkbox/radio (React sets `type` via setAttribute after createElement),
    // strip the value-change tracker: those inputs track `checked`, not `value`, so a value-tracker
    // would mis-detect changes and break onChange. Removing the own `value`/`_valueTracker` lets the
    // framework use its checked path (we keep the own `checked` accessor).
    if key == "type" {
        let v = value.to_ascii_lowercase();
        if v == "checkbox" || v == "radio" {
            let this = args.this();
            for k in ["value", "_valueTracker"] {
                if let Some(key) = v8::String::new(scope, k) {
                    if this.has_own_property(scope, key.into()).unwrap_or(false) {
                        this.delete(scope, key.into());
                    }
                }
            }
        }
    }
}

/// Attribute lookup with the HTML case-insensitivity rule: try the exact name, then its lowercase
/// form (React writes `tabindex`/`maxlength` lowercased; tests read `tabIndex`/`maxLength`). Exact-
/// first preserves SVG camelCase attributes (`viewBox`).
fn attr_get(t: &Tree, h: Handle, name: &str) -> Option<String> {
    if let Some(v) = t.get_attribute(h, name) {
        return Some(v.to_string());
    }
    let lower = name.to_ascii_lowercase();
    if t.namespace_id(h) == 0 && lower != name {
        if let Some(v) = t.get_attribute(h, &lower) {
            return Some(v.to_string());
        }
    }
    None
}

fn el_get_attribute<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let Some(mut name) = dom_units(scope, args.get(0)) else {
        return;
    };
    let val = with_tree(|t| {
        if t.namespace_id(h) == 0 {
            lower_ascii_units(&mut name);
        }
        t.get_attribute_units(h, &name)
    })
    .flatten();
    match val {
        Some(s) => rv.set(unit_string(scope, &s).into()),
        None => rv.set(v8::null(scope).into()),
    }
}

fn attr_type_error(scope: &mut v8::PinScope, message: &str) {
    let message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}
fn el_get_attribute_node<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(owner) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 1) {
        return;
    }
    let Some(mut name) = dom_units(scope, args.get(0)) else {
        return;
    };
    if with_tree(|tree| tree.namespace_id(owner) == 0).unwrap_or(false) {
        lower_ascii_units(&mut name);
    }
    let data = with_tree(|tree| {
        tree.attributes_units(owner)
            .into_iter()
            .find(|attribute| attribute.qualified_name == name)
    })
    .flatten();
    let node = data.and_then(|data| {
        with_tree_mut(|tree| {
            tree.attribute_node_units(owner, data.namespace.as_deref(), &data.local_name)
        })
        .flatten()
    });
    rv.set(wrap_opt(scope, node));
}
fn el_get_attribute_node_ns<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(owner) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(namespace) = namespace_units_argument(scope, &args) else {
        return;
    };
    let Some(local) = dom_units(scope, args.get(1)) else {
        return;
    };
    let node = with_tree_mut(|tree| tree.attribute_node_units(owner, namespace.as_deref(), &local))
        .flatten();
    rv.set(wrap_opt(scope, node));
}
fn el_set_attribute_node<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(owner) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 1) {
        return;
    }
    let Ok(attribute) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        attr_type_error(scope, "An Attr node is required");
        return;
    };
    let target_id = dom_id_of(scope, args.this());
    let source_id = dom_id_of(scope, attribute);
    let valid = {
        let _source = DomScope::for_object(scope, attribute);
        handle_of(scope, attribute).and_then(|node| {
            with_tree(|tree| (tree.node_type_id(node), tree.attribute_owner(node)))
        })
    };
    let Some((2, attached)) = valid else {
        attr_type_error(scope, "An Attr node is required");
        return;
    };
    if attached.is_some_and(|current| source_id != target_id || current != owner) {
        throw_dom_error(
            scope,
            "InUseAttributeError",
            "The attribute already belongs to an element",
        );
        return;
    }
    let attribute = if source_id != target_id {
        match document_basics::adopt(scope, args.this(), attribute) {
            Ok(attribute) => attribute,
            Err(error) => {
                throw_dom_error(scope, error, "The attribute could not be adopted");
                return;
            }
        }
    } else {
        attribute
    };
    let Some(node) = handle_of(scope, attribute) else {
        return;
    };
    if let Some(value) = with_tree(|tree| {
        tree.attribute_data(node)
            .filter(|data| data.namespace.is_none() && data.local_name == "popover")
            .map(|data| data.value.clone())
    })
    .flatten()
    {
        html_popover::attribute_change(scope, owner, Some(&value));
    }
    match with_tree_mut(|tree| tree.set_attribute_node(owner, node)) {
        Some(Ok(replaced)) => {
            sync_attr_nonce(scope, args.this(), node);
            if let Some(name) = with_tree(|tree| {
                tree.attribute_data(node)
                    .filter(|data| data.namespace.is_none())
                    .map(|data| data.local_name.clone())
            })
            .flatten()
            {
                element_relations::attribute_changed(scope, args.this(), &name);
            }
            if with_tree(|tree| {
                tree.attribute_data(node)
                    .is_some_and(|data| data.namespace.is_none() && data.local_name == "style")
            })
            .unwrap_or(false)
            {
                css_policy::attribute_changed(scope, owner);
            }
            rv.set(wrap_opt(scope, replaced));
        }
        Some(Err(error)) => throw_dom_error(scope, error, "The attribute could not be attached"),
        None => {}
    }
}
fn el_remove_attribute_node<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(owner) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if !require_arguments(scope, &args, 1) {
        return;
    }
    let Ok(attribute) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        attr_type_error(scope, "An Attr node is required");
        return;
    };
    if dom_id_of(scope, attribute) != dom_id_of(scope, args.this()) {
        throw_dom_error(
            scope,
            "NotFoundError",
            "The attribute does not belong to this element",
        );
        return;
    }
    let Some(node) = handle_of(scope, attribute) else {
        attr_type_error(scope, "An Attr node is required");
        return;
    };
    if with_tree(|tree| tree.attribute_owner(node) != Some(owner)).unwrap_or(true) {
        throw_dom_error(
            scope,
            "NotFoundError",
            "The attribute does not belong to this element",
        );
        return;
    }
    html_popover::attribute_node_change(scope, node, None);
    let aria_name = with_tree(|tree| {
        tree.attribute_data(node)
            .filter(|data| data.namespace.is_none())
            .map(|data| data.local_name.clone())
    })
    .flatten();
    let nonce = with_tree(|tree| {
        tree.attribute_data(node)
            .is_some_and(|data| data.namespace.is_none() && data.local_name == "nonce")
    })
    .unwrap_or(false);
    let style = with_tree(|tree| {
        tree.attribute_data(node)
            .is_some_and(|data| data.namespace.is_none() && data.local_name == "style")
    })
    .unwrap_or(false);
    match with_tree_mut(|tree| tree.remove_attribute_node(owner, node)) {
        Some(Ok(_)) => {
            if let Some(name) = &aria_name {
                element_relations::attribute_changed(scope, args.this(), name);
            }
            if style {
                css_policy::attribute_changed(scope, owner);
            }
            if nonce {
                update_node_nonce(scope, args.this(), "");
            }
            rv.set(attribute.into());
        }
        Some(Err(error)) => throw_dom_error(
            scope,
            error,
            "The attribute does not belong to this element",
        ),
        None => {}
    }
}
fn sync_attr_nonce(scope: &mut v8::PinScope, owner: v8::Local<v8::Object>, attribute: Handle) {
    if let Some(value) = with_tree(|tree| {
        tree.attribute_data(attribute)
            .filter(|data| data.namespace.is_none() && data.local_name == "nonce")
            .map(|_| tree.node_value_utf16(attribute).unwrap_or_default())
    })
    .flatten()
    {
        update_node_nonce_units(scope, owner, &value);
    }
}
fn doc_create_attribute<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[9]).is_none() || !require_arguments(scope, &args, 1) {
        return;
    }
    let Some(mut name) = dom_units(scope, args.get(0)) else {
        return;
    };
    if name.is_empty()
        || name
            .iter()
            .copied()
            .any(|c| matches!(c, 0 | 9 | 10 | 12 | 13 | 32 | 47 | 61 | 62))
    {
        throw_dom_error(scope, "InvalidCharacterError", "Invalid attribute name");
        return;
    }
    lower_ascii_units(&mut name);
    if let Some(node) = with_tree_mut(|tree| tree.create_attribute_units(None, &name, &[])) {
        let object = wrap(scope, node);
        rv.set(object.into());
    }
}
fn doc_create_attribute_ns<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[9]).is_none() || !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(namespace) = namespace_units_argument(scope, &args) else {
        return;
    };
    let Some(name) = dom_units(scope, args.get(1)) else {
        return;
    };
    if let Err(error) = validate_namespace_name(
        namespace
            .as_deref()
            .map(String::from_utf16_lossy)
            .as_deref(),
        &String::from_utf16_lossy(&name),
        true,
    ) {
        throw_dom_error(scope, error, "Invalid namespace or qualified name");
        return;
    }
    if let Some(node) =
        with_tree_mut(|tree| tree.create_attribute_units(namespace.as_deref(), &name, &[]))
    {
        let object = wrap(scope, node);
        rv.set(object.into());
    }
}
fn get_named_node_name(
    scope: &mut v8::PinScope,
    name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    rv: v8::ReturnValue<v8::Value>,
) {
    get_node_name(scope, name, args, rv);
}
fn get_doctype_public(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let value = handle_of(scope, args.holder())
        .and_then(|node| {
            with_tree(|tree| tree.doctype_public_id(node).map(str::to_owned)).flatten()
        })
        .unwrap_or_default();
    rv.set(v8::String::new(scope, &value).unwrap().into());
}
fn get_doctype_system(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let value = handle_of(scope, args.holder())
        .and_then(|node| {
            with_tree(|tree| tree.doctype_system_id(node).map(str::to_owned)).flatten()
        })
        .unwrap_or_default();
    rv.set(v8::String::new(scope, &value).unwrap().into());
}
fn get_attr_owner(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let owner = handle_of(scope, args.holder())
        .and_then(|node| with_tree(|tree| tree.attribute_owner(node)).flatten());
    rv.set(wrap_opt(scope, owner));
}
fn get_attr_specified(
    _scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    _args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    rv.set_bool(true);
}
fn set_attr_value<'s, 'i, T>(
    scope: &mut v8::PinScope<'s, 'i>,
    _name: v8::Local<'s, v8::Name>,
    value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    _rv: v8::ReturnValue<T>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(node) = handle_of(scope, args.holder()) else {
        return;
    };
    let Some(value) = dom_units(scope, value) else {
        return;
    };
    html_popover::attribute_node_change(scope, node, Some(&String::from_utf16_lossy(&value)));
    with_tree_mut(|tree| tree.set_attribute_value_utf16(node, &value));
    if let Some(owner) = with_tree(|tree| tree.attribute_owner(node)).flatten() {
        let owner = wrap(scope, owner);
        sync_attr_nonce(scope, owner, node);
        if let Some(name) = with_tree(|tree| {
            tree.attribute_data(node)
                .filter(|data| data.namespace.is_none())
                .map(|data| data.local_name.clone())
        })
        .flatten()
        {
            element_relations::attribute_changed(scope, owner, &name);
        }
        if with_tree(|tree| {
            tree.attribute_data(node)
                .is_some_and(|data| data.namespace.is_none() && data.local_name == "style")
        })
        .unwrap_or(false)
        {
            let handle = handle_of(scope, owner).unwrap();
            css_policy::attribute_changed(scope, handle);
        }
    }
}

fn el_has_attribute<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let Some(mut name) = dom_units(scope, args.get(0)) else {
        return;
    };
    let has = with_tree(|t| {
        if t.namespace_id(h) == 0 {
            lower_ascii_units(&mut name);
        }
        t.get_attribute_units(h, &name).is_some()
    })
    .unwrap_or(false);
    rv.set_bool(has);
}

fn el_remove_attribute<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let Some(mut name) = dom_units(scope, args.get(0)) else {
        return;
    };
    if with_tree(|t| t.namespace_id(h) == 0).unwrap_or(false) {
        lower_ascii_units(&mut name);
    }
    let lower = String::from_utf16_lossy(&name);
    if lower == "nonce" {
        DOM.with(|dom| {
            dom.borrow_mut()
                .as_mut()
                .unwrap()
                .nonces
                .insert(h, Vec::new());
        });
    }
    if lower == "popover" {
        html_popover::attribute_change(scope, h, None);
    }
    with_tree_mut(|t| {
        t.remove_attribute_units(h, &name);
    });
    event_handlers::changed(scope, h, &lower);
    element_relations::attribute_changed(scope, args.this(), &lower);
    if lower == "style" {
        css_policy::attribute_changed(scope, h);
    }
}

fn selector_argument<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
    node: Handle,
) -> Option<(String, Vec<u16>, bool)> {
    let units = dom_units(scope, args.get(0))?;
    let scalar = String::from_utf16_lossy(&units);
    // The matcher is lenient; admission uses the same grammar as stylesheets.
    if css_selector_syntax::selector_list(&scalar).is_none() {
        throw_dom_error(
            scope,
            "SyntaxError",
            &format!("'{scalar}' is not a valid selector."),
        );
        return None;
    }
    let transformed = html_popover::selector(scope, node, &scalar)?;
    let transformed = html_dialog::selector(scope, node, &transformed)?;
    let transformed = interest_invokers::selector(scope, node, &transformed)?;
    let transformed = form_pseudo::selector(scope, node, &transformed)?;
    let transformed = view_transitions::selector(scope, node, &transformed)?;
    let original = if transformed == scalar {
        units
    } else {
        transformed.encode_utf16().collect()
    };
    let unit_path = String::from_utf16(&original).is_err() || original.contains(&92);
    Some((transformed, original, unit_path))
}
/// element-scoped querySelector via NodeRef.
fn el_query_selector<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1, 11]) else {
        return;
    };
    let Some((sel, units, unit_path)) = selector_argument(scope, &args, h) else {
        return;
    };
    let found = with_tree(|t| {
        if unit_path {
            t.query_selector_all_from_units(h, &units).first().copied()
        } else {
            NodeRef::new(t, h).query_selector(&sel).map(|n| n.handle())
        }
    })
    .flatten();
    let v = wrap_opt(scope, found);
    rv.set(v);
}

// ---- innerHTML / outerHTML / children / form props / classList (P3.3 batch) -----------------

fn get_inner_html(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let node = wrap(scope, h);
    let html = template_contents::serialize(scope, node, false);
    rv.set(unit_string(scope, &html).into());
}
fn set_inner_html<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    _name: v8::Local<'s, v8::Name>,
    value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    _rv: v8::ReturnValue<()>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let Some(html) = trusted_types::units(
        scope,
        args.holder(),
        value,
        "TrustedHTML",
        "Element innerHTML",
        "Failed to set the 'innerHTML' property on 'Element': ",
        true,
    ) else {
        return;
    };
    let node = wrap(scope, h);
    if template_contents::set_html(scope, node, &html) {
        return;
    }
    html_popover::disconnect(scope, h, false);
    range_live::replacing_children(scope, h);
    with_tree_mut(|t| t.set_inner_html_utf16(h, &html));
    form_checked_state::parsed(h);
    template_contents::realize(scope, node);
    for child in with_tree(|tree| tree.children(h)).unwrap_or_default() {
        let object = wrap(scope, child);
        template_contents::mark_parsed(scope, object, true);
    }
    css_policy::invalidate_sheet(h);
}
fn get_outer_html(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let node = wrap(scope, h);
    let html = template_contents::serialize(scope, node, true);
    rv.set(unit_string(scope, &html).into());
}

fn set_outer_html<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    _name: v8::Local<'s, v8::Name>,
    value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    _rv: v8::ReturnValue<()>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(node) = handle_of(scope, args.holder()) else {
        return;
    };
    let parent = with_tree(|tree| {
        NodeRef::new(tree, node)
            .parent()
            .map(|parent| parent.handle())
    })
    .flatten();
    let Some(parent) = parent else {
        return;
    };
    if with_tree(|tree| tree.node_type_id(parent) == 9).unwrap_or(false) {
        throw_dom_error(
            scope,
            "NoModificationAllowedError",
            "The document element cannot be replaced with outerHTML",
        );
        return;
    }
    let Some(html) = trusted_types::units(
        scope,
        args.holder(),
        value,
        "TrustedHTML",
        "Element outerHTML",
        "Failed to set the 'outerHTML' property on 'Element': ",
        true,
    ) else {
        return;
    };
    html_popover::disconnect(scope, node, true);
    let position = with_tree(|tree| {
        let children = tree.children(parent);
        children
            .iter()
            .position(|&child| child == node)
            .map(|index| (index, children.len()))
    })
    .flatten();
    range_live::removing(scope, node);
    with_tree_mut(|tree| {
        tree.insert_adjacent_html_utf16(node, "beforebegin", &html);
        tree.remove_child(parent, node);
    });
    let object = wrap(scope, parent);
    template_contents::realize(scope, object);
    if let Some((index, previous_count)) = position {
        let count = with_tree(|tree| tree.children(parent).len()).unwrap_or(previous_count - 1);
        let added = count.saturating_add(1).saturating_sub(previous_count);
        for child in with_tree(|tree| {
            tree.children(parent)
                .into_iter()
                .skip(index)
                .take(added)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        {
            let object = wrap(scope, child);
            template_contents::mark_parsed(scope, object, true);
        }
        range_live::inserting(scope, parent, index, added);
    }
}

/// `children` = element-only child nodes (HTMLCollection-ish array).
fn get_children(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let kids: Vec<Handle> = with_tree(|t| {
        t.children(h)
            .into_iter()
            .filter(|&c| t.node_type_id(c) == 1)
            .collect()
    })
    .unwrap_or_default();
    let arr = node_array(scope, kids);
    bind_method(scope, arr.into(), "item", nodelist_item);
    bind_method(scope, arr.into(), "namedItem", nodelist_named_item);
    rv.set(arr.into());
}

fn get_parent_element(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let p = with_tree(|t| {
        NodeRef::new(t, h)
            .parent()
            .map(|x| x.handle())
            .filter(|&ph| t.node_type_id(ph) == 1)
    })
    .flatten();
    let v = wrap_opt(scope, p);
    rv.set(v);
}

fn get_first_element_child(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let c = with_tree(|t| NodeRef::new(t, h).first_element_child().map(|x| x.handle())).flatten();
    let v = wrap_opt(scope, c);
    rv.set(v);
}

fn get_namespace_uri(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let namespace = with_tree(|tree| tree.namespace_uri_utf16(h)).flatten();
    match namespace {
        Some(namespace) => rv.set(unit_string(scope, &namespace).into()),
        None => rv.set(v8::null(scope).into()),
    }
}

fn get_prefix(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let prefix = handle_of(scope, args.holder())
        .and_then(|h| with_tree(|tree| tree.prefix_utf16(h)).flatten());
    match prefix {
        Some(prefix) => rv.set(unit_string(scope, &prefix).into()),
        None => rv.set(v8::null(scope).into()),
    }
}

// no-op / stub element methods commonly called during render + queries.
thread_local! {
    /// the focused element handle (document.activeElement). userEvent.type targets it.
    static ACTIVE: RefCell<Option<Handle>> = const { RefCell::new(None) };
}
/// Native `HTMLElement.focus()` — like a real browser / jsdom: blur the previously-focused element
/// (blur + bubbling focusout), make this the activeElement, then fire focus + bubbling focusin. The
/// bubbling focusout/focusin reach React's delegated onBlur/onFocus at the root container. userEvent
/// drives blur-on-tab purely through native focus(), so this must dispatch the events itself.
fn el_focus(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if html_dialog::blocks_focus(h) {
        return;
    }
    let prev = ACTIVE.with(|a| *a.borrow());
    if prev == Some(h) {
        return;
    }
    ACTIVE.with(|a| *a.borrow_mut() = Some(h));
    // Only blur a previously-focused element that is still attached. A detached `prev` is a stale
    // pointer left by a prior, now-unmounted tree (testing-library cleanup); dispatching blur/focusout
    // on it walks a detached subtree and would corrupt the focus transition into the new element.
    if let Some(p) = prev.filter(|&p| with_tree(|t| is_connected(t, p)).unwrap_or(false)) {
        dispatch_synthetic(scope, p, "blur", false, false);
        dispatch_synthetic(scope, p, "focusout", true, false);
    }
    dispatch_synthetic(scope, h, "focus", false, false);
    dispatch_synthetic(scope, h, "focusin", true, false);
    selection::focused(scope, args.this());
}
fn el_blur(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    if ACTIVE.with(|a| *a.borrow()) != Some(h) {
        return;
    }
    ACTIVE.with(|a| *a.borrow_mut() = None);
    dispatch_synthetic(scope, h, "blur", false, false);
    dispatch_synthetic(scope, h, "focusout", true, false);
}
/// `document.activeElement` → the focused element, or `<body>`.
fn get_active_element(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    // Real DOM resets focus to <body> when the active element is removed from the tree. If our
    // tracked handle is detached (e.g. the focused input of a prior, now-unmounted React tree —
    // testing-library cleanup leaves the JS ACTIVE pointer dangling), drop it so the stale node
    // never leaks into the next test's focus/userEvent logic.
    let active = ACTIVE.with(|a| *a.borrow());
    let active = active.filter(|&h| with_tree(|t| is_connected(t, h)).unwrap_or(false));
    if active.is_none() {
        ACTIVE.with(|a| *a.borrow_mut() = None);
    }
    // Focus inside shadow trees is retargeted to the document-level host.
    let active = active.and_then(|h| with_tree(|t| shadow_html::retarget(t, h, t.root())));
    let h = active.or_else(|| with_tree(|t| t.query_selector("body")).flatten());
    if let Some(h) = h {
        let node = wrap(scope, h);
        rv.set(node.into());
    }
}
/// Native `HTMLElement.click()` — dispatch a bubbling, cancelable click event so React's delegated
/// onClick at the root container fires. Tests call `el.click()` directly (no userEvent).
fn el_click(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    synthetic_click(scope, args.this(), h, false);
}

/// Fire a synthetic pointer `click` (click() and label activation); a disabled control or
/// one already running its click-in-progress steps ignores it.
fn synthetic_click(
    scope: &mut v8::PinScope,
    receiver: v8::Local<v8::Object>,
    h: Handle,
    trusted: bool,
) {
    if html_invokers::disabled_control(h) {
        return;
    }
    let name = v8::String::new(scope, "turbo-dom.click.active").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    if receiver
        .get_private(scope, key)
        .is_some_and(|value| value.is_true())
    {
        return;
    }
    let active = v8::Boolean::new(scope, true);
    receiver.set_private(scope, key, active.into());
    if let Some(context) = receiver.get_creation_context(scope) {
        let context_scope = &mut v8::ContextScope::new(scope, context);
        dispatch_synthetic_with_trust(context_scope, h, "click", true, true, trusted);
    }
    let inactive = v8::Boolean::new(scope, false);
    receiver.set_private(scope, key, inactive.into());
}
fn el_scroll_into_view(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    receiver_handle(scope, &args, &[1]);
}

// Form-control methods with no headless effect (no constraint-validation UI, no text
// selection). They must EXIST though: React's hydration/commit calls `setCustomValidity`
// / `setSelectionRange` on inputs, and without the method the NON_MASKING interceptor
// returns `undefined` → "x is not a function" throws mid-hydration → React discards the
// SSR tree and client-renders the whole root (#425), losing server-rendered content.
fn el_noop(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let _dom = DomScope::for_object(scope, args.this());
    receiver_handle(scope, &args, &[1]);
}

/// Constraint-validation predicates (`checkValidity`/`reportValidity`) → always valid
/// (there is no interactive validation UI to fail against).
fn el_valid_true(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[1]).is_none() {
        return;
    }
    rv.set(v8::Boolean::new(scope, true).into());
}
fn el_get_bounding_client_rect(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[1]).is_none() {
        return;
    }
    let o = v8::Object::new(scope);
    for k in [
        "x", "y", "top", "left", "right", "bottom", "width", "height",
    ] {
        let key = v8::String::new(scope, k).unwrap();
        let zero = v8::Number::new(scope, 0.0);
        o.set(scope, key.into(), zero.into());
    }
    rv.set(o.into());
}

/// `append(...nodes)` / `prepend(...)` / `remove()` (DOM ChildNode/ParentNode mixins).
fn el_append<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    parent_insert(scope, &args, false);
}
fn el_prepend<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    parent_insert(scope, &args, true);
}
fn parent_insert<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
    prepend: bool,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(parent) = receiver_handle(scope, args, &[1, 9, 11]) else {
        return;
    };
    let Some(nodes) = resolve_nodes(scope, args, 0) else {
        return;
    };
    let Some(nodes) = dom_mutations::convert_variadic(scope, &nodes) else {
        return;
    };
    let reference = if prepend {
        with_tree(|tree| tree.first_child(parent)).flatten()
    } else {
        None
    };
    dom_mutations::insert_inputs(scope, parent, &nodes, reference, &[]);
}
fn el_remove_self(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1, 3, 4, 7, 8, 10]) else {
        return;
    };
    let parent = with_tree(|t| NodeRef::new(t, h).parent().map(|p| p.handle())).flatten();
    if let Some(p) = parent {
        html_popover::disconnect(scope, h, true);
        range_live::removing(scope, h);
        with_tree_mut(|t| t.remove_child(p, h));
        form_checked_state::owner_changes();
    }
}

// document native methods (reliable — no JS-bootstrap dependency).
fn doc_create_element_ns<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[9]).is_none() {
        return;
    }
    if !require_arguments(scope, &args, 2) {
        return;
    }
    let Some(namespace) = namespace_units_argument(scope, &args) else {
        return;
    };
    let Some(qualified) = dom_units(scope, args.get(1)) else {
        return;
    };
    if let Err(error) = validate_namespace_name(
        namespace
            .as_deref()
            .map(String::from_utf16_lossy)
            .as_deref(),
        &String::from_utf16_lossy(&qualified),
        false,
    ) {
        throw_dom_error(scope, error, "Invalid namespace or qualified name");
        return;
    }
    let made =
        with_tree_mut(|tree| tree.create_element_namespace_units(namespace.as_deref(), &qualified));
    if let Some(h) = made {
        let node = wrap(scope, h);
        rv.set(node.into());
    }
}
fn doc_create_fragment(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[9]).is_none() {
        return;
    }
    if let Some(h) = with_tree_mut(|t| t.create_document_fragment()) {
        let node = wrap(scope, h);
        rv.set(node.into());
    }
}
fn doc_create_comment(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[9]).is_none() {
        return;
    }
    let Some(data) = dom_units(scope, args.get(0)) else {
        return;
    };
    if let Some(h) = with_tree_mut(|t| t.create_comment_utf16(&data)) {
        let node = wrap(scope, h);
        rv.set(node.into());
    }
}

/// Clone native nodes with an explicit traversal stack for deeply nested input.
fn clone_subtree(t: &mut Tree, h: Handle, deep: bool) -> Handle {
    fn shallow(t: &mut Tree, h: Handle) -> Handle {
        let nt = t.node_type_id(h);
        let new_h = match nt {
            2 => {
                let data = t.attribute_data_units(h).unwrap();
                t.create_attribute_units(
                    data.namespace.as_deref(),
                    &data.qualified_name,
                    &t.node_value_utf16(h).unwrap_or_default(),
                )
            }
            3 => t.create_text_node_utf16(&t.node_value_utf16(h).unwrap_or_default()),
            7 => {
                let target = t.processing_instruction_target_utf16(h).unwrap_or_default();
                t.create_processing_instruction_units(
                    &target,
                    &t.node_value_utf16(h).unwrap_or_default(),
                )
            }
            8 => t.create_comment_utf16(&t.node_value_utf16(h).unwrap_or_default()),
            10 => {
                let name = t.doctype_name_utf16(h).unwrap_or_default();
                let public = t.doctype_public_id_utf16(h).unwrap_or_default();
                let system = t.doctype_system_id_utf16(h).unwrap_or_default();
                t.create_document_type_units(&name, &public, &system)
            }
            11 => t.create_document_fragment(),
            _ => {
                let tag = t
                    .qualified_name_utf16(h)
                    .unwrap_or_else(|| "div".encode_utf16().collect());
                let namespace = t.namespace_uri_utf16(h);
                let attributes = t.attributes_units(h);
                let e = t.create_element_namespace_units(namespace.as_deref(), &tag);
                for attribute in attributes {
                    t.set_attribute_ns_units(
                        e,
                        attribute.namespace.as_deref(),
                        &attribute.qualified_name,
                        &attribute.value,
                    );
                }
                // a clone keeps the source's `is` value: undefined until upgraded (turbo-dom#24)
                let state = t.custom_element_copy_state(h);
                if state != turbo_dom_parser::rtdom::tree::CustomElementState::Uncustomized {
                    t.set_custom_element_state(e, state);
                }
                e
            }
        };
        new_h
    }
    let root = shallow(t, h);
    if !deep {
        return root;
    }
    let mut pending = vec![(h, root)];
    while let Some((source, target)) = pending.pop() {
        for child in t.children(source) {
            let copy = shallow(t, child);
            t.append_child(target, copy);
            pending.push((child, copy));
        }
    }
    root
}

/// `getRootNode()` → the topmost ancestor (the document root if connected). jest-dom's
/// `toBeInTheDocument` compares `ownerDocument === getRootNode()`.
fn el_get_root_node(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let composed = v8::Local::<v8::Object>::try_from(args.get(0))
        .ok()
        .is_some_and(|options| {
            let key = v8::String::new(scope, "composed").unwrap();
            options
                .get(scope, key.into())
                .is_some_and(|value| value.boolean_value(scope))
        });
    let root = with_tree(|t| {
        let mut cur = h;
        while let Some(parent) = t
            .parent(cur)
            .or_else(|| composed.then(|| t.shadow_host(cur)).flatten())
        {
            cur = parent;
        }
        cur
    });
    if let Some(r) = root {
        let node = wrap(scope, r);
        rv.set(node.into());
    }
}

fn el_clone_node<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    if with_tree(|t| t.is_shadow_root(h)).unwrap_or(false) {
        throw_dom_error(scope, "NotSupportedError", "A shadow root cannot be cloned");
        return;
    }
    let deep = args.get(0).boolean_value(scope);
    if let Some(new_h) = with_tree_mut(|t| clone_subtree(t, h, deep)) {
        document_basics::clone_nonces(h, new_h, deep);
        let source = wrap(scope, h);
        let clone = wrap(scope, new_h);
        script_state::copy(scope, source, clone, deep);
        template_contents::copy_contents(scope, source, clone, deep);
        let node = wrap(scope, new_h);
        // Carry over the source's JS prototype so a cloned control element keeps its interface
        // accessors (type/value/checked/selected). createElement applies these protos, but cloneNode
        // bypasses it; without this a cloned <input type=date> loses date-value validation, which
        // userEvent's isValidDateOrTimeValue (clone, assign, check it stuck) relies on.
        let src_proto = args.this().get_prototype(scope);
        if let Some(p) = src_proto {
            node.set_prototype(scope, p);
        }
        copy_control_state(scope, args.this(), node, deep);
        custom_elements::cloned(scope, source, node, deep);
        shadow_html::clone_roots(scope, source, node, deep);
        rv.set(node.into());
    }
}

/// whether `h`'s root (walking parents) is the document root → attached to the document.
fn is_connected(t: &Tree, h: Handle) -> bool {
    let root = t.root();
    let mut cur = Some(h);
    while let Some(c) = cur {
        if c == root {
            return true;
        }
        cur = t.parent(c).or_else(|| t.shadow_host(c));
    }
    false
}

fn get_is_connected(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let c = with_tree(|t| is_connected(t, h)).unwrap_or(false);
    rv.set(v8::Boolean::new(scope, c).into());
}

/// `parent.contains(node)` — true if node === parent or a descendant.
fn el_contains_node(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(p) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let other = arg_handle(scope, &args, 0);
    let yes = match other {
        Some(o) => with_tree(|t| {
            let mut cur = Some(o);
            while let Some(c) = cur {
                if c == p {
                    return true;
                }
                cur = NodeRef::new(t, c).parent().map(|x| x.handle());
            }
            false
        })
        .unwrap_or(false),
        None => false,
    };
    rv.set(v8::Boolean::new(scope, yes).into());
}

/// Native Attr snapshot backing the public live NamedNodeMap projection.
fn get_attributes(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let attrs = with_tree_mut(|tree| {
        let records = tree.attributes_units(h);
        records
            .into_iter()
            .filter_map(|data| {
                tree.attribute_node_units(h, data.namespace.as_deref(), &data.local_name)
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    let arr = node_array(scope, attrs);
    // NamedNodeMap accessors (`attributes.item(i)` / `attributes.namedItem(name)`).
    bind_method(scope, arr.into(), "item", nodelist_item);
    bind_method(scope, arr.into(), "namedItem", nodelist_named_item);
    rv.set(arr.into());
}

/// `dataset` → `{ camelCaseKey: value }` for each `data-*` attribute.
fn get_dataset(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let attrs = with_tree(|tree| tree.attributes_units(h)).unwrap_or_default();
    let o = v8::Object::new(scope);
    for attribute in attrs {
        if let Some(rest) = attribute
            .qualified_name
            .strip_prefix(&[100, 97, 116, 97, 45])
        {
            let mut camel = Vec::new();
            let mut index = 0;
            while index < rest.len() {
                if rest[index] == 45
                    && rest
                        .get(index + 1)
                        .is_some_and(|unit| (97..=122).contains(unit))
                {
                    camel.push(rest[index + 1] - 32);
                    index += 2;
                } else {
                    camel.push(rest[index]);
                    index += 1;
                }
            }
            let key = unit_string(scope, &camel);
            let value = unit_string(scope, &attribute.value);
            o.set(scope, key.into(), value.into());
        }
    }
    rv.set(o.into());
}

/// `closest(selector)` → nearest ancestor (incl. self) that matches, or null.
fn el_closest<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let Some((sel, units, unit_path)) = selector_argument(scope, &args, h) else {
        return;
    };
    let found = with_tree(|t| {
        let mut cur = Some(h);
        while let Some(c) = cur {
            if t.node_type_id(c) == 1
                && if unit_path {
                    t.matches_units(c, &units)
                } else {
                    t.matches(c, &sel)
                }
            {
                return Some(c);
            }
            cur = NodeRef::new(t, c).parent().map(|p| p.handle());
        }
        None
    })
    .flatten();
    let v = wrap_opt(scope, found);
    rv.set(v);
}

fn el_matches<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let Some((sel, units, unit_path)) = selector_argument(scope, &args, h) else {
        return;
    };
    let m = with_tree(|t| {
        if unit_path {
            t.matches_units(h, &units)
        } else {
            t.matches(h, &sel)
        }
    })
    .unwrap_or(false);
    rv.set(v8::Boolean::new(scope, m).into());
}

fn el_contains(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let other = arg_handle(scope, &args, 0);
    let contains = match other {
        Some(o) => with_tree(|t| {
            // walk up from `o` to see if `h` is an ancestor (or equal).
            let mut cur = Some(o);
            while let Some(c) = cur {
                if c == h {
                    return true;
                }
                cur = NodeRef::new(t, c).parent().map(|p| p.handle());
            }
            false
        })
        .unwrap_or(false),
        None => false,
    };
    rv.set(v8::Boolean::new(scope, contains).into());
}

/// `node.hasChildNodes()` — true when the node has at least one child. Without this
/// native method the NON_MASKING interceptor returns `undefined`, so React's
/// hydration/commit path (which calls `hasChildNodes()` while diffing) throws
/// "hasChildNodes is not a function" and aborts the client-render fallback.
fn el_has_child_nodes(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let has = with_tree(|t| NodeRef::new(t, h).first_child().is_some()).unwrap_or(false);
    rv.set(v8::Boolean::new(scope, has).into());
}

/// `collection.namedItem(name)` for the array-like NodeList/HTMLCollection we hand
/// back — returns the first element whose `id` (or `name`) matches, else null. Bound
/// alongside `.item` so libs that call `namedItem` (React-DOM, form-collection code)
/// don't hit "namedItem is not a function".
fn nodelist_named_item(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let name = arg_str(scope, &args, 0);
    let this = args.this();
    let len = this
        .get(scope, v8::String::new(scope, "length").unwrap().into())
        .and_then(|v| v.uint32_value(scope))
        .unwrap_or(0);
    for i in 0..len {
        let Some(item) = this.get_index(scope, i) else {
            continue;
        };
        let Ok(obj) = v8::Local::<v8::Object>::try_from(item) else {
            continue;
        };
        for attr in ["id", "name"] {
            let key = v8::String::new(scope, attr).unwrap();
            if let Some(val) = obj.get(scope, key.into()) {
                if val.is_string() && val.to_rust_string_lossy(scope) == name {
                    rv.set(item);
                    return;
                }
            }
        }
    }
    rv.set_null();
}

fn el_query_selector_all<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[1, 11]) else {
        return;
    };
    let Some((sel, units, unit_path)) = selector_argument(scope, &args, h) else {
        return;
    };
    let handles = with_tree(|t| {
        if unit_path {
            t.query_selector_all_from_units(h, &units).to_vec()
        } else {
            NodeRef::new(t, h)
                .query_selector_all(&sel)
                .into_iter()
                .map(|n| n.handle())
                .collect::<Vec<_>>()
        }
    })
    .unwrap_or_default();
    let arr = node_array(scope, handles);
    bind_method(scope, arr.into(), "item", nodelist_item);
    bind_method(scope, arr.into(), "namedItem", nodelist_named_item);
    rv.set(arr.into());
}

// ---- element accessors -----------------------------------------------------------------------
// Getter sig: (scope, name, PropertyCallbackArguments, ReturnValue<Value>).
// Setter sig: (scope, name, Local<Value>, PropertyCallbackArguments, ReturnValue<()>).

fn get_tag_name(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let tag = with_tree(|t| t.tag_name_utf16(h))
        .flatten()
        .unwrap_or_default();
    rv.set(unit_string(scope, &tag).into());
}

fn get_text_content(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let txt = with_tree(|t| t.text_content_utf16(h)).unwrap_or_default();
    rv.set(unit_string(scope, &txt).into());
}

fn set_text_content<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    _name: v8::Local<'s, v8::Name>,
    value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    _rv: v8::ReturnValue<()>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let Some(units) = trusted_types::script_units(scope, args.holder(), value, "textContent", true)
    else {
        return;
    };
    let txt = String::from_utf16_lossy(&units);
    html_popover::attribute_node_change(scope, h, Some(&txt));
    html_popover::disconnect(scope, h, false);
    if with_tree(|tree| matches!(tree.node_type_id(h), 3 | 4 | 7 | 8)).unwrap_or(false) {
        range_live::replacing_data(scope, h, 0, character_units(h).len(), units.len());
    } else {
        range_live::replacing_children(scope, h);
    }
    with_tree_mut(|t| t.set_text_content_utf16(h, &units));
    element_relations::attribute_node_changed(scope, h);
    css_policy::invalidate_sheet(h);
}

fn get_id(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let id = with_tree(|t| t.get_attribute_utf16(h, "id"))
        .flatten()
        .unwrap_or_default();
    rv.set(unit_string(scope, &id).into());
}

fn set_id(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    value: v8::Local<v8::Value>,
    args: v8::PropertyCallbackArguments,
    _rv: v8::ReturnValue<()>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let Some(v) = dom_units(scope, value) else {
        return;
    };
    with_tree_mut(|t| t.set_attribute_utf16(h, "id", &v));
    let node = wrap(scope, h);
    element_relations::attribute_changed(scope, node, "id");
}

fn get_class_name(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let c = with_tree(|t| t.get_attribute_utf16(h, "class"))
        .flatten()
        .unwrap_or_default();
    rv.set(unit_string(scope, &c).into());
}

fn set_class_name(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    value: v8::Local<v8::Value>,
    args: v8::PropertyCallbackArguments,
    _rv: v8::ReturnValue<()>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let Some(v) = dom_units(scope, value) else {
        return;
    };
    with_tree_mut(|t| t.set_attribute_utf16(h, "class", &v));
}

// `element.contentEditable` IDL property — reflects the `contenteditable` content attribute as the
// enumerated string 'true' | 'false' | 'plaintext-only' | 'inherit'. Lexical reads this property
// (NOT the attribute) to decide the root is editable and to wire up its beforeinput pipeline; with
// it missing (undefined) Lexical treats the editor as non-editable and never emits onChange.
fn get_content_editable(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let attr = with_tree(|t| attr_get(t, h, "contenteditable")).flatten();
    let out = match attr.as_deref() {
        Some("") | Some("true") => "true",
        Some("false") => "false",
        Some("plaintext-only") => "plaintext-only",
        _ => "inherit",
    };
    rv.set(v8::String::new(scope, out).unwrap().into());
}

fn set_content_editable(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    value: v8::Local<v8::Value>,
    args: v8::PropertyCallbackArguments,
    _rv: v8::ReturnValue<()>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let v = value.to_rust_string_lossy(scope);
    with_tree_mut(|t| t.set_attribute(h, "contenteditable", &v));
}

// `element.isContentEditable` — true when the element itself is editable (contenteditable=""|"true")
// or it inherits editability from an ancestor.
fn get_is_content_editable(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(mut h) = handle_of(scope, args.holder()) else {
        return;
    };
    let mut editable = false;
    loop {
        match with_tree(|t| attr_get(t, h, "contenteditable"))
            .flatten()
            .as_deref()
        {
            Some("") | Some("true") | Some("plaintext-only") => {
                editable = true;
                break;
            }
            Some("false") => {
                editable = false;
                break;
            }
            _ => {}
        }
        match with_tree(|t| {
            NodeRef::new(t, h)
                .parent()
                .map(|p| p.handle())
                .filter(|&ph| t.node_type_id(ph) == 1)
        })
        .flatten()
        {
            Some(p) => h = p,
            None => break,
        }
    }
    rv.set(v8::Boolean::new(scope, editable).into());
}

fn get_parent_node(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let found = with_tree(|t| NodeRef::new(t, h).parent().map(|x| x.handle())).flatten();
    let v = wrap_opt(scope, found);
    rv.set(v);
}

fn get_first_child(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let found = with_tree(|t| NodeRef::new(t, h).first_child().map(|x| x.handle())).flatten();
    let v = wrap_opt(scope, found);
    rv.set(v);
}

fn get_next_sibling(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let found = with_tree(|t| NodeRef::new(t, h).next_sibling().map(|x| x.handle())).flatten();
    let v = wrap_opt(scope, found);
    rv.set(v);
}

// ---- events: real dispatch (addEventListener/removeEventListener/dispatchEvent) --------------
// React attaches one delegated listener at the root container and relies on bubbling; fireEvent /
// userEvent call `node.dispatchEvent(new Event(type, {bubbles:true}))`. So we store JS listeners
// per (handle, type) and walk target→ancestors firing them, with target/currentTarget set on the
// event object (the `Event` class lives in the JS bootstrap).

thread_local! {
    /// (node handle, event type) → registered JS listeners (function, capture-phase flag).
    static LISTENERS: RefCell<HashMap<(Handle, String), Vec<event_listeners::Listener>>> = RefCell::new(HashMap::new());
}

fn get_prop<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let key = v8::String::new(scope, name)?;
    object.get(scope, key.into())
}
fn listener_options<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments,
    remove: bool,
) -> Option<v8::Local<'s, v8::Object>> {
    if args.length() < 2 {
        let message =
            v8::String::new(scope, "Event listener registration requires 2 arguments").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return None;
    }
    let top = with_tree(|tree| {
        handle_of(scope, args.this()).is_some_and(|node| {
            tree.node_type_id(node) == 9 || tree.local_name(node) == Some("body")
        })
    })
    .unwrap_or(false);
    event_listeners::options(scope, args, remove, top)
}
fn el_add_event_listener<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let Some(options) = listener_options(scope, &args, false) else {
        return;
    };
    let ty = get_str_prop(scope, options, "type").unwrap_or_default();
    let Some(callback) = get_prop(scope, options, "callback")
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return;
    };
    let capture = get_bool_prop(scope, options, "capture");
    let once = get_bool_prop(scope, options, "once");
    let passive = get_bool_prop(scope, options, "passive");
    let signal = get_prop(scope, options, "signal")
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        .map(|value| v8::Weak::new(scope, value));
    if signal
        .as_ref()
        .is_some_and(|signal| event_listeners::aborted(scope, signal))
    {
        return;
    }
    event_handlers::refresh(scope, node, &format!("on{ty}"));
    let duplicate = LISTENERS.with(|listeners| {
        let mut listeners = listeners.borrow_mut();
        let list = listeners.entry((node, ty.clone())).or_default();
        list.retain(|listener| !listener.removed.get());
        list.iter().any(|listener| {
            listener.capture == capture
                && dom_ownership::local(scope, &listener.callback).eq(&callback)
        })
    });
    if duplicate {
        return;
    }
    let signal_local = signal.as_ref().and_then(|signal| signal.to_local(scope));
    let Some(id) = event_listeners::own_listener(scope, args.this(), callback, signal_local) else {
        return;
    };
    let mut listener = event_listeners::Listener {
        callback: v8::Weak::new(scope, callback),
        id,
        node: v8::Weak::new(scope, args.this()),
        capture,
        once,
        passive,
        signal,
        removed: std::rc::Rc::new(std::cell::Cell::new(false)),
        abort_hook: None,
    };
    event_listeners::attach(scope, args.this(), &ty, &mut listener);
    LISTENERS.with(|listeners| {
        listeners
            .borrow_mut()
            .entry((node, ty))
            .or_default()
            .push(listener)
    });
}
fn el_remove_event_listener(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[]) else {
        return;
    };
    let Some(options) = listener_options(scope, &args, true) else {
        return;
    };
    let ty = get_str_prop(scope, options, "type").unwrap_or_default();
    let Some(callback) = get_prop(scope, options, "callback")
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return;
    };
    let capture = get_bool_prop(scope, options, "capture");
    let removed = LISTENERS.with(|listeners| {
        let mut removed = Vec::new();
        if let Some(list) = listeners.borrow_mut().get_mut(&(node, ty)) {
            list.retain(|listener| {
                let matching = listener.capture == capture
                    && dom_ownership::local(scope, &listener.callback).eq(&callback);
                if matching {
                    listener.removed.set(true);
                    removed.push(listener.clone());
                }
                !matching
            });
        }
        removed
    });
    for listener in removed {
        event_listeners::detach(scope, &listener);
    }
}

struct EventRecordAccessor(v8::Weak<v8::Function>);
struct HostEventDispatcher(v8::Weak<v8::Function>);

fn event_record<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    event: v8::Local<v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let context = event.get_creation_context(scope)?;
    let accessor = context.get_slot::<EventRecordAccessor>()?;
    let function = dom_ownership::local(scope, &accessor.0);
    let receiver = v8::undefined(scope).into();
    v8::Local::<v8::Object>::try_from(function.call(scope, receiver, &[event.into()])?).ok()
}

fn lookup_event_record(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(event) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    if let Some(record) = event_record(scope, event) {
        rv.set(record.into());
    }
}

/// Mark an engine-created event before callbacks inspect it. Script dispatch
/// always clears trust; host dispatch uses a separate private entry point.
pub fn mark_event_trusted(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
    timestamp: Option<f64>,
) {
    if let Some(record) = event_record(scope, event) {
        let yes = v8::Boolean::new(scope, true);
        set_prop(scope, record, "trusted", yes.into());
        if let Some(timestamp) = timestamp {
            let timestamp = v8::Number::new(scope, timestamp);
            set_prop(scope, record, "timeStamp", timestamp.into());
        }
    }
}

/// Dispatch an engine-originated event without invoking a guest override of
/// dispatchEvent. Only the host can obtain this dispatch capability.
pub fn dispatch_host_event(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
    target: v8::Local<v8::Object>,
) {
    if handle_of(scope, target).is_some() {
        let host = v8::Boolean::new(scope, true);
        if let Some(function) = v8::FunctionTemplate::builder(el_dispatch_event)
            .data(host.into())
            .constructor_behavior(v8::ConstructorBehavior::Throw)
            .build(scope)
            .get_function(scope)
        {
            function.call(scope, target.into(), &[event.into()]);
        }
    } else if let Some(context) = target.get_creation_context(scope) {
        if let Some(dispatcher) = context.get_slot::<HostEventDispatcher>() {
            let function = dom_ownership::local(scope, &dispatcher.0);
            let receiver = v8::undefined(scope).into();
            function.call(scope, receiver, &[event.into(), target.into()]);
        }
    }
}

/// Enter or leave an engine-delivered callback on a non-DOM target.
pub fn event_callback_state(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
    target: v8::Local<v8::Object>,
    active: bool,
) {
    let Some(record) = event_record(scope, event) else {
        return;
    };
    let flag = v8::Boolean::new(scope, active);
    let phase = v8::Integer::new(scope, if active { 2 } else { 0 });
    let current: v8::Local<v8::Value> = if active {
        target.into()
    } else {
        v8::null(scope).into()
    };
    let values = if active {
        vec![target.into()]
    } else {
        Vec::new()
    };
    let path = v8::Array::new_with_elements(scope, &values);
    if active {
        set_prop(scope, record, "target", target.into());
        let trusted = v8::Boolean::new(scope, true);
        set_prop(scope, record, "trusted", trusted.into());
    }
    set_prop(scope, record, "currentTarget", current);
    set_prop(scope, record, "eventPhase", phase.into());
    set_prop(scope, record, "path", path.into());
    set_prop(scope, record, "dispatching", flag.into());
    if !active {
        for key in ["stop", "stopImmediate"] {
            set_prop(scope, record, key, flag.into());
        }
    }
}

fn el_dispatch_event(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(target) = receiver_handle(scope, &args, &[]) else {
        rv.set(v8::Boolean::new(scope, true).into());
        return;
    };
    let event = args.get(0);
    let Ok(event_obj) = v8::Local::<v8::Object>::try_from(event) else {
        let message = v8::String::new(scope, "dispatchEvent requires an Event").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    };
    let Some(record) = event_record(scope, event_obj) else {
        let message = v8::String::new(scope, "Argument must be an Event").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    };
    let ty = get_str_prop(scope, record, "type").unwrap_or_default();
    if ty.is_empty() || get_bool_prop(scope, record, "dispatching") {
        throw_dom_error(
            scope,
            "InvalidStateError",
            "The event is already being dispatched or uninitialized",
        );
        return;
    }
    event_handlers::refresh(scope, target, &format!("on{ty}"));
    let bubbles = get_bool_prop(scope, record, "bubbles");
    let trusted = args.data().is_true();
    let yes = v8::Boolean::new(scope, true);
    let no = v8::Boolean::new(scope, false);
    let trusted = v8::Boolean::new(scope, trusted);
    set_prop(scope, record, "dispatching", yes.into());
    set_prop(scope, record, "trusted", trusted.into());
    let target_node = wrap(scope, target);
    set_prop(scope, record, "target", target_node.into());

    // PRE-CLICK ACTIVATION: a checkbox/radio toggles `checked` BEFORE the click listeners run (per
    // the activation-behavior spec), so React's ChangeEventPlugin — which detects checkbox change on
    // the `click` event by reading node.checked — sees the new value. Reverted below if the click is
    // canceled. `change`/`input` DOM events fire afterward (also below).
    // Only a MouseEvent click has an activation target (the target or a bubbling ancestor).
    let activation_target = if ty == "click" {
        html_invokers::activation_target(scope, event_obj, target)
    } else {
        None
    };
    let checked_target = activation_target.unwrap_or(target);
    let click_check = activation_target.and_then(checkable_kind);
    let check_snapshot = click_check.map(|_| form_checked_state::snapshot(checked_target));
    let check_was = click_check.map(|_| form_checked_state::checked(checked_target));
    let indeterminate_was = (click_check == Some(CheckKind::Checkbox))
        .then(|| input_members::pre_activation(checked_target));
    if let (Some(kind), Some(was)) = (click_check, check_was) {
        let next = if kind == CheckKind::Radio { true } else { !was };
        form_checked_state::set(checked_target, next);
    }

    // propagation path: ancestors (root … parent), then the target. Bubbling reverses it.
    let mut ancestors = Vec::new();
    let mut cur = with_tree(|t| NodeRef::new(t, target).parent().map(|p| p.handle())).flatten();
    while let Some(c) = cur {
        ancestors.push(c);
        cur = with_tree(|t| NodeRef::new(t, c).parent().map(|p| p.handle())).flatten();
    }

    let window = if ty != "load"
        && (ancestors
            .last()
            .is_some_and(|node| with_tree(|tree| tree.node_type_id(*node)) == Some(9))
            || with_tree(|tree| tree.node_type_id(target)) == Some(9))
    {
        event_listeners::window(scope)
    } else {
        None
    };
    let mut values = Vec::with_capacity(ancestors.len() + 2);
    values.push(target_node.into());
    for &handle in &ancestors {
        values.push(wrap(scope, handle).into());
    }
    if let Some(window) = window {
        values.push(window.into());
    }
    let path = v8::Array::new_with_elements(scope, &values);
    set_prop(scope, record, "path", path.into());

    // CAPTURE phase: root → target's parent (capture listeners), then TARGET (both), then BUBBLE:
    // target's parent → root (non-capture listeners). React 18 registers some delegated root
    // listeners in capture phase, so a bubble-only dispatch would miss them.
    let mut capture_path: Vec<Handle> = ancestors.iter().rev().copied().collect();
    let bubble_path: Vec<Handle> = if bubbles {
        ancestors.clone()
    } else {
        Vec::new()
    };
    capture_path.push(target); // target fires in both phases (handled by the at_target flag below)

    let fire =
        |scope: &mut v8::PinScope, node_h: Handle, want_capture: bool, at_target: bool| -> bool {
            if get_bool_prop(scope, record, "stop") && !at_target {
                return false;
            }
            let listeners: Vec<event_listeners::Listener> = LISTENERS.with(|m| {
                m.borrow()
                    .get(&(node_h, ty.clone()))
                    .cloned()
                    .unwrap_or_default()
            });
            let cur_node = wrap(scope, node_h);
            set_prop(scope, record, "currentTarget", cur_node.into());
            let phase = v8::Integer::new(
                scope,
                if at_target {
                    2
                } else if want_capture {
                    1
                } else {
                    3
                },
            );
            set_prop(scope, record, "eventPhase", phase.into());
            for listener in listeners {
                // at the target, both capture and bubble listeners fire; elsewhere only the phase's.
                if listener.capture != want_capture || listener.removed.get() {
                    continue;
                }
                if get_bool_prop(scope, record, "stopImmediate") {
                    break;
                }
                if listener
                    .signal
                    .as_ref()
                    .is_some_and(|signal| event_listeners::aborted(scope, signal))
                {
                    listener.removed.set(true);
                    continue;
                }
                let Some(callback) = listener.callback.to_local(scope) else {
                    continue;
                };
                if listener.once {
                    listener.removed.set(true);
                    LISTENERS.with(|listeners| {
                        if let Some(list) = listeners.borrow_mut().get_mut(&(node_h, ty.clone())) {
                            list.retain(|entry| {
                                !std::rc::Rc::ptr_eq(&entry.removed, &listener.removed)
                            });
                        }
                    });
                    event_listeners::detach(scope, &listener);
                }
                let passive = v8::Boolean::new(scope, listener.passive);
                set_prop(scope, record, "passive", passive.into());
                let scope = std::pin::pin!(v8::TryCatch::new(scope));
                let scope = &mut scope.init();
                if let Ok(function) = v8::Local::<v8::Function>::try_from(callback) {
                    function.call(scope, cur_node.into(), &[event]);
                } else if let Some(function) = get_prop(scope, callback, "handleEvent")
                    .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
                {
                    function.call(scope, callback.into(), &[event]);
                }
                let passive = v8::Boolean::new(scope, false);
                set_prop(scope, record, "passive", passive.into());
                if scope.is_execution_terminating() {
                    break;
                }
            }

            true
        };

    if window.is_some() {
        event_listeners::phase(scope, event_obj, true);
    }
    // capture: root → parent
    for &node_h in capture_path
        .iter()
        .take(capture_path.len().saturating_sub(1))
    {
        if !fire(scope, node_h, true, false) {
            break;
        }
    }
    // at target (both phases)
    if !get_bool_prop(scope, record, "stop") {
        fire(scope, target, true, true);
        fire(scope, target, false, true);
    }
    // bubble: parent → root
    for node_h in bubble_path {
        if !fire(scope, node_h, false, false) {
            break;
        }
    }
    if window.is_some() && bubbles && !get_bool_prop(scope, record, "stop") {
        event_listeners::phase(scope, event_obj, false);
    }
    let not_prevented = !get_bool_prop(scope, record, "defaultPrevented");
    let null = v8::null(scope);
    let phase = v8::Integer::new(scope, 0);
    let path = v8::Array::new(scope, 0);
    set_prop(scope, record, "currentTarget", null.into());
    set_prop(scope, record, "eventPhase", phase.into());
    set_prop(scope, record, "path", path.into());
    for key in ["dispatching", "stop", "stopImmediate", "passive"] {
        set_prop(scope, record, key, no.into());
    }

    // DEFAULT ACTION: a non-prevented click on a submit control fires the form's `submit` event
    // (jsdom does this natively; userEvent/fireEvent.click rely on it for form submission).
    let submission_blocked = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|dom| dom.form_submission_blocked)
    });
    let submitter = activation_target
        .filter(|&node| is_submit_control(node) && !html_invokers::disabled_control(node));
    if let (true, Some(submitter), false) = (not_prevented, submitter, submission_blocked) {
        if let Some(form) = closest_form(submitter) {
            let form = wrap(scope, form);
            let submitter = wrap(scope, submitter);
            if let Some(context) = form.get_creation_context(scope) {
                if let Some(dispatcher) = context.get_slot::<FormSubmissionDispatcher>() {
                    let dispatcher = dom_ownership::local(scope, &dispatcher.0);
                    let receiver = v8::undefined(scope);
                    v8::tc_scope!(let call_scope, scope);
                    dispatcher.call(
                        call_scope,
                        receiver.into(),
                        &[form.into(), submitter.into()],
                    );
                    if !call_scope.has_terminated() {
                        if let Some(error) = call_scope.exception() {
                            call_scope.reset();
                            custom_elements::report_value(call_scope, error);
                        }
                    }
                }
            }
        }
    }

    if let (true, Some(activation)) = (not_prevented, activation_target) {
        html_invokers::activate(scope, event_obj, activation, target);
    }
    if args.data().is_true() {
        interest_invokers::observe(scope, event_obj, target, &ty);
        if not_prevented && ty == "keydown" {
            focus_group::keydown(scope, event_obj, target);
        }
    }

    // Finish the checkbox/radio activation: a canceled click reverts the pre-toggle; otherwise fire
    // the DOM `input` + `change` events (jsdom does this; some libs listen for them directly).
    if let (Some(_kind), Some(_was)) = (click_check, check_was) {
        if not_prevented {
            dispatch_synthetic(scope, checked_target, "input", true, false);
            dispatch_synthetic(scope, checked_target, "change", true, false);
        } else if let Some(snapshot) = check_snapshot {
            form_checked_state::restore(snapshot);
            if let Some(previous) = indeterminate_was {
                input_members::cancel_activation(checked_target, previous);
            }
        }
    }

    rv.set(v8::Boolean::new(scope, not_prevented).into());
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum CheckKind {
    Checkbox,
    Radio,
}

/// `<input type=checkbox|radio>` → its kind, else None (drives the click toggle default action).
fn checkable_kind(h: Handle) -> Option<CheckKind> {
    with_tree(|t| {
        if t.node_type_id(h) != 1
            || !t
                .tag_name(h)
                .unwrap_or_default()
                .eq_ignore_ascii_case("INPUT")
        {
            return None;
        }
        match t
            .get_attribute(h, "type")
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("checkbox") => Some(CheckKind::Checkbox),
            Some("radio") => Some(CheckKind::Radio),
            _ => None,
        }
    })
    .flatten()
}

/// A `<button>` (type submit or unset — submit is the default) or `<input type=submit>`.
fn is_submit_control(h: Handle) -> bool {
    with_tree(|t| {
        if t.node_type_id(h) != 1 {
            return false;
        }
        let tag = t.tag_name(h).unwrap_or_default().to_ascii_uppercase();
        let ty = t.get_attribute(h, "type").map(|s| s.to_ascii_lowercase());
        match tag.as_str() {
            "BUTTON" => match ty.as_deref() {
                Some("submit") => true,
                Some("button" | "reset") => false,
                // Auto state: an invoker button with a missing/invalid type never submits.
                _ => !t.has_attribute(h, "command") && !t.has_attribute(h, "commandfor"),
            },
            "INPUT" => matches!(ty.as_deref(), Some("submit" | "image")),
            _ => false,
        }
    })
    .unwrap_or(false)
}

fn form_checked_receiver_error(scope: &mut v8::PinScope) {
    let message = v8::String::new(scope, "Illegal invocation").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}
fn form_checked_native(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        form_checked_receiver_error(scope);
        return;
    };
    let _dom = DomScope::for_object(scope, node);
    let Some(handle) = handle_of(scope, node) else {
        form_checked_receiver_error(scope);
        return;
    };
    let valid = with_tree(|tree| {
        tree.namespace_id(handle) == 0 && tree.local_name(handle) == Some("input")
    })
    .unwrap_or(false);
    if !valid {
        form_checked_receiver_error(scope);
        return;
    }
    let operation = args.get(0).int32_value(scope).unwrap_or(0);
    let prop = turbo_dom_parser::rtdom::tree::FormProp::Checked;
    if operation == 1 {
        let value = args.get(2).boolean_value(scope);
        form_checked_state::set(handle, value);
    }
    if operation == 2 {
        with_tree_mut(|tree| tree.clear_form_property(handle, prop));
        form_checked_state::group(handle);
    }
    if operation == 4 {
        form_checked_state::group(handle);
    }
    if operation == 3 {
        rv.set_bool(with_tree(|tree| tree.form_property(handle, prop).is_some()).unwrap_or(false));
        return;
    }
    rv.set_bool(
        with_tree(|tree| {
            tree.form_property(handle, prop)
                .unwrap_or_else(|| tree.get_attribute(handle, "checked").is_some())
        })
        .unwrap_or(false),
    );
}

// Chromium retains a canonical serialization even when a special URL is invalid.
fn invalid_form_action(raw: &str) -> String {
    let Some((scheme, rest)) = raw.split_once("://") else {
        return raw.to_owned();
    };
    if !matches!(
        scheme.to_ascii_lowercase().as_str(),
        "http" | "https" | "ftp" | "ws" | "wss"
    ) {
        return raw.to_owned();
    }
    if rest.is_empty() {
        return format!("{}:", scheme.to_ascii_lowercase());
    }
    let split = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, suffix) = rest.split_at(split);
    let bytes = authority.as_bytes();
    let mut result = format!("{}://", scheme.to_ascii_lowercase());
    for (index, ch) in authority.char_indices() {
        if ch == ' ' {
            result.push_str("%20");
        } else if ch == '%'
            && !(bytes.get(index + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(index + 2).is_some_and(u8::is_ascii_hexdigit))
        {
            result.push_str("%25");
        } else {
            result.push(ch);
        }
    }
    if !suffix.starts_with('/') {
        result.push('/');
    }
    result.push_str(suffix);
    result
}
fn form_controls_native(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    if operation == "install-labels" || operation == "labels-list" {
        let Some(context) = node.get_creation_context(scope) else {
            return;
        };
        let scope = &mut v8::ContextScope::new(scope, context);
        let global = context.global(scope);
        let name = v8::String::new(scope, "turbo.forms.labels-factory").unwrap();
        let key = v8::Private::for_api(scope, Some(name));
        if operation == "install-labels" {
            global.set_private(scope, key, args.get(2));
        } else if let Some(callback) = global
            .get_private(scope, key)
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        {
            let receiver = v8::undefined(scope);
            if let Some(value) = callback.call(scope, receiver.into(), &[node.into()]) {
                rv.set(value);
            }
        }
        return;
    }
    if operation == "private" {
        let family = args.get(2).to_rust_string_lossy(scope);
        let name = v8::String::new(scope, &format!("turbo.forms.private.{family}")).unwrap();
        let key = v8::Private::for_api(scope, Some(name));
        if !args.get(3).is_undefined() {
            node.set_private(scope, key, args.get(3));
        }
        if let Some(value) = node.get_private(scope, key) {
            rv.set(value);
        }
        return;
    }
    let _dom = DomScope::for_object(scope, node);
    let Some(handle) = handle_of(scope, node) else {
        return;
    };
    match operation.as_str() {
        "labels-hidden" => {
            rv.set(v8::Boolean::new(scope, control_labels::hidden(handle)).into());
        }
        "labels-rows" => {
            let rows = control_labels::labels(scope, handle);
            let array = v8::Array::new(scope, rows.len() as i32);
            for (index, row) in rows.into_iter().enumerate() {
                let value = wrap(scope, row);
                array.set_index(scope, index as u32, value.into());
            }
            rv.set(array.into());
        }
        "label-control" => {
            if let Some(target) = control_labels::control(scope, handle) {
                rv.set(wrap(scope, target).into());
            } else {
                rv.set(v8::null(scope).into());
            }
        }
        "associated" => {
            let name = v8::String::new(scope, "turbo.forms.associated-record").unwrap();
            let key = v8::Private::for_api(scope, Some(name));
            if !args.get(2).is_undefined() {
                node.set_private(scope, key, args.get(2));
            }
            if let Some(value) = node.get_private(scope, key) {
                rv.set(value);
            }
        }
        "html-element" => {
            rv.set(
                v8::Boolean::new(
                    scope,
                    with_tree(|tree| {
                        tree.node_type_id(handle) == 1 && tree.namespace_id(handle) == 0
                    })
                    .unwrap_or(false),
                )
                .into(),
            );
        }
        "owner" => {
            if let Some(form) = closest_form(handle) {
                rv.set(wrap(scope, form).into());
            } else {
                rv.set(v8::null(scope).into());
            }
        }
        "value-snapshot" => {
            let id = DOM_ID.with(std::cell::Cell::get);
            let context = DOM_CONTEXTS.with(|contexts| {
                contexts
                    .borrow()
                    .get(&id)
                    .and_then(|context| context.to_local(scope))
            });
            let Some(context) = context else {
                rv.set(v8::null(scope).into());
                return;
            };
            let callback = context
                .get_slot::<FormValueReader>()
                .and_then(|reader| reader.0.to_local(scope));
            let Some(callback) = callback else {
                rv.set(v8::null(scope).into());
                return;
            };
            let scope = &mut v8::ContextScope::new(scope, context);
            let receiver = v8::undefined(scope);
            if let Some(value) = callback.call(scope, receiver.into(), &[args.get(2)]) {
                rv.set(value);
            }
        }
        "active" | "submit" => {
            let callback = DOM.with(|dom| {
                dom.borrow().as_ref().and_then(|dom| {
                    dom.form_submission_handler
                        .as_ref()
                        .and_then(|callback| callback.to_local(scope))
                })
            });
            let Some(callback) = callback else {
                // Standalone bindings still support validation and canceled submit events.
                rv.set(v8::Boolean::new(scope, operation == "active").into());
                return;
            };
            let Some(context) = callback.get_creation_context(scope) else {
                return;
            };
            let scope = &mut v8::ContextScope::new(scope, context);
            let receiver = v8::undefined(scope);
            let values = [args.get(0), node.into(), args.get(2), args.get(3)];
            if let Some(value) = callback.call(scope, receiver.into(), &values) {
                rv.set(value);
            }
        }
        "action-url" | "submitter-action-url" => {
            let document_url = event_handlers::document_url();
            let raw = with_tree(|tree| {
                tree.get_attribute(
                    handle,
                    if operation == "submitter-action-url" {
                        "formaction"
                    } else {
                        "action"
                    },
                )
                .map(|value| value.to_string())
            })
            .flatten()
            .unwrap_or_default();
            let value = if raw.is_empty() {
                document_url
            } else {
                let base = form_submission_base_href(scope, node)
                    .and_then(|base| url::Url::parse(&document_url).ok()?.join(&base).ok())
                    .or_else(|| url::Url::parse(&document_url).ok());
                base.and_then(|base| base.join(&raw).ok())
                    .map(|url| url.to_string())
                    .unwrap_or_else(|| invalid_form_action(&raw))
            };
            rv.set(v8::String::new(scope, &value).unwrap().into());
        }
        "absolute-url" => {
            let value = args.get(2).to_rust_string_lossy(scope);
            rv.set(v8::Boolean::new(scope, url::Url::parse(&value).is_ok()).into());
        }
        "allowed" => {
            let blocked = DOM.with(|dom| {
                dom.borrow()
                    .as_ref()
                    .is_some_and(|dom| dom.form_submission_blocked)
            });
            rv.set(v8::Boolean::new(scope, !blocked).into());
        }
        "emit" => {
            let Ok(event) = v8::Local::<v8::Object>::try_from(args.get(2)) else {
                return;
            };
            dispatch_host_event(scope, event, node);
            let canceled = event_record(scope, event)
                .is_some_and(|record| get_bool_prop(scope, record, "defaultPrevented"));
            rv.set(v8::Boolean::new(scope, !canceled).into());
        }
        _ => {}
    }
}

/// nearest ancestor `<form>` (incl. self).
fn closest_form(h: Handle) -> Option<Handle> {
    with_tree(|t| {
        if let Some(id) = t.get_attribute_utf16(h, "form") {
            if !is_connected(t, h) || id.is_empty() {
                return None;
            }
            let mut root = h;
            while let Some(parent) = t.parent(root) {
                root = parent;
            }
            let rows = if root == t.root() {
                t.query_selector_all("[id]")
            } else {
                t.query_selector_all_from_units(root, &[91, 105, 100, 93])
            };
            for &candidate in rows.iter() {
                if t.get_attribute_utf16(candidate, "id").as_deref() == Some(id.as_slice()) {
                    return (t.namespace_id(candidate) == 0
                        && t.local_name(candidate) == Some("form"))
                    .then_some(candidate);
                }
            }
            return None;
        }
        let mut cur = Some(h);
        while let Some(c) = cur {
            if t.namespace_id(c) == 0 && t.local_name(c) == Some("form") {
                return Some(c);
            }
            cur = NodeRef::new(t, c).parent().map(|p| p.handle());
        }
        None
    })
    .flatten()
}

/// Construct `new Event(type, {bubbles, cancelable})` and dispatch it on `target` via its native
/// dispatchEvent (re-enters the event system → fires listeners).
fn dispatch_synthetic(
    scope: &mut v8::PinScope,
    target: Handle,
    ty: &str,
    bubbles: bool,
    cancelable: bool,
) {
    dispatch_synthetic_with_trust(scope, target, ty, bubbles, cancelable, true);
}

fn dispatch_synthetic_with_trust(
    scope: &mut v8::PinScope,
    target: Handle,
    ty: &str,
    bubbles: bool,
    cancelable: bool,
    host: bool,
) {
    let global = scope.get_current_context().global(scope);
    let constructor = if ty == "click" {
        "PointerEvent"
    } else {
        "Event"
    };
    let Some(ctor) = v8::String::new(scope, constructor)
        .and_then(|k| global.get(scope, k.into()))
        .and_then(|v| v8::Local::<v8::Function>::try_from(v).ok())
    else {
        return;
    };
    let init = v8::Object::new(scope);
    for (k, val) in [
        ("bubbles", bubbles),
        ("cancelable", cancelable),
        ("composed", ty == "click"),
    ] {
        if let Some(key) = v8::String::new(scope, k) {
            let b = v8::Boolean::new(scope, val);
            init.set(scope, key.into(), b.into());
        }
    }
    if ty == "click" {
        let key = v8::String::new(scope, "pointerId").unwrap();
        let value = v8::Integer::new(scope, -1);
        init.set(scope, key.into(), value.into());
    }
    let ty_str = v8::String::new(scope, ty).unwrap();
    let Some(ev) = ctor.new_instance(scope, &[ty_str.into(), init.into()]) else {
        return;
    };
    let node = wrap(scope, target);
    if host {
        dispatch_host_event(scope, ev, node);
    } else {
        let flag = v8::Boolean::new(scope, false);
        if let Some(function) = v8::FunctionTemplate::builder(el_dispatch_event)
            .data(flag.into())
            .constructor_behavior(v8::ConstructorBehavior::Throw)
            .build(scope)
            .get_function(scope)
        {
            function.call(scope, node.into(), &[ev.into()]);
        }
    }
}

// small prop helpers
fn get_str_prop(
    scope: &mut v8::PinScope,
    obj: v8::Local<v8::Object>,
    name: &str,
) -> Option<String> {
    let key = v8::String::new(scope, name)?;
    let v = obj.get(scope, key.into())?;
    if v.is_string() {
        Some(v.to_rust_string_lossy(scope))
    } else {
        None
    }
}
fn get_bool_prop(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>, name: &str) -> bool {
    v8::String::new(scope, name)
        .and_then(|k| obj.get(scope, k.into()))
        .map(|v| v.boolean_value(scope))
        .unwrap_or(false)
}
fn set_prop(
    scope: &mut v8::PinScope,
    obj: v8::Local<v8::Object>,
    name: &str,
    val: v8::Local<v8::Value>,
) {
    if let Some(k) = v8::String::new(scope, name) {
        obj.set(scope, k.into(), val);
    }
}

/// `constructor` → the named global DOM constructor for this node's type, so libraries keying on
/// `node.constructor.name` (pretty-format's DOMElementFilter regex `/^((HTML|SVG)\w*)?Element$/`)
/// treat it as a DOM node, not a plain Object they'd recurse into and crash on.
fn get_constructor(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let (nt, tag) = with_tree(|t| (t.node_type_id(h), t.tag_name(h))).unwrap_or((1, None));
    let cname = match nt {
        3 => "Text",
        7 => "ProcessingInstruction",
        8 => "Comment",
        9 => "HTMLDocument",
        11 => "DocumentFragment",
        1 => match tag.as_deref().map(|s| s.to_ascii_uppercase()).as_deref() {
            Some("INPUT") => "HTMLInputElement",
            Some("TEXTAREA") => "HTMLTextAreaElement",
            Some("SELECT") => "HTMLSelectElement",
            Some("OPTION") => "HTMLOptionElement",
            // Extra elements whose ctor globals the bootstrap registers — keep `.constructor.name`
            // specific (not generic HTMLElement) so libs keying on it resolve the right interface.
            Some("DIALOG") => "HTMLDialogElement",
            Some("DATALIST") => "HTMLDataListElement",
            Some("FIELDSET") => "HTMLFieldSetElement",
            Some("LEGEND") => "HTMLLegendElement",
            Some("OL") => "HTMLOListElement",
            Some("DL") => "HTMLDListElement",
            Some("PRE") => "HTMLPreElement",
            Some("TR") => "HTMLTableRowElement",
            Some("TD") | Some("TH") => "HTMLTableCellElement",
            Some("THEAD") | Some("TBODY") | Some("TFOOT") => "HTMLTableSectionElement",
            Some("COL") | Some("COLGROUP") => "HTMLTableColElement",
            Some("CAPTION") => "HTMLTableCaptionElement",
            Some("PROGRESS") => "HTMLProgressElement",
            Some("METER") => "HTMLMeterElement",
            Some("DETAILS") => "HTMLDetailsElement",
            Some("PICTURE") => "HTMLPictureElement",
            Some("SOURCE") => "HTMLSourceElement",
            Some("VIDEO") => "HTMLVideoElement",
            Some("AUDIO") => "HTMLAudioElement",
            Some("TEMPLATE") => "HTMLTemplateElement",
            Some("SLOT") => "HTMLSlotElement",
            Some("BODY") => "HTMLBodyElement",
            Some("HTML") => "HTMLHtmlElement",
            Some("HEAD") => "HTMLHeadElement",
            Some("META") => "HTMLMetaElement",
            Some("LINK") => "HTMLLinkElement",
            Some("TITLE") => "HTMLTitleElement",
            Some("BASE") => "HTMLBaseElement",
            Some("BR") => "HTMLBRElement",
            Some("HR") => "HTMLHRElement",
            Some("OPTGROUP") => "HTMLOptGroupElement",
            Some("MAP") => "HTMLMapElement",
            Some("AREA") => "HTMLAreaElement",
            Some("OBJECT") => "HTMLObjectElement",
            Some("EMBED") => "HTMLEmbedElement",
            Some("OUTPUT") => "HTMLOutputElement",
            Some("MENU") => "HTMLMenuElement",
            Some("DATA") => "HTMLDataElement",
            Some("TIME") => "HTMLTimeElement",
            Some("Q") | Some("BLOCKQUOTE") => "HTMLQuoteElement",
            _ => "HTMLElement",
        },
        _ => "HTMLElement",
    };
    let global = scope.get_current_context().global(scope);
    if let Some(key) = v8::String::new(scope, cname) {
        if let Some(ctor) = global.get(scope, key.into()) {
            if !ctor.is_undefined() {
                rv.set(ctor);
            }
        }
    }
}

fn get_node_type(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let nt = with_tree(|t| t.node_type_id(h)).unwrap_or(1);
    rv.set(v8::Integer::new(scope, nt as i32).into());
}

/// `data` / `nodeValue` → text of a text/comment node (null for elements). Serializers call
/// `.replace` on these, so they must be strings (not undefined) for text nodes.
fn get_node_data(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let nt = with_tree(|t| t.node_type_id(h)).unwrap_or(1);
    if matches!(nt, 2 | 3 | 7 | 8) {
        let data = with_tree(|t| t.node_value_utf16(h))
            .flatten()
            .unwrap_or_default();
        rv.set(unit_string(scope, &data).into());
    } else {
        rv.set(v8::null(scope).into());
    }
}
fn set_node_data(
    scope: &mut v8::PinScope,
    name: v8::Local<v8::Name>,
    value: v8::Local<v8::Value>,
    args: v8::PropertyCallbackArguments,
    _rv: v8::ReturnValue<()>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    if !with_tree(|tree| matches!(tree.node_type_id(h), 2 | 3 | 7 | 8)).unwrap_or(false) {
        return;
    }
    let data = if value.is_null() && name.to_rust_string_lossy(scope) == "nodeValue" {
        Vec::new()
    } else {
        let Some(value) = value.to_string(scope) else {
            return;
        };
        dom_units(scope, value.into()).unwrap()
    };
    html_popover::attribute_node_change(scope, h, Some(&String::from_utf16_lossy(&data)));
    range_live::replacing_data(scope, h, 0, character_units(h).len(), data.len());
    with_tree_mut(|t| t.set_text_content_utf16(h, &data));
    element_relations::attribute_node_changed(scope, h);
}

// `Element.localName` — the lowercase local name (rtdom stores tags lowercased). Non-elements have
// no local name (null). React/Next devtools read node.localName in places.
fn get_local_name(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    match with_tree(|t| t.local_name_utf16(h)).flatten() {
        Some(ln) => rv.set(unit_string(scope, &ln).into()),
        None => rv.set(v8::null(scope).into()),
    }
}

fn get_node_name(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    if let Some(units) = with_tree(|tree| match tree.node_type_id(h) {
        1 => tree.tag_name_utf16(h),
        2 => tree.qualified_name_utf16(h),
        7 => tree.processing_instruction_target_utf16(h),
        10 => tree.doctype_name_utf16(h),
        _ => None,
    })
    .flatten()
    {
        rv.set(unit_string(scope, &units).into());
        return;
    }
    let (nt, tag) = with_tree(|t| (t.node_type_id(h), t.tag_name(h))).unwrap_or((1, None));
    let name = match nt {
        2 => with_tree(|tree| tree.qualified_name(h).unwrap_or_default().to_owned())
            .unwrap_or_default(),
        3 => "#text".to_string(),
        7 => with_tree(|tree| {
            tree.processing_instruction_target(h)
                .unwrap_or_default()
                .to_owned()
        })
        .unwrap_or_default(),
        8 => "#comment".to_string(),
        9 => "#document".to_string(),
        10 => with_tree(|tree| tree.doctype_name(h).unwrap_or_default().to_owned())
            .unwrap_or_default(),
        11 => "#document-fragment".to_string(),
        _ => tag.unwrap_or_default(),
    };
    rv.set(v8::String::new(scope, &name).unwrap().into());
}

fn get_child_nodes(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(h) = handle_of(scope, args.holder()) else {
        return;
    };
    let kids = with_tree(|t| t.children(h)).unwrap_or_default();
    let arr = node_array(scope, kids);
    bind_method(scope, arr.into(), "item", nodelist_item);
    bind_method(scope, arr.into(), "namedItem", nodelist_named_item);
    rv.set(arr.into());
}

/// `NodeList.item(i)` → the i-th node, or null when out of range. Bound on every node-collection
/// array we hand back (childNodes / querySelectorAll / getElementsBy*), so libs calling `.item(i)`
/// instead of `[i]` (e.g. React-DOM's child reconciliation) don't crash.
fn nodelist_item(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let i = args.get(0).number_value(scope).unwrap_or(f64::NAN);
    if i.is_nan() || i < 0.0 {
        rv.set_null();
        return;
    }
    match args.this().get_index(scope, i as u32) {
        Some(v) if !v.is_undefined() => rv.set(v),
        _ => rv.set_null(),
    }
}

fn get_owner_document(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let _dom = DomScope::for_object(scope, args.holder());
    let Some(handle) = handle_of(scope, args.holder()) else {
        return;
    };
    if with_tree(|tree| tree.node_type_id(handle)) == Some(9) {
        rv.set_null();
        return;
    }
    if let Some(root) = with_tree(|t| t.root()) {
        let doc = wrap(scope, root);
        rv.set(doc.into());
    }
}

thread_local! {
    /// per-element `style` object (a plain JS object React reads/writes); cleared per file.
    static STYLE: RefCell<HashMap<Handle, v8::Weak<v8::Object>>> = RefCell::new(HashMap::new());
}

/// Inline declarations are cached by native node and use the context's shared
/// CSSStyleDeclaration factory, which keeps values private and reflects mutations
/// to this element's actual style content attribute.
fn get_style(
    scope: &mut v8::PinScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
) {
    let receiver = args.holder();
    let context = receiver
        .get_creation_context(scope)
        .unwrap_or_else(|| scope.get_current_context());
    let scope = &mut v8::ContextScope::new(scope, context);
    let _dom = DomScope::for_object(scope, receiver);
    let Some(h) = handle_of(scope, receiver) else {
        return;
    };
    if let Some(object) = STYLE.with(|s| s.borrow().get(&h).and_then(|weak| weak.to_local(scope))) {
        rv.set(object.into());
        return;
    }
    let global = context.global(scope);
    let name = v8::String::new(scope, "ts").unwrap();
    let key = v8::Symbol::for_key(scope, name);
    let Some(namespace) = global
        .get(scope, key.into())
        .and_then(|v| v.to_object(scope))
    else {
        return;
    };
    let key = v8::String::new(scope, "makeInlineStyleDeclaration").unwrap();
    let Some(factory) = namespace
        .get(scope, key.into())
        .and_then(|v| v8::Local::<v8::Function>::try_from(v).ok())
    else {
        return;
    };
    let node = wrap(scope, h);
    let Some(declaration) = factory
        .call(scope, namespace.into(), &[node.into()])
        .and_then(|v| v.to_object(scope))
    else {
        return;
    };
    dom_ownership::link_style(scope, node, declaration);
    let saved = v8::Weak::new(scope, declaration);
    STYLE.with(|s| {
        s.borrow_mut().insert(h, saved);
    });
    rv.set(declaration.into());
}

// ---- document methods ------------------------------------------------------------------------

fn doc_create_element<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[9]).is_none() {
        return;
    }
    let Some(tag) = dom_units(scope, args.get(0)) else {
        return;
    };
    let xml = dom_implementation::is_xml_document(scope, args.this());
    let namespace = dom_implementation::default_element_namespace(scope, args.this());
    let Some(h) = with_tree_mut(|tree| {
        if xml {
            let namespace = if namespace.is_some() {
                turbo_dom_parser::rtdom::tree::Namespace::Html
            } else {
                turbo_dom_parser::rtdom::tree::Namespace::Other
            };
            tree.create_element_ns_units(&tag, namespace)
        } else {
            tree.create_element_units(&tag)
        }
    }) else {
        return;
    };
    let node = wrap(scope, h);
    rv.set(node.into());
}

fn doc_create_text_node(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[9]).is_none() {
        return;
    }
    let Some(data) = dom_units(scope, args.get(0)) else {
        return;
    };
    let Some(h) = with_tree_mut(|t| t.create_text_node_utf16(&data)) else {
        return;
    };
    let node = wrap(scope, h);
    rv.set(node.into());
}

fn doc_get_element_by_id<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[9]).is_none() {
        return;
    }
    let Some(id) = dom_units(scope, args.get(0)) else {
        return;
    };
    let found = with_tree(|t| {
        String::from_utf16(&id).map_or_else(
            |_| t.get_element_by_id_units(&id),
            |id| t.get_element_by_id(&id),
        )
    })
    .flatten();
    let v = wrap_opt(scope, found);
    rv.set(v);
}

fn doc_query_selector<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[9]) else {
        return;
    };
    let Some((sel, units, unit_path)) = selector_argument(scope, &args, h) else {
        return;
    };
    let found = with_tree(|t| {
        if unit_path {
            t.query_selector_units(&units)
        } else {
            t.query_selector(&sel)
        }
    })
    .flatten();
    let v = wrap_opt(scope, found);
    rv.set(v);
}

fn doc_query_selector_all<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(h) = receiver_handle(scope, &args, &[9]) else {
        return;
    };
    let Some((sel, units, unit_path)) = selector_argument(scope, &args, h) else {
        return;
    };
    let handles: Vec<Handle> = with_tree(|t| {
        if unit_path {
            t.query_selector_all_units(&units).to_vec()
        } else {
            t.query_selector_all(&sel).to_vec()
        }
    })
    .unwrap_or_default();
    let arr = node_array(scope, handles);
    bind_method(scope, arr.into(), "item", nodelist_item);
    bind_method(scope, arr.into(), "namedItem", nodelist_named_item);
    rv.set(arr.into());
}

// ---- install ---------------------------------------------------------------------------------

fn method_arity(name: &str) -> i32 {
    match name {
        "appendChild"
        | "removeChild"
        | "getAttribute"
        | "createAttribute"
        | "setAttributeNodeNS"
        | "getAttributeNode"
        | "setAttributeNode"
        | "removeAttributeNode"
        | "hasAttribute"
        | "removeAttribute"
        | "querySelector"
        | "querySelectorAll"
        | "dispatchEvent"
        | "matches"
        | "closest"
        | "contains"
        | "setCustomValidity"
        | "setRangeText"
        | "appendData"
        | "splitText"
        | "toggleAttribute"
        | "item"
        | "namedItem"
        | "getPropertyValue"
        | "getPropertyPriority"
        | "removeProperty"
        | "createElement"
        | "createTextNode"
        | "getElementById"
        | "createComment"
        | "__cascadeProp"
        | "__parseCssNative"
        | "containsNode"
        | "attachShadow" => 1,
        "insertBefore"
        | "replaceChild"
        | "setAttribute"
        | "addEventListener"
        | "removeEventListener"
        | "setSelectionRange"
        | "insertData"
        | "deleteData"
        | "substringData"
        | "insertAdjacentElement"
        | "insertAdjacentHTML"
        | "createAttributeNS"
        | "getAttributeNodeNS"
        | "hasAttributeNS"
        | "removeAttributeNS"
        | "getAttributeNS"
        | "setProperty"
        | "createElementNS" => 2,
        "replaceData" | "set" | "setAttributeNS" => 3,
        _ => 0,
    }
}

fn ownership_callback<'s, 'i, F>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    rv: v8::ReturnValue<'s>,
) where
    F: v8::MapFnTo<v8::FunctionCallback>
        + for<'a, 'b> Fn(
            &mut v8::PinScope<'a, 'b>,
            v8::FunctionCallbackArguments<'a>,
            v8::ReturnValue<'a>,
        ),
{
    let caught = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut caught.init();
    // Source documents can lose their last node edge during adoption. Keep
    // their actual wrappers/realms alive through private record projection.
    let _owners = dom_ownership::pin_callback_owners(
        scope,
        args.this(),
        (0..args.length()).map(|index| args.get(index)),
    );
    if scope.has_terminated() {
        scope.rethrow();
        return;
    }
    if scope.is_execution_terminating() {
        return;
    }
    let reactions = custom_elements::reactions(scope);
    F::get()(scope, args, rv);
    if scope.has_terminated() {
        scope.rethrow();
        return;
    }
    if scope.is_execution_terminating() {
        return;
    }
    let exception = scope.exception();
    if exception.is_some() {
        scope.reset();
    }
    mutation_observers::flush(scope);
    reactions.flush(scope);
    if scope.has_terminated() || scope.is_execution_terminating() {
        scope.rethrow();
        return;
    }
    dom_ownership::reconcile_all(scope);
    if scope.has_terminated() {
        scope.rethrow();
        return;
    }
    if let Some(exception) = exception {
        scope.throw_exception(exception);
        scope.rethrow();
    }
}
fn method_template<'s, F>(
    scope: &mut v8::PinScope<'s, '_>,
    name: &str,
    _f: F,
) -> v8::Local<'s, v8::FunctionTemplate>
where
    F: v8::MapFnTo<v8::FunctionCallback>
        + for<'a, 'i> Fn(
            &mut v8::PinScope<'a, 'i>,
            v8::FunctionCallbackArguments<'a>,
            v8::ReturnValue<'a>,
        ),
{
    // DOM operations are callable methods, not constructors. Enforce that in
    // V8 itself rather than removing a property from a constructible function.
    let template = v8::FunctionTemplate::builder(ownership_callback::<F>)
        .constructor_behavior(v8::ConstructorBehavior::Throw)
        .length(method_arity(name))
        .build(scope);
    let name = v8::String::new(scope, name).unwrap();
    template.set_class_name(name);
    template.remove_prototype();
    template
}

fn bind_method<F>(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>, name: &str, f: F)
where
    F: v8::MapFnTo<v8::FunctionCallback>
        + for<'a, 'i> Fn(
            &mut v8::PinScope<'a, 'i>,
            v8::FunctionCallbackArguments<'a>,
            v8::ReturnValue<'a>,
        ),
{
    let tmpl = method_template(scope, name, f);
    let func = tmpl.get_function(scope).unwrap();
    let key = v8::String::new(scope, name).unwrap();
    func.set_name(key);
    obj.set(scope, key.into(), func.into());
}

/// Set a method (FunctionTemplate) on an object template.
fn tmpl_method<F>(scope: &mut v8::PinScope, tmpl: v8::Local<v8::ObjectTemplate>, name: &str, f: F)
where
    F: v8::MapFnTo<v8::FunctionCallback>
        + for<'a, 'i> Fn(
            &mut v8::PinScope<'a, 'i>,
            v8::FunctionCallbackArguments<'a>,
            v8::ReturnValue<'a>,
        ),
{
    let ft = method_template(scope, name, f);
    let key = v8::String::new(scope, name).unwrap();
    tmpl.set(key.into(), ft.into());
}

fn tmpl_getter(
    scope: &mut v8::PinScope,
    tmpl: v8::Local<v8::ObjectTemplate>,
    name: &str,
    g: impl v8::MapFnTo<v8::AccessorNameGetterCallback>,
) {
    let key = v8::String::new(scope, name).unwrap();
    tmpl.set_accessor(key.into(), g);
}

fn ownership_setter<'s, 'i, S>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    rv: v8::ReturnValue<()>,
) where
    S: v8::MapFnTo<v8::AccessorNameSetterCallback>
        + for<'a, 'b> Fn(
            &mut v8::PinScope<'a, 'b>,
            v8::Local<'a, v8::Name>,
            v8::Local<'a, v8::Value>,
            v8::PropertyCallbackArguments<'a>,
            v8::ReturnValue<()>,
        ),
{
    let caught = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut caught.init();
    let _owners = dom_ownership::pin_callback_owners(scope, args.holder(), [value]);
    if scope.has_terminated() {
        scope.rethrow();
        return;
    }
    if scope.is_execution_terminating() {
        return;
    }
    S::get()(scope, name, value, args, rv);
    if scope.has_terminated() {
        scope.rethrow();
        return;
    }
    if scope.is_execution_terminating() {
        return;
    }
    let exception = scope.exception();
    if exception.is_some() {
        scope.reset();
    }
    mutation_observers::flush(scope);
    if scope.has_terminated() || scope.is_execution_terminating() {
        scope.rethrow();
        return;
    }
    dom_ownership::reconcile_all(scope);
    if scope.has_terminated() {
        scope.rethrow();
        return;
    }
    if let Some(exception) = exception {
        scope.throw_exception(exception);
        scope.rethrow();
    }
}
fn tmpl_accessor<S>(
    scope: &mut v8::PinScope,
    tmpl: v8::Local<v8::ObjectTemplate>,
    name: &str,
    g: impl v8::MapFnTo<v8::AccessorNameGetterCallback>,
    _s: S,
) where
    S: v8::MapFnTo<v8::AccessorNameSetterCallback>
        + for<'a, 'b> Fn(
            &mut v8::PinScope<'a, 'b>,
            v8::Local<'a, v8::Name>,
            v8::Local<'a, v8::Value>,
            v8::PropertyCallbackArguments<'a>,
            v8::ReturnValue<()>,
        ),
{
    let key = v8::String::new(scope, name).unwrap();
    tmpl.set_accessor_with_setter(key.into(), g, ownership_setter::<S>);
}

/// Build the shared element template (methods + accessors).
fn build_el_template<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::ObjectTemplate> {
    let tmpl = v8::ObjectTemplate::new(scope);
    tmpl.set_internal_field_count(2);

    tmpl_method(scope, tmpl, "attachShadow", el_attach_shadow);
    tmpl_getter(scope, tmpl, "shadowRoot", get_shadow_root);
    tmpl_getter(scope, tmpl, "host", get_shadow_host);
    tmpl_getter(scope, tmpl, "mode", get_shadow_mode);
    tmpl_getter(scope, tmpl, "__isShadowRoot", get_is_shadow_root);
    tmpl_method(scope, tmpl, "appendChild", el_append_child);
    tmpl_method(scope, tmpl, "removeChild", el_remove_child);
    tmpl_method(scope, tmpl, "insertBefore", el_insert_before);
    tmpl_method(scope, tmpl, "replaceChild", el_replace_child);
    tmpl_method(scope, tmpl, "setAttribute", el_set_attribute);
    tmpl_method(scope, tmpl, "getAttribute", el_get_attribute);
    tmpl_method(scope, tmpl, "getAttributeNode", el_get_attribute_node);
    tmpl_method(scope, tmpl, "getAttributeNodeNS", el_get_attribute_node_ns);
    tmpl_method(scope, tmpl, "setAttributeNodeNS", el_set_attribute_node);
    tmpl_accessor(scope, tmpl, "__attrValue", get_node_data, set_attr_value);
    tmpl_getter(scope, tmpl, "__attrOwner", get_attr_owner);
    tmpl_getter(scope, tmpl, "__attrSpecified", get_attr_specified);
    tmpl_getter(scope, tmpl, "__nodeName", get_named_node_name);
    tmpl_getter(scope, tmpl, "publicId", get_doctype_public);
    tmpl_getter(scope, tmpl, "systemId", get_doctype_system);
    tmpl_method(scope, tmpl, "setAttributeNode", el_set_attribute_node);
    tmpl_method(scope, tmpl, "removeAttributeNode", el_remove_attribute_node);
    tmpl_method(scope, tmpl, "hasAttribute", el_has_attribute);
    tmpl_method(scope, tmpl, "removeAttribute", el_remove_attribute);
    tmpl_method(scope, tmpl, "querySelector", el_query_selector);
    tmpl_method(scope, tmpl, "querySelectorAll", el_query_selector_all);
    tmpl_method(scope, tmpl, "addEventListener", el_add_event_listener);
    tmpl_method(scope, tmpl, "removeEventListener", el_remove_event_listener);
    tmpl_method(scope, tmpl, "dispatchEvent", el_dispatch_event);
    tmpl_method(scope, tmpl, "matches", el_matches);
    tmpl_method(scope, tmpl, "closest", el_closest);
    tmpl_method(scope, tmpl, "contains", el_contains);
    tmpl_method(scope, tmpl, "hasChildNodes", el_has_child_nodes);
    tmpl_method(scope, tmpl, "cloneNode", el_clone_node);
    tmpl_method(scope, tmpl, "getRootNode", el_get_root_node);
    tmpl_method(scope, tmpl, "containsNode", el_contains_node);
    tmpl_method(scope, tmpl, "append", el_append);
    tmpl_method(scope, tmpl, "prepend", el_prepend);
    tmpl_method(scope, tmpl, "remove", el_remove_self);
    tmpl_method(scope, tmpl, "focus", el_focus);
    tmpl_method(scope, tmpl, "blur", el_blur);
    tmpl_method(scope, tmpl, "click", el_click);
    tmpl_method(scope, tmpl, "scrollIntoView", el_scroll_into_view);
    // Form-control methods (no headless effect, but must exist so React's commit path
    // doesn't throw "not a function" and discard the SSR tree — see el_noop).
    tmpl_method(scope, tmpl, "setCustomValidity", el_noop);
    tmpl_method(scope, tmpl, "setSelectionRange", el_noop);
    tmpl_method(scope, tmpl, "setRangeText", el_noop);
    tmpl_method(scope, tmpl, "select", el_noop);
    tmpl_method(scope, tmpl, "scrollTo", el_noop);
    tmpl_method(scope, tmpl, "scrollBy", el_noop);
    tmpl_method(scope, tmpl, "checkValidity", el_valid_true);
    tmpl_method(scope, tmpl, "reportValidity", el_valid_true);
    tmpl_method(
        scope,
        tmpl,
        "getBoundingClientRect",
        el_get_bounding_client_rect,
    );
    // CharacterData (text/comment node) mutation — native over rtdom's text storage.
    tmpl_method(scope, tmpl, "insertData", el_insert_data);
    tmpl_method(scope, tmpl, "deleteData", el_delete_data);
    tmpl_method(scope, tmpl, "appendData", el_append_data);
    tmpl_method(scope, tmpl, "replaceData", el_replace_data);
    tmpl_method(scope, tmpl, "substringData", el_substring_data);
    tmpl_method(scope, tmpl, "splitText", el_split_text);
    // native cascade lookup (inherited-property resolution) used by getComputedStyle.
    tmpl_method(scope, tmpl, "__cascadeProp", el_cascade_prop);
    // ChildNode / ParentNode manipulation + insertAdjacent* + toggleAttribute + getAttributeNS (native).
    tmpl_method(scope, tmpl, "before", el_before);
    tmpl_method(scope, tmpl, "after", el_after);
    tmpl_method(scope, tmpl, "replaceWith", el_replace_with);
    tmpl_method(scope, tmpl, "replaceChildren", el_replace_children);
    tmpl_method(
        scope,
        tmpl,
        "insertAdjacentElement",
        el_insert_adjacent_element,
    );
    tmpl_method(scope, tmpl, "insertAdjacentHTML", el_insert_adjacent_html);
    tmpl_method(scope, tmpl, "toggleAttribute", el_toggle_attribute);
    tmpl_method(scope, tmpl, "getAttributeNS", el_get_attribute_ns);
    tmpl_method(scope, tmpl, "hasAttributeNS", el_has_attribute_ns);
    tmpl_method(scope, tmpl, "setAttributeNS", el_set_attribute_ns);
    tmpl_method(scope, tmpl, "removeAttributeNS", el_remove_attribute_ns);

    tmpl_getter(scope, tmpl, "tagName", get_tag_name);
    tmpl_getter(scope, tmpl, "parentNode", get_parent_node);
    tmpl_getter(scope, tmpl, "firstChild", get_first_child);
    tmpl_getter(scope, tmpl, "nextSibling", get_next_sibling);
    tmpl_getter(scope, tmpl, "style", get_style);
    tmpl_getter(scope, tmpl, "constructor", get_constructor);
    tmpl_getter(scope, tmpl, "nodeType", get_node_type);
    tmpl_getter(scope, tmpl, "activeElement", get_active_element);
    tmpl_accessor(scope, tmpl, "data", get_node_data, set_node_data);
    tmpl_accessor(scope, tmpl, "nodeValue", get_node_data, set_node_data);
    tmpl_getter(scope, tmpl, "nodeName", get_node_name);
    tmpl_getter(scope, tmpl, "localName", get_local_name);
    tmpl_getter(scope, tmpl, "childNodes", get_child_nodes);
    tmpl_getter(scope, tmpl, "ownerDocument", get_owner_document);
    tmpl_accessor(scope, tmpl, "outerHTML", get_outer_html, set_outer_html);
    tmpl_getter(scope, tmpl, "children", get_children);
    tmpl_getter(scope, tmpl, "parentElement", get_parent_element);
    tmpl_getter(scope, tmpl, "firstElementChild", get_first_element_child);
    tmpl_getter(scope, tmpl, "namespaceURI", get_namespace_uri);
    tmpl_getter(scope, tmpl, "prefix", get_prefix);
    tmpl_getter(scope, tmpl, "isConnected", get_is_connected);
    tmpl_getter(scope, tmpl, "attributes", get_attributes);
    tmpl_getter(scope, tmpl, "dataset", get_dataset);
    tmpl_accessor(scope, tmpl, "innerHTML", get_inner_html, set_inner_html);

    tmpl_accessor(
        scope,
        tmpl,
        "textContent",
        get_text_content,
        set_text_content,
    );
    tmpl_accessor(scope, tmpl, "id", get_id, set_id);
    tmpl_accessor(scope, tmpl, "className", get_class_name, set_class_name);
    tmpl_accessor(
        scope,
        tmpl,
        "contentEditable",
        get_content_editable,
        set_content_editable,
    );
    tmpl_getter(scope, tmpl, "isContentEditable", get_is_content_editable);

    // Node-type constants on every node (`node.TEXT_NODE === 3`, …). dom-accessibility-api compares
    // `node.nodeType === node.TEXT_NODE`, so without these the accessible-name walk skips all text
    // nodes → empty names → getByRole({name}) finds nothing.
    for (name, val) in [
        ("ELEMENT_NODE", 1),
        ("ATTRIBUTE_NODE", 2),
        ("TEXT_NODE", 3),
        ("CDATA_SECTION_NODE", 4),
        ("PROCESSING_INSTRUCTION_NODE", 7),
        ("COMMENT_NODE", 8),
        ("DOCUMENT_NODE", 9),
        ("DOCUMENT_TYPE_NODE", 10),
        ("DOCUMENT_FRAGMENT_NODE", 11),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        let v = v8::Integer::new(scope, val);
        tmpl.set(key.into(), v.into());
    }

    // Always-on graceful-degradation interceptor: unimplemented members read as undefined (no
    // crash); recording the access list is gated by log_enabled() (TURBO_RUST_DOM_LOG).
    let handler = v8::NamedPropertyHandlerConfiguration::new()
        .getter(missing_getter)
        .setter(passthrough_setter)
        .flags(
            v8::PropertyHandlerFlags::NON_MASKING
                | v8::PropertyHandlerFlags::ONLY_INTERCEPT_STRINGS,
        );
    tmpl.set_named_property_handler(handler);
    select_element::indexed_handler(tmpl);

    tmpl
}

fn parse_css_native(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let source = args.get(0).to_rust_string_lossy(scope);
    if args.get(1).is_true() {
        rv.set_bool(cssom_parser::is_supported_property(&source));
        return;
    }
    if args.get(1).is_string() && args.get(1).to_rust_string_lossy(scope) == "media" {
        if let Some(text) = v8::String::new(scope, &cssom_parser::parse_media(&source)) {
            rv.set(text.into());
        }
        return;
    }
    if args.get(2).is_true() {
        let parsed = cssom_parser::parse_one(&source);
        if let Some(text) = parsed.and_then(|parsed| v8::String::new(scope, &parsed)) {
            rv.set(text.into());
        } else {
            rv.set(v8::null(scope).into());
        }
        return;
    }
    let parsed = cssom_parser::parse(&source);
    if let Some(text) = v8::String::new(scope, &parsed) {
        rv.set(text.into());
    }
}

/// Register the native stylesheet grammar parser before running the JS bootstrap.
// A projected prototype accessor must keep using the native tree, rather than
// capturing the instance's initial descriptor value. The uncached view retains
// the original ObjectTemplate callbacks without inheriting projected JS APIs.
fn dom_accessor_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let receiver = args.this();
    let _dom = DomScope::for_object(scope, receiver);
    let descriptor = args.data().to_rust_string_lossy(scope);
    let mut fields = descriptor.split('\0');
    let setter = fields.next() == Some("set");
    let owner = fields.next().unwrap_or("Node");
    let property = fields.next().unwrap_or("");
    let handle = handle_of(scope, receiver);
    let valid = handle
        .and_then(|h| {
            with_tree(|tree| {
                let kind = tree.node_type_id(h);
                match owner {
                    "Node" => true,
                    "Element" => kind == 1,
                    "Attr" => kind == 2,
                    "DocumentType" => kind == 10,
                    "HTMLElement" => kind == 1 && tree.namespace_id(h) == 0,
                    "CharacterData" => matches!(kind, 3 | 4 | 7 | 8),
                    "Text" => matches!(kind, 3 | 4),
                    "Document" => kind == 9,
                    "DocumentFragment" => kind == 11,
                    "ShadowRoot" => kind == 11 && tree.is_shadow_root(h),
                    _ => owner.strip_prefix("HTML:").is_some_and(|names| {
                        kind == 1
                            && tree.namespace_id(h) == 0
                            && tree
                                .local_name(h)
                                .is_some_and(|local| names.split('|').any(|name| name == local))
                    }),
                }
            })
        })
        .unwrap_or(false);
    if !valid {
        let message = v8::String::new(scope, "Illegal invocation").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    }
    let handle = handle.unwrap();
    let Some(context) = receiver.get_creation_context(scope) else {
        return;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let template = DOM.with(|d| d.borrow().as_ref().unwrap().el_template.clone());
    let template = dom_ownership::local(scope, template);
    let raw = template.new_instance(scope).unwrap();
    let handle = v8::Integer::new_from_unsigned(scope, handle.raw());
    raw.set_internal_field(0, handle.into());
    let owner = v8::BigInt::new_from_u64(scope, DOM_ID.with(|id| id.get()));
    raw.set_internal_field(1, owner.into());
    let key = v8::String::new(scope, property).unwrap();
    if setter {
        raw.set(scope, key.into(), args.get(0));
    } else if let Some(value) = raw.get(scope, key.into()) {
        rv.set(value);
    }
}

fn make_dom_accessor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let property = args.get(0).to_rust_string_lossy(scope);
    let setter = args.get(1).boolean_value(scope);
    let owner = args.get(2).to_rust_string_lossy(scope);
    let operation = if setter { "set" } else { "get" };
    let data = v8::String::new(scope, &format!("{operation}\0{owner}\0{property}")).unwrap();
    let template = v8::FunctionTemplate::builder(dom_accessor_dispatch)
        .data(data.into())
        .constructor_behavior(v8::ConstructorBehavior::Throw)
        .length(i32::from(setter))
        .build(scope);
    let name = v8::String::new(scope, &format!("{operation} {property}")).unwrap();
    template.set_class_name(name);
    template.remove_prototype();
    if let Some(function) = template.get_function(scope) {
        rv.set(function.into());
    }
}

fn native_dom_function_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(data) = v8::Local::<v8::Array>::try_from(args.data()) else {
        return;
    };
    let Some(delegate) = data
        .get_index(scope, 0)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    else {
        return;
    };
    let require_new = data
        .get_index(scope, 1)
        .is_some_and(|value| value.boolean_value(scope));
    if require_new && !args.is_construct_call() {
        // An interface may name its own call-without-new message (Blink's legacy
        // factory functions say "Failed to construct 'Option': Please use the 'new' …").
        let message = match data
            .get_index(scope, 4)
            .and_then(|value| v8::Local::<v8::String>::try_from(value).ok())
        {
            Some(message) => message,
            None => v8::String::new(scope, "Constructor requires 'new'").unwrap(),
        };
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    }
    let values: Vec<_> = (0..args.length()).map(|index| args.get(index)).collect();
    let class_construct = data
        .get_index(scope, 2)
        .is_some_and(|value| value.boolean_value(scope));
    if class_construct && args.is_construct_call() {
        let Some(construct) = data
            .get_index(scope, 3)
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        else {
            return;
        };
        let arguments = v8::Array::new_with_elements(scope, &values);
        let receiver = v8::undefined(scope).into();
        if let Some(value) = construct.call(
            scope,
            receiver,
            &[delegate.into(), arguments.into(), args.new_target()],
        ) {
            rv.set(value);
        }
        return;
    }
    if let Some(value) = delegate.call(scope, args.this().into(), &values) {
        rv.set(value);
    }
}

fn native_range_record(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let key_name = v8::String::new(scope, "turbo-dom.Range.record").unwrap();
    let key = v8::Private::for_api(scope, Some(key_name));
    let receiver = if args.get(0).is_undefined() && args.get(1).is_object() {
        let template = v8::ObjectTemplate::new(scope);
        template.set_internal_field_count(1);
        let object = template.new_instance(scope).unwrap();
        if args.get(2).is_object() {
            object.set_prototype(scope, args.get(2));
        }
        Some(object)
    } else {
        v8::Local::<v8::Object>::try_from(args.get(0)).ok()
    };
    if let Some(receiver) = receiver {
        if args.length() > 1 && args.get(1).is_object() {
            let record = args.get(1);
            receiver.set_private(scope, key, record);
            if args.get(0).is_undefined() {
                if let Ok(record) = v8::Local::<v8::Object>::try_from(record) {
                    range_live::register(scope, record);
                }
            }
            rv.set(if args.get(0).is_undefined() {
                receiver.into()
            } else {
                record
            });
            return;
        }
        if let Some(record) = receiver
            .get_private(scope, key)
            .filter(|value| value.is_object())
        {
            rv.set(record);
            return;
        }
    }
    let message = v8::String::new(scope, "Illegal invocation").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}

// The implementation closure stays in V8's private function data. This is a
// native callable over the facade's live guest state, not a source-string mask.
fn make_native_dom_function(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(delegate) = v8::Local::<v8::Function>::try_from(args.get(0)) else {
        let message = v8::String::new(scope, "Expected a function").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    };
    let name = args.get(1).to_rust_string_lossy(scope);
    let length = args.get(2).int32_value(scope).unwrap_or(0).max(0);
    let constructor = args.get(3).boolean_value(scope);
    let class_construct = args.get(4).boolean_value(scope);
    let require_new = if args.get(5).is_undefined() {
        constructor
    } else {
        args.get(5).boolean_value(scope)
    };
    let require_new = v8::Boolean::new(scope, require_new);
    let class_construct = v8::Boolean::new(scope, class_construct);
    let call_message = if args.get(6).is_string() {
        args.get(6)
    } else {
        v8::undefined(scope).into()
    };
    let data = v8::Array::new_with_elements(
        scope,
        &[
            delegate.into(),
            require_new.into(),
            class_construct.into(),
            args.data(),
            call_message,
        ],
    );
    let template = v8::FunctionTemplate::builder(native_dom_function_dispatch)
        .data(data.into())
        .constructor_behavior(if constructor {
            v8::ConstructorBehavior::Allow
        } else {
            v8::ConstructorBehavior::Throw
        })
        .length(length)
        .build(scope);
    let name = v8::String::new(scope, &name).unwrap();
    template.set_class_name(name);
    if !constructor {
        template.remove_prototype();
    }
    if let Some(function) = template.get_function(scope) {
        rv.set(function.into());
    }
}

// Named-property objects preserve Web IDL's non-masking lookup rules. The
// dispatch closure owns its guest/host capability state and is only kept in V8
// callback data; no backing map is projected as a JavaScript property.
fn named_dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::PropertyCallbackArguments<'s>,
    operation: &str,
    name: Option<v8::Local<'s, v8::Name>>,
    value: Option<v8::Local<'s, v8::Value>>,
) -> Option<v8::Local<'s, v8::Value>> {
    let callback = v8::Local::<v8::Function>::try_from(args.data()).ok()?;
    let operation = v8::String::new(scope, operation)?;
    let name = name
        .map(Into::into)
        .unwrap_or_else(|| v8::undefined(scope).into());
    let value = value.unwrap_or_else(|| v8::undefined(scope).into());
    callback.call(
        scope,
        args.holder().into(),
        &[operation.into(), name, value],
    )
}
fn named_getter<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) -> v8::Intercepted {
    if let Some(value) = named_dispatch(scope, &args, "get", Some(name), None) {
        if !value.is_undefined() {
            rv.set(value);
            return v8::Intercepted::kYes;
        }
    }
    v8::Intercepted::kNo
}
fn named_query<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue<v8::Integer>,
) -> v8::Intercepted {
    if let Some(descriptor) = named_dispatch(scope, &args, "descriptor", Some(name), None)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    {
        let mut attributes = 0;
        for (field, flag) in [("writable", 1), ("enumerable", 2), ("configurable", 4)] {
            let key = v8::String::new(scope, field).unwrap();
            if descriptor
                .get(scope, key.into())
                .is_some_and(|value| !value.is_true())
            {
                attributes |= flag;
            }
        }
        rv.set_int32(attributes);
        return v8::Intercepted::kYes;
    }
    if named_dispatch(scope, &args, "has", Some(name), None).is_some_and(|value| value.is_true()) {
        rv.set_int32(0);
        return v8::Intercepted::kYes;
    }
    v8::Intercepted::kNo
}
fn named_descriptor<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) -> v8::Intercepted {
    if let Some(value) = named_dispatch(scope, &args, "descriptor", Some(name), None) {
        if value.is_object() {
            rv.set(value);
            return v8::Intercepted::kYes;
        }
    }
    v8::Intercepted::kNo
}
fn named_enumerator<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue<v8::Array>,
) {
    if let Some(value) = named_dispatch(scope, &args, "keys", None, None)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
    {
        rv.set(value);
    }
}
trait NamedMutationResult {
    fn set_success(&mut self, success: bool);
}
impl NamedMutationResult for v8::ReturnValue<'_, ()> {
    fn set_success(&mut self, success: bool) {
        self.set_bool(success);
    }
}
impl NamedMutationResult for v8::ReturnValue<'_, v8::Boolean> {
    fn set_success(&mut self, success: bool) {
        self.set_bool(success);
    }
}
fn named_setter<'s, 'i, T: 'static>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue<T>,
) -> v8::Intercepted
where
    for<'cb> v8::ReturnValue<'cb, T>: NamedMutationResult,
{
    if let Some(result) = named_dispatch(scope, &args, "set", Some(name), Some(value)) {
        if result.is_undefined() {
            return v8::Intercepted::kNo;
        }
        rv.set_success(result.is_true());
        return v8::Intercepted::kYes;
    }
    v8::Intercepted::kYes
}
fn named_deleter<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue<v8::Boolean>,
) -> v8::Intercepted {
    if let Some(result) = named_dispatch(scope, &args, "delete", Some(name), None) {
        if result.is_undefined() {
            return v8::Intercepted::kNo;
        }
        rv.set_bool(result.is_true());
    }
    v8::Intercepted::kYes
}
fn named_definer<'s, 'i, T: 'static>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    descriptor: &v8::PropertyDescriptor,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue<T>,
) -> v8::Intercepted
where
    for<'cb> v8::ReturnValue<'cb, T>: NamedMutationResult,
{
    let object = v8::Object::new(scope);
    let fields = [
        (
            "configurable",
            descriptor.has_configurable(),
            descriptor.configurable(),
        ),
        (
            "enumerable",
            descriptor.has_enumerable(),
            descriptor.enumerable(),
        ),
        ("writable", descriptor.has_writable(), descriptor.writable()),
    ];
    for (name, present, value) in fields {
        if present {
            let key = v8::String::new(scope, name).unwrap();
            let value = v8::Boolean::new(scope, value);
            object.set(scope, key.into(), value.into());
        }
    }
    for (name, present, value) in [
        (
            "value",
            descriptor.has_value(),
            descriptor.has_value().then(|| descriptor.value()),
        ),
        (
            "get",
            descriptor.has_get(),
            descriptor.has_get().then(|| descriptor.get()),
        ),
        (
            "set",
            descriptor.has_set(),
            descriptor.has_set().then(|| descriptor.set()),
        ),
    ] {
        if present {
            let key = v8::String::new(scope, name).unwrap();
            object.set(scope, key.into(), value.unwrap());
        }
    }
    // The dispatcher also receives V8's should-throw mode: an interface whose definer rejects a
    // descriptor throws only where the definition must (Object.defineProperty) and reports the
    // Reflect form quietly, as Blink's interceptors do (HTMLOptionsCollection accessors).
    let Ok(callback) = v8::Local::<v8::Function>::try_from(args.data()) else {
        return v8::Intercepted::kYes;
    };
    let operation = v8::String::new(scope, "define").unwrap();
    let throws = interceptor_mode::should_throw(scope, &args);
    let throws = v8::Boolean::new(scope, throws);
    if let Some(result) = callback.call(
        scope,
        args.holder().into(),
        &[operation.into(), name.into(), object.into(), throws.into()],
    ) {
        if result.is_undefined() {
            return v8::Intercepted::kNo;
        }
        rv.set_success(result.is_true());
    }
    v8::Intercepted::kYes
}
fn indexed_name<'s>(scope: &mut v8::PinScope<'s, '_>, index: u32) -> v8::Local<'s, v8::Name> {
    v8::String::new(scope, &index.to_string()).unwrap().into()
}
fn indexed_getter<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    args: v8::PropertyCallbackArguments<'s>,
    rv: v8::ReturnValue,
) -> v8::Intercepted {
    let name = indexed_name(scope, index);
    named_getter(scope, name, args, rv)
}
fn indexed_query<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    args: v8::PropertyCallbackArguments<'s>,
    rv: v8::ReturnValue<v8::Integer>,
) -> v8::Intercepted {
    let name = indexed_name(scope, index);
    named_query(scope, name, args, rv)
}
fn indexed_descriptor<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    args: v8::PropertyCallbackArguments<'s>,
    rv: v8::ReturnValue,
) -> v8::Intercepted {
    let name = indexed_name(scope, index);
    named_descriptor(scope, name, args, rv)
}
fn indexed_setter<'s, 'i, T: 'static>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    rv: v8::ReturnValue<T>,
) -> v8::Intercepted
where
    for<'cb> v8::ReturnValue<'cb, T>: NamedMutationResult,
{
    let name = indexed_name(scope, index);
    named_setter(scope, name, value, args, rv)
}
fn indexed_deleter<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    args: v8::PropertyCallbackArguments<'s>,
    rv: v8::ReturnValue<v8::Boolean>,
) -> v8::Intercepted {
    let name = indexed_name(scope, index);
    named_deleter(scope, name, args, rv)
}
fn indexed_definer<'s, 'i, T: 'static>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    descriptor: &v8::PropertyDescriptor,
    args: v8::PropertyCallbackArguments<'s>,
    rv: v8::ReturnValue<T>,
) -> v8::Intercepted
where
    for<'cb> v8::ReturnValue<'cb, T>: NamedMutationResult,
{
    let name = indexed_name(scope, index);
    named_definer(scope, name, descriptor, args, rv)
}
fn indexed_enumerator<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue<v8::Array>,
) {
    if let Some(value) = named_dispatch(scope, &args, "indices", None, None)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
    {
        rv.set(value);
    }
}
fn make_native_named_object(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if !(args.get(0).is_object() || args.get(0).is_null()) || !args.get(1).is_function() {
        let message =
            v8::String::new(scope, "Expected a prototype and named-property dispatcher").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    }
    let template = v8::ObjectTemplate::new(scope);
    template.set_named_property_handler(
        v8::NamedPropertyHandlerConfiguration::new()
            .getter(named_getter)
            .setter(named_setter)
            .query(named_query)
            .descriptor(named_descriptor)
            .deleter(named_deleter)
            .enumerator(named_enumerator)
            .definer(named_definer)
            .data(args.get(1))
            .flags(if args.get(3).is_true() {
                v8::PropertyHandlerFlags::ONLY_INTERCEPT_STRINGS
            } else {
                v8::PropertyHandlerFlags::NON_MASKING
                    | v8::PropertyHandlerFlags::ONLY_INTERCEPT_STRINGS
            }),
    );
    if args.get(2).is_true() {
        template.set_indexed_property_handler(
            v8::IndexedPropertyHandlerConfiguration::new()
                .getter(indexed_getter)
                .setter(indexed_setter)
                .query(indexed_query)
                .descriptor(indexed_descriptor)
                .deleter(indexed_deleter)
                .enumerator(indexed_enumerator)
                .definer(indexed_definer)
                .data(args.get(1)),
        );
    }
    if let Some(object) = template.new_instance(scope) {
        object.set_prototype(scope, args.get(0));
        rv.set(object.into());
    }
}

fn normalize_computed_color(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let value = arg_str(scope, &args, 0);
    let normalized =
        turbo_dom_parser::rtdom::color::canonicalize_color(&value, true).unwrap_or(value);
    if let Some(value) = v8::String::new(scope, &normalized) {
        rv.set(value.into());
    }
}

struct ComputedFontDefaults {
    family: String,
    size: f64,
}

/// Host font preferences, retained as native scalars rather than guest properties.
/// macOS defaults match the independent Chrome control; other platforms retain
/// the conventional Times New Roman preference pending a matching local oracle.
pub fn set_computed_font_defaults(
    context: v8::Local<v8::Context>,
    family: &str,
    size: f64,
) -> bool {
    if family.trim().is_empty() || !size.is_finite() || size <= 0.0 {
        return false;
    }
    context.set_slot(std::rc::Rc::new(ComputedFontDefaults {
        family: family.into(),
        size,
    }));
    true
}

fn computed_font_context<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) {
    let Ok(element) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let _owner = DomScope::for_object(scope, element);
    if handle_of(scope, element).is_none() {
        return;
    }
    let Some((document, root)) = with_tree(|tree| {
        let root = tree
            .children(tree.root())
            .into_iter()
            .find(|node| tree.node_type_id(*node) == 1);
        (tree.root(), root)
    }) else {
        return;
    };
    let document = wrap(scope, document);
    let context = document
        .get_creation_context(scope)
        .unwrap_or_else(|| scope.get_current_context());
    let defaults = context.get_slot::<ComputedFontDefaults>();
    let default_family = if cfg!(target_os = "macos") {
        "Times"
    } else {
        "Times New Roman"
    };
    let family = defaults
        .as_ref()
        .map_or(default_family, |defaults| defaults.family.as_str());
    let size = defaults.as_ref().map_or(16.0, |defaults| defaults.size);
    let family = v8::String::new(scope, family).unwrap();
    let size = v8::Number::new(scope, size);
    let composed_parent = shadow_slots::composed_parent(scope, element).filter(|parent| {
        handle_of(scope, *parent)
            .is_some_and(|handle| with_tree(|tree| tree.node_type_id(handle) == 1).unwrap_or(false))
    });
    let parent = if let Some(parent) = composed_parent {
        parent.into()
    } else {
        v8::null(scope).into()
    };
    let root = if let Some(root) = root {
        wrap(scope, root).into()
    } else {
        v8::null(scope).into()
    };
    let active = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|state| state.browsing_context)
    });
    let active = v8::Boolean::new(scope, active);
    result.set(
        v8::Array::new_with_elements(
            scope,
            &[family.into(), size.into(), parent, root, active.into()],
        )
        .into(),
    );
}

pub fn register_css_parser(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__parseCssNative", parse_css_native);
    bind_method(
        scope,
        global,
        "__computedFontContext",
        computed_font_context,
    );
    bind_method(
        scope,
        global,
        "__normalizeComputedColor",
        normalize_computed_color,
    );
}

/// Install `window`/`document` + the DOM onto `globalThis` for the current context.
pub fn install(scope: &mut v8::PinScope) {
    install_with_html(
        scope,
        "<!DOCTYPE html><html><head></head><body></body></html>",
    );
}

pub fn install_with_html(scope: &mut v8::PinScope, html: &str) {
    install_with_tree(scope, Tree::parse(html));
}
/// Opt-in parser positions for embedders that report authentic source locations.
pub fn install_with_html_source_positions(scope: &mut v8::PinScope, html: &str) {
    install_with_tree(scope, Tree::parse_with_source_positions(html));
}
pub fn install_with_utf16_source_positions(scope: &mut v8::PinScope, units: &[u16]) {
    install_with_tree(scope, Tree::parse_utf16_with_source_positions(units));
}
fn install_with_tree(scope: &mut v8::PinScope, mut tree: Tree) {
    dom_isolation::install(scope);
    // The document parser attaches declarative shadow roots (fragment parsing
    // for innerHTML never does); their scripts are ordinary parser scripts.
    let root = tree.root();
    let parser_scripts = script_state::parse_order(&tree);
    let declared = shadow_html::declarative(&mut tree, root, None);
    // A replacement document is seeded before its new bootstrap probes native
    // methods. Projecting with the previous bootstrap here would delete those
    // raw own methods before their replacement prototypes have been constructed.
    let global = scope.get_current_context().global(scope);
    let name = v8::String::new(scope, "ts").unwrap();
    let key = v8::Symbol::for_key(scope, name);
    if let Some(namespace) = global
        .get(scope, key.into())
        .and_then(|v| v.to_object(scope))
    {
        let key = v8::String::new(scope, "projectNativeNode").unwrap();
        namespace.delete(scope, key.into());
    }
    STYLE.with(|s| s.borrow_mut().clear());
    LISTENERS.with(|s| s.borrow_mut().clear());
    ACTIVE.with(|s| *s.borrow_mut() = None);
    let el_template = {
        let t = build_el_template(scope);
        v8::Global::new(scope, t)
    };
    DOM.with(|d| {
        *d.borrow_mut() = Some(DomState {
            scripts: script_state::initial(&tree),
            parser_scripts,
            resource_active: true,
            nonces: initial_nonces(&tree),
            tree,
            cache: HashMap::new(),
            checked_radios: Default::default(),
            dialog: Default::default(),
            el_template,
            // Declarative roots get their modes once the binding can wrap them.
            shadow_modes: HashMap::new(),
            basics_state: None,
            control_clone: None,
            css_policy: Default::default(),
            event_handlers: Default::default(),
            popover: Default::default(),
            pointer: Default::default(),
            activation: Default::default(),
            view_transitions: Default::default(),
            input_members: Default::default(),
            browsing_context: true,
            ownership: None,
            ownership_dirty: true,
            template_contents: Default::default(),
            custom_elements_active: false,
            form_submission_blocked: false,
            form_submission_handler: None,
        });
    });

    let context = scope.get_current_context();
    let global = v8::Weak::new(scope, context);
    DOM_CONTEXTS.with(|d| {
        d.borrow_mut().insert(DOM_ID.with(|id| id.get()), global);
    });
    // Parsed declarative roots exist before any binding JS mutates the tree.
    shadow_html::ensure_keys(scope);
    shadow_slots::ensure_keys(scope);
    if let Some(root) = with_tree(|tree| tree.root()) {
        form_checked_state::parsed(root);
    }
    bind_document(scope);

    // Window-level globals that libs read as BARE identifiers (`navigator`, `HTMLElement`, …) — a
    // missing bare global is a ReferenceError the element interceptor can't catch, so they must
    // exist on globalThis. Plain-data / stub surface (refined natively as the logger surfaces hot
    // members); document extras that are pure JS go here too.
    register_css_parser(scope);
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__domAccessor", make_dom_accessor);
    bind_method(
        scope,
        global,
        "__nativeNamedObject",
        make_native_named_object,
    );
    let reflect_key = v8::String::new(scope, "Reflect").unwrap();
    let construct_key = v8::String::new(scope, "construct").unwrap();
    let reflect = global.get(scope, reflect_key.into()).unwrap();
    let reflect = v8::Local::<v8::Object>::try_from(reflect).unwrap();
    let construct = reflect.get(scope, construct_key.into()).unwrap();
    let factory = v8::FunctionTemplate::builder(make_native_dom_function)
        .data(construct)
        .constructor_behavior(v8::ConstructorBehavior::Throw)
        .build(scope)
        .get_function(scope)
        .unwrap();
    let factory_key = v8::String::new(scope, "__nativeDOMFunction").unwrap();
    global.set(scope, factory_key.into(), factory.into());
    bind_method(scope, global, "__nativeEventRecord", lookup_event_record);
    bind_method(scope, global, "__nativeRangeRecord", native_range_record);
    range_contents::capture(scope);
    collection_records::register(scope);
    css_policy::register(scope);
    event_handlers::register(scope);
    bind_method(scope, global, "__checkedNative", form_checked_native);
    run_js(scope, BOOTSTRAP);
    trusted_types::install(scope);
    script_state::install_text(scope);
    capture_node_projector(scope);
    let exception_name = v8::String::new(scope, "DOMException").unwrap();
    if let Some(constructor) = global.get(scope, exception_name.into()) {
        let key = dom_exception_key(scope);
        global.set_private(scope, key, constructor);
    }
    let name = v8::String::new(scope, "ts").unwrap();
    let key = v8::Symbol::for_key(scope, name);
    if let Some(namespace) = global
        .get(scope, key.into())
        .and_then(|value| value.to_object(scope))
    {
        let key = v8::String::new(scope, "cloneControlState").unwrap();
        if let Some(function) = namespace
            .get(scope, key.into())
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        {
            let Some(function) = dom_ownership::helper_function(scope, function) else {
                return;
            };
            DOM.with(|dom| dom.borrow_mut().as_mut().unwrap().control_clone = Some(function));
        }
    }
    bind_method(
        scope,
        global,
        "__documentBasicsNative",
        document_basics::dispatch,
    );
    run_js(scope, include_str!("element_tokens.js"));
    run_js(scope, include_str!("element_relations.js"));
    // HTMLImageElement image-state accessors + the document record's `imageHost` (needs the
    // collection-record helpers, which document_basics.js retires).
    run_js(scope, include_str!("html_image.js"));
    run_js(scope, include_str!("document_basics.js"));
    document_legacy::install(scope);
    capture_node_projector(scope);
    dom_implementation::install(scope);
    // Root flags must exist before template realization adopts nested contents.
    let declared = shadow_html::register(scope, declared);
    template_contents::install(scope);
    for root in declared.iter().rev() {
        template_contents::realize(scope, *root);
    }
    mutation_observers::install(scope);
    shadow_slots::install(scope);
    shadow_html::install(scope);
    sanitizer::install(scope);
    script_state::install(scope);
    range_contents::install(scope);
    bind_method(scope, global, "__htmlTextNative", html_text::dispatch);
    run_js(scope, include_str!("html_text.js"));
    html_popover::install(scope);
    document_xpath::install(scope);
    event_listeners::install(scope);
    element_members::install(scope);
    html_dialog::install(scope);
    pointer_events::install(scope);
    bind_method(scope, global, "__formNative", form_controls_native);
    run_js(scope, include_str!("form_outputs.js"));
    run_js(scope, include_str!("form_controls.js"));
    run_js(scope, include_str!("form_collections.js"));
    reference_target::install(scope);
    interest_invokers::install(scope);
    html_invokers::install(scope);
    focus_group::install(scope);
    input_members::install(scope);
    file_inputs::install(scope);
    select_element::install(scope);
    animations::install(scope);
    typed_om::install(scope);
    hyperlink_utils::install(scope);
    form_pseudo::install(scope);
    user_activation_gates::install(scope);
    view_transitions::install(scope);
    selection::install(scope);
    editing::install(scope);
    let key = v8::String::new(scope, "__captureFormDataEvent").unwrap();
    if let Some(callback) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(callback) = dom_ownership::helper_function(scope, callback) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(FormDataEventDispatcher(callback)));
        }
    }
    global.delete(scope, key.into());
    let entries = v8::String::new(scope, "__captureFormEntries").unwrap();
    if let Some(callback) = global
        .get(scope, entries.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(callback) = dom_ownership::helper_function(scope, callback) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(FormEntryListReader(callback)));
        }
    }
    global.delete(scope, entries.into());
    let name = v8::String::new(scope, "__captureFormSubmission").unwrap();
    if let Some(function) = global
        .get(scope, name.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(function) = dom_ownership::helper_function(scope, function) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(FormSubmissionDispatcher(function)));
        }
    }
    global.delete(scope, name.into());

    let key = v8::String::new(scope, "__captureEventRecord").unwrap();
    if let Some(accessor) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        let Some(accessor) = dom_ownership::helper_function(scope, accessor) else {
            return;
        };
        scope
            .get_current_context()
            .set_slot(std::rc::Rc::new(EventRecordAccessor(accessor)));
        global.delete(scope, key.into());
    }
    let key = v8::String::new(scope, "__captureHostEventDispatch").unwrap();
    if let Some(dispatcher) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        let Some(dispatcher) = dom_ownership::helper_function(scope, dispatcher) else {
            return;
        };
        scope
            .get_current_context()
            .set_slot(std::rc::Rc::new(HostEventDispatcher(dispatcher)));
        global.delete(scope, key.into());
    }
    event_handlers::install(scope, None, "about:blank");
    custom_elements::install(scope);
    element_internals::install(scope);
    // After custom_elements re-creates the HTML constructors, so static members stay on them.
    run_js(scope, include_str!("interface_members.js"));
    // Last: reorder every interface's own keys to Chrome's order.
    key_order::installed(scope);
}

fn bind_document(scope: &mut v8::PinScope) {
    let root = with_tree(|t| t.root()).unwrap();
    let body_h = with_tree(|t| t.query_selector("body")).flatten();
    let head_h = with_tree(|t| t.query_selector("head")).flatten();
    let html_h = with_tree(|t| t.query_selector("html")).flatten();

    // document = the root node object + document methods.
    let document = wrap(scope, root);
    bind_method(scope, document, "createElement", doc_create_element);
    bind_method(scope, document, "createTextNode", doc_create_text_node);
    bind_method(scope, document, "getElementById", doc_get_element_by_id);
    bind_method(scope, document, "querySelector", doc_query_selector);
    bind_method(scope, document, "querySelectorAll", doc_query_selector_all);
    bind_method(scope, document, "createElementNS", doc_create_element_ns);
    bind_method(scope, document, "createAttribute", doc_create_attribute);
    bind_method(
        scope,
        document,
        "createAttributeNS",
        doc_create_attribute_ns,
    );
    bind_method(
        scope,
        document,
        "createDocumentFragment",
        doc_create_fragment,
    );
    bind_method(scope, document, "createComment", doc_create_comment);
    if let Some(b) = body_h {
        let body = wrap(scope, b);
        let key = v8::String::new(scope, "body").unwrap();
        document.set(scope, key.into(), body.into());
    }
    if let Some(head) = head_h {
        let head = wrap(scope, head);
        let key = v8::String::new(scope, "head").unwrap();
        document.set(scope, key.into(), head.into());
    }
    if let Some(html) = html_h {
        let de = wrap(scope, html);
        let key = v8::String::new(scope, "documentElement").unwrap();
        document.set(scope, key.into(), de.into());
    }

    let global = scope.get_current_context().global(scope);
    let name = v8::String::new(scope, "turbo-dom.ownership.window-document").unwrap();
    let owner = v8::Private::for_api(scope, Some(name));
    global.set_private(scope, owner, document.into());
    let key = v8::String::new(scope, "document").unwrap();
    global.set(scope, key.into(), document.into());
    // window = globalThis
    let key = v8::String::new(scope, "window").unwrap();
    global.set(scope, key.into(), global.into());
}

/// Compile + run a JS snippet in the current context (best-effort).
fn run_js(scope: &mut v8::PinScope, src: &str) {
    let tc = std::pin::pin!(v8::TryCatch::new(scope));
    let tc = &mut tc.init();
    if let Some(code) = v8::String::new(tc, src) {
        if let Some(s) = v8::Script::compile(tc, code, None) {
            s.run(tc);
        }
    }
    if tc.has_caught() && log_enabled() {
        let msg = tc
            .exception()
            .map(|e| e.to_rust_string_lossy(tc))
            .unwrap_or_default();
        eprintln!("[rust-dom] bootstrap threw: {msg}");
    }
}

/// JS enrichment shims layered over the native rtdom binding — browser globals/constructors
/// (navigator, Event subclasses, DOM-interface ctors with Symbol.hasInstance), CSSOM `.sheet`,
/// and pure-JS document extras. Kept in a sibling .js file (not Rust) so the shims are editable
/// as JS; the Rust side is purely the rtdom↔V8 binding.
const BOOTSTRAP: &str = include_str!("browser_env.js");

#[path = "range_contents.rs"]
mod range_contents;
#[path = "range_live.rs"]
mod range_live;
#[path = "selection.rs"]
mod selection;

/// Reclaim native tree records only after their actual owner Document is collected.
/// Call at a host execution/GC boundary; this never clears a retained document.
pub fn sweep_collected_documents() -> usize {
    dom_ownership::sweep_collected()
}
#[cfg(test)]
pub fn native_listener_subscription_count_for_testing() -> usize {
    event_listeners::subscription_count()
}
#[cfg(test)]
pub fn native_document_count_for_testing() -> usize {
    OTHER_DOMS.with(|states| {
        states
            .borrow()
            .values()
            .filter(|state| state.dom.is_some())
            .count()
    }) + DOM.with(|state| usize::from(state.borrow().is_some()))
}

/// Reset the per-thread DOM (call between test files when reusing an isolate).
pub fn reset() {
    // An embedding's cached runtime may be destroyed after a binding TLS cell.
    // That cell has already released its handles; clear every cell still alive.
    let _ = OTHER_DOMS.try_with(|d| d.borrow_mut().clear());
    let _ = DOM_CONTEXTS.try_with(|d| d.borrow_mut().clear());
    let _ = DOM_ID.try_with(|d| d.set(0));
    let _ = NEXT_DOM_ID.try_with(|d| d.set(1));
    let _ = STYLE.try_with(|s| s.borrow_mut().clear());
    let _ = LISTENERS.try_with(|s| s.borrow_mut().clear());
    event_listeners::reset_node_subscriptions();
    let _ = ACTIVE.try_with(|a| *a.borrow_mut() = None);
    if log_enabled() {
        dump_missing();
    }
    let _ = DOM.try_with(|d| *d.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();
    pub(super) fn init_v8() {
        INIT.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
    }

    #[test]
    fn native_parser_positions_use_actual_units_and_private_window_ownership() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html_source_positions(scope, "😀\r\n<style id=active>x{color:red}</style><template><style id=inert>x{color:blue}</style><b style=color:red></b></template>");
        let code = v8::String::new(scope, "document.getElementById('active')").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let value = script.run(scope).unwrap();
        let active = v8::Local::<v8::Object>::try_from(value).unwrap();
        let position = node_parser_position(scope, active, None).unwrap();
        assert_eq!(
            (position.utf16_offset, position.line, position.column),
            (21, 2, 18)
        );
        assert!(node_has_window_owner(scope, active));
        let code = v8::String::new(
            scope,
            "document.querySelector('template').content.querySelector('#inert')",
        )
        .unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let inert = v8::Local::<v8::Object>::try_from(script.run(scope).unwrap()).unwrap();
        // The inert association imports storage, so it does not claim a new input token.
        assert!(node_parser_position(scope, inert, None).is_none());
        assert!(!node_has_window_owner(scope, inert));
        let code = v8::String::new(
            scope,
            "document.implementation.createHTMLDocument().createElement('b')",
        )
        .unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let value = script.run(scope).unwrap();
        let factory = v8::Local::<v8::Object>::try_from(value).unwrap();
        assert!(node_parser_position(scope, factory, None).is_none());
        assert!(!node_has_window_owner(scope, factory));
        let fake = v8::Object::new(scope);
        assert!(node_parser_position(scope, fake, None).is_none());
        assert!(!node_has_window_owner(scope, fake));
        let code = v8::String::new(scope, "Object.defineProperty(document.getElementById('active'),'ownerDocument',{get(){throw Error('guest getter')}});Object.defineProperty(globalThis,'document',{get(){throw Error('guest document')}})").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        assert!(script.run(scope).is_some());
        assert_eq!(node_parser_position(scope, active, None), Some(position));
        assert!(node_has_window_owner(scope, active));
        reset();
    }

    #[test]
    fn parser_positions_follow_actual_adoption_owner_across_creation_realms() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        install_with_html_source_positions(scope, "<style id=foreign>x{}</style>");
        let node = with_tree(|tree| tree.query_selector("#foreign"))
            .flatten()
            .unwrap();
        let node = wrap(scope, node);
        let original = node_parser_position(scope, node, None).unwrap();
        let child = v8::Context::new(scope, Default::default());
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            let mut units = vec![0xd800, 13, 10];
            units.extend("<style id=child>x{}</style>".encode_utf16());
            install_independent_utf16_source_positions(scope, &units);
            let code = v8::String::new(scope, "document.getElementById('child')").unwrap();
            let script = v8::Script::compile(scope, code, None).unwrap();
            let actual = script.run(scope).unwrap();
            let actual = v8::Local::<v8::Object>::try_from(actual).unwrap();
            let position = node_parser_position(scope, actual, None).unwrap();
            assert_eq!(
                (position.utf16_offset, position.line, position.column),
                (19, 2, 17)
            );
            let key = v8::String::new(scope, "foreignNode").unwrap();
            child.global(scope).set(scope, key.into(), node.into());
            let code = v8::String::new(
                scope,
                "document.body.appendChild(document.adoptNode(foreignNode))",
            )
            .unwrap();
            let script = v8::Script::compile(scope, code, None).unwrap();
            assert!(script.run(scope).is_some());
            assert!(node_has_window_owner(scope, node));
            // Adoption currently imports native storage, so lexical provenance
            // is deliberately absent instead of claiming a new input token.
            assert!(node_parser_position(scope, node, None).is_none());
        }
        assert_eq!(original.line, 1);
        reset();
    }

    #[test]
    fn css_authorization_preserves_original_source_code_units() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let mut units = "<style>/*".encode_utf16().collect::<Vec<_>>();
        units.extend([0xd800, 120, 0xdc00]);
        units.extend("*/</style><b style='".encode_utf16());
        units.extend([0xdc01, 121, 0xd802]);
        units.extend("'></b>".encode_utf16());
        install_with_utf16_source_positions(scope, &units);
        let code=v8::String::new(scope,"globalThis.rows=[];(doc,node,kind,text)=>{rows.push([kind,Array.from({length:text.length},(_,i)=>text.charCodeAt(i))]);return false}").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let callback = script.run(scope).unwrap();
        install_css_policy(
            scope,
            v8::Local::<v8::Function>::try_from(callback).unwrap(),
        );
        let code = v8::String::new(scope, "JSON.stringify(rows)").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let value = script.run(scope).unwrap();
        let rows: serde_json::Value =
            serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap();
        assert_eq!(
            rows,
            serde_json::json!([
                ["style-src-elem", [47, 42, 55296, 120, 56320, 42, 47]],
                ["style-src-attr", [56321, 121, 55298]]
            ])
        );
        reset();
    }

    #[test]
    fn css_parser_metadata_keeps_template_sheets_inert_and_reauthorizes_adoption() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html_source_positions(scope, "<style id=active>x{color:red}</style><template><style id=inert>x{color:blue}</style><b style=color:red></b></template>");
        let code = v8::String::new(scope, "globalThis.rows=[];(document,node,kind,text,metas,source)=>{rows.push({tag:node.localName,kind,initial:Array.isArray(metas),...source});return false}").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let value = script.run(scope).unwrap();
        let callback = v8::Local::<v8::Function>::try_from(value).unwrap();
        install_css_policy(scope, callback);
        let code = v8::String::new(
            scope,
            "document.querySelector('template').content.querySelector('#inert')",
        )
        .unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let inert = v8::Local::<v8::Object>::try_from(script.run(scope).unwrap()).unwrap();
        let key = v8::String::new(scope, "inertStyle").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), inert.into());
        let code = v8::String::new(scope, r#"(()=>{
          if(rows.length!==2||rows[0].tag!=='style'||rows[0].kind!=='style-src-elem'||rows[0].line!==1||rows[0].column!==18||rows[0].offset!==17||!rows[0].ownerWindow||!rows[0].initial)throw Error('active lexical metadata '+JSON.stringify(rows));
          if(rows[1].tag!=='b'||rows[1].kind!=='style-src-attr'||rows[1].ownerWindow||!rows[1].initial||'line'in rows[1])throw Error('template attribute parser authorization '+JSON.stringify(rows));
          Object.defineProperty(Object.prototype,'ownerWindow',{set(){throw Error('guest metadata setter')},configurable:true});
          const style=inertStyle;if(style.ownerDocument===document||style.ownerDocument.defaultView!==null)throw Error('associated inert ownership');
          void style.sheet;if(rows.length!==2)throw Error('inert sheet authorization');
          document.body.appendChild(style);void style.sheet;
          if(rows.length!==3||rows[2].tag!=='style'||rows[2].kind!=='style-src-elem'||!rows[2].ownerWindow||rows[2].initial||'column'in rows[2])throw Error('activation authorization '+JSON.stringify(rows));
          return 'ok';
        })()"#).unwrap();
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let catch = &mut catch.init();
        let script = v8::Script::compile(catch, code, None).unwrap();
        let value = script.run(catch).unwrap_or_else(|| {
            panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch))
        });
        assert_eq!(value.to_rust_string_lossy(catch), "ok");
        reset();
    }

    #[test]
    fn native_range_clones_preserve_boundaries_without_exposing_or_forging_records() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><p id=target>abcdef</p></body>");
        let code = v8::String::new(scope, r#"(()=>{
          const range=document.createRange(),text=document.getElementById('target').firstChild;
          if(range.startContainer!==document||range.endContainer!==document||range.startOffset!==0||Object.keys(range).length)throw Error('initial range');
          range.setStart(text,1);range.setEnd(text,4);
          const clone=range.cloneRange();
          if(clone===range||Object.getPrototypeOf(clone)!==Range.prototype||clone.startContainer!==text||clone.endContainer!==text||clone.startOffset!==1||clone.endOffset!==4||String(clone)!=='bcd')throw Error('cloned range');
          clone.collapse(true);if(!clone.collapsed||range.collapsed||range.endOffset!==4)throw Error('independent boundaries');
          clone.startOffset=5;if(clone.startOffset!==1)throw Error('readonly boundary');
          if(Object.prototype.toString.call(clone)!=='[object Range]'||!Function.prototype.toString.call(Range.prototype.cloneRange).includes('[native code]'))throw Error('native shape');
          let errors=0;for(const value of [{},Object.create(Range.prototype)])try{Range.prototype.cloneRange.call(value);}catch(e){if(e.name==='TypeError')errors++;}
          if(errors!==2||'__nativeRangeRecord'in globalThis)throw Error('private range brand');
          return 'ok';
        })()"#).unwrap();
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let catch = &mut catch.init();
        let script = v8::Script::compile(catch, code, None).unwrap();
        let value = script.run(catch).unwrap_or_else(|| {
            panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch))
        });
        assert_eq!(value.to_rust_string_lossy(catch), "ok");
        reset();
    }

    #[test]
    fn borrowed_range_clone_uses_method_realm_and_preserves_foreign_document_boundaries() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        install_with_html(scope, "<body><p>parent</p></body>");
        let child = v8::Context::new(scope, Default::default());
        let foreign = {
            let child_scope = &mut v8::ContextScope::new(scope, child);
            install_with_html(child_scope, "<body><p>foreign</p></body>");
            let code=v8::String::new(child_scope,"(()=>{const range=document.createRange();range.selectNodeContents(document.querySelector('p').firstChild);return range;})()").unwrap();
            let script = v8::Script::compile(child_scope, code, None).unwrap();
            v8::Global::new(child_scope, script.run(child_scope).unwrap())
        };
        let foreign = dom_ownership::local(scope, foreign);
        let key = v8::String::new(scope, "foreignRange").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), foreign);
        let code=v8::String::new(scope,r#"(()=>{
          const clone=Range.prototype.cloneRange.call(foreignRange);
          if(Object.getPrototypeOf(clone)!==Range.prototype||Object.getPrototypeOf(foreignRange)===Range.prototype||clone.startContainer!==foreignRange.startContainer||String(clone)!=='foreign')throw Error('borrowed range ownership');
          const fragment=clone.createContextualFragment('<b>child</b>');
          if(fragment.ownerDocument!==foreignRange.startContainer.ownerDocument||fragment.firstChild.ownerDocument!==fragment.ownerDocument)throw Error('associated document');
          clone.collapse(true);if(foreignRange.collapsed||!clone.collapsed)throw Error('foreign independent boundaries');
          return 'ok';
        })()"#).unwrap();
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let catch = &mut catch.init();
        let script = v8::Script::compile(catch, code, None).unwrap();
        let value = script.run(catch).unwrap_or_else(|| {
            panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch))
        });
        assert_eq!(value.to_rust_string_lossy(catch), "ok");
        reset();
    }

    #[test]
    fn native_media_queries_validate_receivers_without_advertising_unavailable_decoders() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let code=v8::String::new(scope,r#"(()=>{
          const video=document.createElement('video'),audio=new Audio();
          if(Object.getPrototypeOf(audio)!==HTMLAudioElement.prototype||Audio.prototype!==HTMLAudioElement.prototype||Object.prototype.toString.call(audio)!=='[object HTMLAudioElement]')throw Error('Audio ownership');
          if(typeof video.canPlayType!=='function'||video.canPlayType.length!==1||!Function.prototype.toString.call(video.canPlayType).includes('[native code]'))throw Error('media native function');
          let conversions=0;const type={toString(){conversions++;return 'audio/wav';}};
          if(audio.canPlayType(type)!==''||conversions!==1||video.canPlayType('video/mp4; codecs="avc1.42E01E"')!==''||video.canPlayType('unsupported/type')!=='')throw Error('unsupported codec query');
          let errors=0;for(const fn of [()=>video.canPlayType(),()=>video.canPlayType(Symbol()),()=>HTMLMediaElement.prototype.canPlayType.call({},'audio/wav'),()=>HTMLMediaElement.prototype.canPlayType.call(document.createElement('div'),'audio/wav')])try{fn();}catch(e){if(e.name==='TypeError')errors++;}
          if(errors!==4)throw Error('media receiver/conversion');
          return 'ok';
        })()"#).unwrap();
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let catch = &mut catch.init();
        let script = v8::Script::compile(catch, code, None).unwrap();
        let value = script.run(catch).unwrap_or_else(|| {
            panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch))
        });
        assert_eq!(value.to_rust_string_lossy(catch), "ok");
        reset();
    }

    #[test]
    fn native_click_prevents_recursive_activation_and_preserves_cancelable_untrusted_dispatch() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><a id=link href='#target'>link</a></body>");
        let code=v8::String::new(scope,r#"(()=>{
          const element=document.getElementById('link');let calls=0,trusted;
          element.addEventListener('click',event=>{calls++;trusted=event.isTrusted;event.preventDefault();if(!event.bubbles||!event.cancelable)throw Error('click flags');element.click();});
          const result=HTMLElement.prototype.click.call(element);
          if(result!==undefined||calls!==1||trusted!==false)throw Error('recursive click activation');
          let failed=false;try{HTMLElement.prototype.click.call({});}catch(e){failed=e.name==='TypeError';}
          if(!failed)throw Error('click receiver brand');
          return 'ok';
        })()"#).unwrap();
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let catch = &mut catch.init();
        let script = v8::Script::compile(catch, code, None).unwrap();
        let value = script.run(catch).unwrap_or_else(|| {
            panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch))
        });
        assert_eq!(value.to_rust_string_lossy(catch), "ok");
        reset();
    }

    #[test]
    fn native_namespace_attributes_preserve_uri_identity_ownership_and_scope() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source=v8::String::new(scope,r#"(()=>{
          const element=document.createElementNS('urn:element','p:Mixed');
          const other=document.createElement('div');
          element.setAttributeNS('urn:first','one:value','first');
          element.setAttributeNS('urn:second','two:value','second');
          element.setAttribute('value','plain');
          const attribute=element.getAttributeNodeNS('urn:first','value');
          const errors=[];for(const action of [()=>element.setAttributeNS(null,'p:value','x'),()=>element.setAttributeNS('urn:wrong','xml:lang','x'),()=>other.setAttributeNode(attribute),()=>other.setAttributeNode({name:'fake',value:'x'}),()=>element.appendChild(attribute)])try{action();errors.push('missing')}catch(error){errors.push(error.name)}
          attribute.value='live';const clone=attribute.cloneNode();
          const before=[element instanceof Element,!(element instanceof HTMLElement),element.namespaceURI,element.prefix,element.localName,element.tagName,attribute instanceof Attr,attribute instanceof Node,attribute.nodeType,attribute.ownerElement===element,attribute.ownerDocument===document,attribute===element.getAttributeNode('one:value'),attribute===element.attributes[0],element.getAttributeNS('urn:first','value'),element.getAttributeNS('urn:second','value'),element.getAttributeNS(null,'value'),element.getAttributeNS('urn:absent','value'),clone.value,clone.ownerElement];
          const replacement=document.createAttributeNS('urn:first','new:value');replacement.value='replaced';
          const removed=element.setAttributeNodeNS(replacement);before.push(removed===attribute,attribute.ownerElement===null,replacement.ownerElement===element,element.getAttributeNodeNS('urn:first','value')===replacement);
          element.removeAttributeNS('urn:first','value');before.push(replacement.ownerElement===null,replacement.value,!element.hasAttributeNS('urn:first','value'));
          const xmlns='http://www.w3.org/2000/xmlns/';element.setAttributeNS(xmlns,'xmlns:q','urn:lookup');
          const child=document.createElementNS(null,'child');element.appendChild(child);
          before.push(child.lookupNamespaceURI('q'),child.lookupPrefix('urn:lookup'),child.lookupNamespaceURI('xml'),element.lookupNamespaceURI('p'),element.getAttributeNodeNS(xmlns,'q').lookupNamespaceURI('q'));
          const copy=element.cloneNode(true);before.push(copy.namespaceURI,copy.prefix,copy.localName,copy.getAttributeNS('urn:second','value'));
          return JSON.stringify([before,errors]);
        })()"#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"[[true,true,"urn:element","p","Mixed","p:Mixed",true,true,2,true,true,true,true,"live","second","plain",null,"live",null,true,true,true,true,true,"replaced",true,"urn:lookup","q","http://www.w3.org/XML/1998/namespace","urn:element","urn:lookup","urn:element","p","Mixed","second"],["NamespaceError","NamespaceError","InUseAttributeError","TypeError","HierarchyRequestError"]]"#
        );
        reset();
    }

    #[test]
    fn processing_instruction_uses_real_tree_and_utf16_character_data() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
        let parsed = parse_html_document(scope, "<body>", false);
        let key = v8::String::new(scope, "targetDocument").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), parsed.into());
        let source=v8::String::new(scope,r#"(()=>{
          const fail=message=>{throw Error(message)};
          const pi=document.createProcessingInstruction('test','a😀b');
          if(!(pi instanceof ProcessingInstruction)||!(pi instanceof CharacterData)||pi.nodeType!==7||pi.nodeName!=='test'||pi.target!=='test'||pi.length!==4)fail('identity');
          pi.insertData(3,'!');if(pi.data!=='a😀!b'||pi.substringData(1,2)!=='😀')fail('UTF16');
          document.body.appendChild(pi);if(document.body.textContent!==''||document.body.innerHTML!=='<?test a😀!b?>')fail('serialize');
          const clone=pi.cloneNode();if(clone===pi||clone.target!==pi.target||clone.data!==pi.data)fail('clone');
          if(targetDocument.adoptNode(pi)!==pi||pi.ownerDocument!==targetDocument||pi.parentNode!==null||pi.target!=='test')fail('adopt');
          const cases=[()=>document.createCDATASection('x'),()=>document.createProcessingInstruction('bad name','x'),()=>document.createProcessingInstruction('x','?>'),()=>pi.deleteData(99,1)];
          const errors=cases.map(action=>{try{action();return null}catch(e){return [e.name,e.code,e instanceof DOMException]}});
          const conversion=document.createProcessingInstruction('convert','data');conversion.nodeValue=null;if(conversion.data!=='')fail('nullable nodeValue');
          try{conversion.data=Symbol();fail('Symbol conversion')}catch(e){if(!(e instanceof TypeError))throw e}
          document.body.nodeValue='ignored';if(document.body.nodeValue!==null||document.body.childNodes.length!==0)fail('element nodeValue');
          const child=document.createProcessingInstruction('child','data');document.body.appendChild(child);child.before('before');child.after('after');child.remove();if(child.parentNode!==null||document.body.textContent!=='beforeafter')fail('child methods');
          document.xmlStandalone=true;document.xmlVersion='1.0';
          return JSON.stringify([errors,document.xmlEncoding,document.xmlStandalone,document.xmlVersion,document.rootElement]);
        })()"#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"[[["NotSupportedError",9,true],["InvalidCharacterError",5,true],["InvalidCharacterError",5,true],["IndexSizeError",1,true]],null,true,"1.0",null]"#
        );
        reset();
    }

    #[test]
    fn native_control_prototypes_match_independent_chrome() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
        v8::tc_scope!(let scope,scope);
        let source = v8::String::new(scope, include_str!("control_prototypes_probe.js")).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope);
        assert!(
            result.is_some(),
            "{}",
            scope
                .exception()
                .map(|value| value.to_rust_string_lossy(scope))
                .unwrap_or_default()
        );
        let actual = v8::json::stringify(scope, result.unwrap())
            .unwrap()
            .to_rust_string_lossy(scope);
        let control: serde_json::Value =
            serde_json::from_str(include_str!("control_prototypes_chrome155.json")).unwrap();
        use sha2::{Digest, Sha256};
        assert_eq!(
            control["probeSha256"],
            format!(
                "{:x}",
                Sha256::digest(include_bytes!("control_prototypes_probe.js"))
            )
        );
        assert_eq!(control["errors"], serde_json::json!([]));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&actual).unwrap(),
            control["result"]
        );
        reset();
    }

    #[test]
    fn native_checkedness_matches_independent_chrome() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
        let source = v8::String::new(scope, include_str!("checkedness_probe.js")).unwrap();
        let actual = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let actual = v8::json::stringify(scope, actual)
            .unwrap()
            .to_rust_string_lossy(scope);
        let control: serde_json::Value =
            serde_json::from_str(include_str!("checkedness_chrome155.json")).unwrap();
        use sha2::{Digest, Sha256};
        assert_eq!(
            control["probeSha256"],
            format!(
                "{:x}",
                Sha256::digest(include_bytes!("checkedness_probe.js"))
            )
        );
        assert_eq!(control["errors"], serde_json::json!([]));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&actual).unwrap(),
            control["result"]
        );
        reset();
    }

    #[test]
    fn native_control_labels_borrow_uses_receiver_realm() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let parent = v8::Context::new(scope, Default::default());
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(parent.get_security_token(scope));
        let child_input = {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<!doctype html><label for=i></label><input id=i>");
            let source = v8::String::new(scope, "document.getElementById('i')").unwrap();
            v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap()
        };
        {
            let scope = &mut v8::ContextScope::new(scope, parent);
            install_independent_html(scope, "<!doctype html><body>");
            let global = parent.global(scope);
            let key = v8::String::new(scope, "childInput").unwrap();
            global.set(scope, key.into(), child_input);
            let key = v8::String::new(scope, "childGlobal").unwrap();
            global.set(scope, key.into(), child.global(scope).into());
            let source=v8::String::new(scope,"(()=>{const list=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'labels').get.call(childInput);return JSON.stringify([list instanceof NodeList,list instanceof childGlobal.NodeList,childInput.labels===list,list.length,NodeList.prototype.item.call(list,0).htmlFor]);})()").unwrap();
            let result = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap()
                .to_rust_string_lossy(scope);
            assert_eq!(result, r#"[false,true,true,1,"i"]"#);
        }
        reset();
    }

    #[test]
    fn native_control_labels_match_chrome() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
        let source = v8::String::new(scope, include_str!("control_labels_probe.js")).unwrap();
        let value = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let actual = v8::json::stringify(scope, value)
            .unwrap()
            .to_rust_string_lossy(scope);
        let control: serde_json::Value =
            serde_json::from_str(include_str!("control_labels_chrome155.json")).unwrap();
        use sha2::{Digest, Sha256};
        assert_eq!(control["browser"], "Chrome/155.0.8059.80");
        assert_eq!(control["v8"], "15.5.35.23");
        assert_eq!(
            control["casesSha256"],
            format!(
                "{:x}",
                Sha256::digest(include_bytes!("radio_dirty_cases.json"))
            )
        );
        assert_eq!(control["errors"], serde_json::json!([]));
        assert_eq!(
            control["probeSha256"],
            format!(
                "{:x}",
                Sha256::digest(include_bytes!("control_labels_probe.js"))
            )
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&actual).unwrap(),
            control["result"][0]["result"]
        );
        reset();
    }

    #[test]
    fn native_form_data_constructor_entry_arguments_match_chrome() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
        let global = scope.get_current_context().global(scope);
        bind_method(
            scope,
            global,
            "__entryConstructor",
            |scope, args, mut rv| {
                if let Some(entries) =
                    form_data_constructor_entries(scope, args.get(0), args.get(1))
                {
                    rv.set(entries.into());
                }
            },
        );
        let source=v8::String::new(scope,"globalThis.FormData=class {constructor(form,submitter){return __entryConstructor(form,submitter);}}").unwrap();
        v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let source = v8::String::new(scope, include_str!("formdata_constructor_probe.js")).unwrap();
        let actual = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let actual = v8::json::stringify(scope, actual)
            .unwrap()
            .to_rust_string_lossy(scope);
        let control: serde_json::Value =
            serde_json::from_str(include_str!("formdata_constructor_chrome155.json")).unwrap();
        use sha2::{Digest, Sha256};
        assert_eq!(control["browser"], "Chrome/155.0.8059.80");
        assert_eq!(control["v8"], "15.5.35.23");
        assert_eq!(control["errors"], serde_json::json!([]));
        for (key, source) in [
            (
                "casesSha256",
                include_bytes!("formdata_constructor_cases.json").as_slice(),
            ),
            (
                "probeSha256",
                include_bytes!("formdata_constructor_probe.js").as_slice(),
            ),
        ] {
            assert_eq!(control[key], format!("{:x}", Sha256::digest(source)));
        }
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&actual).unwrap(),
            control["result"][0]["result"]
        );
        reset();
    }

    #[test]
    fn native_form_value_reader_uses_only_registered_private_authority() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><form id=f>");
        let global = scope.get_current_context().global(scope);
        bind_method(scope, global, "__readerTest", form_controls_native);
        let source=v8::String::new(scope,"(()=>{const token={};globalThis.readerToken=token;const brands=new WeakSet([token]);return value=>brands.has(value)?Object.freeze({kind:'private-test'}):null;})()").unwrap();
        let reader = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert!(install_form_value_reader(
            scope,
            v8::Local::<v8::Function>::try_from(reader).unwrap()
        ));
        let source=v8::String::new(scope,"JSON.stringify([__readerTest('value-snapshot',document.getElementById('f'),readerToken).kind,__readerTest('value-snapshot',document.getElementById('f'),{kind:'private-test'})])").unwrap();
        let actual = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        assert_eq!(actual, r#"["private-test",null]"#);
        reset();
    }

    #[test]
    fn native_form_submission_snapshots_are_ordered_and_host_admitted() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        use sha2::{Digest, Sha256};
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("form_entrylist_cases.json")).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("form_entrylist_chrome155.json")).unwrap();
        assert_eq!(expected["browser"], "Chrome/155.0.8059.80");
        assert_eq!(expected["v8"], "15.5.35.23");
        assert_eq!(expected["errors"], serde_json::json!([]));
        for (key, source) in [
            (
                "casesSha256",
                include_bytes!("form_entrylist_cases.json").as_slice(),
            ),
            (
                "probeSha256",
                include_bytes!("form_entrylist_probe.js").as_slice(),
            ),
        ] {
            assert_eq!(expected[key], format!("{:x}", Sha256::digest(source)));
        }
        install_independent_html(scope, cases[0]["html"].as_str().unwrap());
        let callback=v8::String::new(scope,r#"(function(op,form,submitter,payload){if(op==='active')return true;if(!Object.isFrozen(payload)||!Object.isFrozen(payload.entries)||!Object.isFrozen(payload.entries[0])||Object.getPrototypeOf(payload)!==null)throw Error('snapshot');globalThis.saved=payload;return true;})"#).unwrap();
        let callback = v8::Script::compile(scope, callback, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert!(install_form_submission_handler(
            scope,
            v8::Local::<v8::Function>::try_from(callback).unwrap()
        ));
        let source=v8::String::new(scope,r#"(()=>{const form=document.getElementById('f');form.requestSubmit(document.getElementById('b'));const entries=saved.entries.map(x=>[x[0],x[1]]);for(const i of saved.charsetEntries)entries[i][1]='UTF-8';form.firstChild.value='changed';return JSON.stringify(entries);})()"#).unwrap();
        let actual = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&actual).unwrap(),
            expected["result"][0]["result"]
        );
        let source = v8::String::new(scope, "document.getElementById('f')").unwrap();
        let form = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let source = v8::String::new(scope, "document.getElementById('b')").unwrap();
        let submitter = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let entries = form_submission_entries(
            scope,
            v8::Local::<v8::Object>::try_from(form).unwrap(),
            submitter,
        )
        .unwrap();
        let entries = v8::json::stringify(scope, entries.into())
            .unwrap()
            .to_rust_string_lossy(scope);
        let mut expected = expected["result"][0]["result"].clone();
        expected[0][1] = serde_json::json!("changed");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&entries).unwrap(),
            expected
        );
        let key = v8::String::new(scope, "document").unwrap();
        let document = scope
            .get_current_context()
            .global(scope)
            .get(scope, key.into())
            .unwrap();
        assert_eq!(
            form_submission_base_href(scope, v8::Local::<v8::Object>::try_from(document).unwrap())
                .as_deref(),
            None
        );
        reset();
    }

    #[test]
    fn native_form_validation_and_request_submit_match_independent_chrome() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body></body>");
        v8::tc_scope!(let scope, scope);
        let source = v8::String::new(scope, include_str!("form_methods_probe.js")).unwrap();
        let result = v8::Script::compile(scope, source, None).unwrap().run(scope);
        if result.is_none() {
            let exception = scope.exception().unwrap();
            let text = exception.to_rust_string_lossy(scope);
            let stack = scope
                .stack_trace()
                .map(|stack| stack.to_rust_string_lossy(scope));
            panic!("{text}: {stack:?}");
        }
        let result = result.unwrap();
        let json = v8::json::stringify(scope, result)
            .unwrap()
            .to_rust_string_lossy(scope);
        let actual: serde_json::Value = serde_json::from_str(&json).unwrap();
        let control: serde_json::Value =
            serde_json::from_str(include_str!("form_methods_chrome155.json")).unwrap();
        assert_eq!(control["browser"], "Chrome/155.0.8059.80");
        assert_eq!(control["v8"], "15.5.35.23");
        assert_eq!(control["errors"], serde_json::json!([]));
        use sha2::{Digest, Sha256};
        for (name, source) in [
            (
                "casesSha256",
                include_bytes!("form_methods_cases.json").as_slice(),
            ),
            (
                "probeSha256",
                include_bytes!("form_methods_probe.js").as_slice(),
            ),
        ] {
            assert_eq!(control[name], format!("{:x}", Sha256::digest(source)));
        }
        for (key, expected) in control["result"][0]["result"].as_object().unwrap() {
            assert_eq!(&actual[key], expected, "{key}");
        }
        assert_eq!(actual, control["result"][0]["result"]);
        reset();
    }

    #[test]
    fn native_form_sandbox_admission_preserves_clicks_and_document_ownership() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<form><button>submit</button></form>");
        let source = v8::String::new(scope, r#"globalThis.events=[];document.querySelector('button').addEventListener('click',()=>events.push('click'));document.querySelector('form').addEventListener('submit',event=>{events.push(event.isTrusted?'submit-trusted':'submit');event.preventDefault()});"#).unwrap();
        v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        for blocked in [true, false, true] {
            set_form_submission_blocked(scope, blocked);
            let source =
                v8::String::new(scope, "document.querySelector('button').click()").unwrap();
            v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap();
        }
        let child = v8::Context::new(scope, Default::default());
        {
            let child_scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(child_scope, "<form><button>child</button></form>");
            set_form_submission_blocked(child_scope, false);
        }
        let source = v8::String::new(
            scope,
            "document.querySelector('button').click();JSON.stringify(events)",
        )
        .unwrap();
        let result = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"["click","click","submit-trusted","click","click"]"#
        );
        reset();
    }

    #[test]
    fn authored_handlers_compile_real_functions_and_preserve_property_override_and_scope() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(
            scope,
            r#"<body><form id='scopeForm'><button id='btn' value='native' onclick="seen.push([this===btn,event.type,value,scopeForm.id]);return false"></button></form></body>"#,
        );
        let source=v8::String::new(scope,r#"(()=>{globalThis.seen=[];const button=document.getElementById('btn');globalThis.btn=button;globalThis.scopeForm=document.getElementById('scopeForm');const handler=button.onclick;const first=button.dispatchEvent(new Event('click',{cancelable:true}));button.onclick=()=>seen.push('manual');button.dispatchEvent(new Event('click'));button.setAttribute('onclick','seen.push("changed")');button.dispatchEvent(new Event('click'));button.removeAttribute('onclick');button.dispatchEvent(new Event('click'));return JSON.stringify([handler.name,handler.length,Function.prototype.toString.call(handler).includes('seen.push'),first,seen,button.onclick]);})()"#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"["onclick",1,true,false,[[true,"click","native","scopeForm"],"manual","changed"],null]"#
        );
        reset();
    }

    #[test]
    fn document_collections_and_adoption_preserve_live_native_nodes() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope,"<!doctype html><body><form id='first' name='named'></form><img><a href='/'></a><a name='anchor'></a><embed></body>");
        let parsed = parse_html_document(scope, "<!doctype html><body></body>", false);
        let key = v8::String::new(scope, "targetDocument").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), parsed.into());
        let child = v8::Context::new(scope, Default::default());
        let child_collection = {
            let child_scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(
                child_scope,
                "<!doctype html><body><span id='child'></span><span></span></body>",
            );
            let code = v8::String::new(child_scope, "document.body.children").unwrap();
            let script = v8::Script::compile(child_scope, code, None).unwrap();
            let value = script.run(child_scope).unwrap();
            v8::Global::new(child_scope, value)
        };
        let value = dom_ownership::local(scope, child_collection);
        let key = v8::String::new(scope, "childCollection").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), value);
        let code = v8::String::new(scope,r#"(()=>{
          const fail=message=>{throw new Error(message)};
          const forms=document.forms, children=document.body.children, nodes=document.body.childNodes;
          if(Object.getOwnPropertyDescriptor(HTMLCollection.prototype,'length').get.call(childCollection)!==2||HTMLCollection.prototype.namedItem.call(childCollection,'child')!==childCollection[0])fail('cross-realm collection brand');
          const forged=globalThis[Symbol.for('ts')].makeNamedObject(HTMLCollection.prototype,()=>({kind:'HTMLCollection',read:()=>[]}),true);let rejected=false;try{Object.getOwnPropertyDescriptor(HTMLCollection.prototype,'length').get.call(forged)}catch(e){rejected=e instanceof TypeError;}if(!rejected)fail('forged collection');

          if(Array.isArray(forms)||!(forms instanceof HTMLCollection)||forms!==document.forms||forms.first!==forms[0]||forms.namedItem('named')!==forms[0])fail('collection brand/identity/names');
          const descriptor=Object.getOwnPropertyDescriptor(forms,'0');
          if(descriptor.writable||!descriptor.enumerable||!descriptor.configurable||Object.getOwnPropertyDescriptor(forms,'first').enumerable)fail('collection descriptors');
          const first=forms[0];Reflect.set(forms,'0',null);Reflect.defineProperty(forms,'0',{value:null});if(forms[0]!==first||Reflect.deleteProperty(forms,'0'))fail('collection mutation');
          forms.expando=7;if(forms.expando!==7)fail('collection expando');
          const extra=document.createElement('form');document.body.append(extra);
          if(forms.length!==2||children.length!==6||nodes.length!==6||!(nodes instanceof NodeList)||Array.from(forms)[1]!==extra)fail('live children');
          const staticList=document.querySelectorAll('form');document.body.append(document.createElement('form'));
          if(staticList.length!==2||forms.length!==3||!(staticList instanceof NodeList)||staticList.namedItem!==undefined)fail('static NodeList');
          if(document.images.length!==1||document.links.length!==1||document.anchors.length!==1||document.embeds!==document.plugins||document.applets.length)fail('document collections');
          const byName=document.getElementsByName('named');first.setAttribute('name','changed');if(byName.length||!(byName instanceof NodeList))fail('live name query');
          if(document.compatMode!=='CSS1Compat'||!(document.doctype instanceof DocumentType)||document.doctype.name!=='html'||document.doctype.nodeName!=='html'||document.doctype.ownerDocument!==document)fail('doctype');
          const doctype=document.doctype;doctype.remove();if(document.doctype!==null||document.compatMode!=='CSS1Compat')fail('doctype mode persistence');
          targetDocument.doctype.remove();if(targetDocument.adoptNode(doctype)!==doctype||doctype.ownerDocument!==targetDocument)fail('doctype adoption');targetDocument.insertBefore(doctype,targetDocument.documentElement);if(targetDocument.doctype!==doctype||doctype.name!=='html')fail('adopted doctype tree');
          document.designMode='ON';document.designMode='invalid';if(document.designMode!=='on')fail('designMode');
          document.dir='RTL';document.bgColor='red';if(document.dir!=='rtl'||document.documentElement.getAttribute('dir')!=='RTL'||document.body.getAttribute('bgcolor')!=='red')fail('document reflected state');
          const div=document.createElement('div'),input=document.createElement('input');div.append(input);document.body.append(div);input.value='dirty';div.nonce='private';div.setAttribute('nonce','updated');
          div.nonce='private';if(div.cloneNode(true).nonce!=='private')fail('private nonce clone');div.setAttribute('nonce','updated');
          let events=0;input.addEventListener('change',()=>events++);
          const attr=div.getAttributeNode('nonce');const shadow=div.attachShadow({mode:'closed'});const shadowText=document.createTextNode('shadow');shadow.append(shadowText);
          const moved=targetDocument.adoptNode(div);
          if(attr.ownerElement!==div||attr.ownerDocument!==targetDocument||div.getAttributeNode('nonce')!==attr||shadow.host!==div||shadow.ownerDocument!==targetDocument||shadowText.ownerDocument!==targetDocument||shadow.firstChild!==shadowText)fail('adoption attr/shadow ownership');
          if(moved!==div||div.parentNode||input.ownerDocument!==targetDocument||div.ownerDocument!==targetDocument||input.value!=='dirty'||div.nonce!=='updated')fail('adoption identity/state');
          targetDocument.querySelector('body').append(div);input.dispatchEvent(new Event('change'));
          if(events!==1||targetDocument.querySelector('input')!==input||document.querySelector('input'))fail('adoption listeners/tree');
          document.adoptNode(div);document.body.append(div);if(input.ownerDocument!==document)fail('adoption roundtrip');
          let order=[];document.onabort=()=>order.push('first');document.addEventListener('abort',()=>order.push('listener'));document.onabort=()=>order.push('replacement');document.dispatchEvent(new Event('abort'));if(order.join()!=='replacement,listener')fail('document handler ordering');
          return 'ok';
        })()"#).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let catch = &mut catch.init();
        let value = script.run(catch).unwrap_or_else(|| {
            panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch))
        });
        assert_eq!(value.to_rust_string_lossy(catch), "ok");
        let code = v8::String::new(
            catch,
            "document.querySelector('div').nonce='unreflected';document.querySelector('div')",
        )
        .unwrap();
        let script = v8::Script::compile(catch, code, None).unwrap();
        let source = v8::Local::<v8::Object>::try_from(script.run(catch).unwrap()).unwrap();
        let copied = import_document_node(catch, parsed, source, true).unwrap();
        let key = v8::String::new(catch, "importedNode").unwrap();
        catch
            .get_current_context()
            .global(catch)
            .set(catch, key.into(), copied.into());
        let code=v8::String::new(catch,"importedNode.nonce==='unreflected'&&importedNode.firstChild.value==='dirty'&&importedNode.ownerDocument===targetDocument&&importedNode!==document.querySelector('div')").unwrap();
        let script = v8::Script::compile(catch, code, None).unwrap();
        assert!(script.run(catch).unwrap().is_true());
        reset();
    }

    #[test]
    fn host_frame_membership_uses_native_light_document_connections() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<body><iframe id=light sandbox='allow-scripts'></iframe><div id=host></div></body>",
        );
        let code = v8::String::new(scope,"const detached=document.createElement('iframe');const shadow=document.getElementById('host').attachShadow({mode:'closed'});const inner=document.createElement('iframe');shadow.append(inner);[document.getElementById('light'),detached,inner]").unwrap();
        let value = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let nodes = v8::Local::<v8::Array>::try_from(value).unwrap();
        for (index, expected) in [(0, true), (1, false), (2, false)] {
            let node = nodes
                .get_index(scope, index)
                .unwrap()
                .to_object(scope)
                .unwrap();
            assert_eq!(is_light_document_node(scope, node), expected);
            assert_eq!(
                node_attribute(scope, node, "sandbox").as_deref(),
                (index == 0).then_some("allow-scripts")
            );
            let key = v8::String::new(scope, "isConnected").unwrap();
            node.set(scope, key.into(), v8::Boolean::new(scope, !expected).into());
            assert_eq!(is_light_document_node(scope, node), expected);
            let document = with_tree(|tree| tree.root())
                .map(|handle| wrap(scope, handle))
                .unwrap();
            assert_eq!(composed_contains_node(scope, document, node), index != 1);
        }
        reset();
    }

    #[test]
    fn event_records_preserve_readonly_descriptors_and_dispatch_phases() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><main><button>press</button></main></body>");
        let source = v8::String::new(scope, r#"(() => {
          const event=new Event('test',{bubbles:true,cancelable:true,composed:true});
          const descriptor=Object.getOwnPropertyDescriptor(event,'isTrusted');
          let invalid=false;try{descriptor.get.call({})}catch(e){invalid=e instanceof TypeError}
          event.type='changed';event.bubbles=false;event.__trusted=true;
          delete event.__trusted;
          const descriptors=['type','target','currentTarget','eventPhase','bubbles','cancelable','defaultPrevented','composed','timeStamp'].every(name=>{
            const d=Object.getOwnPropertyDescriptor(Event.prototype,name);return d.enumerable&&d.configurable&&typeof d.get==='function'&&!d.set&&!Object.hasOwn(event,name);
          });
          const phases=[],button=document.querySelector('button'),main=document.querySelector('main');
          main.addEventListener('test',e=>phases.push([e.eventPhase,e.currentTarget===main,e.composedPath()[0]===button]),true);
          button.addEventListener('test',e=>{phases.push([e.eventPhase,e.currentTarget===button,e.target===button]);e.preventDefault();});
          main.addEventListener('test',e=>phases.push([e.eventPhase,e.currentTarget===main,e.defaultPrevented]));
          const result=button.dispatchEvent(event);
          return JSON.stringify({keys:Object.keys(event),descriptor:[descriptor.enumerable,descriptor.configurable,!descriptor.set,descriptor.get.name,descriptor.get.length,!Object.hasOwn(descriptor.get,'prototype')],invalid,descriptors,
            readonly:event.type==='test'&&event.bubbles&&event.isTrusted===false,
            stamp:event.timeStamp>=0&&event.timeStamp<1000,tag:Object.prototype.toString.call(event),phases,
            after:[result,event.defaultPrevented,event.currentTarget,event.eventPhase,event.composedPath().length,event.target===button],
            private:!('__nativeEventRecord'in globalThis)&&!('__captureEventRecord'in globalThis)&&!('ttEventRecords'in globalThis[Symbol.for('ts')])});
        })()"#).unwrap();
        let value = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap();
        assert_eq!(value["keys"], serde_json::json!(["isTrusted"]));
        assert_eq!(
            value["descriptor"],
            serde_json::json!([true, false, true, "get isTrusted", 0, true])
        );
        for name in ["invalid", "descriptors", "readonly", "stamp", "private"] {
            assert_eq!(value[name], true, "{value}");
        }
        assert_eq!(value["tag"], "[object Event]");
        assert_eq!(
            value["phases"],
            serde_json::json!([[1, true, true], [2, true, true], [3, true, true]])
        );
        assert_eq!(
            value["after"],
            serde_json::json!([false, true, null, 0, 0, true])
        );
    }

    #[test]
    fn host_event_trust_is_private_and_script_redispatch_clears_it() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let global_template = v8::ObjectTemplate::new(scope);
        global_template.set_internal_field_count(2);
        let context = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(global_template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source = v8::String::new(scope, "globalThis.event=new Event('test');event").unwrap();
        let event = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let event = v8::Local::<v8::Object>::try_from(event).unwrap();
        let source=v8::String::new(scope,"globalThis.seen=[];document.addEventListener('test',e=>seen.push(e.isTrusted));document").unwrap();
        let target = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let target = v8::Local::<v8::Object>::try_from(target).unwrap();
        mark_event_trusted(scope, event, Some(37.5));
        dispatch_host_event(scope, event, target);
        let source = v8::String::new(
            scope,
            r#"(() => {
          event.__trusted=true;document.dispatchEvent(event);
          return JSON.stringify([seen,event.isTrusted,event.timeStamp]);
        })()"#,
        )
        .unwrap();
        let value = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(
            value.to_rust_string_lossy(scope),
            "[[true,false],false,37.5]"
        );
        let source=v8::String::new(scope,"globalThis.windowTrust=null;window.addEventListener('host-window',event=>windowTrust=event.isTrusted);new Event('host-window')").unwrap();
        let event = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let event = v8::Local::<v8::Object>::try_from(event).unwrap();
        let target = context.global(scope);
        dispatch_host_event(scope, event, target);
        let source = v8::String::new(scope, "String(windowTrust)").unwrap();
        let value = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(
            value.to_rust_string_lossy(scope),
            "true",
            "host Window dispatch must use private captured registry"
        );
        let source=v8::String::new(scope,"globalThis.loadSeen=null;window.addEventListener('load',e=>loadSeen=[e.target===document,e.currentTarget===window,e.eventPhase,e.isTrusted]);globalThis.loadEvent=new Event('load');loadEvent").unwrap();
        let event = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let event = v8::Local::<v8::Object>::try_from(event).unwrap();
        dispatch_host_event(scope, event, target);
        let source=v8::String::new(scope,"JSON.stringify([loadSeen,loadEvent.currentTarget===window,loadEvent.eventPhase,loadEvent.composedPath().length])").unwrap();
        let value = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(
            value.to_rust_string_lossy(scope),
            "[[true,true,2,true],true,2,1]"
        );
        let source=v8::String::new(scope,"try{window.dispatchEvent(loadEvent);}catch(error){globalThis.redispatchError=error.name;}JSON.stringify([loadSeen,redispatchError,loadEvent.currentTarget===window,loadEvent.eventPhase])").unwrap();
        let value = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(
            value.to_rust_string_lossy(scope),
            "[[true,true,2,true],\"InvalidStateError\",true,2]"
        );
        install_with_html(scope, "<body><button>new document</button></body>");
        let source=v8::String::new(scope,r#"(() => {const bus=new EventTarget(),e=new Event('x',{cancelable:true});let phase;bus.addEventListener('x',ev=>{phase=ev.eventPhase;ev.returnValue=false;});return JSON.stringify([bus.dispatchEvent(e),phase,e.currentTarget,e.defaultPrevented]);})()"#).unwrap();
        let value = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(value.to_rust_string_lossy(scope), "[false,2,null,true]");
    }

    #[test]
    fn page_intrinsic_overrides_and_listener_caller_cannot_capture_trusted_event_state() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source = v8::String::new(
            scope,
            r#"(() => {
            const get=WeakMap.prototype.get,set=WeakMap.prototype.set;
            globalThis.capturedRecords=0;
            WeakMap.prototype.set=function(key,value){
                if(key instanceof Event){capturedRecords++;value.trusted=true;}
                return set.call(this,key,value);
            };
            WeakMap.prototype.get=function(key){
                const record=get.call(this,key);
                if(key instanceof Event && record){capturedRecords++;record.trusted=true;}
                return record;
            };
            globalThis.scriptEvent=new Event('host-private');
            globalThis.scriptTrust=scriptEvent.isTrusted;
            scriptEvent.type='changed';scriptEvent.target=window;scriptEvent.isTrusted=true;
            globalThis.initialReadonly=scriptEvent.target===null&&scriptEvent.type==='host-private'&&!scriptEvent.isTrusted;
            globalThis.hostSeen=[];globalThis.leakedDispatcher=null;
            function listener(event){
                try{leakedDispatcher=listener.caller;}catch(error){leakedDispatcher=null;}
                hostSeen.push(event.isTrusted);
            }
            window.addEventListener('host-private',listener);
            return new Event('host-private');
        })()"#,
        )
        .unwrap();
        let event = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let event = v8::Local::<v8::Object>::try_from(event).unwrap();
        let target = context.global(scope);
        dispatch_host_event(scope, event, target);
        let source = v8::String::new(
            scope,
            r#"(() => {
            const leak=leakedDispatcher;
            if(typeof leak==='function')leak(scriptEvent,window);
            window.dispatchEvent(scriptEvent);
            return JSON.stringify({capturedRecords,scriptTrust,callerHidden:leak===null,
                hostSeen,scriptTrusted:scriptEvent.isTrusted,type:scriptEvent.type,
                initialReadonly,
                trustedReadonly:!Object.getOwnPropertyDescriptor(scriptEvent,'isTrusted').set});
        })()"#,
        )
        .unwrap();
        let result = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"{"capturedRecords":0,"scriptTrust":false,"callerHidden":true,"hostSeen":[true,false],"scriptTrusted":false,"type":"host-private","initialReadonly":true,"trustedReadonly":true}"#
        );
        reset();
    }

    #[test]
    fn native_dom_methods_reject_construction_and_preserve_call_behavior() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source = v8::String::new(
            scope,
            r#"
          const element = document.createElement('div');
          const methods = [document.createTextNode, element.setAttribute,
            element.getAttribute, element.style.setProperty];
          const notConstructible = methods.every(fn => {
            if (Object.hasOwn(fn, 'prototype')) return false;
            try { Reflect.construct(function(){}, [], fn); return false; }
            catch (error) { return error instanceof TypeError; }
          });
          let rejectsNew = false;
          try { new document.createTextNode('text'); }
          catch (error) { rejectsNew = error instanceof TypeError; }
          element.setAttribute('data-token', 'kept');
          element.style.setProperty('color', 'red');
          const attributes = Object.getOwnPropertyDescriptor(document.createTextNode, 'length');
          JSON.stringify([notConstructible, rejectsNew,
            methods.map(fn => [fn.name, fn.length]),
            element.getAttribute('data-token'), element.style.getPropertyValue('color'),
            new Event('preserved').type,
            [attributes.writable, attributes.enumerable, attributes.configurable]]);
        "#,
        )
        .unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"[true,true,[["createTextNode",1],["setAttribute",2],["getAttribute",1],["setProperty",2]],"kept","red","preserved",[false,false,true]]"#
        );
        reset();
    }

    #[test]
    fn projected_native_accessors_are_shared_live_and_receiver_checked() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source = v8::String::new(scope, r#"
          const element=document.createElement('div');
          const id=Object.getOwnPropertyDescriptor(Element.prototype,'id');
          const type=Object.getOwnPropertyDescriptor(Node.prototype,'nodeType');
          const rejected=[{},document,document.createTextNode('text')].every(receiver=>{
            try{id.get.call(receiver);return false}catch(error){return error instanceof TypeError}
          });
          id.set.call(element,'live');
          element.innerHTML='<span>child</span>';
          const clone=element.cloneNode(true);
          JSON.stringify([!Object.hasOwn(element,'id'),element.getAttribute('id'),id.get.call(element),
            id.get.name,id.get.length,id.set.name,id.set.length,!Object.hasOwn(id.get,'prototype'),
            rejected,type.get.call(element),document.ownerDocument===null,
            Object.getPrototypeOf(element)===HTMLDivElement.prototype,
            element.firstChild.parentNode===element,clone.textContent,
            clone.appendChild===Node.prototype.appendChild]);
        "#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"[true,"live","live","get id",0,"set id",1,true,true,1,true,true,true,"child",true]"#
        );
        reset();
    }

    #[test]
    fn native_named_objects_preserve_nonmasking_properties_and_private_mutations() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source=v8::String::new(scope,r#"(()=>{
          const data=new Map([['first','one'],['getItem','stored']]);
          const proto={getItem(){return 'method'}};
          const object=globalThis[Symbol.for('ts')].makeNamedObject(proto,(operation,key,value)=>{
            if(operation==='get')return data.get(key);
            if(operation==='has')return data.has(key);
            if(operation==='keys')return Array.from(data.keys());
            if(operation==='descriptor')return data.has(key)?{value:data.get(key),writable:true,enumerable:true,configurable:true}:undefined;
            if(operation==='set'){data.set(key,String(value));return true;}
            if(operation==='delete'){data.delete(key);return true;}
            if(operation==='define'){if('get'in value||'set'in value)throw new TypeError('Named accessors unsupported');data.set(key,String(value.value));return true;}
          });
          const collisionBefore=[typeof object.getItem,Object.hasOwn(object,'getItem'),Object.getOwnPropertyDescriptor(object,'getItem')===undefined,Object.keys(object).includes('getItem')];
          object.second='two';object.getItem='replacement';Object.defineProperty(object,'defined',{value:'value'});
          const symbol=Symbol('expando');object[symbol]=42;
          const before={collisionBefore,keys:Object.keys(object).sort(),method:typeof object.getItem,collision:data.get('getItem'),own:Object.hasOwn(object,'getItem'),descriptor:Object.getOwnPropertyDescriptor(object,'getItem'),first:object.first,defined:data.get('defined'),symbol:object[symbol],accessorThrows:false};
          try{Object.defineProperty(object,'accessor',{get(){return 3}})}catch(e){before.accessorThrows=e instanceof TypeError;}
          delete object.second;before.deleted=!data.has('second');
          return JSON.stringify(before);
        })()"#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"{"collisionBefore":["function",false,true,true],"keys":["defined","first","getItem","second"],"method":"string","collision":"stored","own":true,"descriptor":{"value":"replacement","writable":true,"enumerable":true,"configurable":true},"first":"one","defined":"value","symbol":42,"accessorThrows":true,"deleted":true}"#
        );
        reset();
    }

    #[test]
    fn native_indexed_objects_keep_live_readonly_entries_and_ordinary_expandos() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source=v8::String::new(scope,r#"(()=>{
          const entries=['first','second'];const proto={item(index){return this[index]??null}};
          const named=new Map([['named','first']]);
          const value=(key)=>/^\d+$/.test(key)?entries[Number(key)]:named.get(key);
          const object=globalThis[Symbol.for('ts')].makeNamedObject(proto,(operation,key,argument)=>{
            if(operation==='indices')return entries.map((_,index)=>index);
            if(operation==='keys')return Array.from(named.keys());
            const current=value(key);const supported=current!==undefined;
            if(operation==='get')return current;
            if(operation==='has')return supported;
            if(operation==='descriptor')return supported?{value:current,writable:false,enumerable:/^\d+$/.test(key),configurable:true}:undefined;
            if(operation==='set'||operation==='delete'||operation==='define')return supported?false:undefined;
          },true);
          const initial=[object[0],object.item(1),Object.keys(object).join(','),Object.getOwnPropertyDescriptor(object,'0').writable,Object.getOwnPropertyDescriptor(object,'named').enumerable];
          object.expando='extra';Object.defineProperty(object,'defined',{value:3});object[9]='ordinary';
          entries.shift();named.set('named','second');
          const result=[initial,object[0],object.named,object.expando,object.defined,object[9],Object.hasOwn(object,'1'),Reflect.deleteProperty(object,'0')];
          delete object.expando;result.push(Object.hasOwn(object,'expando'));
          return JSON.stringify(result);
        })()"#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"[["first","second","0,1",false,false],"second","second","extra",3,"ordinary",false,false,false]"#
        );
        reset();
    }

    #[test]
    fn replacing_a_document_rebuilds_projected_native_methods() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        for html in [
            "<body><p>first</p></body>",
            "<body><p>second</p></body>",
            "<body><p>third</p></body>",
        ] {
            install_independent_html(scope, html);
            let source = v8::String::new(
                scope,
                r#"(()=>{
              const node=document.createElement('span');node.textContent='new';
              document.body.appendChild(node);
              const style=document.createElement('style');style.textContent='sentinel {}';
              document.head.appendChild(style);
              const rules=style.sheet.cssRules;
              globalThis.retainedSheets=globalThis.retainedSheets||[];
              if(retainedSheets.some(sheet=>sheet.cssRules.length!==1||sheet.cssRules.item(0).selectorText!=='.retained'||sheet.cssRules.item(0).style.getPropertyValue('color')!=='red'||sheet.cssRules.item(0).cssText!=='.retained { color: red; }'))return false;
              const constructed=new CSSStyleSheet();constructed.replaceSync('.retained { color: red; }');retainedSheets.push(constructed);
              return document.body.querySelector('span')===node &&
                node.appendChild===Node.prototype.appendChild &&
                node.ownerDocument===document && rules.length===1 &&
                rules.item(0).selectorText==='sentinel' &&
                rules instanceof CSSRuleList && document.styleSheets.length===1;
            })()"#,
            )
            .unwrap();
            let script = v8::Script::compile(scope, source, None).unwrap();
            assert!(script.run(scope).unwrap().boolean_value(scope));
        }
        reset();
    }

    #[test]
    fn native_facade_functions_preserve_class_construction_and_live_collections() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let code = r#"(() => {
          const make=globalThis[Symbol.for('ts')].makeNativeFunction;
          class State { constructor(value){if(value==='bad')throw new RangeError('bad');this.value=value;this.target=new.target;} }
          const Wrapped=make(State,'State',1,true,true); Wrapped.prototype=State.prototype;
          class Child extends Wrapped {}
          const construct=Reflect.construct; Reflect.construct=()=>{throw new Error('modified');};
          const instance=new Child(7); Reflect.construct=construct;
          let missingNew=false,error=false;
          try{Wrapped(7)}catch(e){missingNew=e instanceof TypeError}
          try{new Wrapped('bad')}catch(e){error=e instanceof RangeError}
          const sheet=new CSSStyleSheet(),rules=sheet.cssRules;sheet.replaceSync('sentinel {}');
          const getter=Object.getOwnPropertyDescriptor(CSSRuleList.prototype,'length').get;
          let invalid=false;try{getter.call({})}catch(e){invalid=e instanceof TypeError}
          return instance.value===7&&instance.target===Child&&instance instanceof Child&&instance instanceof State&&instance instanceof Wrapped
            &&missingNew&&error&&invalid&&rules===sheet.cssRules&&rules.length===1&&rules.item(0).selectorText==='sentinel'
            &&!Object.hasOwn(rules.item,'prototype')&&!Object.hasOwn(getter,'prototype')&&getter.name==='get length'
            &&Function.prototype.toString.call(rules.item)==='function item() { [native code] }';
        })()"#;
        let src = v8::String::new(scope, code).unwrap();
        let script = v8::Script::compile(scope, src, None).unwrap();
        assert!(script.run(scope).unwrap().boolean_value(scope));
        reset();
    }

    #[test]
    fn native_facade_constructor_can_preserve_date_call_and_subclass_contracts() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source = v8::String::new(scope, r#"(() => {
          const make=globalThis[Symbol.for('ts')].makeNativeFunction,RealDate=Date,construct=Reflect.construct;
          function Clock(...args){if(!new.target)return new RealDate(1234).toString();return construct(RealDate,args.length?args:[1234],new.target);}
          const Wrapped=make(Clock,'Date',7,true,true,false);Wrapped.prototype=RealDate.prototype;
          class Child extends Wrapped {}
          const value=new Child();
          let method=false;try{new (make(()=>7,'method',0))()}catch(e){method=e instanceof TypeError;}
          return Wrapped()===new RealDate(1234).toString()&&value.getTime()===1234&&new Wrapped(42).getTime()===42
            &&value instanceof Child&&value instanceof RealDate&&method&&Wrapped.length===7
            &&Function.prototype.toString.call(Wrapped)==='function Date() { [native code] }';
        })()"#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        assert!(script.run(scope).unwrap().boolean_value(scope));
        reset();
    }

    #[test]
    fn native_dom_methods_check_receiver_type_before_tree_access() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source = v8::String::new(scope, r#"
          const element=document.createElement('div'),text=document.createTextNode('text');
          const calls=[
            ()=>document.querySelector.call(null,'body'),
            ()=>document.querySelector.call(element,'body'),
            ()=>element.querySelector.call(document,'body'),
            ()=>element.getAttribute.call({},'id'),
            ()=>element.getAttribute.call(text,'id'),
            ()=>element.appendChild.call({},text),
            ()=>document.createTextNode.call({},'text'),
          ];
          JSON.stringify([calls.every(call=>{try{call();return false}catch(error){return error instanceof TypeError}}),
            document.querySelector('body')===document.body,
            element.getAttribute('id')===null,
            element.appendChild(text)===text,
            element.firstChild===text]);
        "#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(result, "[true,true,true,true,true]");
        reset();
    }

    #[test]
    fn inert_same_realm_documents_keep_native_tree_ownership_and_imports() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let parsed=parse_html_document(scope,"<html><head><title>parsed</title></head><body><p id='source'>native</p><script>throw new Error('inert')</script></body></html>",false);
        let global = context.global(scope);
        let key = v8::String::new(scope, "parsed").unwrap();
        global.set(scope, key.into(), parsed.into());
        let key = v8::String::new(scope, "document").unwrap();
        let doc = global
            .get(scope, key.into())
            .unwrap()
            .to_object(scope)
            .unwrap();
        let source = {
            let _parsed = DomScope::for_object(scope, parsed);
            let node = with_tree(|tree| {
                NodeRef::new(tree, tree.root())
                    .query_selector("#source")
                    .unwrap()
                    .handle()
            })
            .unwrap();
            wrap(scope, node)
        };
        let imported = import_document_node(scope, doc, source, true).unwrap();
        let key = v8::String::new(scope, "imported").unwrap();
        global.set(scope, key.into(), imported.into());
        let src=v8::String::new(scope,r#"(() => {const p=parsed.querySelector('#source');p.textContent='changed';return parsed!==document&&Object.getPrototypeOf(parsed)===HTMLDocument.prototype&&p.ownerDocument===parsed&&document.querySelector('#source')===null&&imported.ownerDocument===document&&imported.textContent==='native'&&imported!==p&&parsed.querySelector('head').textContent==='parsed';})()"#).unwrap();
        let script = v8::Script::compile(scope, src, None).unwrap();
        assert!(script.run(scope).unwrap().boolean_value(scope));
        reset();
    }

    #[test]
    fn document_policy_installation_routes_by_context_and_preserves_parent_handlers() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        install_with_html(
            scope,
            "<style id=sheet>div{color:red}</style><body><div id=node style='opacity:0.4'></div>",
        );
        let source = v8::String::new(scope, "(()=>true)").unwrap();
        let allow = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let allow = v8::Local::<v8::Function>::try_from(allow).unwrap();
        install_inline_event_handlers(scope, allow, "https://parent.test/");
        install_css_policy(scope, allow);
        let source = v8::String::new(
            scope,
            "globalThis.calls=0;document.getElementById('node').onload=()=>calls++;",
        )
        .unwrap();
        v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(parent.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<style id=sheet>div{color:blue}</style><body><div id=node style='opacity:0.8' onload='globalThis.authored=true'></div>");
            // The independent installer restores the parent's ambient DOM before
            // these public hooks run, while the child V8 context remains current.
            let source = v8::String::new(scope, "(()=>false)").unwrap();
            let deny = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap();
            let deny = v8::Local::<v8::Function>::try_from(deny).unwrap();
            install_inline_event_handlers(scope, deny, "https://child.test/");
            install_css_policy(scope, deny);
        }
        let key = v8::String::new(scope, "child").unwrap();
        let child_global = child.global(scope);
        parent
            .global(scope)
            .set(scope, key.into(), child_global.into());
        let source = v8::String::new(scope, r#"(()=>{
            const node=document.getElementById('node'),foreign=child.document.getElementById('node');
            const parentHandler=typeof node.onload==='function';node.dispatchEvent(new Event('load'));
            foreign.dispatchEvent(new child.Event('load'));
            return JSON.stringify({parentHandler,calls,parentSheet:document.getElementById('sheet').sheet!==null,
                parentInline:node.style.opacity,childSheet:child.document.getElementById('sheet').sheet===null,
                childInline:foreign.style.opacity,childHandler:foreign.onload===null,childAuthored:child.authored===undefined});
        })()"#).unwrap();
        let result = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        assert_eq!(
            result,
            r#"{"parentHandler":true,"calls":1,"parentSheet":true,"parentInline":"0.4","childSheet":true,"childInline":"","childHandler":true,"childAuthored":true}"#
        );
        reset();
    }

    #[test]
    fn independent_context_documents_route_native_nodes_and_reentrant_events() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        install(scope);
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(parent.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<html><head><title>child</title><style>child-sentinel {}</style></head><body><div id='only-child'>child</div></body></html>");
        }
        let global = parent.global(scope);
        let key = v8::String::new(scope, "child").unwrap();
        let child_global = child.global(scope);
        global.set(scope, key.into(), child_global.into());
        let src = v8::String::new(
            scope,
            r#"
          document.body.innerHTML = '<div id="only-parent">parent</div>';
          const node = child.document.getElementById('only-child');
          const independent = document.getElementById('only-child') === null
            && child.document.getElementById('only-parent') === null;
          node.setAttribute('data-touched', 'yes');
          if (child.document.head.querySelector('title').textContent !== 'child'
              || child.document.querySelectorAll('head').length !== 1) throw new Error('parsed head was replaced');
          if(child.document.styleSheets.length!==1||child.document.styleSheets.item(0).cssRules.length!==1
              ||child.document.styleSheets.item(0).cssRules.item(0).selectorText!=='child-sentinel')throw new Error('child CSSOM receiver identity was lost');
          let nested = '';
          node.addEventListener('probe', () => {
            document.getElementById('only-parent').textContent = 'updated';
            nested = child.document.getElementById('only-child').textContent;
          });
          node.dispatchEvent(new Event('probe'));
          const ownNode=document.getElementById('only-parent');
          document.body.replaceChildren(node);
          if (node.ownerDocument !== document || document.body.firstChild !== node
              || child.document.getElementById('only-child') !== null || ownNode.parentNode !== null)
            throw new Error('variadic adoption lost native node identity');
          const separateApis = child.document.createElement.constructor === child.Function
            && child.Event !== Event && child.HTMLElement !== HTMLElement;
          JSON.stringify([independent, node.ownerDocument === document,
            node.getAttribute('data-touched'), nested,
            ownNode.textContent, separateApis]);
        "#,
        )
        .unwrap();
        let script = v8::Script::compile(scope, src, None).unwrap();
        let out = script.run(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(out, r#"[true,true,"yes","child","updated",true]"#);
        reset();
    }

    #[test]
    fn dom_slice_smoke() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        install(scope);

        let run = |scope: &mut v8::PinScope, code: &str| -> String {
            let src = v8::String::new(scope, code).unwrap();
            let script = v8::Script::compile(scope, src, None).unwrap();
            let r = script.run(scope).unwrap();
            r.to_rust_string_lossy(scope)
        };

        // create + append + attribute + textContent + querySelector + identity
        let out = run(
            scope,
            r#"
            const d = document.createElement('div');
            d.setAttribute('id', 'x');
            d.className = 'card';
            d.textContent = 'hello';
            document.body.appendChild(d);
            const found = document.querySelector('#x');
            JSON.stringify({
                tag: d.tagName,
                id: d.id,
                cls: d.className,
                text: found.textContent,
                identity: found === d,
                getAttr: d.getAttribute('id'),
                inBody: document.body.firstChild === d,
            });
        "#,
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["tag"], "DIV", "{out}");
        assert_eq!(v["id"], "x", "{out}");
        assert_eq!(v["cls"], "card", "{out}");
        assert_eq!(v["text"], "hello", "{out}");
        assert_eq!(
            v["identity"], true,
            "querySelector must return the SAME object: {out}"
        );
        assert_eq!(v["getAttr"], "x", "{out}");
        assert_eq!(v["inBody"], true, "{out}");
        reset();
    }

    #[test]
    fn private_css_policy_preserves_authored_dom_and_effective_cssom() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope,"<style id=early>#a{color:red}</style><meta http-equiv=Content-Security-Policy content=deny><style id=late>#a{color:blue}</style><div id=a style='color:blue'><span id=child></span></div>");
        let source=v8::String::new(scope,"(function(doc,node,kind,text,metas){return kind==='style-src-elem' && Array.isArray(metas) && metas.length===0;})").unwrap();
        let function = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        install_css_policy(
            scope,
            v8::Local::<v8::Function>::try_from(function).unwrap(),
        );
        let source=v8::String::new(scope,r#"(()=>{const a=document.getElementById('a'),early=document.getElementById('early'),late=document.getElementById('late');
            const initial=early.sheet!==null && late.sheet===null && document.styleSheets.length===1 && a.getAttribute('style')==='color:blue' && a.style.color==='';
            early.remove();a.style.color='green';const cssom=a.style.color==='green' && a.getAttribute('style')==='color: green;';
            a.setAttribute('style','color:red');const blocked=a.style.color==='green' && a.getAttribute('style')==='color:red' && getComputedStyle(a).color==='rgb(0, 128, 0)';
            const inherited=getComputedStyle(document.getElementById('child')).color==='rgb(0, 128, 0)';
            a.removeAttribute('style');const removed=a.style.color==='';
            const privateHooks=typeof globalThis.__writeAppliedInlineStyle==='undefined' && !globalThis[Symbol.for('ts')].attachLinkedStyleSheet && !globalThis[Symbol.for('ts')].readStyleSheetRules;
            return JSON.stringify({initial,cssom,blocked,inherited,removed,privateHooks});})()"#).unwrap();
        let result = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        let result: serde_json::Value = serde_json::from_str(&result).unwrap();
        for field in [
            "initial",
            "cssom",
            "blocked",
            "inherited",
            "removed",
            "privateHooks",
        ] {
            assert_eq!(result[field], true, "{field}: {result}");
        }
        reset();
    }

    #[test]
    fn css_shared_records_allow_cross_realm_borrowed_getters_and_guard_opaque_sheets() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let first = v8::Context::new(scope, Default::default());
        let retained = {
            let scope = &mut v8::ContextScope::new(scope, first);
            install(scope);
            let source=v8::String::new(scope,"(()=>{const sheet=new CSSStyleSheet({media:'screen'});sheet.replaceSync('.a {color:red}');return sheet;})()").unwrap();
            let sheet = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap();
            v8::Global::new(scope, sheet)
        };
        let second = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, second);
        install(scope);
        let sheet = dom_ownership::local(scope, retained);
        let key = v8::String::new(scope, "foreignSheet").unwrap();
        second.global(scope).set(scope, key.into(), sheet);
        let document = wrap(scope, with_tree(Tree::root).unwrap());
        let record = collection_records::get(scope, document)
            .unwrap()
            .to_object(scope)
            .unwrap();
        let key = v8::String::new(scope, "cssHost").unwrap();
        let host = record.get(scope, key.into()).unwrap();
        let key = v8::String::new(scope, "hostCSSForTest").unwrap();
        second.global(scope).set(scope, key.into(), host);
        let source=v8::String::new(scope,r#"(()=>{const rules=Object.getOwnPropertyDescriptor(CSSStyleSheet.prototype,'cssRules').get.call(foreignSheet);const rule=rules[0];
        const style=Object.getOwnPropertyDescriptor(CSSStyleRule.prototype,'style').get.call(rule);const value=CSSStyleDeclaration.prototype.getPropertyValue.call(style,'color');
        const media=Object.getOwnPropertyDescriptor(StyleSheet.prototype,'media').get.call(foreignSheet);const crossRealm=value==='red' && Object.getOwnPropertyDescriptor(CSSRuleList.prototype,'length').get.call(rules)===1 && MediaList.prototype.item.call(media,0)==='screen';
        const link=document.createElement('link');document.head.appendChild(link);const opaque=hostCSSForTest.attachLinked(link,'.a {color:blue}','https://other.invalid/sheet.css',false);let errors=[];
        for(const fn of [()=>opaque.cssRules,()=>opaque.rules,()=>opaque.insertRule('.b {}'),()=>opaque.deleteRule(0)]){try{fn();}catch(error){errors.push(error.name);}}
        const internal=hostCSSForTest.readRules(opaque).length===1;
        const parsed=document.createElement('style');parsed.textContent='@import "actual.css";';document.head.appendChild(parsed);const importRule=parsed.sheet.cssRules[0],liveRules=parsed.sheet.cssRules;
        Object.defineProperty(importRule,'href',{get(){throw new Error('Guest href override');}});Object.defineProperty(importRule,'type',{get(){throw new Error('Guest type override');}});Object.defineProperty(liveRules,Symbol.iterator,{value(){throw new Error('Guest iterator override');}});
        const arrayIterator=Array.prototype[Symbol.iterator],arraySlice=Array.prototype.slice;
        Array.prototype[Symbol.iterator]=function(){throw new Error('Guest Array iterator override');};Array.prototype.slice=function(){throw new Error('Guest Array slice override');};
        const snapshot=hostCSSForTest.readRules(parsed.sheet),info=hostCSSForTest.readRule(snapshot[0]);
        Array.prototype[Symbol.iterator]=arrayIterator;Array.prototype.slice=arraySlice;const hostReads=Array.isArray(snapshot)&&info.type===3&&info.href==='actual.css';
        snapshot.length=0;const snapshotIsolated=hostCSSForTest.readRules(parsed.sheet).length===1;
        let branded=false;try{Object.getOwnPropertyDescriptor(CSSStyleSheet.prototype,'cssRules').get.call({});}catch(error){branded=error instanceof TypeError;}
        return JSON.stringify({crossRealm,security:errors.length===4&&errors.every(name=>name==='SecurityError'),internal,hostReads,snapshotIsolated,branded});})()"#).unwrap();
        let result = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        let result: serde_json::Value = serde_json::from_str(&result).unwrap();
        for field in [
            "crossRealm",
            "security",
            "internal",
            "hostReads",
            "snapshotIsolated",
            "branded",
        ] {
            assert_eq!(result[field], true, "{field}: {result}");
        }
        reset();
    }

    #[test]
    fn cssom_parses_style_text_and_parser_created_controls_have_idl_values() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let source = v8::String::new(scope, r#"
            document.body.innerHTML='<input id="token" type="hidden" value="abc&amp;def"><textarea id="q">search</textarea><style>body { color: red; } sentinel {}</style>';
            const input=document.querySelector('#token'), textarea=document.querySelector('#q');
            const style=document.querySelector('style'), sheet=style.sheet;
            const parsed=sheet.cssRules.length===2 && sheet.cssRules[1].selectorText==='sentinel' && sheet.cssRules[1] instanceof CSSStyleRule;
            const value=input.value==='abc&def' && input instanceof HTMLInputElement && textarea.value==='search';
            const text=style.textContent;
            sheet.insertRule('a { color: blue; }',1);
            const inserted=sheet.cssRules.length===3 && sheet.cssRules[1].selectorText==='a' && style.textContent===text;
            style.textContent='/*comment*/ h1 { content: "}"; } @media screen { p { color: red; } }';
            const replacement=style.sheet;
            const updated=replacement!==sheet && sheet.cssRules.length===3 && sheet.ownerNode===null && replacement.cssRules.length===2 && replacement.cssRules[0].style.getPropertyValue('content')==='"}"' && replacement.cssRules[1].cssRules[0].selectorText==='p';
            const registered=document.styleSheets.length===1 && document.styleSheets[0]===replacement;
            style.remove();
            const removed=document.styleSheets.length===0 && style.sheet===null;
            JSON.stringify({parsed,value,inserted,updated,registered,removed});
        "#).unwrap();
        let script = v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        let result: serde_json::Value = serde_json::from_str(&result).unwrap();
        for field in [
            "parsed",
            "value",
            "inserted",
            "updated",
            "registered",
            "removed",
        ] {
            assert_eq!(result[field], true, "{field}: {result}");
        }
        reset();
    }

    // A cross-origin <iframe> hosts a bridged second window realm: contentWindow is a
    // distinct object with its own document, and postMessage crosses the realm boundary
    // in BOTH directions carrying the correct source/origin (the reCAPTCHA anchor↔bframe
    // handshake shape). Delivery is async (setTimeout), so a queue shim + drain stands in
    // for the real message loop.
    #[test]
    fn iframe_second_realm_bridges_post_message_both_ways() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);

        let run = |scope: &mut v8::PinScope, code: &str| -> String {
            let src = v8::String::new(scope, code).unwrap();
            let script = v8::Script::compile(scope, src, None).unwrap();
            let r = script.run(scope).unwrap();
            r.to_rust_string_lossy(scope)
        };

        let out = run(
            scope,
            r#"
            // Virtual timer queue so postMessage delivery is drainable (no wall clock).
            const __q = [];
            globalThis.setTimeout = (fn) => { __q.push(fn); return __q.length; };
            const drain = () => { let n = 0; while (__q.length && n++ < 1000) { (__q.shift())(); } };
            globalThis.location = { origin: 'https://page.test', href: 'https://page.test/' };

            const iframe = document.createElement('iframe');
            iframe.setAttribute('src', 'https://recaptcha.test/frame');
            const cw = iframe.contentWindow;
            const cd = iframe.contentDocument;

            const distinct = cw !== globalThis && cw.document === cd && cw.self === cw;
            const ownDoc = cd.body != null && cd.body !== document.body;

            // Child builds its own DOM into its own document and queries it back through
            // the child document facade. (The underlying binding has a single native tree,
            // so the child subtree is not tree-level isolated from the parent — the shared
            // object graph noted in browser_env.js; the realm boundary is the window +
            // message channel, which is what a cross-frame protocol relies on.)
            const el = cd.createElement('div'); el.setAttribute('id', 'inchild');
            cd.body.appendChild(el);
            const childFindsOwn = cd.getElementById('inchild') != null &&
                                  cd.querySelector('#inchild') === el;

            // Child listens for a challenge and replies through e.source (→ parent).
            let childSaw = '';
            cw.addEventListener('message', (e) => {
                childSaw = e.data + '|' + e.origin + '|' + (e.source === cw.parent);
                e.source.postMessage('token:CHILD-REPLY', e.origin);
            });
            // Parent listens for the reply and checks source identity + child origin.
            let parentSaw = '';
            addEventListener('message', (e) => {
                parentSaw = e.data + '|' + e.origin + '|' + (e.source === iframe.contentWindow);
            });

            // Parent → child.
            cw.postMessage('challenge:GO', 'https://recaptcha.test');
            drain(); drain();

            JSON.stringify({ distinct, ownDoc, childFindsOwn, childSaw, parentSaw, childOrigin: cw.origin });
        "#,
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["distinct"], true,
            "contentWindow is a distinct realm: {out}"
        );
        assert_eq!(v["ownDoc"], true, "child has its own document body: {out}");
        assert_eq!(
            v["childFindsOwn"], true,
            "child finds its own element via its own document: {out}"
        );
        assert_eq!(v["childOrigin"], "https://recaptcha.test", "{out}");
        // Child received the parent's challenge with the parent origin + a source it can reply to.
        assert_eq!(
            v["childSaw"], "challenge:GO|https://page.test|true",
            "{out}"
        );
        // Parent received the child's reply with source === the iframe + the child origin.
        assert_eq!(
            v["parentSaw"], "token:CHILD-REPLY|https://recaptcha.test|true",
            "cross-realm reply must carry the child origin + iframe source: {out}"
        );
        reset();
    }

    // WebGL emulates a coherent headless-Chrome SwiftShader context: the identity strings
    // (VENDOR/RENDERER + UNMASKED_* via WEBGL_debug_renderer_info), the numeric limits, the
    // extension list, shader precision, and deterministic content-dependent readPixels — the
    // surface a GPU fingerprinter reads. No longer null (a null WebGL is a stronger bot tell).
    #[test]
    fn native_document_fragments_clone_and_move_owned_children() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let code = v8::String::new(scope, r#"
          const f=document.createDocumentFragment();
          const one=document.createElement('b');one.id='one';
          const two=document.createTextNode('two');
          f.appendChild(one);f.appendChild(two);
          const clone=f.cloneNode(true);
          const host=document.createElement('div');
          const ref=document.createElement('i');host.appendChild(ref);
          const returned=host.insertBefore(f,ref);
          const moved=returned===f && f.childNodes.length===0 && host.firstChild===one && one.nextSibling===two && two.nextSibling===ref;
          host.appendChild(clone);
          const own=f.nodeType===11 && f instanceof DocumentFragment && f.ownerDocument===document && f.parentNode===null && f.host===undefined;
          const cloned=clone.nodeType===11 && clone.parentNode===null && clone.childNodes.length===0 && host.childNodes.length===5 && host.childNodes[3]!==one && host.childNodes[3].id==='one';
          JSON.stringify({own,moved,cloned,type:f.nodeType,interface:f instanceof DocumentFragment,owner:f.ownerDocument===document,detached:f.parentNode===null,hostProperty:('host' in f)})
        "#).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        for key in ["own", "moved", "cloned"] {
            assert_eq!(value[key], true, "{key}: {result}");
        }
        reset();
    }

    #[test]
    fn native_outer_html_replaces_and_detaches_the_original_element() {
        init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<body><iframe id=old></iframe><b id=after></b></body>",
        );
        let code = v8::String::new(scope, r#"
          const old=document.getElementById('old');
          old.outerHTML='<p id=first>one</p><span id=second>two</span>';
          const detached=document.createElement('div');detached.outerHTML='<b></b>';
          let protectedRoot=false;try{document.documentElement.outerHTML='<p></p>'}catch(e){protectedRoot=e.name==='NoModificationAllowedError'}
          JSON.stringify({old:!old.isConnected&&old.parentNode===null,
            order:Array.from(document.body.children,e=>e.id).join(',')==='first,second,after',
            detached:detached.tagName==='DIV',protectedRoot})
        "#).unwrap();
        let out = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            result,
            serde_json::json!({"old":true,"order":true,"detached":true,"protectedRoot":true})
        );
        reset();
    }

    #[test]
    fn native_shadow_roots_have_separate_contents_brand_and_open_closed_modes() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let code=v8::String::new(scope,r#"
          const host=document.createElement('div');document.body.appendChild(host);
          host.innerHTML='<i id=light>light</i>';
          const root=host.attachShadow({mode:'open'});
          root.innerHTML='<span id=secret>shadow</span>';
          const secret=root.querySelector('#secret');
          const branded=root!==host && root instanceof ShadowRoot && root instanceof DocumentFragment && root.nodeType===11 && root.host===host && root.mode==='open' && root.parentNode===null && host.shadowRoot===root;
          const boundary=secret && secret.parentNode===root && root.getRootNode()===root && secret.getRootNode()===root && secret.getRootNode({composed:true})===document && secret.isConnected && !host.contains(secret) && document.querySelector('#secret')===null && host.querySelector('#secret')===null && host.querySelector('#light')!==null;
          const hidden=document.createElement('div');document.body.appendChild(hidden);
          const closed=hidden.attachShadow({mode:'closed'});
          const modes=hidden.shadowRoot===null && closed.host===hidden && closed.mode==='closed';
          let duplicate=false,invalid=false,insertion=false,clone=false;
          try{hidden.attachShadow({mode:'open'})}catch(e){duplicate=e.name==='NotSupportedError'}
          try{Element.prototype.attachShadow.call({},{mode:'open'})}catch(e){invalid=e instanceof TypeError}
          insertion=host.appendChild(root)===root && root.childNodes.length===0 && secret.parentNode===host && root.host===host && root.ownerDocument===document;
          try{root.cloneNode(true)}catch(e){clone=e.name==='NotSupportedError'}
          JSON.stringify({branded,boundary,modes,duplicate,invalid,insertion,clone})
        "#).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        for key in [
            "branded",
            "boundary",
            "modes",
            "duplicate",
            "invalid",
            "insertion",
            "clone",
        ] {
            assert_eq!(value[key], true, "{key}: {result}");
        }
        reset();
    }

    #[test]
    fn webgl_reports_a_coherent_swiftshader_signature() {
        init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);

        let run = |scope: &mut v8::PinScope, code: &str| -> String {
            let src = v8::String::new(scope, code).unwrap();
            let script = v8::Script::compile(scope, src, None).unwrap();
            let r = script.run(scope).unwrap();
            r.to_rust_string_lossy(scope)
        };

        let out = run(
            scope,
            r#"
            const c = document.createElement('canvas'); c.width = 256; c.height = 256;
            const gl = c.getContext('webgl');
            const notNull = gl != null;
            const dbg = gl.getExtension('WEBGL_debug_renderer_info');
            const unmaskedVendor = gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL);
            const unmaskedRenderer = gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL);
            const vendor = gl.getParameter(gl.VENDOR);
            const renderer = gl.getParameter(gl.RENDERER);
            const version = gl.getParameter(gl.VERSION);
            const maxTex = gl.getParameter(gl.MAX_TEXTURE_SIZE);
            const exts = gl.getSupportedExtensions();
            const hasDbg = exts.indexOf('WEBGL_debug_renderer_info') >= 0;
            const prec = gl.getShaderPrecisionFormat(gl.FRAGMENT_SHADER || 0x8B30, gl.HIGH_FLOAT);

            // readPixels: deterministic + content-dependent (draw ops change the hash).
            const rp = () => { const px = new Uint8Array(4*16); gl.readPixels(0,0,4,4,0x1908,0x1401,px); return Array.from(px.slice(0,8)).join(','); };
            const a1 = rp();
            gl.clearColor(0.1,0.2,0.3,1.0); gl.clear(0x4000); gl.drawArrays(0x0004,0,3);
            const a2 = rp();
            const a3 = rp(); // same op-state, read again → must be identical (deterministic)

            const gl2 = document.createElement('canvas').getContext('webgl2');
            const version2 = gl2.getParameter(gl2.VERSION);

            JSON.stringify({ notNull, unmaskedVendor, unmaskedRenderer, vendor, renderer, version,
              version2, maxTex, hasDbg, extCount: exts.length,
              precOk: prec.rangeMin === 127 && prec.rangeMax === 127 && prec.precision === 23,
              detStable: a2 === a3, contentDep: a1 !== a2 });
        "#,
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["notNull"], true, "webgl context must not be null: {out}");
        assert_eq!(v["unmaskedVendor"], "Google Inc. (Google)", "{out}");
        assert_eq!(
            v["unmaskedRenderer"],
            "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)",
            "{out}"
        );
        assert_eq!(v["vendor"], "WebKit", "{out}");
        assert_eq!(v["renderer"], "WebKit WebGL", "{out}");
        assert_eq!(v["version"], "WebGL 1.0 (OpenGL ES 2.0 Chromium)", "{out}");
        assert_eq!(v["version2"], "WebGL 2.0 (OpenGL ES 3.0 Chromium)", "{out}");
        assert_eq!(v["maxTex"], 8192, "{out}");
        assert_eq!(
            v["hasDbg"], true,
            "WEBGL_debug_renderer_info must be advertised: {out}"
        );
        assert!(
            v["extCount"].as_u64().unwrap() > 20,
            "full extension list: {out}"
        );
        assert_eq!(
            v["precOk"], true,
            "highp float precision is SwiftShader's 127/127/23: {out}"
        );
        assert_eq!(
            v["detStable"], true,
            "identical GL draws hash identically: {out}"
        );
        assert_eq!(
            v["contentDep"], true,
            "different GL draws change readback: {out}"
        );
        reset();
    }
}

#[cfg(test)]
#[path = "shadow_slots_tests.rs"]
mod shadow_slots_tests;

#[cfg(test)]
#[path = "frame_tokens_tests.rs"]
mod frame_tokens_tests;

#[cfg(test)]
#[path = "html_meta_tests.rs"]
mod html_meta_tests;

#[cfg(test)]
#[path = "computed_defaults_tests.rs"]
mod computed_defaults_tests;
#[cfg(test)]
#[path = "html_image_tests.rs"]
mod html_image_tests;

#[cfg(test)]
#[path = "css_live_tests.rs"]
mod css_live_tests;

#[cfg(test)]
#[path = "dom_projection_tests.rs"]
mod dom_projection_tests;

#[cfg(test)]
#[path = "css_adoption_tests.rs"]
mod css_adoption_tests;

#[cfg(test)]
#[path = "css_admission_tests.rs"]
mod css_admission_tests;

#[cfg(test)]
#[path = "dom_implementation_tests.rs"]
mod dom_implementation_tests;
#[cfg(test)]
#[path = "mutation_observers_tests.rs"]
mod mutation_observers_tests;

#[cfg(test)]
#[path = "ownership_guard_tests.rs"]
mod ownership_guard_tests;

#[cfg(test)]
#[path = "iframe_credentialless_tests.rs"]
mod iframe_credentialless_tests;

#[cfg(test)]
#[path = "utf16_contract_tests.rs"]
mod utf16_contract_tests;

#[cfg(test)]
#[path = "form_outputs_tests.rs"]
mod form_outputs_tests;

#[cfg(test)]
#[path = "animations_tests.rs"]
mod animations_tests;
#[cfg(test)]
#[path = "file_inputs_tests.rs"]
mod file_inputs_tests;
#[cfg(test)]
#[path = "form_pseudo_tests.rs"]
mod form_pseudo_tests;
#[cfg(test)]
#[path = "html_dialog_tests.rs"]
mod html_dialog_tests;
#[cfg(test)]
#[path = "key_order_tests.rs"]
mod key_order_tests;
#[cfg(test)]
#[path = "pointer_events_tests.rs"]
mod pointer_events_tests;
#[cfg(test)]
#[path = "sanitizer_tests.rs"]
mod sanitizer_tests;
#[cfg(test)]
#[path = "typed_om_tests.rs"]
mod typed_om_tests;

#[cfg(test)]
#[path = "view_transitions_tests.rs"]
mod view_transitions_tests;

#[cfg(test)]
#[path = "editing_tests.rs"]
mod editing_tests;

// turbo-dom-browser-env crate surface (scripts/browser-env-rewrite.sh).
#[path = "crate_surface.rs"]
mod crate_surface;
pub use crate_surface::*;

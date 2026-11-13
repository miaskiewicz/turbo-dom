//! Auxiliary browsing-context identities and synchronous host admission.
//!
//! The host allocates a real context in the opener's isolate and queues its
//! network/navigation work. No guest object is accepted as a substitute for a
//! WindowProxy. The registry is weak: Page ownership belongs to the host.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

/// Why a browsing context was selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenSource {
    Script,
    Hyperlink,
}

/// Stable within one host isolate/actor, including across document navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AuxiliaryContextId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuxiliaryIdentity {
    pub id: AuxiliaryContextId,
    /// Immutable initiating context; not cleared by noopener or opener=null.
    pub creation_source: Option<AuxiliaryContextId>,
    pub auxiliary: bool,
    pub closed: bool,
}

/// Converted input to the synchronous admission callback. URLs are resolved by
/// the producer; named targets retain their original UTF-16 code units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRequest {
    pub source_id: AuxiliaryContextId,
    pub target_id: AuxiliaryContextId,
    pub new_context: bool,
    /// False for an omitted/empty URL: an existing target must not navigate.
    pub navigate: bool,
    /// Creator document base inherited by a new initial about:blank document.
    pub source_base_url: String,
    pub url: String,
    pub target: Vec<u16>,
    pub features: BTreeMap<String, String>,
    pub noopener: bool,
    pub noreferrer: bool,
    pub source: OpenSource,
}

/// The host must enforce popup activation/sandbox/navigation policy before
/// returning a context. New contexts must already contain an independent DOM
/// and a genuine WindowProxy, with initial about:blank origin inheritance. A
/// selected context must be returned unchanged. Fetching and script execution
/// are queued after admission; no guest callback may run during this callback.
pub type OpenHost = for<'s, 'i> fn(
    &mut v8::PinScope<'s, 'i>,
    v8::Local<'s, v8::Context>,
    Option<v8::Local<'s, v8::Context>>,
    &OpenRequest,
) -> Option<v8::Local<'s, v8::Context>>;

/// Accept closure of this owned auxiliary only, never the shared runtime.
pub type CloseHost = for<'s, 'i> fn(&mut v8::PinScope<'s, 'i>, v8::Local<'s, v8::Context>) -> bool;

#[derive(Clone, Copy)]
pub struct AuxiliaryHost {
    pub open: OpenHost,
    pub close: CloseHost,
}

/// A configured host owns init failures: they must never silently fall back to
/// scripts registered for another Page in the shared actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadyAction {
    InitializedByHost,
    RunRegisteredInitScripts,
    Abort,
}

/// Called in the target context after canonical identity/surface commit and
/// before open returns. All native borrows are dropped. The host can publish
/// the Page and run per-target init scripts, including reentrant open/close.
pub type ReadyHost = for<'s, 'i> fn(
    &mut v8::PinScope<'s, 'i>,
    v8::Local<'s, v8::Context>,
    v8::Local<'s, v8::Context>,
    &OpenRequest,
) -> ReadyAction;

struct ReadySlot(ReadyHost);

/// Install before guest code. New auxiliaries inherit this native capability;
/// the host selects context-wide rather than opener Page-only init scripts.
pub fn set_ready_host(context: v8::Local<v8::Context>, host: Option<ReadyHost>) {
    if let Some(host) = host {
        context.set_slot(Rc::new(ReadySlot(host)));
    } else {
        context.remove_slot::<ReadySlot>();
    }
}

#[derive(Default)]
struct Group(RefCell<Vec<Weak<State>>>);

struct State {
    id: AuxiliaryContextId,
    creation_source: Option<AuxiliaryContextId>,
    context: RefCell<v8::Weak<v8::Context>>,
    proxy: v8::Weak<v8::Object>,
    group: RefCell<Rc<Group>>,
    opener: RefCell<Option<v8::Weak<v8::Object>>>,
    parent: Option<v8::Weak<v8::Context>>,
    name: RefCell<Vec<u16>>,
    url: RefCell<String>,
    base_url: RefCell<String>,
    closed: Cell<bool>,
    init_scripts_run: Cell<bool>,
    auxiliary: bool,
    host: Option<AuxiliaryHost>,
}

struct Slot(Rc<State>);

struct IdCounter(Cell<u64>);

fn next_id(scope: &mut v8::PinScope) -> Option<AuxiliaryContextId> {
    let counter = scope
        .get_slot::<Rc<IdCounter>>()
        .cloned()
        .unwrap_or_else(|| {
            let counter = Rc::new(IdCounter(Cell::new(0)));
            scope.set_slot(Rc::clone(&counter));
            counter
        });
    let value = counter.0.get().checked_add(1)?;
    counter.0.set(value);
    Some(AuxiliaryContextId(value))
}

pub fn identity(context: v8::Local<v8::Context>) -> Option<AuxiliaryIdentity> {
    let record = state(context)?;
    Some(AuxiliaryIdentity {
        id: record.id,
        creation_source: record.creation_source,
        auxiliary: record.auxiliary,
        closed: record.closed.get(),
    })
}

pub fn document_url(context: v8::Local<v8::Context>) -> Option<String> {
    state(context).map(|record| record.url.borrow().clone())
}

pub fn document_fallback_base(context: v8::Local<v8::Context>) -> Option<String> {
    state(context).map(|record| record.base_url.borrow().clone())
}

fn state(context: v8::Local<v8::Context>) -> Option<Rc<State>> {
    context.get_slot::<Slot>().map(|slot| Rc::clone(&slot.0))
}

fn weak_proxy(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> v8::Weak<v8::Object> {
    let proxy = context.global(scope);
    v8::Weak::new(scope, proxy)
}

fn installed_document_id(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> Option<u64> {
    if let Some(id) = context.get_slot::<super::DomContextId>() {
        return Some(id.0);
    }
    // The primary install uses the main tree rather than the independent-tree
    // helper. Recover its real document owner through the native private edge;
    // never read guest window.document or assume the current tree is its owner.
    let scope = &mut v8::ContextScope::new(scope, context);
    let name = v8::String::new(scope, "turbo-dom.ownership.window-document")?;
    let key = v8::Private::for_api(scope, Some(name));
    let document =
        v8::Local::<v8::Object>::try_from(context.global(scope).get_private(scope, key)?).ok()?;
    if document.get_creation_context(scope) != Some(context)
        || super::handle_of(scope, document).is_none()
    {
        return None;
    }
    let id = super::dom_id_of(scope, document);
    context.set_slot(Rc::new(super::DomContextId(id)));
    Some(id)
}

/// Install only the independent engine/DOM, deferring guest init scripts until
/// canonical popup identity and host admission have been committed.
pub fn initialize_document(scope: &mut v8::PinScope, html: &str) {
    super::install_independent_tree_without_init(
        scope,
        super::Tree::parse_with_processing_instructions(html),
    );
}

/// Independent-document staged install retaining original UTF-16 token and
/// attribute provenance for the host's CSP hash/nonce/source checks.
pub fn initialize_utf16_source_positions(scope: &mut v8::PinScope, units: &[u16]) {
    super::install_independent_tree_without_init(
        scope,
        super::Tree::parse_utf16_with_source_positions_and_processing_instructions(units),
    );
}

/// Primary-document equivalent for a host whose root services address the
/// binding's current main tree. It also defers guest init scripts.
pub fn initialize_root_document(scope: &mut v8::PinScope, html: &str) {
    if !super::is_worker_context(scope.get_current_context()) {
        super::install_document_body(scope, super::Tree::parse_with_processing_instructions(html));
    }
}

/// Primary-document staged install with the same original-source provenance.
pub fn initialize_root_utf16_source_positions(scope: &mut v8::PinScope, units: &[u16]) {
    if !super::is_worker_context(scope.get_current_context()) {
        super::install_document_body(
            scope,
            super::Tree::parse_utf16_with_source_positions_and_processing_instructions(units),
        );
    }
}

/// Run configured init scripts once, only after this context has a committed
/// identity. The host queues this after admission, before child page scripts.
/// Use with initialize_document; the normal install helper already runs scripts.
pub fn run_init_scripts(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    let Some(record) = state(context) else {
        return false;
    };
    if record.closed.get()
        || record.context.borrow().to_local(scope) != Some(context)
        || context.global(scope).get_creation_context(scope) != Some(context)
    {
        return false;
    }
    if record.init_scripts_run.replace(true) {
        return false;
    }
    let scope = &mut v8::ContextScope::new(scope, context);
    let Some(document_id) = context.get_slot::<super::DomContextId>() else {
        return false;
    };
    let _document = super::DomScope::enter(document_id.0);
    super::run_init_scripts(scope);
    true
}

/// Install the canonical open/close/opener/closed surface after initializing a
/// top-level Window, before guest code. `None` means popup admission is denied.
/// Calling this again does not replace an existing group's ownership records.
pub fn install(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
    document_url: &str,
    host: Option<AuxiliaryHost>,
) -> bool {
    if state(context).is_some()
        || installed_document_id(scope, context).is_none()
        || !super::auxiliary_window_proxy::adopt(scope, context)
    {
        return false;
    }
    let Some(id) = next_id(scope) else {
        return false;
    };
    let record = Rc::new(State {
        id,
        creation_source: None,
        context: RefCell::new(v8::Weak::new(scope, context)),
        proxy: weak_proxy(scope, context),
        group: RefCell::new(Rc::new(Group::default())),
        opener: RefCell::new(None),
        parent: None,
        name: RefCell::new(Vec::new()),
        url: RefCell::new(document_url.to_owned()),
        base_url: RefCell::new(document_url.to_owned()),
        closed: Cell::new(false),
        init_scripts_run: Cell::new(false),
        auxiliary: false,
        host,
    });
    record
        .group
        .borrow()
        .0
        .borrow_mut()
        .push(Rc::downgrade(&record));
    context.set_slot(Rc::new(Slot(record)));
    install_registered_surface(scope, context)
}

/// Register a real nested browsing context as an opening source. Frame parent
/// relationships are separate from auxiliary opener relationships: a popup's
/// parent/top stay self. The host still enforces the frame's sandbox policy.
pub fn install_nested(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
    parent: v8::Local<v8::Context>,
    document_url: &str,
    name: &[u16],
    host: Option<AuxiliaryHost>,
) -> bool {
    let Some(parent) = state(parent) else {
        return false;
    };
    if state(context).is_some() || installed_document_id(scope, context).is_none() {
        return false;
    }
    let Some(parent_context) = parent.context.borrow().to_local(scope) else {
        return false;
    };
    let Some(id) = next_id(scope) else {
        return false;
    };
    let record = Rc::new(State {
        id,
        creation_source: None,
        context: RefCell::new(v8::Weak::new(scope, context)),
        proxy: weak_proxy(scope, context),
        group: RefCell::new(Rc::clone(&parent.group.borrow())),
        opener: RefCell::new(None),
        parent: Some(v8::Weak::new(scope, parent_context)),
        name: RefCell::new(name.to_vec()),
        url: RefCell::new(document_url.to_owned()),
        base_url: RefCell::new(document_url.to_owned()),
        closed: Cell::new(false),
        init_scripts_run: Cell::new(false),
        auxiliary: false,
        host,
    });
    record
        .group
        .borrow()
        .0
        .borrow_mut()
        .push(Rc::downgrade(&record));
    context.set_slot(Rc::new(Slot(record)));
    install_registered_surface(scope, context)
}

/// Trusted opener identity for a native WindowProxy cross-origin handler.
pub fn opener<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'s, v8::Context>,
) -> Option<v8::Local<'s, v8::Object>> {
    let record = state(context)?;
    let result = record.opener.borrow().as_ref()?.to_local(scope);
    result
}

/// Native lifecycle state; guest replacement of `window.closed` cannot forge it.
pub fn is_closed(context: v8::Local<v8::Context>) -> Option<bool> {
    state(context).map(|record| record.closed.get())
}

/// Sever an opener relationship after an accepted noopener/COOP policy change.
/// The host must also perform any required WindowProxy backing/group swap; this
/// function never changes origin permission or pretends navigation succeeded.
pub fn sever_opener(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    let Some(record) = state(context) else {
        return false;
    };
    record.opener.borrow_mut().take();
    clear_opener_edge(scope, context);
    *record.group.borrow_mut() = Rc::new(Group::default());
    record
        .group
        .borrow()
        .0
        .borrow_mut()
        .push(Rc::downgrade(&record));
    true
}

/// Retire a host-owned auxiliary after close/crash. Retained WindowProxy reads
/// observe closed; the weak registry stops selecting its name immediately.
pub fn retire(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    let Some(record) = state(context) else {
        return false;
    };
    if record.closed.replace(true) {
        return false;
    }
    record.opener.borrow_mut().take();
    clear_opener_edge(scope, context);
    super::host_internals::retire_context(context);
    true
}

/// Transfer identity after a committed navigation that reused the real global
/// proxy. A cross-origin/COOP host must update V8 access checks before exposing
/// the new document. Different proxy identities are refused, never papered over.
pub fn commit_navigation(
    scope: &mut v8::PinScope,
    previous: v8::Local<v8::Context>,
    current: v8::Local<v8::Context>,
    url: &str,
) -> bool {
    let Some(record) = state(previous) else {
        return false;
    };
    if record.closed.get()
        || !record
            .proxy
            .to_local(scope)
            .is_some_and(|proxy| proxy.strict_equals(current.global(scope).into()))
        || current.get_slot::<super::DomContextId>().is_none()
        || current.global(scope).get_creation_context(scope) != Some(current)
        || url::Url::parse(url).is_err()
    {
        return false;
    }
    *record.context.borrow_mut() = v8::Weak::new(scope, current);
    *record.url.borrow_mut() = url.to_owned();
    *record.base_url.borrow_mut() = url.to_owned();
    record.init_scripts_run.set(false);
    current.set_slot(Rc::new(Slot(record)));
    if previous != current {
        super::host_internals::retire_context(previous);
    }
    super::auxiliary_window_proxy::synchronize_origins(scope, current);
    install_surface(scope, current)
}

/// Commit a host-admitted browsing-context-group separation (for example COOP).
/// A new genuine proxy is required. The opener's retained old proxy becomes
/// closed, while the host Page's stable actor ID continues on the new context.
/// This does not emit a Page-close event or shut down the actor/network owner.
pub fn commit_isolated_navigation(
    scope: &mut v8::PinScope,
    previous: v8::Local<v8::Context>,
    current: v8::Local<v8::Context>,
    url: &str,
) -> bool {
    let Some(previous_record) = state(previous) else {
        return false;
    };
    if previous_record.closed.get()
        || previous_record.parent.is_some()
        || state(current).is_some()
        || !super::auxiliary_window_proxy::is_managed(current)
        || previous
            .global(scope)
            .strict_equals(current.global(scope).into())
        || current.global(scope).get_creation_context(scope) != Some(current)
        || current.get_slot::<super::DomContextId>().is_none()
        || url::Url::parse(url).is_err()
    {
        return false;
    }
    let record = Rc::new(State {
        id: previous_record.id,
        creation_source: previous_record.creation_source,
        context: RefCell::new(v8::Weak::new(scope, current)),
        proxy: weak_proxy(scope, current),
        group: RefCell::new(Rc::new(Group::default())),
        opener: RefCell::new(None),
        parent: None,
        name: RefCell::new(Vec::new()),
        url: RefCell::new(url.to_owned()),
        base_url: RefCell::new(url.to_owned()),
        closed: Cell::new(false),
        init_scripts_run: Cell::new(false),
        auxiliary: previous_record.auxiliary,
        host: previous_record.host,
    });
    record
        .group
        .borrow()
        .0
        .borrow_mut()
        .push(Rc::downgrade(&record));
    current.set_slot(Rc::new(Slot(record)));
    if !install_surface(scope, current) {
        return false;
    }
    retire(scope, previous);
    true
}

fn install_surface(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    let scope = &mut v8::ContextScope::new(scope, context);
    if !super::auxiliary_window_proxy::install_document(scope, context) {
        return false;
    }
    super::auxiliary_window_proxy::capture_exception(scope, context);
    let global = context.global(scope);
    if let Some(opener) = state(context).and_then(|record| {
        let opener = record.opener.borrow().as_ref()?.to_local(scope);
        opener
    }) {
        let key = opener_key(scope);
        if global.set_private(scope, key, opener.into()) != Some(true) {
            return false;
        }
    }
    super::bind_method(scope, global, "open", open_callback);
    super::bind_method(scope, global, "close", close_callback);
    let accessors: [(&str, v8::FunctionCallback, Option<v8::FunctionCallback>); 4] = [
        (
            "closed",
            v8::MapFnTo::map_fn_to(ext!(FunctionCallback, closed_get)),
            None,
        ),
        (
            "opener",
            v8::MapFnTo::map_fn_to(ext!(FunctionCallback, opener_get)),
            Some(v8::MapFnTo::map_fn_to(ext!(FunctionCallback, opener_set))),
        ),
        (
            "name",
            v8::MapFnTo::map_fn_to(ext!(FunctionCallback, name_get)),
            Some(v8::MapFnTo::map_fn_to(ext!(FunctionCallback, name_set))),
        ),
        (
            "length",
            v8::MapFnTo::map_fn_to(ext!(FunctionCallback, length_get)),
            None,
        ),
    ];
    for (name, getter, setter) in accessors {
        let getter = v8::Function::builder_raw(getter)
            .constructor_behavior(v8::ConstructorBehavior::Throw)
            .build(scope)
            .unwrap();
        let getter_name = v8::String::new(scope, &format!("get {name}")).unwrap();
        getter.set_name(getter_name);
        let setter = setter
            .map(|setter| {
                let setter = v8::Function::builder_raw(setter)
                    .constructor_behavior(v8::ConstructorBehavior::Throw)
                    .length(1)
                    .build(scope)
                    .unwrap();
                let setter_name = v8::String::new(scope, &format!("set {name}")).unwrap();
                setter.set_name(setter_name);
                setter.into()
            })
            .unwrap_or_else(|| v8::undefined(scope).into());
        let mut descriptor = v8::PropertyDescriptor::new_from_get_set(getter.into(), setter);
        descriptor.set_enumerable(true);
        descriptor.set_configurable(true);
        let name = v8::String::new(scope, name).unwrap();
        if global.define_property(scope, name.into(), &descriptor) != Some(true) {
            return false;
        }
    }
    true
}

fn install_registered_surface(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    if install_surface(scope, context) {
        return true;
    }
    clear_opener_edge(scope, context);
    context.remove_slot::<Slot>();
    false
}

fn owner<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    receiver: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Context>> {
    let context = receiver.get_creation_context(scope)?;
    context
        .global(scope)
        .strict_equals(receiver.into())
        .then_some(context)
}

fn same_origin_owner<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    receiver: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Context>> {
    let Some(target) = owner(scope, receiver) else {
        let message = v8::String::new(scope, "Illegal invocation").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return None;
    };
    let incumbent = v8::Global::new(scope, scope.get_incumbent_context());
    let source = v8::Local::new(scope, &incumbent);
    if !source
        .get_security_token(scope)
        .strict_equals(target.get_security_token(scope))
    {
        super::throw_dom_error(scope, "SecurityError", "Blocked cross-origin window access");
        return None;
    }
    Some(target)
}

fn closed_get<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue<'s>,
) {
    if let Some(closed) = owner(scope, args.this()).and_then(is_closed) {
        rv.set_bool(closed);
    }
}

fn length_get<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue<'s>,
) {
    if let Some(context) = owner(scope, args.this()) {
        let length = super::auxiliary_window_proxy::children(scope, context).len();
        rv.set_uint32(length.min(u32::MAX as usize) as u32);
    }
}

fn opener_get<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue<'s>,
) {
    rv.set(v8::null(scope).into());
    if let Some(context) = owner(scope, args.this()) {
        if let Some(opener) = opener(scope, context) {
            rv.set(opener.into());
        }
    }
}

fn opener_set<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _: v8::ReturnValue<'s>,
) {
    let Some(context) = same_origin_owner(scope, args.this()) else {
        return;
    };
    if args.get(0).is_null() {
        if let Some(record) = state(context) {
            record.opener.borrow_mut().take();
            clear_opener_edge(scope, context);
        }
    } else {
        let key = v8::String::new(scope, "opener").unwrap();
        args.this()
            .create_data_property(scope, key.into(), args.get(0));
    }
}

fn name_get<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue<'s>,
) {
    let Some(record) = same_origin_owner(scope, args.this()).and_then(state) else {
        return;
    };
    if let Some(name) =
        v8::String::new_from_two_byte(scope, &record.name.borrow(), v8::NewStringType::Normal)
    {
        rv.set(name.into());
    };
}

fn name_set<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _: v8::ReturnValue<'s>,
) {
    let Some(record) = same_origin_owner(scope, args.this()).and_then(state) else {
        return;
    };
    if let Some(name) = string_argument(scope, &args, 0, "undefined") {
        *record.name.borrow_mut() = name;
    }
}

fn close_callback<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _: v8::ReturnValue<'s>,
) {
    let Some(context) = owner(scope, args.this()) else {
        return;
    };
    request_close(scope, context);
}

/// Shared by the genuine WindowProxy cross-origin close method.
pub(super) fn request_close<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'s, v8::Context>,
) {
    let Some(record) = state(context) else { return };
    if record.auxiliary
        && !record.closed.get()
        && record.host.is_some_and(|host| (host.close)(scope, context))
    {
        retire(scope, context);
    }
}

fn string_argument(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    index: i32,
    default: &str,
) -> Option<Vec<u16>> {
    if args.length() <= index || args.get(index).is_undefined() {
        return Some(default.encode_utf16().collect());
    }
    let text = args.get(index).to_string(scope)?;
    let mut units = vec![0; text.length()];
    text.write_v2(scope, 0, &mut units, v8::WriteFlags::empty());
    Some(units)
}

fn open_callback<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue<'s>,
) {
    let Some(context) = same_origin_owner(scope, args.this()) else {
        return;
    };
    // Web IDL conversions happen before denial, in argument order.
    let Some(url) = string_argument(scope, &args, 0, "") else {
        return;
    };
    let Some(target) = string_argument(scope, &args, 1, "_blank") else {
        return;
    };
    let Some(features) = string_argument(scope, &args, 2, "") else {
        return;
    };
    rv.set(v8::null(scope).into());
    let Some(record) = state(context) else { return };
    let Some(document_id) = context.get_slot::<super::DomContextId>() else {
        return;
    };
    let _document = super::DomScope::enter(document_id.0);
    let base_href = super::with_tree(|tree| {
        tree.query_selector_all("base[href]")
            .iter()
            .find_map(|&node| tree.get_attribute(node, "href").map(str::to_owned))
    })
    .flatten();
    let document_url = record.base_url.borrow().clone();
    let base = url::Url::parse(&document_url).ok();
    let base = base
        .map(|base| {
            base_href
                .and_then(|href| base.join(&href).ok())
                .unwrap_or(base)
                .to_string()
        })
        .unwrap_or_else(|| document_url.clone());
    let request = request(
        &base,
        &url,
        target,
        &features,
        OpenSource::Script,
        false,
        false,
    );
    let Some(request) = request else {
        super::throw_dom_error(
            scope,
            "SyntaxError",
            "Failed to execute 'open' on 'Window': Invalid URL",
        );
        return;
    };
    if let Some(window) = open(scope, context, &request) {
        rv.set(window.into());
    }
}

fn request(
    base: &str,
    url: &[u16],
    mut target: Vec<u16>,
    features: &[u16],
    source: OpenSource,
    force_noopener: bool,
    force_noreferrer: bool,
) -> Option<OpenRequest> {
    let features = parse_features(&String::from_utf16_lossy(features));
    let noreferrer = force_noreferrer || feature_bool(&features, "noreferrer");
    let noopener = force_noopener || noreferrer || feature_bool(&features, "noopener");
    if target.is_empty() {
        target = "_blank".encode_utf16().collect();
    }
    let raw = String::from_utf16_lossy(url);
    let url = if raw.is_empty() {
        "about:blank".to_owned()
    } else {
        url::Url::parse(base).ok()?.join(&raw).ok()?.to_string()
    };
    Some(OpenRequest {
        source_id: AuxiliaryContextId(0),
        target_id: AuxiliaryContextId(0),
        new_context: false,
        navigate: !raw.is_empty(),
        source_base_url: base.to_owned(),
        url,
        target,
        features,
        noopener,
        noreferrer,
        source,
    })
}

fn feature_bool(features: &BTreeMap<String, String>, name: &str) -> bool {
    features.get(name).is_some_and(|value| {
        value.is_empty()
            || value.eq_ignore_ascii_case("yes")
            || value.eq_ignore_ascii_case("true")
            || {
                let value = value.strip_prefix(['+', '-']).unwrap_or(value);
                value
                    .bytes()
                    .take_while(u8::is_ascii_digit)
                    .any(|byte| byte != b'0')
            }
    })
}

fn parse_features(source: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let bytes = source.as_bytes();
    let separator = |byte: u8| byte.is_ascii_whitespace() || byte == b',' || byte == b'=';
    let mut position = 0;
    while position < bytes.len() {
        while position < bytes.len() && separator(bytes[position]) {
            position += 1;
        }
        let name_start = position;
        while position < bytes.len() && !separator(bytes[position]) {
            position += 1;
        }
        if name_start == position {
            break;
        }
        let name = source[name_start..position].to_ascii_lowercase();
        let name = match name.as_str() {
            "screenx" => "left".to_owned(),
            "screeny" => "top".to_owned(),
            "innerwidth" => "width".to_owned(),
            "innerheight" => "height".to_owned(),
            _ => name,
        };
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        let value = if position < bytes.len() && bytes[position] == b'=' {
            while position < bytes.len()
                && (bytes[position].is_ascii_whitespace() || bytes[position] == b'=')
            {
                position += 1;
            }
            let value_start = position;
            while position < bytes.len() && !separator(bytes[position]) {
                position += 1;
            }
            source[value_start..position].to_ascii_lowercase()
        } else {
            String::new()
        };
        result.insert(name, value);
    }
    result
}

/// Select/admit a context through native host policy, preserving real identity.
pub fn open<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    request: &OpenRequest,
) -> Option<v8::Local<'s, v8::Object>> {
    let source_state = state(source)?;
    if source_state.closed.get()
        || source_state.context.borrow().to_local(scope) != Some(source)
        || source.global(scope).get_creation_context(scope) != Some(source)
    {
        return None;
    }
    let _source_document = super::host_internals::enter_context_document(source)?;
    if !super::host_internals::is_browsing_context() {
        return None;
    }
    let host = source_state.host?;
    let ready = source.get_slot::<ReadySlot>().map(|slot| slot.0);
    let selected = select(scope, source, &source_state, request);
    let mut request = request.clone();
    request.source_id = source_state.id;
    request.target_id = match selected {
        Some(context) => state(context)?.id,
        None => next_id(scope)?,
    };
    request.new_context = selected.is_none();
    let context = (host.open)(scope, source, selected, &request)?;
    if context.get_slot::<super::DomContextId>().is_none()
        || context.global(scope).get_creation_context(scope) != Some(context)
    {
        return None;
    }
    if let Some(selected) = selected {
        if state(selected).is_some_and(|state| state.closed.get())
            || !selected
                .global(scope)
                .strict_equals(context.global(scope).into())
        {
            return None;
        }
        if !request.noopener && !reserved_existing_target(&request.target) {
            let record = state(context)?;
            let proxy = source.global(scope);
            *record.opener.borrow_mut() = Some(v8::Weak::new(scope, proxy));
            let scope = &mut v8::ContextScope::new(scope, context);
            let key = opener_key(scope);
            if context.global(scope).set_private(scope, key, proxy.into()) != Some(true) {
                return None;
            }
        }
    } else {
        if state(context).is_some()
            || context == source
            || !super::auxiliary_window_proxy::is_managed(context)
        {
            return None;
        }
        let group = if request.noopener {
            Rc::new(Group::default())
        } else {
            Rc::clone(&source_state.group.borrow())
        };
        let record = Rc::new(State {
            id: request.target_id,
            creation_source: Some(source_state.id),
            context: RefCell::new(v8::Weak::new(scope, context)),
            proxy: weak_proxy(scope, context),
            group: RefCell::new(group),
            opener: RefCell::new((!request.noopener).then(|| {
                let proxy = source.global(scope);
                v8::Weak::new(scope, proxy)
            })),
            parent: None,
            name: RefCell::new(if special(&request.target, "_blank") {
                Vec::new()
            } else {
                request.target.clone()
            }),
            url: RefCell::new("about:blank".to_owned()),
            base_url: RefCell::new(request.source_base_url.clone()),
            closed: Cell::new(false),
            init_scripts_run: Cell::new(false),
            auxiliary: true,
            host: Some(host),
        });
        record
            .group
            .borrow()
            .0
            .borrow_mut()
            .push(Rc::downgrade(&record));
        context.set_slot(Rc::new(Slot(record)));
        if let Some(ready) = ready {
            set_ready_host(context, Some(ready));
        }
        if !request.noopener {
            let opener = source.global(scope);
            let scope = &mut v8::ContextScope::new(scope, context);
            let key = opener_key(scope);
            if context.global(scope).set_private(scope, key, opener.into()) != Some(true) {
                return None;
            }
        }
        if !install_registered_surface(scope, context) {
            (host.close)(scope, context);
            super::host_internals::retire_context(context);
            return None;
        }
        if let Err(failure) = finish_ready(scope, source, context, &request, ready) {
            (host.close)(scope, context);
            retire(scope, context);
            if let Some(error) = failure.exception {
                let error = v8::Local::new(scope, &error);
                scope.throw_exception(error);
            }
            return None;
        }
    }
    (!request.noopener || (selected.is_some() && reserved_existing_target(&request.target)))
        .then(|| context.global(scope))
}

struct ReadyFailure {
    exception: Option<v8::Global<v8::Value>>,
}

fn finish_ready<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    target: v8::Local<'s, v8::Context>,
    request: &OpenRequest,
    ready: Option<ReadyHost>,
) -> Result<(), ReadyFailure> {
    let action = if let Some(ready) = ready {
        let scope = &mut v8::ContextScope::new(scope, target);
        let Some(_document) = super::host_internals::enter_context_document(target) else {
            return Err(ReadyFailure { exception: None });
        };
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut catch.init();
        let action = ready(scope, source, target, request);
        if scope.has_caught() || scope.has_terminated() {
            let exception = scope.exception().map(|error| v8::Global::new(scope, error));
            return Err(ReadyFailure { exception });
        }
        action
    } else {
        ReadyAction::RunRegisteredInitScripts
    };
    match action {
        ReadyAction::Abort => Err(ReadyFailure { exception: None }),
        ReadyAction::RunRegisteredInitScripts => {
            run_init_scripts(scope, target);
            if scope.is_execution_terminating() {
                Err(ReadyFailure { exception: None })
            } else {
                Ok(())
            }
        }
        ReadyAction::InitializedByHost => {
            if let Some(record) = state(target) {
                if record.context.borrow().to_local(scope) == Some(target) {
                    record.init_scripts_run.set(true);
                }
            }
            Ok(())
        }
    }
}

fn opener_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Private> {
    let name = v8::String::new(scope, "turbo.auxiliary.opener").unwrap();
    v8::Private::for_api(scope, Some(name))
}

fn clear_opener_edge(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) {
    let scope = &mut v8::ContextScope::new(scope, context);
    let key = opener_key(scope);
    context.global(scope).delete_private(scope, key);
}

/// An uncancelled hyperlink activation selected by the host's navigation
/// pipeline. Native attributes/base URL are read directly; guest href/target/rel
/// getters cannot forge the request. This function does not dispatch click or
/// bypass the host's event/default-action, download, CSP or sandbox checks.
pub fn open_hyperlink<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    anchor: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let _document = super::DomScope::for_object(scope, anchor);
    let handle = super::handle_of(scope, anchor)?;
    let context = super::host_internals::current_document_context(scope)?;
    let record = state(context)?;
    let (href, target, rel, base) = super::with_tree(|tree| {
        if !super::is_connected(tree, handle)
            || !matches!(tree.local_name(handle), Some("a" | "area"))
        {
            return None;
        }
        let href = tree.get_attribute_utf16(handle, "href")?;
        let base_node = tree.query_selector_all("base[href],base[target]");
        let target = tree
            .get_attribute_utf16(handle, "target")
            .or_else(|| {
                base_node
                    .iter()
                    .find_map(|&node| tree.get_attribute_utf16(node, "target"))
            })
            .unwrap_or_else(|| "_self".encode_utf16().collect());
        let rel = tree.get_attribute(handle, "rel").unwrap_or("").to_owned();
        let base = base_node
            .iter()
            .find_map(|&node| tree.get_attribute(node, "href").map(str::to_owned));
        Some((href, target, rel, base))
    })??;
    let document_url = record.base_url.borrow().clone();
    let document = url::Url::parse(&document_url).ok()?;
    let resolved = base
        .and_then(|href| document.join(&href).ok())
        .unwrap_or(document)
        .to_string();
    let noreferrer = rel
        .split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case("noreferrer"));
    let explicit_opener = rel
        .split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case("opener"));
    let noopener = rel
        .split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case("noopener"))
        || (!explicit_opener && special(&target, "_blank"));
    let request = request(
        &resolved,
        &href,
        target,
        &[],
        OpenSource::Hyperlink,
        noopener,
        noreferrer,
    )?;
    open(scope, context, &request)
}

fn special(target: &[u16], keyword: &str) -> bool {
    target.len() == keyword.len()
        && target
            .iter()
            .zip(keyword.bytes())
            .all(|(&a, b)| a <= 127 && (a as u8).eq_ignore_ascii_case(&b))
}

fn reserved_existing_target(target: &[u16]) -> bool {
    ["_self", "_parent", "_top"]
        .iter()
        .any(|keyword| special(target, keyword))
}

fn select<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    record: &State,
    request: &OpenRequest,
) -> Option<v8::Local<'s, v8::Context>> {
    if special(&request.target, "_self") {
        return Some(source);
    }
    if special(&request.target, "_parent") {
        return Some(
            record
                .parent
                .as_ref()
                .and_then(|parent| parent.to_local(scope))
                .unwrap_or(source),
        );
    }
    if special(&request.target, "_top") {
        let mut top = source;
        while let Some(parent) = state(top).and_then(|record| {
            record
                .parent
                .as_ref()
                .and_then(|parent| parent.to_local(scope))
        }) {
            top = parent;
        }
        return Some(top);
    }
    if special(&request.target, "_blank") {
        return None;
    }
    let group = record.group.borrow();
    let mut members = group.0.borrow_mut();
    members.retain(|member| member.strong_count() != 0);
    members
        .iter()
        .filter_map(Weak::upgrade)
        .find_map(|candidate| {
            if candidate.closed.get()
                || *candidate.name.borrow() != request.target
                || !Rc::ptr_eq(&candidate.group.borrow(), &group)
            {
                return None;
            }
            candidate.context.borrow().to_local(scope)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }

    #[test]
    fn features_have_ascii_case_and_whitespace_semantics() {
        let values = parse_features("WIDTH = 320, HEIGHT=200, NoOpener = YES, noreferrer=0");
        assert_eq!(values["width"], "320");
        assert_eq!(values["height"], "200");
        assert!(feature_bool(&values, "noopener"));
        assert!(!feature_bool(&values, "noreferrer"));
        assert!(!feature_bool(&values, "missing"));
        for value in [
            "",
            "yes",
            "true",
            "1",
            "-1",
            "1.5",
            "1x",
            "999999999999999999999999999999999",
        ] {
            assert!(feature_bool(
                &parse_features(&format!("noopener={value}")),
                "noopener"
            ));
        }
        for value in ["0", "no", "false", "bogus"] {
            assert!(!feature_bool(
                &parse_features(&format!("noopener={value}")),
                "noopener"
            ));
        }
    }

    #[test]
    fn request_resolution_retains_target_units_and_noreferrer_implies_noopener() {
        let name = vec![b'x' as u16, 0xd800];
        let request = request(
            "https://example.test/path/page",
            &units("../other"),
            name.clone(),
            &units("noreferrer"),
            OpenSource::Script,
            false,
            false,
        )
        .unwrap();
        assert_eq!(request.url, "https://example.test/other");
        assert!(request.navigate);
        assert_eq!(request.target, name);
        assert!(request.noopener && request.noreferrer);
        let request = super::request(
            "about:blank",
            &[],
            Vec::new(),
            &[],
            OpenSource::Script,
            false,
            false,
        )
        .unwrap();
        assert_eq!(request.url, "about:blank");
        assert!(!request.navigate);
        assert!(special(&request.target, "_blank"));
    }

    #[test]
    fn only_reserved_targets_ignore_ascii_case() {
        assert!(special(&units("_BLANK"), "_blank"));
        assert!(!special(&units("Ｎ_blank"), "_blank"));
        assert_ne!(units("Report"), units("report"));
        assert!(!special(&[0xd800], "x"));
    }

    #[test]
    fn legacy_feature_names_normalize_before_last_value_wins() {
        let features =
            parse_features("screenX=10,left=20,screenY=30,innerWidth=400,innerHeight=300");
        assert_eq!(features["left"], "20");
        assert_eq!(features["top"], "30");
        assert_eq!(features["width"], "400");
        assert_eq!(features["height"], "300");
        assert!(!features.contains_key("screenx"));
    }
}

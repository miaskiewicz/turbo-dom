//! Obsolete and capability-gated Document members whose results follow from binding state.
//!
//! `document.all` is V8's undetectable, callable HTMLAllCollection: the same
//! ObjectTemplate flags Blink uses. Rusty V8 does not wrap those two template
//! methods, so they are linked directly against the public V8 API symbols the
//! prebuilt archive exports (one name per C++ ABI).
use super::*;
use v8::MapFnTo;

extern "C" {
    #[cfg_attr(
        not(target_env = "msvc"),
        link_name = "_ZN2v814ObjectTemplate18MarkAsUndetectableEv"
    )]
    #[cfg_attr(
        target_env = "msvc",
        link_name = "?MarkAsUndetectable@ObjectTemplate@v8@@QEAAXXZ"
    )]
    fn object_template_mark_as_undetectable(this: *const v8::ObjectTemplate);
    #[cfg_attr(
        not(target_env = "msvc"),
        link_name = "_ZN2v814ObjectTemplate24SetCallAsFunctionHandlerEPFvRKNS_20FunctionCallbackInfoINS_5ValueEEEENS_5LocalIS2_EE"
    )]
    #[cfg_attr(
        target_env = "msvc",
        link_name = "?SetCallAsFunctionHandler@ObjectTemplate@v8@@QEAAXP6AXAEBV?$FunctionCallbackInfo@VValue@v8@@@2@@ZV?$Local@VValue@v8@@@2@@Z"
    )]
    fn object_template_set_call_as_function_handler(
        this: *const v8::ObjectTemplate,
        callback: v8::FunctionCallback,
        data: *const v8::Value,
    );
}

/// The legacy caller forwards to the collection's own `item` algorithm, which
/// the template data closure already binds to this collection.
fn legacy_call(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(caller) = v8::Local::<v8::Function>::try_from(args.data()) else {
        return;
    };
    let values: Vec<_> = (0..args.length()).map(|index| args.get(index)).collect();
    let receiver = v8::undefined(scope).into();
    if let Some(value) = caller.call(scope, receiver, &values) {
        rv.set(value);
    }
}

/// HTMLAllCollection has neither setters nor deleters. Blink's interceptors
/// reject a write to any index, or to a visible named property, with these
/// messages only where V8 says the operation must throw (strict assignment and
/// deletion, Object.defineProperty). Otherwise assignment and definition are
/// quietly reported as done (Reflect.set/defineProperty return true) and
/// deletion as refused.
fn reject<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::PropertyCallbackArguments<'s>,
    message: &str,
) -> bool {
    if !interceptor_mode::should_throw(scope, args) {
        return false;
    }
    let message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
    true
}
fn index_message(index: u32, operation: &str) -> String {
    let prefix = "Failed to set an indexed property";
    match operation {
        "set" => format!("{prefix} [{index}] on 'HTMLAllCollection': Indexed property setter is not supported."),
        "define" => format!("{prefix} [{index}] on 'HTMLAllCollection': Index property setter is not supported."),
        "define-accessor" => format!("{prefix} [{index}] on 'HTMLAllCollection': Accessor properties are not allowed."),
        _ => format!("Failed to delete an indexed property [{index}] from 'HTMLAllCollection': Index property deleter is not supported."),
    }
}
fn named_message(name: &str, operation: &str) -> String {
    if operation == "delete" {
        format!("Failed to delete a named property '{name}' from 'HTMLAllCollection': Named property deleter is not supported.")
    } else {
        format!("Failed to set a named property '{name}' on 'HTMLAllCollection': Named property setter is not supported.")
    }
}
fn supported<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::PropertyCallbackArguments<'s>,
    name: v8::Local<'s, v8::Value>,
) -> Option<bool> {
    let callback = v8::Local::<v8::Function>::try_from(args.data()).ok()?;
    let operation = v8::String::new(scope, "has")?;
    let value = v8::undefined(scope).into();
    let result = callback.call(
        scope,
        args.holder().into(),
        &[operation.into(), name, value],
    )?;
    Some(result.is_true())
}
// Stock rusty_v8 types the setter/definer return slot `ReturnValue<()>`;
// source-built engines (V8 15.4+, turbo-surf native156) type it
// `ReturnValue<Boolean>`. These interceptors never write the slot, so the
// generic payload satisfies either ABI with binding-quirks' reject()/
// should_throw semantics unchanged.
fn all_index_setter<'s, 'i, T>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    _value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    _rv: v8::ReturnValue<T>,
) -> v8::Intercepted {
    reject(scope, &args, &index_message(index, "set"));
    v8::Intercepted::kYes
}
fn all_index_definer<'s, 'i, T>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    descriptor: &v8::PropertyDescriptor,
    args: v8::PropertyCallbackArguments<'s>,
    _rv: v8::ReturnValue<T>,
) -> v8::Intercepted {
    // Blink refuses an indexed accessor definition before consulting setter support.
    let operation = if descriptor.has_get() || descriptor.has_set() {
        "define-accessor"
    } else {
        "define"
    };
    reject(scope, &args, &index_message(index, operation));
    v8::Intercepted::kYes
}
fn all_index_deleter<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    index: u32,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue<v8::Boolean>,
) -> v8::Intercepted {
    let name = v8::String::new(scope, &index.to_string()).unwrap();
    if supported(scope, &args, name.into()) != Some(true) {
        return v8::Intercepted::kNo;
    }
    if !reject(scope, &args, &index_message(index, "delete")) {
        rv.set_bool(false);
    }
    v8::Intercepted::kYes
}
/// `None` leaves an expando or absent name to ordinary property semantics;
/// otherwise reports whether the rejection threw.
fn all_named_mutation<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    name: v8::Local<'s, v8::Name>,
    args: &v8::PropertyCallbackArguments<'s>,
    operation: &str,
) -> Option<bool> {
    if !supported(scope, args, name.into())? {
        return None;
    }
    let text = name.to_rust_string_lossy(scope);
    Some(reject(scope, args, &named_message(&text, operation)))
}
fn all_named_setter<'s, 'i, T>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    _value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    _rv: v8::ReturnValue<T>,
) -> v8::Intercepted {
    match all_named_mutation(scope, name, &args, "set") {
        None => v8::Intercepted::kNo,
        Some(_) => v8::Intercepted::kYes,
    }
}
fn all_named_definer<'s, 'i, T>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    _descriptor: &v8::PropertyDescriptor,
    args: v8::PropertyCallbackArguments<'s>,
    _rv: v8::ReturnValue<T>,
) -> v8::Intercepted {
    match all_named_mutation(scope, name, &args, "define") {
        None => v8::Intercepted::kNo,
        Some(_) => v8::Intercepted::kYes,
    }
}
fn all_named_deleter<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    args: v8::PropertyCallbackArguments<'s>,
    mut rv: v8::ReturnValue<v8::Boolean>,
) -> v8::Intercepted {
    let Some(thrown) = all_named_mutation(scope, name, &args, "delete") else {
        return v8::Intercepted::kNo;
    };
    if !thrown {
        rv.set_bool(false);
    }
    v8::Intercepted::kYes
}

/// Named and indexed lookups use distinct dispatchers: V8 also offers an
/// index-like name such as "7" to the named enumerator's attribute query, where
/// it must answer for the unenumerable named property rather than the index.
fn collection<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    prototype: v8::Local<'s, v8::Value>,
    named: v8::Local<'s, v8::Function>,
    indexed: v8::Local<'s, v8::Function>,
    caller: v8::Local<'s, v8::Function>,
) -> Option<v8::Local<'s, v8::Object>> {
    let template = v8::ObjectTemplate::new(scope);
    template.set_named_property_handler(
        v8::NamedPropertyHandlerConfiguration::new()
            .getter(named_getter)
            .setter(all_named_setter)
            .query(named_query)
            .descriptor(named_descriptor)
            .deleter(all_named_deleter)
            .enumerator(named_enumerator)
            .definer(all_named_definer)
            .data(named.into())
            .flags(
                v8::PropertyHandlerFlags::NON_MASKING
                    | v8::PropertyHandlerFlags::ONLY_INTERCEPT_STRINGS,
            ),
    );
    template.set_indexed_property_handler(
        v8::IndexedPropertyHandlerConfiguration::new()
            .getter(indexed_getter)
            .setter(all_index_setter)
            .query(indexed_query)
            .descriptor(indexed_descriptor)
            .deleter(all_index_deleter)
            .enumerator(indexed_enumerator)
            .definer(all_index_definer)
            .data(indexed.into()),
    );
    let callback: v8::FunctionCallback = legacy_call.map_fn_to();
    let caller: v8::Local<v8::Value> = caller.into();
    // SAFETY: both symbols are non-virtual public V8 methods on the template
    // this local refers to; `Local<Value>` is a trivially copied handle slot.
    unsafe {
        object_template_mark_as_undetectable(&*template);
        object_template_set_call_as_function_handler(&*template, callback, &*caller);
    }
    let object = template.new_instance(scope)?;
    object.set_prototype(scope, prototype);
    Some(object)
}

fn private_key<'s>(scope: &mut v8::PinScope<'s, '_>, name: &str) -> v8::Local<'s, v8::Private> {
    let name = v8::String::new(scope, name).unwrap();
    v8::Private::for_api(scope, Some(name))
}

/// The per-realm factory builds a Document's collection in that Document's
/// own realm, whichever realm's accessor first asks for it.
struct AllFactory(v8::Weak<v8::Function>);

fn all_of<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Value>> {
    let key = private_key(scope, "turbo-dom.Document.all");
    if let Some(existing) = document.get_private(scope, key) {
        if !existing.is_undefined() {
            return Some(existing);
        }
    }
    let context = document.get_creation_context(scope)?;
    let factory = context.get_slot::<AllFactory>()?.0.to_local(scope)?;
    let created = {
        let scope = &mut v8::ContextScope::new(scope, context);
        let receiver = v8::undefined(scope).into();
        factory.call(scope, receiver, &[document.into()])?
    };
    document.set_private(scope, key, created);
    Some(created)
}

// Host-supplied response facts live on the realm's own Document object, so
// they are released with it and never reach guest-visible properties.
const LAST_MODIFIED: &str = "turbo-dom.Document.lastModified";
const DOMAIN_SANDBOXED: &str = "turbo-dom.Document.domainSandboxed";
const ORIGIN_KEYED: &str = "turbo-dom.Document.originKeyed";
const RELAXED_DOMAIN: &str = "turbo-dom.Document.domain";

/// The document a browsing context presents, resolved through `document`'s own
/// realm so inert documents report their creator's origin, as Blink's shared
/// execution context does.
struct Owner<'s> {
    document: v8::Local<'s, v8::Object>,
    /// `document` itself is that browsing context's active document.
    active: bool,
    /// The owner still has a browsing context (not inert, not retired).
    browsing: bool,
    url: String,
}
type DocumentState = ((bool, bool), String);
fn document_state(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
) -> Option<DocumentState> {
    let _dom = DomScope::for_object(scope, document);
    let browsing = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .map(|dom| (dom.browsing_context, dom.resource_active))
    })?;
    Some((browsing, event_handlers::document_url()))
}
fn owner<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
) -> Option<Owner<'s>> {
    let ((context_document, resource_active), url) = document_state(scope, document)?;
    if context_document {
        return Some(Owner {
            document,
            active: true,
            browsing: resource_active,
            url,
        });
    }
    let context = document.get_creation_context(scope)?;
    // Contexts without an independent-document slot own the default document.
    let id = context.get_slot::<DomContextId>().map_or(0, |id| id.0);
    let root = {
        let _dom = DomScope::enter(id);
        let root = with_tree(Tree::root)?;
        wrap(scope, root)
    };
    let ((browsing, resource_active), url) = document_state(scope, root)?;
    Some(Owner {
        document: root,
        active: false,
        browsing: browsing && resource_active,
        url,
    })
}
fn private_value<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let key = private_key(scope, name);
    document
        .get_private(scope, key)
        .filter(|value| !value.is_undefined())
}
fn set_private_value(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    name: &str,
    value: v8::Local<v8::Value>,
) {
    let key = private_key(scope, name);
    document.set_private(scope, key, value);
}

fn origin_host(url: &str) -> Option<url::Host> {
    match url::Url::parse(url).ok()?.origin() {
        url::Origin::Tuple(_, host, _) => Some(host),
        url::Origin::Opaque(_) => None,
    }
}
/// Chrome's default agent clustering keys every potentially trustworthy origin.
fn trustworthy(url: &str) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    if matches!(url.scheme(), "https" | "wss") {
        return true;
    }
    match url.host() {
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        Some(url::Host::Domain(name)) => name == "localhost" || name.ends_with(".localhost"),
        None => false,
    }
}
fn effective_domain<'s>(scope: &mut v8::PinScope<'s, '_>, owner: &Owner<'s>) -> String {
    if !owner.browsing {
        return String::new();
    }
    if let Some(relaxed) = private_value(scope, owner.document, RELAXED_DOMAIN) {
        return relaxed.to_rust_string_lossy(scope);
    }
    origin_host(&owner.url)
        .map(|host| host.to_string())
        .unwrap_or_default()
}

/// WHATWG "ends in a number": such a host must parse as IPv4.
fn ends_in_number(host: &str) -> bool {
    let mut labels: Vec<&str> = host.split('.').collect();
    if labels.len() > 1 && labels.last() == Some(&"") {
        labels.pop();
    }
    let last = labels.last().copied().unwrap_or_default();
    if !last.is_empty() && last.bytes().all(|byte| byte.is_ascii_digit()) {
        return true;
    }
    last.strip_prefix("0x")
        .or_else(|| last.strip_prefix("0X"))
        .is_some_and(|hex| hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}
/// Chromium's host canonicalizer as measured for document.domain: escapes are
/// decoded, non-ASCII input takes the IDNA path, a handful of ASCII characters
/// are escaped rather than rejected, and numeric hosts are IPv4 addresses.
fn canonical_host(raw: &str) -> Option<String> {
    if raw.starts_with('[') {
        return match url::Host::parse(raw).ok()? {
            host @ url::Host::Ipv6(_) => Some(host.to_string()),
            _ => None,
        };
    }
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escape = bytes
            .get(index + 1..index + 3)
            .filter(|_| bytes[index] == b'%');
        match escape.and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()) {
            Some(byte) => {
                decoded.push(byte);
                index += 3;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }
    let decoded = String::from_utf8(decoded).ok()?;
    let ascii = if decoded.is_ascii() {
        decoded
    } else {
        match url::Host::parse(&decoded).ok()? {
            url::Host::Domain(name) => name,
            host => host.to_string(),
        }
    };
    let mut host = String::with_capacity(ascii.len());
    for byte in ascii.bytes() {
        match byte {
            0..=0x1f
            | 0x7f
            | b'#'
            | b'%'
            | b'/'
            | b':'
            | b'<'
            | b'>'
            | b'?'
            | b'@'
            | b'['
            | b'\\'
            | b']'
            | b'^'
            | b'|' => return None,
            b' ' => host.push_str("%20"),
            b'*' => host.push_str("%2A"),
            _ => host.push(byte.to_ascii_lowercase() as char),
        }
    }
    if ends_in_number(&host) {
        return match url::Host::parse(&host).ok()? {
            host @ url::Host::Ipv4(_) => Some(host.to_string()),
            _ => None,
        };
    }
    Some(host)
}
fn is_public_suffix(domain: &str) -> bool {
    psl::suffix(domain.as_bytes()).is_some_and(|suffix| suffix.as_bytes() == domain.as_bytes())
}
/// Blink's Document::setDomain; returns the SecurityError message on refusal.
fn set_domain<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    raw: &str,
) -> Option<String> {
    let owner = owner(scope, document)?;
    if !owner.active || !owner.browsing {
        return Some("A browsing context is required to set a domain.".into());
    }
    if private_value(scope, document, DOMAIN_SANDBOXED).is_some_and(|value| value.is_true()) {
        return Some("Assignment is forbidden for sandboxed iframes.".into());
    }
    let Some(new_domain) = canonical_host(raw) else {
        return Some(format!("'{raw}' could not be parsed properly."));
    };
    if new_domain.is_empty() {
        return Some("'' is an empty domain.".into());
    }
    let current = effective_domain(scope, &owner);
    // The match runs against the origin's own host, so a relaxed document may
    // restore any suffix of it; the message names the current effective domain.
    let host = origin_host(&owner.url).map(|host| host.to_string());
    let ip = matches!(
        url::Host::parse(&new_domain),
        Ok(url::Host::Ipv4(_) | url::Host::Ipv6(_))
    );
    let exact = host.as_deref() == Some(new_domain.as_str());
    let suffix = !ip
        && host.as_deref().is_some_and(|host| {
            host.len() > new_domain.len()
                && host.ends_with(new_domain.as_str())
                && host.as_bytes()[host.len() - new_domain.len() - 1] == b'.'
        });
    if !exact && !suffix {
        return Some(format!("'{new_domain}' is not a suffix of '{current}'."));
    }
    if suffix && is_public_suffix(&new_domain) {
        return Some(format!("'{new_domain}' is a top-level domain."));
    }
    let keyed = private_value(scope, document, ORIGIN_KEYED)
        .map(|value| value.is_true())
        .unwrap_or_else(|| trustworthy(&owner.url));
    // An origin-keyed agent cluster ignores the assignment (Chrome only warns).
    if !keyed {
        let value = v8::String::new(scope, &new_domain).unwrap();
        set_private_value(scope, document, RELAXED_DOMAIN, value.into());
    }
    None
}

fn host_document<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
) -> bool {
    owner(scope, document).is_some_and(|owner| owner.active)
}
/// Record the document response's `Last-Modified` instant (ms since the epoch,
/// already parsed by the host) for `document.lastModified`; `None` clears it.
pub fn set_document_last_modified<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    epoch_ms: Option<f64>,
) -> bool {
    if !host_document(scope, document) {
        return false;
    }
    let value = match epoch_ms.filter(|value| value.is_finite()) {
        Some(value) => v8::Number::new(scope, value).into(),
        None => v8::undefined(scope).into(),
    };
    set_private_value(scope, document, LAST_MODIFIED, value);
    true
}
/// Apply the sandboxed document.domain browsing context flag, which every
/// iframe `sandbox` and CSP `sandbox` sets regardless of its allow tokens.
pub fn set_document_domain_sandboxed<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    sandboxed: bool,
) -> bool {
    if !host_document(scope, document) {
        return false;
    }
    let value = v8::Boolean::new(scope, sandboxed).into();
    set_private_value(scope, document, DOMAIN_SANDBOXED, value);
    true
}
/// Override the default origin-keyed agent cluster decision (potentially
/// trustworthy origins) from the response's `Origin-Agent-Cluster` header.
pub fn set_document_origin_keyed<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    keyed: bool,
) -> bool {
    if !host_document(scope, document) {
        return false;
    }
    let value = v8::Boolean::new(scope, keyed).into();
    set_private_value(scope, document, ORIGIN_KEYED, value);
    true
}

fn elements(tree: &Tree, node: Handle, out: &mut Vec<Handle>) {
    for child in tree.children(node) {
        if tree.node_type_id(child) == 1 {
            out.push(child);
        }
        elements(tree, child, out);
    }
}

fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    match operation.as_str() {
        "collection" => {
            let (Ok(named), Ok(indexed), Ok(caller)) = (
                v8::Local::<v8::Function>::try_from(args.get(2)),
                v8::Local::<v8::Function>::try_from(args.get(3)),
                v8::Local::<v8::Function>::try_from(args.get(4)),
            ) else {
                return;
            };
            if let Some(object) = collection(scope, args.get(1), named, indexed, caller) {
                let key = private_key(scope, "turbo-dom.HTMLAllCollection.record");
                object.set_private(scope, key, args.get(5));
                rv.set(object.into());
            }
            return;
        }
        "record" => {
            // The brand is isolate-wide so another realm's accessors accept it, as
            // Blink's wrapper type check does; guest code cannot name the key.
            if let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) {
                let key = private_key(scope, "turbo-dom.HTMLAllCollection.record");
                if let Some(record) = object
                    .get_private(scope, key)
                    .filter(|value| value.is_object())
                {
                    rv.set(record);
                }
            }
            return;
        }
        "register" => {
            if let Ok(factory) = v8::Local::<v8::Function>::try_from(args.get(1)) {
                if let Some(factory) = dom_ownership::helper_function(scope, factory) {
                    scope
                        .get_current_context()
                        .set_slot(std::rc::Rc::new(AllFactory(factory)));
                }
            }
            return;
        }
        _ => {}
    }
    let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    match operation.as_str() {
        "domain" => {
            if let Some(owner) = owner(scope, node) {
                let domain = effective_domain(scope, &owner);
                rv.set(v8::String::new(scope, &domain).unwrap().into());
            }
            return;
        }
        "setDomain" => {
            let raw = args.get(2).to_rust_string_lossy(scope);
            if let Some(message) = set_domain(scope, node, &raw) {
                rv.set(v8::String::new(scope, &message).unwrap().into());
            }
            return;
        }
        "pointerLockElement" => {
            let lock = pointer_events::lock_element_in(scope, node);
            let _dom = DomScope::for_object(scope, node);
            let value = wrap_opt(scope, lock);
            rv.set(value);
            return;
        }
        "exitPointerLock" => {
            pointer_events::exit(scope, node);
            return;
        }
        "lastModified" => {
            if owner(scope, node).is_some_and(|owner| owner.active) {
                if let Some(value) = private_value(scope, node, LAST_MODIFIED) {
                    rv.set(value);
                }
            }
            return;
        }
        _ => {}
    }
    if operation == "all" {
        if let Some(value) = all_of(scope, node) {
            rv.set(value);
        }
        return;
    }
    let _dom = DomScope::for_object(scope, node);
    let Some(handle) = handle_of(scope, node) else {
        return;
    };
    match operation.as_str() {
        "stamp" => {
            let stamp = with_tree(|tree| tree.version).unwrap_or_default();
            rv.set(v8::Number::new(scope, stamp as f64).into());
        }
        "elements" => {
            let handles = with_tree(|tree| {
                let mut out = Vec::new();
                elements(tree, handle, &mut out);
                out
            })
            .unwrap_or_default();
            let values: Vec<v8::Local<v8::Value>> = handles
                .into_iter()
                .map(|handle| wrap(scope, handle).into())
                .collect();
            rv.set(v8::Array::new_with_elements(scope, &values).into());
        }
        _ => {}
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__documentLegacyNative", dispatch);
    run_js(scope, include_str!("document_legacy.js"));
}

#[cfg(test)]
#[path = "document_legacy_tests.rs"]
mod tests;

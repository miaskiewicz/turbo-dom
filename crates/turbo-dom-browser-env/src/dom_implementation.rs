//! Genuine independent-document factories. All identities belong to native Tree
//! nodes; implementation receivers and metadata stay in private interface records.
use super::*;

fn key<'s>(scope: &mut v8::PinScope<'s, '_>, name: &str) -> Option<v8::Local<'s, v8::String>> {
    v8::String::new(scope, name)
}
fn field<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    record: v8::Local<'s, v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let name = key(scope, name)?;
    if record.has_own_property(scope, name.into()) != Some(true) {
        return None;
    }
    record.get(scope, name.into())
}
fn put(
    scope: &mut v8::PinScope,
    record: v8::Local<v8::Object>,
    name: &str,
    value: v8::Local<v8::Value>,
) -> Option<()> {
    let name = key(scope, name)?;
    (record.create_data_property(scope, name.into(), value) == Some(true)).then_some(())
}
fn record<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    collection_records::get(scope, object)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
}
fn null_record<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Object>> {
    let object = v8::Object::new(scope);
    let null = v8::null(scope);
    (object.set_prototype(scope, null.into()) == Some(true)).then_some(object)
}
fn document<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    implementation: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let record = record(scope, implementation)?;
    let kind = field(scope, record, "kind")?;
    if !kind.is_string() || kind.to_rust_string_lossy(scope) != "DOMImplementation" {
        return None;
    }
    field(scope, record, "document")?.to_object(scope)
}
pub(crate) fn is_xml_document<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
) -> bool {
    record(scope, document)
        .and_then(|record| field(scope, record, "nativeContentType"))
        .is_some_and(|value| value.is_string() && value.to_rust_string_lossy(scope) != "text/html")
}
pub(crate) fn default_element_namespace<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
) -> Option<&'static str> {
    let xhtml = record(scope, document)
        .and_then(|record| field(scope, record, "nativeContentType"))
        .is_some_and(|value| {
            value.is_string() && value.to_rust_string_lossy(scope) == "application/xhtml+xml"
        });
    xhtml.then_some("http://www.w3.org/1999/xhtml")
}
fn initializer_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Private>> {
    let name = key(scope, "turbo.native.inert-document.initializer")?;
    Some(v8::Private::for_api(scope, Some(name)))
}
pub fn set_initializer(scope: &mut v8::PinScope, function: v8::Local<v8::Function>) -> bool {
    // Shared embedder ops can execute in their own creation realm. The trusted
    // initializer belongs to the document realm that created the callback.
    let Some(owner) = function.get_creation_context(scope) else {
        return false;
    };
    let global = owner.global(scope);
    initializer_key(scope)
        .is_some_and(|key| global.set_private(scope, key, function.into()) == Some(true))
}
pub(super) fn initialize<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    doc: v8::Local<'s, v8::Object>,
    content_type: &str,
) -> Option<()> {
    let metadata = record(scope, doc).or_else(|| null_record(scope))?;
    let content = key(scope, content_type)?;
    put(scope, metadata, "nativeContentType", content.into())?;
    if !collection_records::set(scope, doc, metadata.into()) {
        return None;
    }
    let global = scope.get_current_context().global(scope);
    let private = initializer_key(scope)?;
    if let Some(function) = global
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        let url = key(scope, "about:blank")?;
        let undefined = v8::undefined(scope);
        function.call(
            scope,
            undefined.into(),
            &[doc.into(), url.into(), content.into()],
        )?;
    }
    Some(())
}
fn invoke<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
) -> Option<v8::Local<'s, v8::Value>> {
    let operation = args.get(0).to_rust_string_lossy(scope);
    let receiver = v8::Local::<v8::Object>::try_from(args.get(1)).ok()?;
    if operation == "implementation" {
        let _dom = DomScope::for_object(scope, receiver);
        let handle = handle_of(scope, receiver)?;
        if with_tree(|tree| tree.node_type_id(handle)) != Some(9) {
            return None;
        }
        let state = record(scope, receiver).or_else(|| null_record(scope))?;
        if let Some(implementation) = field(scope, state, "nativeImplementation") {
            return Some(implementation);
        }
        let prototype = args.get(2).to_object(scope)?;
        let template = v8::ObjectTemplate::new(scope);
        template.set_internal_field_count(1);
        let object = template.new_instance(scope)?;
        if object.set_prototype(scope, prototype.into()) != Some(true) {
            return None;
        }
        let implementation_record = null_record(scope)?;
        let kind = key(scope, "DOMImplementation")?;
        put(scope, implementation_record, "kind", kind.into())?;
        put(scope, implementation_record, "document", receiver.into())?;
        if !collection_records::set(scope, object, implementation_record.into()) {
            return None;
        }
        put(scope, state, "nativeImplementation", object.into())?;
        if !collection_records::set(scope, receiver, state.into()) {
            return None;
        }
        return Some(object.into());
    }
    if operation == "baseURI" {
        let _owner = DomScope::for_object(scope, receiver);
        handle_of(scope, receiver)?;
        return key(scope, "about:blank").map(Into::into);
    }
    if matches!(
        operation.as_str(),
        "contentType" | "view" | "url" | "encoding"
    ) {
        let _dom = DomScope::for_object(scope, receiver);
        let handle = handle_of(scope, receiver)?;
        if with_tree(|tree| tree.node_type_id(handle)) != Some(9) {
            return None;
        }
        if operation == "contentType" {
            let value = record(scope, receiver)
                .and_then(|record| field(scope, record, "nativeContentType"));
            return value.or_else(|| key(scope, "text/html").map(Into::into));
        }
        if operation == "encoding" {
            return key(scope, "UTF-8").map(Into::into);
        }
        if operation == "url" {
            return key(scope, "about:blank").map(Into::into);
        }
        let active = DOM.with(|state| {
            state
                .borrow()
                .as_ref()
                .is_some_and(|state| state.browsing_context)
        });
        if !active {
            return Some(v8::null(scope).into());
        }
        let context = receiver.get_creation_context(scope)?;
        return Some(context.global(scope).into());
    }
    if operation == "qualifiedName" {
        let _dom = DomScope::for_object(scope, receiver);
        let handle = handle_of(scope, receiver)?;
        if with_tree(|tree| tree.node_type_id(handle)) != Some(1) {
            return Some(v8::undefined(scope).into());
        }
        let root = with_tree(Tree::root)?;
        let doc = wrap(scope, root);
        let Some(state) = record(scope, doc) else {
            return Some(v8::undefined(scope).into());
        };
        let Some(content) = field(scope, state, "nativeContentType") else {
            return Some(v8::undefined(scope).into());
        };
        if content.to_rust_string_lossy(scope) == "text/html" {
            return Some(v8::undefined(scope).into());
        }
        let name = with_tree(|tree| tree.qualified_name_utf16(handle)).flatten()?;
        return Some(unit_string(scope, &name).into());
    }
    let owner = document(scope, receiver)?;
    if operation == "check" {
        return Some(v8::undefined(scope).into());
    }
    if operation == "doctype" {
        let name = dom_units(scope, args.get(2))?;
        let public = dom_units(scope, args.get(3))?;
        let system = dom_units(scope, args.get(4))?;
        let _dom = DomScope::for_object(scope, owner);
        let node = with_tree_mut(|tree| tree.create_document_type_units(&name, &public, &system))?;
        return Some(wrap(scope, node).into());
    }
    let doc = parse_html_document(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
        false,
    );
    if operation == "html" {
        if args.get(2).boolean_value(scope) {
            let title = dom_units(scope, args.get(3))?;
            let _dom = DomScope::for_object(scope, doc);
            with_tree_mut(|tree| {
                let head = tree.query_selector("head")?;
                let element = tree.create_element("title");
                let text = tree.create_text_node_utf16(&title);
                tree.append_child(element, text);
                tree.append_child(head, element);
                Some(())
            })
            .flatten()?;
        }
        initialize(scope, doc, "text/html")?;
        return Some(doc.into());
    }
    if operation != "xml" {
        return None;
    }
    let namespace = if args.get(2).is_null() {
        None
    } else {
        Some(dom_units(scope, args.get(2))?)
    };
    let qualified = dom_units(scope, args.get(3))?;
    {
        let _dom = DomScope::for_object(scope, doc);
        with_tree_mut(|tree| {
            let root = tree.root();
            for child in tree.children(root) {
                tree.remove_child(root, child);
            }
        })?;
    }
    if !args.get(4).is_null_or_undefined() {
        let doctype = args.get(4).to_object(scope)?;
        {
            let _dom = DomScope::for_object(scope, doctype);
            let handle = handle_of(scope, doctype)?;
            if with_tree(|tree| tree.node_type_id(handle)) != Some(10) {
                return None;
            }
        }
        let node = document_basics::adopt(scope, doc, doctype).ok()?;
        let _dom = DomScope::for_object(scope, doc);
        let handle = handle_of(scope, node)?;
        with_tree_mut(|tree| tree.append_child(tree.root(), handle))?;
    }
    if !qualified.is_empty() {
        let _dom = DomScope::for_object(scope, doc);
        with_tree_mut(|tree| {
            let root = tree.root();
            let node = tree.create_element_namespace_units(namespace.as_deref(), &qualified);
            tree.append_child(root, node);
        })?;
    }
    let content = match namespace
        .as_deref()
        .map(String::from_utf16_lossy)
        .as_deref()
    {
        Some("http://www.w3.org/1999/xhtml") => "application/xhtml+xml",
        Some("http://www.w3.org/2000/svg") => "image/svg+xml",
        _ => "application/xml",
    };
    initialize(scope, doc, content)?;
    Some(doc.into())
}
fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    if let Some(value) = invoke(scope, &args) {
        rv.set(value);
        return;
    }
    if scope.is_execution_terminating() {
        return;
    }
    if let Some(message) = key(scope, "Illegal invocation") {
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
    }
}
pub(crate) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__domImplementationNative", dispatch);
    run_js(scope, include_str!("dom_implementation.js"));
}

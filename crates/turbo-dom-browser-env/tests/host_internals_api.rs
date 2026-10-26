//! Proves the `host-internals` surface is sufficient OUTSIDE the crate for
//! turbo-surf's native layout capture (live_layout_dom.rs) and compression
//! dictionary discovery (compression_dictionary_dom.rs): every exported item is
//! used here, through `turbo_dom_browser_env::host_internals::*` only.
#![cfg(feature = "host-internals")]

use std::sync::Once;
use turbo_dom_browser_env::browser_env;
use turbo_dom_browser_env::host_internals::*;

fn init_v8() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
}

fn object<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Object> {
    v8::Local::<v8::Object>::try_from(eval(scope, source)).unwrap()
}

fn own<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    name: &str,
) -> v8::Local<'s, v8::Value> {
    let key = v8::String::new(scope, name).unwrap();
    assert_eq!(
        object.has_own_property(scope, key.into()),
        Some(true),
        "{name}"
    );
    object.get(scope, key.into()).unwrap()
}

fn call<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    host: v8::Local<'s, v8::Object>,
    name: &str,
    args: &[v8::Local<'s, v8::Value>],
) -> v8::Local<'s, v8::Value> {
    let method = v8::Local::<v8::Function>::try_from(own(scope, host, name)).unwrap();
    method.call(scope, host.into(), args).unwrap()
}

/// compression_dictionary_dom.rs: composed `<link rel=compression-dictionary>`
/// discovery over the native tree, then wrapping the handles.
fn dictionary_links<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<v8::Object>,
) -> Vec<v8::Local<'s, v8::Object>> {
    let _owner: DocumentScope = enter_document_of(scope, document);
    let root: Handle = handle_of(scope, document).unwrap();
    let handles = with_tree(|tree: &Tree| {
        let mut pending = vec![root];
        let mut found = Vec::new();
        while let Some(node) = pending.pop() {
            if tree.local_name(node) == Some("link")
                && tree
                    .get_attribute(node, "rel")
                    .is_some_and(|rel| rel.eq_ignore_ascii_case("compression-dictionary"))
            {
                found.push(node);
            }
            if let Some(shadow) = tree.shadow_root(node) {
                pending.push(shadow);
            }
            pending.extend(tree.children(node).into_iter().rev());
        }
        found
    })
    .unwrap();
    handles
        .into_iter()
        .map(|handle| wrap(scope, handle))
        .collect()
}

#[test]
fn host_internals_cover_native_layout_capture_and_link_discovery() {
    init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    // A context without an independent document has nothing to enter.
    assert!(enter_context_document(context).is_none());
    browser_env::install_independent_html(
        scope,
        "<!doctype html><html><head><link rel=compression-dictionary href=/d>\
         <style id=s>p{color:red}@media screen{p{width:1px}}</style></head>\
         <body><div id=host></div><p id=p>x</p></body></html>",
    );
    eval(
        scope,
        "document.getElementById('host').attachShadow({mode:'closed'}).innerHTML=\
         '<link rel=compression-dictionary href=/shadow><slot></slot>'",
    );
    let document = object(scope, "document");

    // Document identity, browsing-context state and owner context.
    let _entered = enter_document_of(scope, document);
    let id = current_document_id();
    assert_ne!(id, 0);
    assert_eq!(document_id_of(scope, document), id);
    assert!(is_browsing_context());
    assert!(current_document_context(scope).unwrap() == context);

    // Handles <-> wrappers, with identity preserved.
    let root = handle_of(scope, document).unwrap();
    assert_eq!(with_tree(|tree| tree.node_type_id(root)), Some(9));
    let p = with_tree(|tree| tree.get_element_by_id("p"))
        .flatten()
        .unwrap();
    let js_p = object(scope, "document.getElementById('p')");
    assert!(wrap(scope, p).strict_equals(js_p.into()));
    assert_eq!(handle_of(scope, js_p), Some(p));
    assert!(with_tree(|tree| is_connected(tree, p)).unwrap());
    let plain = v8::Object::new(scope);
    assert_eq!(handle_of(scope, plain), None);

    // Composed children of a shadow host: the closed root's slot distributes the
    // host's (empty) light tree.
    let host = object(scope, "document.getElementById('host')");
    let composed = host_composed_children(scope, host).unwrap();
    assert!(!composed.is_empty());
    let body = object(scope, "document.body");
    let body_children = host_composed_children(scope, body).unwrap();
    assert_eq!(body_children.len(), 2);

    // Link discovery crosses the closed shadow root.
    let links = dictionary_links(scope, document);
    assert_eq!(links.len(), 2);

    // The private CSS host: export a sheet into the JSON contract.
    let record =
        v8::Local::<v8::Object>::try_from(interface_record(scope, document).unwrap()).unwrap();
    let css_host = v8::Local::<v8::Object>::try_from(own(scope, record, "cssHost")).unwrap();
    let style = object(scope, "document.getElementById('s')");
    let sheet = call(scope, css_host, "sheet", &[style.into()]);
    assert!(!sheet.is_null_or_undefined());
    let exported = call(scope, css_host, "exportSheet", &[sheet]);
    let json = v8::json::stringify(scope, exported).unwrap();
    let json = json.to_rust_string_lossy(scope);
    let snapshot: SheetSnapshot = serde_json::from_str(&json).unwrap();
    let rule: &RuleSnapshot = &snapshot.rules[0];
    assert_eq!(
        (rule.kind, rule.css_text.as_str()),
        (1, "p { color: red; }")
    );
    assert_eq!(snapshot.rules[1].kind, 4);
    assert_eq!(snapshot.rules[1].rules.as_ref().unwrap().len(), 1);
    let adopted = call(scope, css_host, "readAdopted", &[document.into()]);
    assert_eq!(
        v8::Local::<v8::Array>::try_from(adopted).unwrap().length(),
        0
    );

    // The host CSS policy and the shared zoom grammar.
    let policy: StylePolicy = cascade_policy(scope);
    assert!(policy.inline_overrides.is_empty());
    assert_eq!(zoom_value("150%"), Some(("150%".into(), 1.5)));
    drop(_entered);

    // Entering by context, then retiring the frame's document.
    {
        let _document = enter_context_document(context).unwrap();
        assert_eq!(current_document_id(), id);
        assert!(is_browsing_context());
    }
    retire_context(context);
    {
        let _document = enter_context_document(context).unwrap();
        assert!(!is_browsing_context());
    }
    browser_env::reset();
}

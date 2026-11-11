//! Host document-stream handoff without replacing a Window or V8 context.
use turbo_dom_browser_env::browser_env::{
    install_independent_html, install_inline_event_handlers, install_with_html, node_nonce,
    prepare_script, replace_document_content, reset,
};

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
fn replacement_preserves_realm_document_and_detached_nodes_and_returns_parser_work() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><div id=target>old</div>");
    eval(scope, "globalThis.savedDocument=document; globalThis.oldNode=document.getElementById('target'); globalThis.marker=40; globalThis.order=[]; document.addEventListener('kept',()=>order.push('listener'))");
    let key = v8::String::new(scope, "document").unwrap();
    let document = context
        .global(scope)
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    let html: Vec<u16> = "<!doctype html><div id=target>new</div><script>order.push('first'); marker+=2</script><script src='/external.js'></script><script>order.push('last'); marker+=2</script>".encode_utf16().collect();
    let scripts = replace_document_content(scope, document, &html).unwrap();
    assert_eq!(eval(scope, "JSON.stringify([document===savedDocument,document.getElementById('target').textContent,oldNode.textContent,oldNode.isConnected,marker])"), "[true,\"new\",\"old\",false,40]");
    assert_eq!(scripts.len(), 3);
    // The embedder retains its existing script policy and loading scheduler. No script
    // executes during replacement; each returned script can be admitted exactly once.
    for (index, node) in scripts.into_iter().enumerate() {
        let prepared = prepare_script(scope, node).unwrap();
        assert!(prepared.parser_inserted);
        assert!(!prepared.asynchronous);
        assert!(prepare_script(scope, node).is_none());
        if index == 1 {
            assert_eq!(prepared.src.as_deref(), Some("/external.js"));
            eval(scope, "order.push('external')");
        } else {
            eval(scope, &prepared.source);
        }
    }
    eval(scope, "document.dispatchEvent(new Event('kept'))");
    assert_eq!(
        eval(scope, "JSON.stringify([marker,order])"),
        "[44,[\"first\",\"external\",\"last\",\"listener\"]]"
    );
    let source = v8::String::new(
        scope,
        "(()=>{globalThis.policyCalls=(globalThis.policyCalls||0)+1;return false})",
    )
    .unwrap();
    let policy = v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let policy = v8::Local::<v8::Function>::try_from(policy).unwrap();
    install_inline_event_handlers(scope, policy, "https://content.test/");
    let html: Vec<u16> = "<button id=target onclick='marker=999'>next</button><script nonce=secret>marker++</script>".encode_utf16().collect();
    let scripts = replace_document_content(scope, document, &html).unwrap();
    assert_eq!(scripts.len(), 1);
    assert_eq!(node_nonce(scope, scripts[0]).as_deref(), Some("secret"));
    eval(scope, "document.getElementById('target').click()");
    assert_eq!(
        eval(
            scope,
            "JSON.stringify([marker,policyCalls>0,document===savedDocument,oldNode.textContent])"
        ),
        "[44,true,true,\"old\"]"
    );
    let invalid = v8::Object::new(scope);
    assert!(replace_document_content(scope, invalid, &[]).is_err());
    let child = v8::Context::new(scope, Default::default());
    {
        let scope = &mut v8::ContextScope::new(scope, child);
        install_independent_html(scope, "<p>other</p>");
        let key = v8::String::new(scope, "document").unwrap();
        let other = child
            .global(scope)
            .get(scope, key.into())
            .unwrap()
            .to_object(scope)
            .unwrap();
        let source: Vec<u16> = "<p>isolated</p>".encode_utf16().collect();
        assert!(replace_document_content(scope, other, &source)
            .unwrap()
            .is_empty());
        assert_eq!(
            eval(
                scope,
                "JSON.stringify([document.body.textContent,typeof marker])"
            ),
            "[\"isolated\",\"undefined\"]"
        );
    }
    assert_eq!(
        eval(
            scope,
            "JSON.stringify([document.getElementById('target').textContent,marker])"
        ),
        "[\"next\",44]"
    );
    reset();
}

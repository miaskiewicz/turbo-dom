use turbo_dom_browser_env::browser_env::{
    install_mutation_consumer, install_with_html, parse_html_document_units, prepare_script,
    replace_document_content, reset, script_state, set_document_resource_active,
};

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
}
fn is_parser(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let node = args.get(0).to_object(scope).unwrap();
    rv.set_bool(script_state(scope, node).is_some_and(|state| state.parser_inserted));
}
fn claim(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let node = args.get(0).to_object(scope).unwrap();
    rv.set_bool(prepare_script(scope, node).is_some());
}

#[test]
fn resource_consumers_observe_final_wrappers_and_parser_state() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><body>old</body>");
    for (name, function) in [
        ("isParser", v8::Function::new(scope, is_parser).unwrap()),
        ("claimScript", v8::Function::new(scope, claim).unwrap()),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        context
            .global(scope)
            .set(scope, key.into(), function.into());
    }
    let doc = eval(scope, "document").to_object(scope).unwrap();
    set_document_resource_active(scope, doc, true);
    let callback = eval(
        scope,
        r#"(() => {
      globalThis.images=[];globalThis.imageURLs=[];globalThis.parserFlags=[];globalThis.dynamicClaims=[];
      let states=new WeakMap();
      globalThis.resetResources=()=>{images=[];imageURLs=[];parserFlags=[];dynamicClaims=[];states=new WeakMap();};
      function inspect(node) {
        if(node.nodeType===1&&node.localName==='img') {
          const url=node.getAttribute('src');
          if(url&&states.get(node)!==url){states.set(node,url);images.push(node);imageURLs.push(url);}
        }
        if(node.nodeType===1&&node.localName==='script'&&node.id.startsWith('parser')){
          const parser=isParser(node);parserFlags.push(parser);
          if(!parser&&claimScript(node))dynamicClaims.push(node.id);
        }
      }
      globalThis.captureResources=node=>{
        inspect(node);
        if(node.querySelectorAll)for(const item of node.querySelectorAll('img,script'))inspect(item);
      };
      return event=>{
        captureResources(event.target);
        for(const node of event.addedNodes)captureResources(node);
        // A real consumer reenters the active document while handling each record.
        captureResources(document);
      };
    })()"#,
    );
    let callback = v8::Local::<v8::Function>::try_from(callback).unwrap();
    assert!(install_mutation_consumer(scope, doc, callback));

    let inert_units =
        "<div><img src='ordinary'><img src='ordinary'><script>ordinary</script></div>"
            .encode_utf16()
            .collect::<Vec<_>>();
    let inert = parse_html_document_units(scope, &inert_units, false);
    let key = v8::String::new(scope, "foreignDocument").unwrap();
    context.global(scope).set(scope, key.into(), inert.into());
    let ordinary=eval(scope,r#"(() => {
      resetResources();const source=foreignDocument.querySelector('div'),image=source.querySelector('img'),other=source.querySelectorAll('img')[1];
      const observer=new MutationObserver(()=>{});observer.observe(source,{attributes:true,subtree:true,attributeOldValue:true});
      document.body.append(document.adoptNode(source));captureResources(document);
      const stable=images.length===2&&images[0]===image&&images[1]===other&&image!==other&&document.querySelector('img')===image;
      const construction=observer.takeRecords();
      image.src='changed';captureResources(document);
      const records=observer.takeRecords();
      return JSON.stringify({stable,mutation:images.length===3&&images[2]===image&&imageURLs.join()==='ordinary,ordinary,changed',
        records:construction.length===0&&records.length===1&&records[0].target===image&&records[0].oldValue==='ordinary'});
    })()"#).to_rust_string_lossy(scope);
    eval(scope, "document.body.textContent='';resetResources()");
    let units="<!doctype html><body><img id=resource src='__proto__' width=3 height=2><script id=parser-blocking src='/blocking.js'></script><script id=parser-inline>inline</script><script id=parser-deferred src='/deferred.js' defer></script>".encode_utf16().collect::<Vec<_>>();
    let scripts = replace_document_content(scope, doc, &units).unwrap();
    let replacement=eval(scope,r#"captureResources(document);JSON.stringify({identities:images.length,stable:images.length===1&&images[0]===document.getElementById('resource'),parserFlags:parserFlags.every(Boolean),dynamicClaims})"#).to_rust_string_lossy(scope);
    let prepared = scripts
        .into_iter()
        .map(|node| prepare_script(scope, node).map(|script| script.parser_inserted))
        .collect::<Vec<_>>();
    eval(
        scope,
        r#"customElements.define('x-resource-reaction',class extends HTMLElement {
      constructor(){super();const image=document.createElement('img');image.src='constructor';document.body.append(image);
        globalThis.observedDuringConstructor=images.includes(image);}
    });document.body.textContent='';resetResources()"#,
    );
    let units = "<body><x-resource-reaction></x-resource-reaction>"
        .encode_utf16()
        .collect::<Vec<_>>();
    assert!(replace_document_content(scope, doc, &units)
        .unwrap()
        .is_empty());
    let reaction = eval(
        scope,
        "JSON.stringify([observedDuringConstructor,images.length,imageURLs])",
    )
    .to_rust_string_lossy(scope);
    assert_eq!(
        (
            ordinary.as_str(),
            replacement.as_str(),
            prepared,
            reaction.as_str()
        ),
        (
            r#"{"stable":true,"mutation":true,"records":true}"#,
            r#"{"identities":1,"stable":true,"parserFlags":true,"dynamicClaims":[]}"#,
            vec![Some(true), Some(true), Some(true)],
            r#"[true,1,["constructor"]]"#
        )
    );
    reset();
}

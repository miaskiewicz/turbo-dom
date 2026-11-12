use sha2::{Digest, Sha256};
use turbo_dom_browser_env::browser_env::{
    install_inline_event_handlers, install_with_html, prepare_script, replace_document_content,
    reset,
};

fn replace(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let source = args
        .get(0)
        .to_rust_string_lossy(scope)
        .encode_utf16()
        .collect::<Vec<_>>();
    let document = args.get(1).to_object(scope).unwrap();
    let nodes = replace_document_content(scope, document, &source).unwrap();
    let array = v8::Array::new(scope, nodes.len() as i32);
    for (index, node) in nodes.into_iter().enumerate() {
        array.set_index(scope, index as u32, node.into());
    }
    rv.set(array.into());
}

fn claim(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let node = args.get(0).to_object(scope).unwrap();
    rv.set_bool(prepare_script(scope, node).is_some());
}

fn claim_parser(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let node = args.get(0).to_object(scope).unwrap();
    rv.set_bool(
        prepare_script(scope, node)
            .is_some_and(|script| script.parser_inserted && !script.asynchronous),
    );
}

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
fn replacement_handles_reentrant_constructors_and_declarative_roots() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><title>old</title><body>old</body>");
    for (name, function) in [
        ("replaceContent", v8::Function::new(scope, replace).unwrap()),
        ("claimScript", v8::Function::new(scope, claim).unwrap()),
        (
            "claimParserScript",
            v8::Function::new(scope, claim_parser).unwrap(),
        ),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        context
            .global(scope)
            .set(scope, key.into(), function.into());
    }
    let source = r#"(() => {
      customElements.define('x-reentrant',class extends HTMLElement {constructor(){super();globalThis.inner=replaceContent('<title>inner</title><body><script id=innerScript>inner</script>',document);}});
      const outer=replaceContent('<title>outer</title><body><script>outer</script><x-reentrant></x-reentrant>',document);
      if(document.title!=='inner'||outer.length!==0||inner.length!==1||!claimScript(inner[0])||claimScript(inner[0]))return 'reentrant parser state';
      globalThis.claims=[];
      customElements.define('x-removal',class extends HTMLElement {constructor(){super();document.getElementById('gone').remove();const script=document.createElement('script');script.textContent='dynamic';document.body.append(script);globalThis.dynamic=script;claims.push(claimScript(script));document.title='constructor';}});
      const kept=replaceContent('<title>new</title><body><script id=gone>removed</script><script id=kept>kept</script><x-removal></x-removal>',document);
      if(document.title!=='constructor'||kept.length!==1||kept[0].id!=='kept'||claims.join()!=='true'||claimScript(dynamic))return 'constructor mutation or dynamic script reset';
      return 'ok';
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let result = v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap();
    assert_eq!(result.to_rust_string_lossy(scope), "ok");
    let html = include_str!("../src/document_replacement_shadow.html");
    let probe = include_str!("../scripts/document-replacement-shadow-probe.js");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "../src/document_replacement_shadow_chrome156.json"
    ))
    .unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("156.0.8078."));
    assert_eq!(control["htmlSha256"], format!("{:x}", Sha256::digest(html)));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe))
    );
    eval(scope, "globalThis.shadowOrder=[];globalThis.handlerMark=0");
    eval(scope, probe);
    eval(
        scope,
        &format!(
            "globalThis.payload={}",
            serde_json::to_string(html).unwrap()
        ),
    );
    assert_eq!(
        eval(
            scope,
            r#"(() => {
      const nodes=replaceContent(payload,document),ids=[];
      for(const node of nodes){
        if(node.ownerDocument!==document||!claimParserScript(node)||claimScript(node))return 'invalid parser flags/owner/admission';
        ids.push(node.id);(0,eval)(node.textContent);
      }
      return JSON.stringify(ids);
    })()"#
        ),
        r#"["before","light-first","shadow","light-last","closed-script","after"]"#
    );
    let actual: serde_json::Value =
        serde_json::from_str(&eval(scope, "JSON.stringify(inspectReplacement())")).unwrap();
    assert_eq!(actual, control["result"]);
    eval(scope, "globalThis.policyCalls=[]");
    let policy=v8::String::new(scope,"(doc,element)=>{policyCalls.push([doc===document,element.ownerDocument===document,element.getRootNode().mode]);return false}").unwrap();
    let policy = v8::Script::compile(scope, policy, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let policy = v8::Local::<v8::Function>::try_from(policy).unwrap();
    install_inline_event_handlers(scope, policy, "https://replacement.test/");
    let key = v8::String::new(scope, "document").unwrap();
    let document = context
        .global(scope)
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    let units = html.encode_utf16().collect::<Vec<_>>();
    assert_eq!(
        replace_document_content(scope, document, &units)
            .unwrap()
            .len(),
        6
    );
    assert_eq!(eval(scope,"handlerMark=0;document.getElementById('open').shadowRoot.querySelector('#action').click();JSON.stringify([handlerMark,policyCalls.some(row=>row[0]&&row[1]&&row[2]==='open')])"),"[0,true]");
    reset();
}

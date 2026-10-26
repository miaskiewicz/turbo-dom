use super::*;

#[test]
fn replaced_document_preserves_traversal_and_equality_on_all_native_node_kinds() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    for _ in 0..3 {
        install_independent_html(scope, "<!doctype html><body><div></div></body>");
        let code = v8::String::new(scope, r#"(()=>{
          const host=document.querySelector('div'),root=host.attachShadow({mode:'open'});
          root.innerHTML='<b>shadow</b>tail';
          const nodes=[root,document.createDocumentFragment(),document.createElement('section')];
          for(const node of nodes){
            if(node!==root)node.append(document.createElement('b'),document.createTextNode('tail'));
            if(node.lastChild!==node.childNodes.item(1)||node.lastChild.nodeName!=='#text'||node.lastChild.previousSibling!==node.firstChild)throw Error('native traversal');
            if(!node.isSameNode(node)||!node.isEqualNode(node)||node!==root&&!node.isEqualNode(node.cloneNode(true)))throw Error('native equality');
          }
          for(const name of ['lastChild','previousSibling','isSameNode','isEqualNode']){
            const descriptor=Object.getOwnPropertyDescriptor(Node.prototype,name);
            if(!descriptor||!descriptor.enumerable||!descriptor.configurable)throw Error('interface ownership '+name);
            const method=descriptor.get||descriptor.value;
            if(!Function.prototype.toString.call(method).includes('[native code]'))throw Error('native delegate '+name);
            let invalid=false;try{method.call({});}catch(error){invalid=error.name==='TypeError';}
            if(!invalid)throw Error('native receiver brand '+name);
          }
          return 'ok';
        })()"#).unwrap();
        v8::tc_scope!(let catch, scope);
        let result = v8::Script::compile(catch, code, None)
            .and_then(|script| script.run(catch))
            .unwrap_or_else(|| {
                panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch))
            });
        assert_eq!(result.to_rust_string_lossy(catch), "ok");
    }
    reset();
}

#[test]
fn first_native_wrapping_does_not_call_page_modified_projection_intrinsics() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body></body>");
    let code = v8::String::new(scope, r#"(()=>{
      const originals=[Function.prototype.call,Object.setPrototypeOf,Object.prototype.hasOwnProperty,
        WeakSet.prototype.has,WeakSet.prototype.add,Array.prototype.forEach,String.prototype.toUpperCase,String.prototype.slice];
      let hooks=0;const poison=()=>{hooks++;throw Error('page projection intrinsic '+new Error().stack);};
      let nodes;
      try {
        Function.prototype.call=poison;Object.setPrototypeOf=poison;Object.prototype.hasOwnProperty=poison;
        WeakSet.prototype.has=poison;WeakSet.prototype.add=poison;Array.prototype.forEach=poison;
        String.prototype.toUpperCase=poison;String.prototype.slice=poison;
        nodes=[document.createTextNode('text'),document.createComment('comment'),
          document.createAttribute('name'),document.createElement('section'),
          document.createElementNS('http://www.w3.org/2000/svg','svg'),document.createDocumentFragment()];
      } finally {
        Function.prototype.call=originals[0];Object.setPrototypeOf=originals[1];Object.prototype.hasOwnProperty=originals[2];
        WeakSet.prototype.has=originals[3];WeakSet.prototype.add=originals[4];Array.prototype.forEach=originals[5];
        String.prototype.toUpperCase=originals[6];String.prototype.slice=originals[7];
      }
      if(hooks!==0)throw Error('page hooks ran');
      const types=[Text,Comment,Attr,HTMLElement,SVGSVGElement,DocumentFragment];
      for(let index=0;index<nodes.length;index++)if(!(nodes[index] instanceof types[index]))throw Error('incorrect projection '+index);
      return 'ok';
    })()"#).unwrap();
    v8::tc_scope!(let catch, scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    assert_eq!(result.to_rust_string_lossy(catch), "ok");
    reset();
}

#[test]
fn parser_created_controls_use_private_projectors_despite_page_constructor_getters() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(
        scope,
        "<body><input id='latent-input' value='native'></body>",
    );
    let code = v8::String::new(scope, r#"(()=>{
      const ns=globalThis[Symbol.for('ts')],define=Object.defineProperty;
      const inputPrototype=HTMLInputElement.prototype,textareaPrototype=HTMLTextAreaElement.prototype;
      const inputDescriptor=Object.getOwnPropertyDescriptor(globalThis,'HTMLInputElement');
      const textareaDescriptor=Object.getOwnPropertyDescriptor(globalThis,'HTMLTextAreaElement');
      const projectionDescriptor=Object.getOwnPropertyDescriptor(ns,'projectNativeNode');
      let hooks=0,input,textarea,inert,custom;
      const poison=()=>{hooks++;throw Error('page node projection getter');};
      try {
        define(globalThis,'HTMLInputElement',{get:poison,configurable:true});
        define(globalThis,'HTMLTextAreaElement',{get:poison,configurable:true});
        define(ns,'projectNativeNode',{get:poison,configurable:true});
        define(Object.prototype,'X-CUSTOM',{value:'HTMLInputElement',configurable:true});
        input=document.getElementById('latent-input');
        textarea=document.createElement('textarea');
        inert=document.implementation.createHTMLDocument('private');
        custom=inert.createElement('x-custom');
        inert.body.innerHTML='<input id="inert-input" value="inert">';
        if(inert.getElementById('inert-input').value!=='inert')throw Error('independent control');
      } finally {
        define(globalThis,'HTMLInputElement',inputDescriptor);
        define(globalThis,'HTMLTextAreaElement',textareaDescriptor);
        define(ns,'projectNativeNode',projectionDescriptor);
        delete Object.prototype['X-CUSTOM'];
      }
      if(hooks!==0||Object.getPrototypeOf(input)!==inputPrototype||input.value!=='native'||Object.getPrototypeOf(textarea)!==textareaPrototype)throw Error('projection intercepted');
      if(Object.getPrototypeOf(custom)!==HTMLElement.prototype||custom.ownerDocument!==inert)throw Error('inherited interface name');
      return 'ok';
    })()"#).unwrap();
    v8::tc_scope!(let catch, scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    assert_eq!(result.to_rust_string_lossy(catch), "ok");
    reset();
}

#[test]
fn interrupted_native_wrapping_leaves_no_unprojected_cached_object() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body></body>");
    let id = context.get_slot::<DomContextId>().unwrap().0;
    let _dom = DomScope::enter(id);
    let handle = with_tree_mut(|tree| tree.create_element("input")).unwrap();
    v8::tc_scope!(let scope, scope);
    scope.terminate_execution();
    let _discarded = wrap(scope, handle);
    // Weak-handle localization can materialize the interrupt into TryCatch
    // before JavaScript runs; the isolate pending flag is then already cleared.
    assert!(scope.is_execution_terminating() || scope.has_terminated());
    assert!(DOM.with(|dom| !dom.borrow().as_ref().unwrap().cache.contains_key(&handle)));
    scope.cancel_terminate_execution();
    scope.reset();
    let node = wrap(scope, handle);
    let key = v8::String::new(scope, "value").unwrap();
    assert_eq!(
        node.get(scope, key.into())
            .unwrap()
            .to_rust_string_lossy(scope),
        ""
    );
    reset();
}

#[test]
fn html_namespace_local_names_and_foreign_control_brands_match_chrome() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<!doctype html><body></body>");
    let probe = include_str!("element_interface_probe.js");
    let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    let source = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope, scope);
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)));
    let actual: serde_json::Value =
        serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("element_interface_chrome155.json")).unwrap();
    let actual = actual.as_array().unwrap();
    let expected = expected["result"].as_array().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        if expected["namespace"] == "http://www.w3.org/2000/svg" {
            // SVG-specific interfaces are a separate implementation backlog.
            for field in [
                "kind",
                "namespace",
                "name",
                "localName",
                "prefix",
                "tagName",
                "html",
                "input",
                "unknown",
                "scriptAsync",
                "inputType",
            ] {
                assert_eq!(
                    actual[field], expected[field],
                    "foreign control brand row {index}: {field}"
                );
            }
        } else {
            assert_eq!(
                actual, expected,
                "HTML/null-namespace interface row {index}"
            );
        }
    }
    reset();
}

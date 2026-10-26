use super::*;

#[test]
fn inert_initializer_uses_its_creation_realm_with_shared_embedder_ops() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, isolate);
        let main = v8::Context::new(scope, Default::default());
        let child = v8::Context::new(scope, Default::default());
        let main_function = {
            let scope = &mut v8::ContextScope::new(scope, main);
            let source = v8::String::new(scope, "(function mainInitializer(){})").unwrap();
            let value = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap();
            v8::Local::<v8::Function>::try_from(value).unwrap()
        };
        let child_function = {
            let scope = &mut v8::ContextScope::new(scope, child);
            let source = v8::String::new(scope, "(function childInitializer(){})").unwrap();
            let value = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap();
            v8::Local::<v8::Function>::try_from(value).unwrap()
        };
        let scope = &mut v8::ContextScope::new(scope, main);
        assert!(dom_implementation::set_initializer(scope, main_function));
        assert!(dom_implementation::set_initializer(scope, child_function));
        let name = v8::String::new(scope, "turbo.native.inert-document.initializer").unwrap();
        let key = v8::Private::for_api(scope, Some(name));
        assert!(main
            .global(scope)
            .get_private(scope, key)
            .unwrap()
            .strict_equals(main_function.into()));
        assert!(child
            .global(scope)
            .get_private(scope, key)
            .unwrap()
            .strict_equals(child_function.into()));
    }
}
#[test]
fn independent_factories_and_constructors_match_authored_chrome155_control() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let probe = include_str!("dom_implementation_probe.js");
    let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    v8::tc_scope!(let scope,scope);
    let source = v8::String::new(scope, &source).unwrap();
    let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    let result = result.unwrap_or_else(|| {
        panic!(
            "factory probe failed: {:?}",
            scope
                .exception()
                .map(|error| error.to_rust_string_lossy(scope))
        )
    });
    let result: serde_json::Value =
        serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("dom_implementation_chrome155.json")).unwrap();
    assert_eq!(result, expected["result"]);
    reset();
}

#[test]
fn raw_parsed_html_without_interface_metadata_keeps_name_getters_exception_free() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><html><body></body></html>");
    let doc = parse_html_document(
        scope,
        "<!doctype html><html><body><div id=x data-value=v>text</div></body></html>",
        false,
    );
    let name = v8::String::new(scope, "rawParsedDocument").unwrap();
    context.global(scope).set(scope, name.into(), doc.into());
    v8::tc_scope!(let scope,scope);
    let source=v8::String::new(scope,"(()=>{const doc=rawParsedDocument,node=doc.getElementById('x'),attribute=node.getAttributeNode('data-value');return doc.documentElement.nodeName==='HTML'&&node.tagName==='DIV'&&attribute.nodeName==='data-value'&&node.firstChild.nodeName==='#text';})()").unwrap();
    let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    assert!(
        result.is_some_and(|value| value.is_true()),
        "raw parsed record name read failed: {:?}",
        scope
            .exception()
            .map(|error| error.to_rust_string_lossy(scope))
    );
    reset();
}

#[test]
fn xml_element_case_namespaces_and_receivers_match_authored_chrome155_control() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let probe = include_str!("xml_factory_elements_probe.js");
    let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    v8::tc_scope!(let scope,scope);
    let source = v8::String::new(scope, &source).unwrap();
    let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    let result = result.unwrap_or_else(|| {
        panic!(
            "factory probe failed: {:?}",
            scope
                .exception()
                .map(|error| error.to_rust_string_lossy(scope))
        )
    });
    let result: serde_json::Value =
        serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("xml_factory_elements_chrome155.json")).unwrap();
    assert_eq!(result, expected["result"]);
    reset();
}

#[test]
fn text_and_fragment_constructors_use_native_factories_and_preserve_subclass_semantics() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let source = v8::String::new(scope, r#"(()=>{
      const descriptor=Object.getOwnPropertyDescriptor,documentText=Document.prototype.createTextNode,documentFragment=Document.prototype.createDocumentFragment;
      let hooks=0;const poison=()=>{hooks++;throw Error('public creation method')};
      Document.prototype.createTextNode=Document.prototype.createDocumentFragment=poison;
      document.createTextNode=document.createDocumentFragment=poison;
      try {
        const values=[new Text().data,new Text(undefined).data,new Text(null).data,new Text(42).data,new Text('a\ud800\udc00\ud800').data];
        if(JSON.stringify(values)!==JSON.stringify(['','','null','42','a\ud800\udc00\ud800']))throw Error('DOMString '+JSON.stringify(values));
        let calls=0;const converted=new Text({toString(){calls++;return 'converted'}});
        if(converted.data!=='converted'||calls!==1)throw Error('conversion');
        for(const operation of [()=>Text(),()=>DocumentFragment(),()=>new Text(Symbol('bad'))]){let name;try{operation()}catch(error){name=error.name}if(name!=='TypeError')throw Error('TypeError required')}
        class ChildText extends Text{}class ChildFragment extends DocumentFragment{}
        const text=new ChildText('child'),fragment=new ChildFragment(Symbol('ignored'));
        if(Object.getPrototypeOf(text)!==ChildText.prototype||!(text instanceof Text)||text.ownerDocument!==document||text.data!=='child'||Object.getPrototypeOf(fragment)!==ChildFragment.prototype||!(fragment instanceof DocumentFragment)||fragment.ownerDocument!==document||fragment.childNodes.length)throw Error('subclass');
        fragment.appendChild(text);document.body.appendChild(fragment);
        if(fragment.childNodes.length||text.parentNode!==document.body||hooks)throw Error('native nodes');
        for(const [constructor,parent] of [[Text,CharacterData],[DocumentFragment,Node],[ShadowRoot,DocumentFragment]])if(Object.getPrototypeOf(constructor)!==parent||constructor.prototype.constructor!==constructor||descriptor(constructor,'prototype').writable||constructor.length!==0)throw Error('constructor descriptor');
        return true;
      }finally{Document.prototype.createTextNode=documentText;Document.prototype.createDocumentFragment=documentFragment;delete document.createTextNode;delete document.createDocumentFragment;}
    })()"#).unwrap();
    v8::tc_scope!(let scope, scope);
    let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    assert!(result
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .is_true());
    reset();
}

#[test]
fn borrowed_text_and_fragment_constructors_keep_their_creation_document() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let parent = v8::Context::new(scope, Default::default());
    {
        let scope = &mut v8::ContextScope::new(scope, parent);
        install_with_html(scope, "<!doctype html><body>parent</body>");
    }
    let child = v8::Context::new(scope, Default::default());
    child.set_security_token(parent.get_security_token(scope));
    {
        let scope = &mut v8::ContextScope::new(scope, child);
        install_independent_html(scope, "<!doctype html><body>child</body>");
    }
    let scope = &mut v8::ContextScope::new(scope, parent);
    let global = parent.global(scope);
    let child_global = child.global(scope);
    set_prop(scope, global, "foreign", child_global.into());
    let source = v8::String::new(scope, r#"(()=>{
      const TextConstructor=foreign.Text,FragmentConstructor=foreign.DocumentFragment;
      foreign.Document.prototype.createTextNode=foreign.Document.prototype.createDocumentFragment=function(){throw Error('foreign public factory')};
      const text=new TextConstructor('foreign'),fragment=new FragmentConstructor();
      if(text.ownerDocument!==foreign.document||fragment.ownerDocument!==foreign.document||!(text instanceof foreign.Text)||text instanceof Text||!(fragment instanceof foreign.DocumentFragment)||fragment instanceof DocumentFragment)throw Error('creation realm');
      class LocalText extends Text{}class LocalFragment extends DocumentFragment{}
      const borrowed=Reflect.construct(TextConstructor,['borrowed'],LocalText),borrowedFragment=Reflect.construct(FragmentConstructor,[],LocalFragment);
      if(borrowed.ownerDocument!==foreign.document||borrowedFragment.ownerDocument!==foreign.document||Object.getPrototypeOf(borrowed)!==LocalText.prototype||Object.getPrototypeOf(borrowedFragment)!==LocalFragment.prototype||borrowed.data!=='borrowed')throw Error('new.target realm');
      return true;
    })()"#).unwrap();
    v8::tc_scope!(let scope, scope);
    let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    assert!(result
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .is_true());
    reset();
}

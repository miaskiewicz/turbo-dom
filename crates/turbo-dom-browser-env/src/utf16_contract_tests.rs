use super::*;
fn import<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) {
    let target = args.get(0).to_object(scope).unwrap();
    let node = args.get(1).to_object(scope).unwrap();
    result.set(
        import_document_node(scope, target, node, args.get(2).boolean_value(scope))
            .unwrap()
            .into(),
    );
}

#[test]
fn native_units_match_fresh_chrome_for_creation_mutation_clone_adoption_and_ranges() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let global = context.global(scope);
    bind_method(scope, global, "__utf16HostImport", import);
    let source=format!("globalThis.__utf16Result=null;({}).then(value=>globalThis.__utf16Result=JSON.stringify(value),error=>globalThis.__utf16Result=String(error.stack));",include_str!("utf16_probe.js"));
    let code = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let catch,scope);
    v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    catch.perform_microtask_checkpoint();
    let code = v8::String::new(catch, "__utf16Result").unwrap();
    let result = v8::Script::compile(catch, code, None)
        .unwrap()
        .run(catch)
        .unwrap()
        .to_rust_string_lossy(catch);
    let actual: serde_json::Value =
        serde_json::from_str(&result).unwrap_or_else(|_| panic!("{result}"));
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("utf16_chrome155.json")).unwrap();
    for key in [
        "innerTextUnits",
        "outerTextUnits",
        "serialized",
        "reflectedValues",
        "source",
        "created",
        "input",
        "attribute",
        "attrNode",
        "imported",
        "adopted",
        "halves",
        "normalized",
        "substring",
        "charOps",
        "rangeValue",
        "rangeClone",
        "mutationOld",
    ] {
        assert_eq!(actual[key], oracle["result"][key], "UTF-16 surface {key}");
    }
    reset();
}

#[test]
fn utf16_coercion_uses_native_units_and_conversion_failure_is_atomic() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let source = r#"(()=>{const units=value=>Array.from({length:value.length},(_,i)=>value.charCodeAt(i));
      const text=document.createTextNode('original'),el=document.createElement('div');el.append(text);
      let calls=0;text.data={toString(){calls++;return String.fromCharCode(0xd800)}};
      const range=document.createRange();range.selectNodeContents(text);let failures=0;
      for(const operation of [()=>text.replaceData(0,1,Symbol()),()=>el.setAttribute('data-u',Symbol()),()=>document.createTextNode(Symbol())]){
        try{operation()}catch(error){if(error instanceof TypeError)failures++}
      }
      let thrown=false;try{text.data={toString(){throw Error('conversion')}}}catch(error){thrown=error.message==='conversion'}
      return JSON.stringify({calls,failures,thrown,text:units(text.data),range:units(range.toString()),missing:el.getAttribute('data-u')===null});
    })()"#;
    let code = v8::String::new(scope, source).unwrap();
    v8::tc_scope!(let catch,scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)))
        .to_rust_string_lossy(catch);
    assert_eq!(
        result,
        r#"{"calls":1,"failures":3,"thrown":true,"text":[55296],"range":[55296],"missing":true}"#
    );
    reset();
}

#[test]
fn exact_surrogate_node_equality_remains_distinct_from_replacement_character() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let source = format!("JSON.stringify({})", include_str!("utf16_names_probe.js"));
    let code = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let catch,scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)))
        .to_rust_string_lossy(catch);
    let actual: serde_json::Value = serde_json::from_str(&result).unwrap();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("utf16_names_chrome155.json")).unwrap();
    assert_eq!(actual, oracle["result"]);
    reset();
}

#[test]
fn native_name_id_class_and_css_string_units_match_fresh_chrome() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let source = format!("JSON.stringify({})", include_str!("utf16_lookups_probe.js"));
    let code = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let catch,scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)))
        .to_rust_string_lossy(catch);
    let actual: serde_json::Value = serde_json::from_str(&result).unwrap();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("utf16_lookups_chrome155.json")).unwrap();
    assert_eq!(actual, oracle["result"]);
    reset();
}

fn parse_units<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) {
    let units = dom_units(scope, args.get(0)).unwrap();
    result.set(parse_html_document_units(scope, &units, false).into());
}
#[test]
fn original_html_parser_units_and_marker_entities_match_fresh_chrome() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let global = context.global(scope);
    bind_method(scope, global, "__utf16HostParse", parse_units);
    let source = format!("JSON.stringify({})", include_str!("utf16_parser_probe.js"));
    let code = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let catch,scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)))
        .to_rust_string_lossy(catch);
    let actual: serde_json::Value = serde_json::from_str(&result).unwrap();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("utf16_parser_chrome155.json")).unwrap();
    for key in [
        "tree",
        "inner",
        "serialized",
        "textarea",
        "script",
        "doctype",
    ] {
        assert_eq!(actual[key], oracle["result"][key], "parser units {key}");
    }
    reset();
}

#[test]
fn original_document_factories_metadata_nonce_and_names_match_fresh_chrome() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let global = context.global(scope);
    bind_method(scope, global, "__utf16HostImport", import);
    let source = format!(
        "JSON.stringify({})",
        include_str!("utf16_metadata_probe.js")
    );
    let code = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let catch,scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)))
        .to_rust_string_lossy(catch);
    let actual: serde_json::Value = serde_json::from_str(&result).unwrap();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("utf16_metadata_chrome155.json")).unwrap();
    assert_eq!(actual, oracle["result"]);
    reset();
}
#[test]
fn prepared_inline_source_keeps_real_units_through_direct_v8_compilation() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let key = v8::String::new(scope, "document").unwrap();
    let document = context
        .global(scope)
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    {
        let _owner = DomScope::for_object(scope, document);
        DOM.with(|dom| {
            let mut dom = dom.borrow_mut();
            let state = dom.as_mut().unwrap();
            state.browsing_context = true;
            state.resource_active = true;
        });
    }
    let code=v8::String::new(scope,"(()=>{const script=document.createElement('script');script.textContent=\"globalThis.__unitSource='\"+String.fromCharCode(0xd800)+\"x\"+String.fromCharCode(0xdc00)+\"';\";document.body.append(script);return script;})()").unwrap();
    let script = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let prepared = prepare_script(scope, script).unwrap();
    assert!(prepared.source_units.contains(&0xd800) && prepared.source_units.contains(&0xdc00));
    let code = unit_string(scope, &prepared.source_units);
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let key = v8::String::new(scope, "__unitSource").unwrap();
    let result = context
        .global(scope)
        .get(scope, key.into())
        .unwrap()
        .to_string(scope)
        .unwrap();
    assert_eq!(string_code_units(scope, result), vec![0xd800, 120, 0xdc00]);
    reset();
}

#[test]
fn dom_string_parser_bom_behavior_matches_fresh_chrome() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let global = context.global(scope);
    bind_method(scope, global, "__utf16HostParse", parse_units);
    let source = format!("JSON.stringify({})", include_str!("utf16_bom_probe.js"));
    let code = v8::String::new(scope, &source).unwrap();
    let actual = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&actual).unwrap();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("utf16_bom_chrome155.json")).unwrap();
    assert_eq!(actual, oracle["result"]);
    reset();
}

#[test]
fn independent_domstring_document_retains_original_units_and_prepared_script_source() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let parent = v8::Context::new(scope, Default::default());
    {
        let scope = &mut v8::ContextScope::new(scope, parent);
        install_independent_html(scope, "<body><p>parent</p>");
    }
    let child = v8::Context::new(scope, Default::default());
    {
        let scope = &mut v8::ContextScope::new(scope, child);
        let mut source = "<body><p data-u='X'>X</p><script>globalThis.exact='X'</script>"
            .encode_utf16()
            .collect::<Vec<_>>();
        for unit in &mut source {
            if *unit == 88 {
                *unit = 0xd800;
            }
        }
        install_independent_html_units(scope, &source);
        let code = v8::String::new(scope, "JSON.stringify([document.querySelector('p').textContent.charCodeAt(0),document.querySelector('p').getAttribute('data-u').charCodeAt(0)])").unwrap();
        let result = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(result.to_rust_string_lossy(scope), "[55296,55296]");
        let code = v8::String::new(scope, "document.querySelector('script')").unwrap();
        let script = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_object(scope)
            .unwrap();
        let prepared = script_state::prepare_script(scope, script).unwrap();
        let mut expected = "globalThis.exact='X'".encode_utf16().collect::<Vec<_>>();
        *expected.iter_mut().find(|unit| **unit == 88).unwrap() = 0xd800;
        assert_eq!(prepared.source_units, expected);
    }
    {
        let scope = &mut v8::ContextScope::new(scope, parent);
        let code = v8::String::new(scope, "document.querySelector('p').textContent").unwrap();
        let result = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(result.to_rust_string_lossy(scope), "parent");
    }
    reset();
}

#[test]
fn native_inline_handler_units_refresh_and_clone_sources_match_fresh_chrome() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<body>");
    let code = v8::String::new(
        scope,
        &format!(
            "JSON.stringify({})",
            include_str!("utf16_handlers_probe.js")
        ),
    )
    .unwrap();
    v8::tc_scope!(let catch,scope);
    let result = v8::Script::compile(catch, code, None)
        .and_then(|script| script.run(catch))
        .unwrap_or_else(|| panic!("{}", catch.exception().unwrap().to_rust_string_lossy(catch)));
    let value = result.to_rust_string_lossy(catch);
    let actual: serde_json::Value = serde_json::from_str(&value).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("utf16_handlers_chrome155.json")).unwrap();
    assert_eq!(actual, expected["result"]);
    reset();
}

//! Trusted values keep their exact strings and callbacks in guest-GC-owned private edges.
use super::*;

struct RecordKey(v8::Global<v8::Private>);

fn key<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Private>> {
    if scope.is_execution_terminating() {
        return None;
    }
    if let Some(key) = scope.get_slot::<std::rc::Rc<RecordKey>>().cloned() {
        return Some(v8::Local::new(scope, &key.0));
    }
    let name = v8::String::new(scope, "turbo.trusted-types.record")?;
    let key = v8::Private::for_api(scope, Some(name));
    let owned = v8::Global::new(scope, key);
    // This primitive key has no creation-context edge and cannot retain a Window.
    scope.set_slot(std::rc::Rc::new(RecordKey(owned)));
    Some(key)
}

fn record(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    if args.get(0).is_number() {
        if let Some(value) = args.get(1).to_string(scope) {
            result.set(value.into());
        }
        return;
    }
    let Some(key) = key(scope) else {
        return;
    };
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    if object.is_proxy() {
        return;
    }
    if args.get(0).is_true() {
        if let Some(value) = object.get_private(scope, key) {
            result.set(value);
        }
    } else {
        let value = args.get(2);
        if object.set_private(scope, key, value) == Some(true) {
            result.set(object.into());
        }
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    if key(scope).is_none() {
        return;
    }
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__trustedTypesRecord", record);
    let mapping = v8::String::new(scope, include_str!("trusted_types_mapping.json")).unwrap();
    set_prop(scope, global, "__trustedTypesMapping", mapping.into());
    run_js(scope, include_str!("trusted_types.js"));
}

fn private_record<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    if object.is_proxy() {
        return None;
    }
    let key = key(scope)?;
    object
        .get_private(scope, key)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
}
/// Whether `value` is a Trusted Types value of `kind` (e.g. "TrustedHTML").
pub(super) fn is_trusted<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
    kind: &str,
) -> bool {
    let Ok(object) = v8::Local::<v8::Object>::try_from(value) else {
        return false;
    };
    private_record(scope, object)
        .and_then(|record| field(scope, record, "kind"))
        .is_some_and(|actual| actual.to_rust_string_lossy(scope) == kind)
}
fn field<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let key = v8::String::new(scope, name)?;
    object.get(scope, key.into())
}
pub(super) fn install_policy(
    scope: &mut v8::PinScope,
    callback: v8::Local<v8::Function>,
) -> Option<()> {
    let global = scope.get_current_context().global(scope);
    let record = private_record(scope, global)?;
    let factory = v8::Local::<v8::Object>::try_from(field(scope, record, "factory")?).ok()?;
    let state = private_record(scope, factory)?;
    let name = v8::String::new(scope, "authorize")?;
    (state.set(scope, name.into(), callback.into()) == Some(true)).then_some(())
}
pub(super) fn convert<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    input: v8::Local<'s, v8::Value>,
    kind: &str,
    sink: &str,
    prefix: &str,
    null_to_empty: bool,
) -> Option<v8::Local<'s, v8::Value>> {
    let _owner = DomScope::for_object(scope, node);
    let document = wrap(scope, with_tree(Tree::root)?);
    let context = document.get_creation_context(scope)?;
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let state = private_record(scope, global)?;
    let converter = v8::Local::<v8::Function>::try_from(field(scope, state, "convert")?).ok()?;
    let kind = v8::String::new(scope, kind)?;
    let sink = v8::String::new(scope, sink)?;
    let prefix = v8::String::new(scope, prefix)?;
    let nullable = v8::Boolean::new(scope, null_to_empty);
    let receiver = v8::undefined(scope);
    converter.call(
        scope,
        receiver.into(),
        &[
            input,
            kind.into(),
            sink.into(),
            prefix.into(),
            nullable.into(),
        ],
    )
}
pub(super) fn units<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    input: v8::Local<'s, v8::Value>,
    kind: &str,
    sink: &str,
    prefix: &str,
    null_to_empty: bool,
) -> Option<Vec<u16>> {
    let value = convert(scope, node, input, kind, sink, prefix, null_to_empty)?;
    if let Ok(object) = v8::Local::<v8::Object>::try_from(value) {
        if let Some(record) = private_record(scope, object) {
            if let Some(value) = field(scope, record, "value")
                .and_then(|value| v8::Local::<v8::String>::try_from(value).ok())
            {
                return Some(string_code_units(scope, value));
            }
        }
    }
    dom_units(scope, value)
}

pub(super) fn script_source<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    input: v8::Local<'s, v8::Value>,
) -> Option<v8::Local<'s, v8::String>> {
    let object = v8::Local::<v8::Object>::try_from(input).ok()?;
    let record = private_record(scope, object)?;
    if field(scope, record, "kind")?.to_rust_string_lossy(scope) != "TrustedScript" {
        return None;
    }
    v8::Local::<v8::String>::try_from(field(scope, record, "value")?).ok()
}

pub(super) fn convert_call<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) {
    let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let kind = args.get(2).to_rust_string_lossy(scope);
    let sink = args.get(3).to_rust_string_lossy(scope);
    let prefix = args.get(4).to_rust_string_lossy(scope);
    if let Some(value) = convert(
        scope,
        node,
        args.get(1),
        &kind,
        &sink,
        &prefix,
        args.get(5).is_true(),
    ) {
        result.set(value);
    }
}

pub(super) fn attribute_units<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    input: v8::Local<'s, v8::Value>,
    attribute: &str,
    namespace: Option<&str>,
    method: &str,
) -> Option<Vec<u16>> {
    let _owner = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    let (tag, ns) = with_tree(|tree| {
        (
            tree.local_name(handle).unwrap_or_default().to_string(),
            tree.namespace_uri(handle).unwrap_or_default().to_string(),
        )
    })?;
    let html = ns == "http://www.w3.org/1999/xhtml";
    let svg = ns == "http://www.w3.org/2000/svg";
    let local = attribute.rsplit(':').next().unwrap_or(attribute);
    static MAPPING: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let mapping = MAPPING.get_or_init(|| {
        serde_json::from_str(include_str!("trusted_types_mapping.json"))
            .expect("recorded trusted sinks")
    });
    let kind = if namespace.is_none()
        && (html || svg || ns == "http://www.w3.org/1998/Math/MathML")
        && mapping["*"]["attributes"].get(attribute).is_some()
    {
        "TrustedScript"
    } else if html
        && namespace.is_none()
        && ((tag == "script" || tag == "embed") && attribute == "src"
            || tag == "object" && matches!(attribute, "data" | "codebase"))
    {
        "TrustedScriptURL"
    } else if html && namespace.is_none() && tag == "iframe" && attribute == "srcdoc" {
        "TrustedHTML"
    } else if svg
        && tag == "script"
        && local == "href"
        && (namespace.is_none() || namespace == Some("http://www.w3.org/1999/xlink"))
    {
        "TrustedScriptURL"
    } else {
        return dom_units(scope, input);
    };
    let owner = match (tag.as_str(), attribute) {
        ("script", "src") => "HTMLScriptElement",
        ("embed", "src") => "HTMLEmbedElement",
        ("object", "data" | "codebase") => "HTMLObjectElement",
        ("iframe", "srcdoc") => "HTMLIFrameElement",
        _ => "Element",
    };
    let sink = format!("{owner} {local}");
    let prefix = format!("Failed to execute '{method}' on 'Element': ");
    units(scope, node, input, kind, &sink, &prefix, false)
}
pub(super) fn script_units<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    input: v8::Local<'s, v8::Value>,
    property: &str,
    nullable: bool,
) -> Option<Vec<u16>> {
    let _owner = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    let script = with_tree(|tree| {
        tree.namespace_uri(handle) == Some("http://www.w3.org/1999/xhtml")
            && tree.local_name(handle) == Some("script")
    })
    .unwrap_or(false);
    if !script {
        return if nullable && input.is_null() {
            Some(Vec::new())
        } else {
            dom_units(scope, input)
        };
    }
    let sink = format!("HTMLScriptElement {property}");
    let prefix = format!("Failed to set the '{property}' property on 'HTMLScriptElement': ");
    units(
        scope,
        node,
        input,
        "TrustedScript",
        &sink,
        &prefix,
        nullable,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
        v8::tc_scope!(let scope, scope);
        let code = v8::String::new(scope, source).unwrap();
        v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
    }
    #[test]
    fn engine_source_reads_only_real_private_script_values_without_guest_coercion() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body></body>");
        let value = eval(
            scope,
            "trustedTypes.createPolicy('native',{createScript:s=>s}).createScript('lexical + 2')",
        );
        let source = super::script_source(scope, value).unwrap();
        assert_eq!(source.to_rust_string_lossy(scope), "lexical + 2");
        let global = context.global(scope);
        set_prop(scope, global, "typed", value);
        eval(
            scope,
            "Object.defineProperty(typed,'toString',{value(){throw Error('poison')}})",
        );
        assert!(super::script_source(scope, value).is_some());
        for expression in [
            "new Proxy(typed,{})",
            "Object.create(TrustedScript.prototype)",
            "({toString(){throw Error('ordinary')}})",
            "'lexical + 2'",
            "trustedTypes.emptyHTML",
        ] {
            let value = eval(scope, expression);
            assert!(super::script_source(scope, value).is_none(), "{expression}");
        }
    }

    #[test]
    fn native_factory_values_descriptors_conversions_and_mapping_match_chrome() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("trusted_types_cases.json")).unwrap();
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("trusted_types_chrome155.json")).unwrap();
        assert_eq!(oracle["errors"], serde_json::json!([]));
        for id in [
            "shape",
            "values",
            "callbacks",
            "missing-rules",
            "option-conversion",
            "mapping",
            "brands",
            "edge-cases",
        ] {
            let context = if id == "shape" {
                context
            } else {
                v8::Context::new(scope, Default::default())
            };
            let scope = &mut v8::ContextScope::new(scope, context);
            let case = cases
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["id"] == id)
                .unwrap();
            install_independent_html(scope, case["html"].as_str().unwrap());
            let expected = oracle["result"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["id"] == id)
                .unwrap()["result"]
                .clone();
            let probe = include_str!("trusted_types_probe.js")
                .trim()
                .trim_end_matches(';');
            eval(scope, &format!("({probe}).then(value=>globalThis.ttResult=JSON.stringify(value),error=>globalThis.ttResult='ERROR '+String(error.stack));"));
            scope.perform_microtask_checkpoint();
            let result = eval(scope, "globalThis.ttResult").to_rust_string_lossy(scope);
            let actual: serde_json::Value =
                serde_json::from_str(&result).unwrap_or_else(|_| panic!("{id}: {result}"));
            assert_eq!(actual, expected, "Chrome case {id}");
        }
        reset();
    }
    #[test]
    fn exhaustive_sink_mapping_and_conversion_errors_match_independent_chrome() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        for (probe, oracle) in [
            (
                include_str!("trusted_types_matrix_probe.js"),
                include_str!("trusted_types_matrix_chrome155.json"),
            ),
            (
                include_str!("trusted_types_conversion_probe.js"),
                include_str!("trusted_types_conversion_chrome155.json"),
            ),
        ] {
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_independent_html(scope, "<!doctype html><body>");
            let expected: serde_json::Value = serde_json::from_str(oracle).unwrap();
            let probe = probe.trim().trim_end_matches(';');
            eval(scope, &format!("Promise.resolve(({probe})).then(value=>globalThis.ttResult=JSON.stringify(value),error=>globalThis.ttResult='ERROR '+String(error.stack));"));
            scope.perform_microtask_checkpoint();
            let result = eval(scope, "globalThis.ttResult").to_rust_string_lossy(scope);
            let actual: serde_json::Value =
                serde_json::from_str(&result).unwrap_or_else(|_| panic!("{result}"));
            let expected = &expected["result"][0]["result"];
            if let (Some(actual), Some(expected)) =
                (actual["matrix"].as_array(), expected["matrix"].as_array())
            {
                let unexpected: Vec<_> = actual
                    .iter()
                    .filter(|row| !expected.contains(row))
                    .collect();
                let missing: Vec<_> = expected
                    .iter()
                    .filter(|row| !actual.contains(row))
                    .collect();
                assert!(
                    unexpected.is_empty() && missing.is_empty(),
                    "unexpected {unexpected:?}; missing {missing:?}"
                );
            }
            assert_eq!(actual, *expected);
        }
        reset();
    }
    #[test]
    fn native_html_script_attribute_and_range_sinks_match_chrome() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("trusted_types_sinks_cases.json")).unwrap();
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("trusted_types_sinks_chrome155.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_independent_html(scope, case["html"].as_str().unwrap());
            let report_only = case["id"] == "sink-report";
            let source = format!(
                "(function(action) {{ return action !== 'sink' || {}; }})",
                report_only
            );
            let callback = v8::Local::<v8::Function>::try_from(eval(scope, &source)).unwrap();
            assert!(install_trusted_types_policy(scope, callback));
            // Binding tests have no host timers or HTML parser API. The independently
            // recorded oracle still contains those rows for the production consumer gate.
            eval(scope, "globalThis.setTimeout=function(callback){callback()};document.addEventListener=function(){};");
            let probe = include_str!("trusted_types_sinks_probe.js").replace("['DOMParser',()=>new DOMParser().parseFromString('<b>parse</b>','text/html').body.innerHTML],", "");
            let probe = probe.trim().trim_end_matches(';');
            eval(scope, &format!("({probe}).then(value=>globalThis.ttResult=JSON.stringify(value),error=>globalThis.ttResult='ERROR '+String(error.stack));"));
            scope.perform_microtask_checkpoint();
            let result = eval(scope, "ttResult").to_rust_string_lossy(scope);
            let actual: serde_json::Value =
                serde_json::from_str(&result).unwrap_or_else(|_| panic!("{} {result}", case["id"]));
            let mut expected = oracle["result"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["id"] == case["id"])
                .unwrap()["result"]
                .clone();
            expected["rows"]
                .as_array_mut()
                .unwrap()
                .retain(|row| row[0] != "DOMParser");
            expected["trace"]
                .as_array_mut()
                .unwrap()
                .retain(|row| row[2][1] != "DOMParser parseFromString");
            assert_eq!(actual["rows"], expected["rows"], "{}", case["id"]);
            assert_eq!(actual["trace"], expected["trace"], "{}", case["id"]);
        }
        reset();
    }
    #[test]
    fn private_rules_and_strings_ignore_guest_collection_prototype_replacements() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
        let result = eval(
            scope,
            r#"(()=>{
          const original=Array.prototype[Symbol.iterator];
          Array.prototype[Symbol.iterator]=function(){throw Error('guest iterator')};
          Object.defineProperty(Array.prototype,'1',{configurable:true,set(){throw Error('guest index')}});
          try{
            const p=trustedTypes.createPolicy('__proto__',{createHTML:s=>s});
            const value=p.createHTML({toString(){return 'exact'}},7);
            if(!trustedTypes.isHTML(value)||String(value)!=='exact')throw Error('conversion');
            return true;
          }finally{Array.prototype[Symbol.iterator]=original;delete Array.prototype[1];}
        })()"#,
        );
        assert!(result.is_true());
        reset();
    }
    #[test]
    fn guest_retained_policy_keeps_its_native_callback_realm_alive_only_until_release() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let (owner, weak) = {
            v8::scope!(let scope, &mut isolate);
            let child = v8::Context::new(scope, Default::default());
            let policy = {
                let scope = &mut v8::ContextScope::new(scope, child);
                install_independent_html(scope, "<!doctype html><body>");
                fn authorize(
                    _: &mut v8::PinScope,
                    _: v8::FunctionCallbackArguments,
                    mut result: v8::ReturnValue,
                ) {
                    result.set_bool(true);
                }
                let callback = v8::Function::new(scope, authorize).unwrap();
                assert!(install_trusted_types_policy(scope, callback));
                eval(scope, "(()=>{const doc=document;return trustedTypes.createPolicy('default',{createHTML:s=>doc===document?s:'wrong'})})()")
            };
            let owner = v8::Context::new(scope, Default::default());
            // A default security token is the child global itself. Sharing it
            // would deliberately root that realm independently of this policy.
            let token = v8::String::new(scope, "trusted-types-test-origin").unwrap();
            child.set_security_token(token.into());
            owner.set_security_token(token.into());
            {
                let scope = &mut v8::ContextScope::new(scope, owner);
                let global = owner.global(scope);
                set_prop(scope, global, "retainedPolicy", policy);
            }
            reset();
            (v8::Global::new(scope, owner), v8::Weak::new(scope, child))
        };
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(
            !weak.is_empty(),
            "a genuine guest policy reference must retain its creation realm"
        );
        {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Local::new(scope, &owner);
            let scope = &mut v8::ContextScope::new(scope, context);
            assert!(eval(
                scope,
                "String(retainedPolicy.createHTML('retained'))==='retained'"
            )
            .is_true());
            eval(scope, "delete globalThis.retainedPolicy");
            scope.perform_microtask_checkpoint();
        }
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(
            weak.is_empty(),
            "installed callbacks must not form host-owned realm roots"
        );
    }

    #[test]
    fn brands_are_cross_realm_and_private_state_has_no_host_realm_roots() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let weak = {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_independent_html(scope, "<!doctype html><body>");
            fn authorize(
                _: &mut v8::PinScope,
                _: v8::FunctionCallbackArguments,
                mut result: v8::ReturnValue,
            ) {
                result.set_bool(true);
            }
            let callback = v8::Function::new(scope, authorize).unwrap();
            assert!(install_trusted_types_policy(scope, callback));
            eval(scope, "(()=>{const doc=document;trustedTypes.createPolicy('default',{createHTML:s=>doc===document?s:'wrong'})})()");
            eval(scope, "globalThis.keep=trustedTypes.createPolicy('x',{createHTML:s=>s}).createHTML('foreign')");
            let foreign = eval(scope, "keep");
            let other = v8::Context::new(scope, Default::default());
            other.set_security_token(context.get_security_token(scope));
            {
                let scope = &mut v8::ContextScope::new(scope, other);
                install_independent_html(scope, "<!doctype html><body>");
                let global = other.global(scope);
                set_prop(scope, global, "foreign", foreign);
                let result = eval(scope, "JSON.stringify([trustedTypes.isHTML(foreign), foreign instanceof TrustedHTML, TrustedHTML.prototype.toString.call(foreign),trustedTypes.isHTML(new Proxy(foreign,{})),typeof __trustedTypesRecord])").to_rust_string_lossy(scope);
                assert_eq!(result, r#"[true,false,"foreign",false,"undefined"]"#);
            }
            reset();
            v8::Weak::new(scope, context)
        };
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(weak.is_empty());
    }
}

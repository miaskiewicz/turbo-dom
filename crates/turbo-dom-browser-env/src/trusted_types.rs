//! Trusted values keep their exact strings and callbacks in guest-GC-owned private edges.
use super::*;

struct RecordKey(v8::Global<v8::Private>);

fn key<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Private>> {
    // The cached key is a V8 primitive handle — reading the slot does not run guest JS,
    // so a pending termination (turbo-dom#401: left by a prior frame's watchdog) must not
    // deny it to host-only paths like trusted-types authorization install, which likewise
    // only mutate V8 private fields. Fresh-key allocation goes through v8::String::new,
    // which can fail while terminating; that path is skipped in that case as before.
    if let Some(key) = scope.get_slot::<std::rc::Rc<RecordKey>>().cloned() {
        return Some(v8::Local::new(scope, &key.0));
    }
    if scope.is_execution_terminating() {
        return None;
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
/// The Trusted Types sink mapping (`trusted_types_mapping.json`), parsed once.
static MAPPING: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();

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
/// Review R2 V8 M6: cancel/restore the isolate's execution-terminating flag
/// as a scope-guard. The restore runs in [`Drop`], so it is panic-safe and
/// cannot be bypassed by a future early return / `?` inside the guarded
/// block. A no-op when the scope was not terminating at construction.
struct TerminationRestore<'a, 'p, 's> {
    scope: &'a mut v8::PinScope<'p, 's>,
    was_terminating: bool,
}
impl<'a, 'p, 's> TerminationRestore<'a, 'p, 's> {
    fn cancel(scope: &'a mut v8::PinScope<'p, 's>) -> Self {
        let was_terminating = scope.is_execution_terminating();
        if was_terminating {
            scope.cancel_terminate_execution();
        }
        TerminationRestore {
            scope,
            was_terminating,
        }
    }
    fn scope(&mut self) -> &mut v8::PinScope<'p, 's> {
        self.scope
    }
}
impl Drop for TerminationRestore<'_, '_, '_> {
    fn drop(&mut self) {
        if self.was_terminating {
            self.scope.terminate_execution();
        }
    }
}

pub(super) fn install_policy(
    scope: &mut v8::PinScope,
    callback: v8::Local<v8::Function>,
) -> Option<()> {
    // turbo-dom#401: a watchdog termination left pending by a prior frame's script
    // can make V8's private-field reads and allocations return None here, even though
    // this path only mutates native state and never runs guest JS. The embedder
    // (turbo-surf) sees the resulting `false` as "authorization initialization
    // interrupted" and aborts the frame bootstrap. The install is race-safe to
    // retry, so clear the pending termination for the duration of the write and
    // restore it before returning — the embedder's next guest-JS call still aborts.
    //
    // Review R2 V8 M6: the cancel/restore pair is a scope-guard
    // (`TerminationRestore`). Its `Drop` runs the restore even on an early
    // `None` / `?` return or a panic from a nested call, closing a class of
    // future failures where a new branch inside the guarded block would
    // forget to restore manually.
    let mut guard = TerminationRestore::cancel(scope);
    let scope = guard.scope();
    // Review R1 M3: the realm global and the Trusted Types `factory` /
    // `state` records are plain `v8::Object`s installed by `install_global`,
    // which is called once from the engine side with no guest JS running —
    // nothing between installation and here can swap them for a Proxy or
    // stamp a `v8::PrivateName` interceptor. The `get_private` / `set` calls
    // below therefore hit native slots only, never guest JS. Review R2 V8
    // H1: the state object's prototype is null, so `state.set('authorize',
    // callback)` cannot walk into an `Object.prototype` setter either.
    let global = scope.get_current_context().global(scope);
    debug_assert!(
        !global.is_proxy(),
        "trusted_types::install_policy: realm global is a Proxy; a user-controlled \
         private-name interceptor could observe the cancel/restore window"
    );
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

#[allow(clippy::too_many_arguments)]
fn attribute_kind(
    mapping: &serde_json::Value,
    tag: &str,
    ns: &str,
    attribute: &str,
    local: &str,
    namespace: Option<&str>,
    html: bool,
    svg: bool,
) -> Option<&'static str> {
    if attribute_is_script(mapping, attribute, ns, namespace, html, svg) {
        return Some("TrustedScript");
    }
    if attribute_is_html_scripturl(tag, attribute, namespace, html) {
        return Some("TrustedScriptURL");
    }
    if html && namespace.is_none() && tag == "iframe" && attribute == "srcdoc" {
        return Some("TrustedHTML");
    }
    if attribute_is_svg_scripturl(tag, local, namespace, svg) {
        return Some("TrustedScriptURL");
    }
    None
}

fn attribute_is_script(
    mapping: &serde_json::Value,
    attribute: &str,
    ns: &str,
    namespace: Option<&str>,
    html: bool,
    svg: bool,
) -> bool {
    namespace.is_none()
        && (html || svg || ns == "http://www.w3.org/1998/Math/MathML")
        && mapping["*"]["attributes"].get(attribute).is_some()
}

fn attribute_is_html_scripturl(
    tag: &str,
    attribute: &str,
    namespace: Option<&str>,
    html: bool,
) -> bool {
    html && namespace.is_none()
        && ((tag == "script" || tag == "embed") && attribute == "src"
            || tag == "object" && matches!(attribute, "data" | "codebase"))
}

fn attribute_is_svg_scripturl(tag: &str, local: &str, namespace: Option<&str>, svg: bool) -> bool {
    svg && tag == "script"
        && local == "href"
        && (namespace.is_none() || namespace == Some("http://www.w3.org/1999/xlink"))
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
    let mapping = MAPPING.get_or_init(|| {
        serde_json::from_str(include_str!("trusted_types_mapping.json"))
            .expect("recorded trusted sinks")
    });
    let Some(kind) = attribute_kind(mapping, &tag, &ns, attribute, local, namespace, html, svg)
    else {
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
        let isolate = &mut crate::browser_env::tests::new_isolate();
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
        let isolate = &mut crate::browser_env::tests::new_isolate();
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
        let isolate = &mut crate::browser_env::tests::new_isolate();
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
        let isolate = &mut crate::browser_env::tests::new_isolate();
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
    /// Namespaces and property names spelled like Object.prototype members are unknown names,
    /// not table hits (turbo-dom#389 review). Expected rows: the frozen Chrome 156 headless shell
    /// running the same expression over a file:// page.
    #[test]
    fn object_prototype_names_are_unknown_namespaces_and_properties_like_chrome() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
        let result = eval(
            scope,
            r#"(()=>{
          const t=trustedTypes,r=[];
          for(const ns of ['constructor','__proto__','toString','hasOwnProperty'])r.push(['attr',ns,t.getAttributeType('div','onclick',ns),t.getAttributeType('script','src',ns),t.getPropertyType('div','innerHTML',ns),t.getPropertyType('script','text',ns)]);
          for(const p of ['constructor','__proto__','toString','valueOf','hasOwnProperty'])r.push(['prop',p,t.getPropertyType('script',p),t.getPropertyType('div',p),t.getAttributeType('script',p)]);
          return JSON.stringify(r);
        })()"#,
        );
        let actual: serde_json::Value =
            serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
        let chrome = serde_json::json!([
            ["attr", "constructor", null, null, "TrustedHTML", null],
            ["attr", "__proto__", null, null, "TrustedHTML", null],
            ["attr", "toString", null, null, "TrustedHTML", null],
            ["attr", "hasOwnProperty", null, null, "TrustedHTML", null],
            ["prop", "constructor", null, null, null],
            ["prop", "__proto__", null, null, null],
            ["prop", "toString", null, null, null],
            ["prop", "valueOf", null, null, null],
            ["prop", "hasOwnProperty", null, null, null]
        ]);
        assert_eq!(actual, chrome);
        reset();
    }
    #[test]
    fn private_rules_and_strings_ignore_guest_collection_prototype_replacements() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
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
        let mut isolate = crate::browser_env::tests::new_isolate();
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
                let callback = v8::Function::new(scope, ext!(FunctionCallback, authorize)).unwrap();
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
        let mut isolate = crate::browser_env::tests::new_isolate();
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
            let callback = v8::Function::new(scope, ext!(FunctionCallback, authorize)).unwrap();
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

    /// Review R2 V8 H1: the `state` object the Trusted Types factory records
    /// was a plain object literal with `Object.prototype` as its prototype.
    /// A hostile prelude that stamps `Object.defineProperty(Object.prototype,
    /// 'authorize', { set(fn) {...} })` would see that setter fire INSIDE the
    /// `install_policy` cancel/restore-terminate window, running guest JS
    /// while the embedder's watchdog termination is cleared. The fix is to
    /// use a null-prototype state; this test asserts the first
    /// `install_policy` write does NOT fire a prototype-chain setter — the
    /// setter's side effect would prove the prototype chain was walked.
    #[test]
    fn install_policy_does_not_walk_object_prototype_setter_for_authorize() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body>");
        // A hostile prelude stamps an `authorize` setter on Object.prototype
        // that, if it fires, writes `globalThis.__pollution = true`. The
        // factory's internal state object must not inherit from
        // Object.prototype — otherwise `state.set('authorize', fn)` walks the
        // prototype chain and invokes this setter inside the Rust-side
        // cancel/restore-terminate window around install_policy.
        eval(
            scope,
            "globalThis.__pollution = false; \
             Object.defineProperty(Object.prototype, 'authorize', { \
               configurable: true, \
               set(_value) { globalThis.__pollution = true; } \
             });",
        );
        fn authorize(
            _: &mut v8::PinScope,
            _: v8::FunctionCallbackArguments,
            mut result: v8::ReturnValue,
        ) {
            result.set_bool(true);
        }
        let callback = v8::Function::new(scope, ext!(FunctionCallback, authorize)).unwrap();
        assert!(install_trusted_types_policy(scope, callback));
        // The prelude's setter must NOT have fired.
        assert!(
            eval(scope, "globalThis.__pollution").is_false(),
            "install_policy walked Object.prototype and fired the setter"
        );
        // Clean the pollution before reset (reset re-uses the isolate).
        eval(scope, "delete Object.prototype.authorize;");
        reset();
    }

    /// turbo-dom#401: a prior cross-origin iframe's script left a watchdog termination
    /// pending on the isolate. The host's host-authorization install is a private-field
    /// mutation that does not run guest JS, so it must still succeed; otherwise the
    /// consumer (turbo-surf#248) throws "native Trusted Types authorization
    /// initialization interrupted" and the whole frame bootstrap aborts.
    #[test]
    fn install_policy_succeeds_while_a_prior_frames_termination_is_pending() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
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
        let callback = v8::Function::new(scope, ext!(FunctionCallback, authorize)).unwrap();
        // A pending termination stays observable only while a TryCatch is on the stack.
        v8::tc_scope!(let scope, scope);
        // A watchdog terminated a prior frame's script; the request is pending on this
        // isolate. install_trusted_types_policy must not treat the pending termination
        // as a reason to abort — it mutates a V8 private field, it does not run guest JS.
        let code = v8::String::new(scope, "0").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        scope.terminate_execution();
        assert!(script.run(scope).is_none());
        assert!(scope.is_execution_terminating());
        assert!(install_trusted_types_policy(scope, callback));
        // Review R3 M1: assert the TerminationRestore guard actually
        // re-terminated. Without this check, a mutant that removes
        // `TerminationRestore::Drop`'s body (no re-terminate) would still
        // pass — the manual `cancel_terminate_execution` below would mask
        // the restore. Observable proof: a script run after install must
        // STILL return None (termination pending) before the manual cancel,
        // because the guard's Drop re-set the isolate-level flag.
        let probe = v8::String::new(scope, "1").unwrap();
        let probe = v8::Script::compile(scope, probe, None).unwrap();
        assert!(
            probe.run(scope).is_none(),
            "install_trusted_types_policy must restore the pre-install \
             pending termination (TerminationRestore::Drop) — a script \
             run after install should still see the termination flag set, \
             but run() returned a value: mutant alert, Drop removed"
        );
        // And when the embedder clears the watchdog, the installed callback is live:
        // the next createPolicy consults it (returns true → allowed).
        scope.cancel_terminate_execution();
        assert!(
            eval(
                scope,
                "trustedTypes.createPolicy('x',{createHTML:s=>s}).createHTML('t') instanceof TrustedHTML"
            )
            .is_true()
        );
        reset();
    }
    /// Unsafe HTML union conversion preserves genuine TrustedHTML without running
    /// a guest-replaced toString before the sink's private brand conversion.
    #[test]
    fn unsafe_html_sinks_preserve_genuine_trusted_values_without_guest_coercion() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body></body>");
        let value = eval(scope,
            "(() => { const policy = trustedTypes.createPolicy('sanitizer-brand', { createHTML: s => s }); \
             const html = policy.createHTML('<b>ok</b>'); \
             Object.defineProperty(html, 'toString', { value() { throw Error('guest coercion'); } }); \
             const host = document.createElement('div'); host.setHTMLUnsafe(html); \
             const parsed = Document.parseHTMLUnsafe(html); \
             return JSON.stringify([host.innerHTML, parsed.body.innerHTML]); })()");
        assert_eq!(
            value.to_rust_string_lossy(scope),
            r#"["<b>ok</b>","<b>ok</b>"]"#
        );
        reset();
    }
    /// TrustedHTML attribute enforcement belongs only to an HTML iframe's
    /// unnamespaced srcdoc; nearby non-sinks must remain ordinary string inputs.
    #[test]
    fn trusted_html_attribute_enforcement_does_not_block_neighboring_non_sinks() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body></body>");
        let deny =
            v8::Local::<v8::Function>::try_from(eval(scope, "(action => action !== 'sink')"))
                .unwrap();
        assert!(install_trusted_types_policy(scope, deny));
        let value = eval(scope,
            "(() => { const attempt = (node, ns, name) => { try { \
             if (ns === null) node.setAttribute(name, 'text'); else node.setAttributeNS(ns, name, 'text'); \
             return 'ok'; } catch (error) { return error.name; } }; \
             return JSON.stringify([attempt(document.createElement('div'), null, 'title'), \
             attempt(document.createElementNS('urn:foreign', 'iframe'), null, 'srcdoc'), \
             attempt(document.createElement('iframe'), 'urn:attribute', 'srcdoc'), \
             attempt(document.createElement('iframe'), null, 'title'), \
             attempt(document.createElement('iframe'), null, 'srcdoc')]); })()");
        assert_eq!(
            value.to_rust_string_lossy(scope),
            r#"["ok","ok","ok","ok","TypeError"]"#
        );
        reset();
    }
    /// Host CSP authorization observes the exact native sink name, not merely
    /// whether a nearby attribute happens to contain script-looking text.
    #[test]
    fn native_attribute_sink_authorization_covers_namespaces_and_owner_names() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body></body>");
        eval(scope, "globalThis.sinkCalls = []");
        let allow = v8::Local::<v8::Function>::try_from(eval(scope,
            "((action, kind, sink) => { if (action === 'sink') sinkCalls.push([kind, sink]); return true; })")).unwrap();
        assert!(install_trusted_types_policy(scope, allow));
        let value = eval(scope,
            "(() => { const H = 'http://www.w3.org/1999/xhtml', S = 'http://www.w3.org/2000/svg', \
             M = 'http://www.w3.org/1998/Math/MathML', X = 'http://www.w3.org/1999/xlink'; \
             const cases = [[H,'script',null,'src'],[H,'embed',null,'src'],[H,'object',null,'data'], \
             [H,'object',null,'codebase'],[H,'iframe',null,'srcdoc'],[H,'div',null,'onclick'], \
             [S,'script',null,'href'],[S,'script',X,'xlink:href'],[S,'script','urn:attr','q:href'], \
             [S,'script',null,'src'],[S,'image',null,'href'],[S,'a',null,'href'],[S,'circle',null,'onclick'], \
             [M,'mrow',null,'onclick'],['urn:foreign','div',null,'onclick'],[H,'script','urn:attr','q:onclick'], \
             [H,'embed','urn:attr','q:src'],[H,'object','urn:attr','q:data'],[H,'iframe','urn:attr','q:srcdoc'], \
             [H,'div',null,'title'],[H,'div',null,'src'],[H,'object',null,'src'],[H,'script',null,'data'], \
             [H,'embed',null,'data'],[H,'iframe',null,'src'],['urn:foreign','iframe',null,'srcdoc'], \
             [H,'div',null,'srcdoc'],[S,'script',X,'xlink:src'],[M,'iframe',null,'srcdoc']]; \
             return JSON.stringify(cases.map(([ns,tag,attrNS,name]) => { sinkCalls.length = 0; \
             const element = document.createElementNS(ns,tag); \
             if (attrNS === null) element.setAttribute(name,'value'); else element.setAttributeNS(attrNS,name,'value'); \
             return sinkCalls.slice(); })); })()");
        let actual: serde_json::Value =
            serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap();
        let script = |owner: &str| serde_json::json!([["TrustedScript", owner]]);
        let url = |owner: &str| serde_json::json!([["TrustedScriptURL", owner]]);
        let none = serde_json::json!([]);
        assert_eq!(
            actual,
            serde_json::json!([
                url("HTMLScriptElement src"),
                url("HTMLEmbedElement src"),
                url("HTMLObjectElement data"),
                url("HTMLObjectElement codebase"),
                [["TrustedHTML", "HTMLIFrameElement srcdoc"]],
                script("Element onclick"),
                url("Element href"),
                url("Element href"),
                none,
                none,
                none,
                none,
                script("Element onclick"),
                script("Element onclick"),
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                none
            ])
        );
        reset();
    }

    #[test]
    fn ordinary_text_content_avoids_trusted_script_enforcement_and_converts_null() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body></body>");
        let deny =
            v8::Local::<v8::Function>::try_from(eval(scope, "(action => action !== 'sink')"))
                .unwrap();
        assert!(install_trusted_types_policy(scope, deny));
        let value = eval(
            scope,
            "(() => { const div = document.createElement('div'); div.textContent = 'ordinary'; \
             const before = div.textContent; div.textContent = null; \
             const script = document.createElement('script'); let error; \
             try { script.textContent = 'run()'; } catch (caught) { error = caught.name; } \
             return JSON.stringify([before, div.textContent, error]); })()",
        );
        assert_eq!(
            value.to_rust_string_lossy(scope),
            r#"["ordinary","","TypeError"]"#
        );
        reset();
    }
    #[test]
    fn unsafe_html_union_converts_ordinary_objects_before_reading_options() {
        super::super::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><body></body>");
        let value = eval(scope,
            "(() => { const probe = element => { const log = []; \
             const html = { toString() { log.push('html'); return '<i>raw</i>'; } }; \
             const options = { get sanitizer() { log.push('options'); return undefined; } }; \
             if (!element) return [Document.parseHTMLUnsafe(html, options).body.innerHTML, log]; \
             const host = document.createElement('div'); host.setHTMLUnsafe(html, options); return [host.innerHTML, log]; \
             }; return JSON.stringify([probe(true), probe(false)]); })()");
        assert_eq!(
            value.to_rust_string_lossy(scope),
            r#"[["<i>raw</i>",["html","options"]],["<i>raw</i>",["html","options"]]]"#
        );
        reset();
    }
}

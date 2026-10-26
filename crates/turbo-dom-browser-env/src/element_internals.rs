//! Native CustomStateSet storage and closed shadow ownership for ElementInternals.
use super::*;
struct Updates(v8::Weak<v8::Function>);
pub(super) fn attribute_changed(scope: &mut v8::PinScope, host: v8::Local<v8::Object>, name: &str) {
    let Some(context) = host.get_creation_context(scope) else {
        return;
    };
    let Some(updates) = context.get_slot::<Updates>() else {
        return;
    };
    let Some(callback) = updates.0.to_local(scope) else {
        return;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let receiver = v8::undefined(scope);
    let name = v8::String::new(scope, name).unwrap();
    callback.call(scope, receiver.into(), &[host.into(), name.into()]);
}
fn dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.get(0).to_rust_string_lossy(scope) == "value-snapshot" {
        form_controls_native(scope, args, rv);
        return;
    }
    if args.get(0).to_rust_string_lossy(scope) == "private" {
        let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
            return;
        };
        let family = args.get(2).to_rust_string_lossy(scope);
        let name = v8::String::new(scope, &format!("turbo.element-internals.{family}")).unwrap();
        let key = v8::Private::for_api(scope, Some(name));
        if !args.get(3).is_undefined() {
            object.set_private(scope, key, args.get(3));
        }
        if let Some(value) = object.get_private(scope, key) {
            rv.set(value);
        }
        return;
    }
    let Ok(host) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, host);
    let Some(handle) = handle_of(scope, host) else {
        return;
    };
    let operation = args.get(0).to_rust_string_lossy(scope);
    if operation == "shadow" {
        if let Some(root) = with_tree(|tree| tree.shadow_root(handle)).flatten() {
            rv.set(wrap(scope, root).into());
        } else {
            rv.set(v8::null(scope).into());
        }
        return;
    }
    if operation == "clear" {
        with_tree_mut(|tree| tree.clear_custom_states(handle));
        return;
    }
    let Some(key) = args.get(2).to_string(scope) else {
        return;
    };
    let key = string_code_units(scope, key);
    match operation.as_str() {
        "add" => {
            with_tree_mut(|tree| tree.set_custom_state_utf16(handle, &key, true));
        }
        "delete" => {
            let present =
                with_tree(|tree| tree.has_custom_state_utf16(handle, &key)).unwrap_or(false);
            with_tree_mut(|tree| tree.set_custom_state_utf16(handle, &key, false));
            rv.set(v8::Boolean::new(scope, present).into());
        }
        _ => {}
    }
}
pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__internalsNative", dispatch);
    run_js(scope, include_str!("element_internals.js"));
    let key = v8::String::new(scope, "__captureInternalsUpdates").unwrap();
    if let Some(callback) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(callback) = dom_ownership::helper_function(scope, callback) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(Updates(callback)));
        }
    }
    global.delete(scope, key.into());
}

#[cfg(test)]
mod tests {
    use super::*;
    // The standalone binding has no networking FormData backend. Model its trusted
    // private-brand/transaction boundary here; production tests use the real backend.
    fn form_data_adapter(scope: &mut v8::PinScope) {
        let global = scope.get_current_context().global(scope);
        bind_method(scope, global, "__fdBegin", |scope, args, mut rv| {
            if let Some(entries) =
                begin_form_data_constructor_entries(scope, args.get(0), args.get(1))
            {
                rv.set(entries.into());
            }
        });
        bind_method(scope, global, "__fdFinish", |scope, args, _rv| {
            finish_form_data_constructor(scope, args.get(0))
        });
        bind_method(scope, global, "__fdEvent", |scope, args, _rv| {
            if let Ok(form) = v8::Local::<v8::Object>::try_from(args.get(0)) {
                dispatch_formdata_event(scope, form, args.get(1));
            }
        });
        let source=v8::String::new(scope,r#"(()=>{const brands=new WeakSet();globalThis.FormData=class FormData{constructor(form,submitter){brands.add(this);if(form!==undefined){__fdBegin(form,submitter);try{const alias=Object.create(FormData.prototype);brands.add(alias);__fdEvent(form,alias);}finally{__fdFinish(form);}}}};return value=>brands.has(value)?Object.freeze({kind:'formdata',entries:Object.freeze([])}):null;})()"#).unwrap();
        let reader = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert!(install_form_value_reader(
            scope,
            v8::Local::<v8::Function>::try_from(reader).unwrap()
        ));
    }
    fn assert_control(cases: &[u8], probe: &str, control: &str, adapter: bool) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let cases_value: serde_json::Value = serde_json::from_slice(cases).unwrap();
        install_independent_html(scope, cases_value[0]["html"].as_str().unwrap());
        if let Some(base) = cases_value[0]["documentBase"].as_str() {
            let id = scope
                .get_current_context()
                .get_slot::<DomContextId>()
                .unwrap()
                .0;
            let _dom = DomScope::enter(id);
            event_handlers::install(scope, None, base);
        }
        if adapter {
            form_data_adapter(scope);
        }
        v8::tc_scope!(let scope,scope);
        let source = v8::String::new(scope, probe).unwrap();
        let result = v8::Script::compile(scope, source, None).unwrap().run(scope);
        if result.is_none() {
            let error = scope.exception().unwrap().to_rust_string_lossy(scope);
            panic!("{error}");
        }
        let actual = v8::json::stringify(scope, result.unwrap())
            .unwrap()
            .to_rust_string_lossy(scope);
        let actual: serde_json::Value = serde_json::from_str(&actual).unwrap();
        let control: serde_json::Value = serde_json::from_str(control).unwrap();
        use sha2::{Digest, Sha256};
        assert_eq!(control["browser"], "Chrome/155.0.8059.80");
        assert_eq!(control["v8"], "15.5.35.23");
        assert_eq!(control["errors"], serde_json::json!([]));
        assert_eq!(
            control["probeSha256"],
            format!("{:x}", Sha256::digest(probe.as_bytes()))
        );
        assert_eq!(
            control["casesSha256"],
            format!("{:x}", Sha256::digest(cases))
        );
        assert_eq!(
            control["result"].as_array().unwrap().len(),
            cases_value.as_array().unwrap().len()
        );
        assert_eq!(actual, control["result"][0]["result"]);
        reset();
    }
    #[test]
    fn native_form_collection_records_retain_only_their_genuine_owner_context() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let (keep, context, document, form, controls, list) = {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_independent_html(scope, "<!doctype html><body>");
            let source=v8::String::new(scope,"(()=>{const form=document.createElement('form');document.body.append(form);form.innerHTML='<input name=group><input name=group>';const controls=form.elements,list=controls.namedItem('group');return [document,form,controls,list]})()").unwrap();
            let values = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap()
                .to_object(scope)
                .unwrap();
            let document = values
                .get_index(scope, 0)
                .unwrap()
                .to_object(scope)
                .unwrap();
            let form = values
                .get_index(scope, 1)
                .unwrap()
                .to_object(scope)
                .unwrap();
            let controls = values
                .get_index(scope, 2)
                .unwrap()
                .to_object(scope)
                .unwrap();
            let list = values
                .get_index(scope, 3)
                .unwrap()
                .to_object(scope)
                .unwrap();
            (
                v8::Global::new(scope, list),
                v8::Weak::new(scope, context),
                v8::Weak::new(scope, document),
                v8::Weak::new(scope, form),
                v8::Weak::new(scope, controls),
                v8::Weak::new(scope, list),
            )
        };
        isolate.clear_kept_objects();
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(
            !context.is_empty()
                && !document.is_empty()
                && !form.is_empty()
                && !controls.is_empty()
                && !list.is_empty()
        );
        drop(keep);
        isolate.clear_kept_objects();
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(
            context.is_empty()
                && document.is_empty()
                && form.is_empty()
                && controls.is_empty()
                && list.is_empty()
        );
        clear_interface_records(&mut isolate);
        reset();
    }
    #[test]
    fn native_form_collection_base_accessors_borrow_across_genuine_contexts() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let child = v8::Context::new(scope, Default::default());
        let collection = {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<!doctype html><body>");
            let source=v8::String::new(scope,"(()=>{const form=document.createElement('form');document.body.append(form);form.innerHTML='<input name=group><input name=group>';return form.elements;})()").unwrap();
            let value = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap();
            v8::Global::new(scope, value)
        };
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        install_independent_html(scope, "<!doctype html><body>");
        let key = v8::String::new(scope, "foreignControls").unwrap();
        let global = parent.global(scope);
        let value = v8::Local::new(scope, &collection);
        global.set(scope, key.into(), value);
        let source=v8::String::new(scope,"(()=>{const list=HTMLFormControlsCollection.prototype.namedItem.call(foreignControls,'group');return Object.getOwnPropertyDescriptor(HTMLCollection.prototype,'length').get.call(foreignControls)===2&&Object.getOwnPropertyDescriptor(NodeList.prototype,'length').get.call(list)===2&&RadioNodeList.prototype.hasOwnProperty('value')&&Object.getOwnPropertyDescriptor(RadioNodeList.prototype,'value').get.call(list)==='';})()").unwrap();
        assert!(v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .is_true());
        reset();
    }
    #[test]
    fn native_dirty_checkedness_survives_gc_and_requery() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        let context = {
            v8::scope!(let scope,isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_independent_html(
                scope,
                "<!doctype html><body><input id='dirty-native' type='radio' checked>",
            );
            let source = v8::String::new(
                scope,
                "document.querySelector('#dirty-native').checked=false;true",
            )
            .unwrap();
            assert!(v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap()
                .is_true());
            v8::Global::new(scope, context)
        };
        isolate.clear_kept_objects();
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        {
            v8::scope!(let scope,isolate);
            let context = v8::Local::new(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            let source=v8::String::new(scope,"(()=>{const node=document.querySelector('#dirty-native');const before=node.checked;node.defaultChecked=false;node.defaultChecked=true;return before===false&&node.checked===false&&node.defaultChecked===true;})()").unwrap();
            assert!(v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap()
                .is_true());
        }
        drop(context);
        reset();
    }
    #[test]
    fn retired_radio_gc_control_records_independent_repeated_real_task_policy() {
        use sha2::{Digest, Sha256};
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("radio_dirty_adoption_gc_chrome155.json")).unwrap();
        let review: serde_json::Value = serde_json::from_str(include_str!(
            "radio_dirty_adoption_gc_review_chrome155.json"
        ))
        .unwrap();
        assert_eq!(
            fixture["probeSha256"],
            format!(
                "{:x}",
                Sha256::digest(include_bytes!("radio_dirty_adoption_gc_probe.js"))
            )
        );
        assert_eq!(
            fixture["casesSha256"],
            format!(
                "{:x}",
                Sha256::digest(include_bytes!("radio_dirty_cases.json"))
            )
        );
        assert_eq!(
            fixture["gcPolicy"],
            serde_json::json!({"rounds":3,"settleMs":50,"taskExpression":"0"})
        );
        for repeated in review["correctedSettledRepeats"].as_array().unwrap() {
            for key in [
                "browser",
                "v8",
                "casesSha256",
                "probeSha256",
                "collectorSha256",
                "errors",
                "gcPolicy",
                "result",
            ] {
                assert_eq!(repeated[key], fixture[key]);
            }
        }
        assert_eq!(
            review["correctedSettledRepeats"].as_array().unwrap().len(),
            3
        );
        assert_eq!(
            review["nativeProof"]["first"],
            fixture["result"][0]["result"]["first"]
        );
        assert_eq!(
            review["nativeProof"]["afterRelease"],
            fixture["result"][0]["result"]["afterRelease"]
        );
        assert_eq!(review["nativeProof"]["probeSha256"], fixture["probeSha256"]);
        assert_eq!(review["nativeProof"]["retiredFrameElementNull"], true);
    }
    #[test]
    fn native_radio_owner_adversarial_review_matches_independent_chrome() {
        assert_control(
            include_bytes!("radio_dirty_cases.json"),
            include_str!("radio_owner_review_probe.js"),
            include_str!("radio_owner_review_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_radio_group_moves_match_independent_chrome() {
        assert_control(
            include_bytes!("radio_dirty_cases.json"),
            include_str!("radio_group_moves_probe.js"),
            include_str!("radio_group_moves_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_radio_dirty_matches_independent_chrome() {
        assert_control(
            include_bytes!("radio_dirty_cases.json"),
            include_str!("radio_dirty_probe.js"),
            include_str!("radio_dirty_chrome155.json"),
            false,
        );
        assert_control(
            include_bytes!("radio_dirty_cases.json"),
            include_str!("radio_dirty_transition_probe.js"),
            include_str!("radio_dirty_transition_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_form_collections_match_independent_chrome() {
        assert_control(
            include_bytes!("form_collections_cases.json"),
            include_str!("form_collections_probe.js"),
            include_str!("form_collections_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_submitter_reflection_matches_independent_chrome() {
        assert_control(
            include_bytes!("submitter_reflection_cases.json"),
            include_str!("submitter_reflection_probe.js"),
            include_str!("submitter_reflection_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_form_reflection_matches_independent_chrome() {
        assert_control(
            include_bytes!("form_reflection_cases.json"),
            include_str!("form_reflection_probe.js"),
            include_str!("form_reflection_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_form_connectivity_matches_independent_chrome() {
        assert_control(
            include_bytes!("form_connectivity_cases.json"),
            include_str!("form_connectivity_probe.js"),
            include_str!("form_connectivity_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_internals_core_matches_independent_chrome() {
        assert_control(
            include_bytes!("element_internals_core_cases.json"),
            include_str!("element_internals_core_probe.js"),
            include_str!("element_internals_core_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_internals_associated_matches_independent_chrome() {
        assert_control(
            include_bytes!("element_internals_associated_cases.json"),
            include_str!("element_internals_associated_probe.js"),
            include_str!("element_internals_associated_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_internals_state_matches_independent_chrome() {
        assert_control(
            include_bytes!("element_internals_state_cases.json"),
            include_str!("element_internals_state_probe.js"),
            include_str!("element_internals_state_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_internals_definition_order_matches_chrome() {
        assert_control(
            include_bytes!("element_internals_definition_cases.json"),
            include_str!("element_internals_definition_probe.js"),
            include_str!("element_internals_definition_chrome155.json"),
            false,
        );
    }
    #[test]
    fn native_formdata_event_matches_independent_chrome() {
        assert_control(
            include_bytes!("formdata_event_cases.json"),
            include_str!("formdata_event_probe.js"),
            include_str!("formdata_event_chrome155.json"),
            true,
        );
    }
    #[test]
    fn native_formdata_guards_matches_independent_chrome() {
        assert_control(
            include_bytes!("formdata_guards_cases.json"),
            include_str!("formdata_guards_probe.js"),
            include_str!("formdata_guards_chrome155.json"),
            true,
        );
    }
    #[test]
    fn native_internals_setlike_matches_independent_chrome() {
        assert_control(
            include_bytes!("element_internals_setlike_cases.json"),
            include_str!("element_internals_setlike_probe.js"),
            include_str!("element_internals_setlike_chrome155.json"),
            false,
        );
    }
}

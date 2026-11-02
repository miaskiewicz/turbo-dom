use super::*;
#[test]
fn native_named_manual_flattened_slots_and_notification_order_match_chrome() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><head></head><body></body>");
    let probe = include_str!("shadow_slots_probe.js")
        .trim()
        .trim_end_matches(';');
    let source=format!("(async()=>{{try{{globalThis.slotResult=JSON.stringify(await({probe}));}}catch(error){{globalThis.slotError=error.stack;}}}})()");
    let code = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope,scope);
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    scope.perform_microtask_checkpoint();
    let code = v8::String::new(
        scope,
        "globalThis.slotResult || globalThis.slotError || 'unsettled'",
    )
    .unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value =
        serde_json::from_str(&value).unwrap_or_else(|error| panic!("{error}: {value}"));
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("shadow_slots_chrome155.json")).unwrap();
    assert_eq!(actual, expected["result"]);
    reset();
}

#[test]
fn replacing_manual_assignment_releases_the_old_slot_edge_after_notification() {
    super::tests::init_v8();
    let mut isolate = v8::Isolate::new(Default::default());
    let (context, old_slot) = {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body></body>");
        let code=v8::String::new(scope,r#"(()=>{const host=document.createElement('div');document.body.append(host);const child=document.createElement('b');host.append(child);const root=host.attachShadow({mode:'open',slotAssignment:'manual'});const slot=document.createElement('slot');root.append(slot);slot.assign(child);slot.assign();slot.remove();globalThis.retainedChild=child;return slot;})()"#).unwrap();
        let slot = v8::Local::<v8::Object>::try_from(
            v8::Script::compile(scope, code, None)
                .unwrap()
                .run(scope)
                .unwrap(),
        )
        .unwrap();
        let weak = v8::Weak::new(scope, slot);
        scope.perform_microtask_checkpoint();
        (v8::Global::new(scope, context), weak)
    };
    for _ in 0..3 {
        isolate.low_memory_notification();
    }
    assert!(
        old_slot.is_empty(),
        "the retained child must not retain a cleared manual slot"
    );
    {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Local::new(scope, &context);
        let scope = &mut v8::ContextScope::new(scope, context);
        let code = v8::String::new(
            scope,
            "retainedChild.assignedSlot===null && retainedChild.parentNode.isConnected",
        )
        .unwrap();
        assert!(v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .is_true());
        reset();
    }
}

#[test]
fn deep_shadow_fallback_distribution_uses_an_iterative_native_walk() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<body></body>");
    let code=v8::String::new(scope,"globalThis.host=document.createElement('div');globalThis.root=host.attachShadow({mode:'open'});globalThis.slot=document.createElement('slot');root.append(slot);").unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let global = context.global(scope);
    let name = v8::String::new(scope, "slot").unwrap();
    let slot = v8::Local::<v8::Object>::try_from(global.get(scope, name.into()).unwrap()).unwrap();
    let handle = handle_of(scope, slot).unwrap();
    with_tree_mut(|tree| {
        let mut parent = handle;
        for _ in 0..4096 {
            let child = tree.create_element("slot");
            tree.append_child(parent, child);
            parent = child;
        }
        let text = tree.create_text_node("deep");
        tree.append_child(parent, text);
        tree.take_mutation_events();
    });
    let code=v8::String::new(scope,"slot.assignedNodes({flatten:true}).length===1 && slot.assignedNodes({flatten:true})[0].data==='deep'").unwrap();
    assert!(v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .is_true());
    scope.perform_microtask_checkpoint();
    reset();
}

#[test]
fn host_composed_projection_uses_private_manual_edges_and_closed_shadow_roots() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<body></body>");
    fn children<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        args: v8::FunctionCallbackArguments<'s>,
        mut rv: v8::ReturnValue,
    ) {
        let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
            return;
        };
        if let Some(children) = host_composed_children(scope, node) {
            let values = children.into_iter().map(Into::into).collect::<Vec<_>>();
            rv.set(v8::Array::new_with_elements(scope, &values).into());
        }
    }
    fn parent<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        args: v8::FunctionCallbackArguments<'s>,
        mut rv: v8::ReturnValue,
    ) {
        let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
            return;
        };
        rv.set(
            host_composed_parent(scope, node).map_or_else(|| v8::null(scope).into(), Into::into),
        );
    }
    let global = context.global(scope);
    bind_method(scope, global, "hostChildren", children);
    bind_method(scope, global, "hostParent", parent);
    let code=v8::String::new(scope,r#"(()=>{const host=document.createElement('div'),child=document.createElement('b'),other=document.createElement('i');host.append(child,other);const root=host.attachShadow({mode:'closed',slotAssignment:'manual'}),slot=document.createElement('slot'),fallback=document.createElement('span');root.append(slot);slot.append(fallback);slot.assign(child);slot.assignedNodes=()=>{throw new Error('public assignedNodes');};Object.defineProperty(child,'assignedSlot',{get(){throw new Error('public assignedSlot');}});const before=hostChildren(host)[0]===slot&&hostChildren(slot)[0]===child&&hostParent(child)===slot&&hostParent(slot)===host&&hostParent(root)===host&&hostParent(other)===null&&hostParent(fallback)===null&&host.shadowRoot===null;slot.assign();return before&&hostChildren(slot)[0]===fallback&&hostParent(fallback)===slot&&hostParent(child)===null;})()"#).unwrap();
    assert!(v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .is_true());
    scope.perform_microtask_checkpoint();
    reset();
}

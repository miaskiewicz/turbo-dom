//! Button/input invoker activation: trusted CommandEvent dispatch and popover-target
//! actions with the invoker as the ToggleEvent source.
use super::*;

struct InvokerActivation(v8::Weak<v8::Function>);
struct ActivationTarget(v8::Weak<v8::Function>);

fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let Ok(target) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, target);
    let Some(node) = handle_of(scope, target) else {
        return;
    };
    match args.get(0).to_rust_string_lossy(scope).as_str() {
        "dispatch" => {
            let Ok(event) = v8::Local::<v8::Object>::try_from(args.get(2)) else {
                return;
            };
            dispatch_host_event(scope, event, target);
            let continued = event_record(scope, event)
                .is_some_and(|record| !get_bool_prop(scope, record, "defaultPrevented"));
            rv.set_bool(continued);
        }
        "disabled" => rv.set_bool(disabled_control(node)),
        "click" => {
            let trusted = v8::Local::<v8::Object>::try_from(args.get(2))
                .ok()
                .and_then(|event| event_record(scope, event))
                .is_some_and(|record| get_bool_prop(scope, record, "trusted"));
            synthetic_click(scope, target, node, trusted);
        }
        "popover" => {
            let action = args.get(2).to_rust_string_lossy(scope);
            if let Ok(source) = v8::Local::<v8::Object>::try_from(args.get(3)) {
                html_popover::invoke(scope, node, &action, source);
            }
        }
        _ => {}
    }
}

/// Whether `click()` is a no-op: a disabled button/input/select/textarea (own attribute or a
/// disabled fieldset outside its first legend), or an option/optgroup with its own attribute.
pub(super) fn disabled_control(h: Handle) -> bool {
    with_tree(|tree| {
        if tree.node_type_id(h) != 1 || tree.namespace_id(h) != 0 {
            return false;
        }
        match tree.local_name(h) {
            Some("option" | "optgroup") => tree.has_attribute(h, "disabled"),
            Some("button" | "input" | "select" | "textarea") => {
                tree.has_attribute(h, "disabled") || fieldset_disabled(tree, h)
            }
            _ => false,
        }
    })
    .unwrap_or(false)
}

fn fieldset_disabled(tree: &Tree, h: Handle) -> bool {
    let html = |node: Handle, name: &str| {
        tree.node_type_id(node) == 1
            && tree.namespace_id(node) == 0
            && tree.local_name(node) == Some(name)
    };
    let (mut child, mut parent) = (h, tree.parent(h));
    while let Some(ancestor) = parent.filter(|&node| tree.node_type_id(node) == 1) {
        if html(ancestor, "fieldset") && tree.has_attribute(ancestor, "disabled") {
            let legend = tree
                .children(ancestor)
                .into_iter()
                .find(|&node| html(node, "legend"));
            if legend != Some(child) {
                return true;
            }
        }
        (child, parent) = (ancestor, tree.parent(ancestor));
    }
    false
}

/// The click's activation target: only MouseEvent clicks have one (the guest-invisible
/// brand decides, never isTrusted).
pub(super) fn activation_target(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
    target: Handle,
) -> Option<Handle> {
    let node = wrap(scope, target);
    let context = node.get_creation_context(scope)?;
    let finder = context.get_slot::<ActivationTarget>()?;
    let function = dom_ownership::local(scope, &finder.0);
    let receiver = v8::undefined(scope);
    let found = function.call(scope, receiver.into(), &[event.into(), node.into()])?;
    let found = v8::Local::<v8::Object>::try_from(found).ok()?;
    handle_of(scope, found)
}

/// Run `activation`'s activation behavior after an uncanceled `click` reached `target`.
pub(super) fn activate(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
    activation: Handle,
    target: Handle,
) {
    let node = wrap(scope, activation);
    let target = wrap(scope, target);
    let Some(context) = node.get_creation_context(scope) else {
        return;
    };
    let Some(activation) = context.get_slot::<InvokerActivation>() else {
        return;
    };
    let function = dom_ownership::local(scope, &activation.0);
    let receiver = v8::undefined(scope);
    v8::tc_scope!(let scope, scope);
    function.call(
        scope,
        receiver.into(),
        &[event.into(), node.into(), target.into()],
    );
    if !scope.has_terminated() {
        if let Some(error) = scope.exception() {
            scope.reset();
            custom_elements::report_value(scope, error);
        }
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__invokerNative", dispatch);
    run_js(scope, include_str!("html_invokers.js"));
    for name in ["__captureInvokerActivation", "__captureActivationTarget"] {
        let key = v8::String::new(scope, name).unwrap();
        if let Some(function) = global
            .get(scope, key.into())
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        {
            if let Some(function) = dom_ownership::helper_function(scope, function) {
                let context = scope.get_current_context();
                if name == "__captureInvokerActivation" {
                    context.set_slot(std::rc::Rc::new(InvokerActivation(function)));
                } else {
                    context.set_slot(std::rc::Rc::new(ActivationTarget(function)));
                }
            }
        }
        global.delete(scope, key.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run an async committed probe against the binding, draining a FIFO timer queue the
    /// popover scheduler shares, and return the resolved JSON value.
    fn run_probe(probe: &str) -> serde_json::Value {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
        );
        run_js(
            scope,
            "globalThis.__tasks=[];globalThis.setTimeout=callback=>{__tasks.push(callback);return __tasks.length};",
        );
        capture_popover_scheduler(scope);
        let source = format!(
            "Promise.resolve({}).then(JSON.stringify)",
            probe.trim().trim_end_matches(';')
        );
        let source = v8::String::new(scope, &source).unwrap();
        v8::tc_scope!(let scope, scope);
        let promise = v8::Script::compile(scope, source, None)
            .and_then(|script| script.run(scope))
            .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
            .unwrap_or_else(|| {
                panic!(
                    "probe threw: {:?}",
                    scope
                        .exception()
                        .map(|error| error.to_rust_string_lossy(scope))
                )
            });
        let step = v8::String::new(scope, "__tasks.length&&__tasks.shift()()").unwrap();
        let step = v8::Script::compile(scope, step, None).unwrap();
        for _ in 0..10_000 {
            scope.perform_microtask_checkpoint();
            if promise.state() != v8::PromiseState::Pending {
                break;
            }
            step.run(scope);
        }
        let value = promise.result(scope).to_rust_string_lossy(scope);
        assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
        reset();
        serde_json::from_str(&value).unwrap()
    }

    /// Report every differing case by name, not just the first section.
    fn assert_control(actual: &serde_json::Value, expected: &serde_json::Value) {
        let mut differences = Vec::new();
        for (section, value) in expected.as_object().unwrap() {
            let cases = value.as_object().unwrap();
            for (key, value) in cases {
                if &actual[section][key] != value {
                    differences.push(format!(
                        "{section}.{key}\n  binding: {}\n  chrome:  {value}",
                        actual[section][key]
                    ));
                }
            }
            if actual[section].as_object().map(|object| object.len()) != Some(cases.len()) {
                differences.push(format!("{section}: case set differs"));
            }
        }
        assert!(differences.is_empty(), "{}", differences.join("\n"));
    }

    #[test]
    fn invoker_reflection_command_events_and_activation_match_chrome() {
        let control: serde_json::Value =
            serde_json::from_str(include_str!("html_invokers_chrome156.json")).unwrap();
        let actual = run_probe(include_str!("html_invokers_probe.js"));
        assert_control(&actual, &control["result"]);
    }

    /// Dialog targets receive Chrome's CommandEvent and then run the dialog command steps:
    /// show-modal/close/request-close events, invoker sources, open state and returnValue.
    #[test]
    fn dialog_invokers_run_chrome_dialog_command_steps() {
        let control: serde_json::Value =
            serde_json::from_str(include_str!("html_invokers_dialog_chrome156.json")).unwrap();
        let actual = run_probe(include_str!("html_invokers_dialog_probe.js"));
        let expected = &control["result"];
        for (label, value) in expected.as_object().unwrap() {
            assert_eq!(&actual[label], value, "{label}");
        }
        assert_eq!(&actual, expected);
    }
}

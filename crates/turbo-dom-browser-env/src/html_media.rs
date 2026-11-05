//! HTMLMediaElement's playback state (turbo-dom#323): the native half of html_media.js.
//!
//! The JS module owns the media element state machine; this module gives it the document's
//! trusted task scheduler, trusted event dispatch, the embedder's sticky activation answer
//! (autoplay policy) and the `src` attribute changed steps (the media element load algorithm).
use super::*;

/// html_media.js's private hook: `hook("src", element)` runs the src attribute changed steps.
struct Hook(v8::Weak<v8::Function>);
fn_slots!([Hook]);

fn dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // html_media.js (the only caller; the native is deleted from the global before guest code
    // runs) passes a media element it brand-checked, or another node of its document.
    let target =
        v8::Local::<v8::Object>::try_from(args.get(1)).expect("the media hook passes a node");
    let _dom = DomScope::for_object(scope, target);
    match args.get(0).to_rust_string_lossy(scope).as_str() {
        "dispatch" => {
            let event = v8::Local::<v8::Object>::try_from(args.get(2))
                .expect("the media hook dispatches an Event it constructed");
            dispatch_host_event(scope, event, target);
        }
        "sticky" => rv.set_bool(user_activation_gates::sticky_activation(scope)),
        // "schedule", the last operation html_media.js sends.
        operation => {
            debug_assert_eq!(operation, "schedule");
            let callback = v8::Local::<v8::Function>::try_from(args.get(2))
                .expect("the media hook schedules a function");
            html_popover::schedule(scope, callback);
        }
    }
}

/// The `src` content attribute of an HTML media element was set, changed or removed.
pub(super) fn attribute_changed(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) {
    let media = handle_of(scope, element).is_some_and(|handle| {
        with_tree(|tree| {
            tree.namespace_id(handle) == 0
                && tree
                    .local_name(handle)
                    .is_some_and(|name| name == "audio" || name == "video")
        })
        .unwrap_or(false)
    });
    if !media {
        return;
    }
    let context = element
        .get_creation_context(scope)
        .unwrap_or_else(|| scope.get_current_context());
    let Some(function) = context
        .get_slot::<Hook>()
        .and_then(|hook| hook.0.to_local(scope))
    else {
        return;
    };
    let operation = v8::String::new(scope, "src").unwrap();
    let receiver = v8::undefined(scope).into();
    v8::tc_scope!(let scope, scope);
    function.call(scope, receiver, &[operation.into(), element.into()]);
    if !scope.has_terminated() {
        if let Some(error) = scope.exception() {
            scope.reset();
            custom_elements::report_value(scope, error);
        }
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__mediaNative", dispatch);
    run_js(scope, include_str!("html_media.js"));
    let key = v8::String::new(scope, "__captureMediaHook").unwrap();
    if let Some(function) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        context_rehydrate::capture::<Hook>(scope, function);
    }
    global.delete(scope, key.into());
}

//! Editing: `document.execCommand`/`queryCommand*` over contenteditable and designMode,
//! `EditContext` and `ariaNotify`. The engine lives in editing.js and edit_context.js; this module
//! supplies trusted event dispatch and the embedder-only user input entry point.
use super::*;

/// The page's trusted typing entry point, captured privately at install.
struct EditingInput(v8::Weak<v8::Function>);

fn dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    match args.get(0).to_rust_string_lossy(scope).as_str() {
        "dispatch" => {
            let (Ok(target), Ok(event)) = (
                v8::Local::<v8::Object>::try_from(args.get(1)),
                v8::Local::<v8::Object>::try_from(args.get(2)),
            ) else {
                return;
            };
            let _dom = DomScope::for_object(scope, target);
            dispatch_host_event(scope, event, target);
            let continued = event_record(scope, event)
                .is_some_and(|record| !get_bool_prop(scope, record, "defaultPrevented"));
            rv.set_bool(continued);
        }
        "capture" => {
            let Ok(function) = v8::Local::<v8::Function>::try_from(args.get(1)) else {
                return;
            };
            if scope
                .get_current_context()
                .get_slot::<EditingInput>()
                .is_some()
            {
                return;
            }
            if let Some(function) = dom_ownership::helper_function(scope, function) {
                scope
                    .get_current_context()
                    .set_slot(std::rc::Rc::new(EditingInput(function)));
            }
        }
        _ => {}
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__editContextNative", dispatch);
    run_js(scope, include_str!("edit_context.js"));
    bind_method(scope, global, "__editingNative", dispatch);
    run_js(scope, include_str!("editing.js"));
}

/// Trusted user text input for embedder input drivers (Chrome's typing path, which unlike
/// `execCommand` fires a cancelable `beforeinput` with target ranges first): `input_type` is one of
/// `insertText`, `deleteContentBackward`, `deleteContentForward`, `insertParagraph` or
/// `insertLineBreak`, applied at the selection of `target`'s document, or to the EditContext of
/// its focused element. Returns whether the input reached editable content.
pub fn editing_user_input(
    scope: &mut v8::PinScope,
    target: v8::Local<v8::Object>,
    input_type: &str,
    data: Option<&str>,
) -> bool {
    let _dom = DomScope::for_object(scope, target);
    let id = DOM_ID.with(std::cell::Cell::get);
    let Some(context) = DOM_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .get(&id)
            .and_then(|context| context.to_local(scope))
    }) else {
        return false;
    };
    let Some(function) = context
        .get_slot::<EditingInput>()
        .and_then(|entry| entry.0.to_local(scope))
    else {
        return false;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let kind = v8::String::new(scope, input_type).unwrap();
    let data: v8::Local<v8::Value> = match data {
        Some(text) => v8::String::new(scope, text).unwrap().into(),
        None => v8::null(scope).into(),
    };
    let receiver = v8::undefined(scope).into();
    function
        .call(scope, receiver, &[target.into(), kind.into(), data])
        .is_some_and(|value| value.is_true())
}

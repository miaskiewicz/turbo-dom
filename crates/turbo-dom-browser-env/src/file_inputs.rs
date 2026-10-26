//! File-state `<input>` members, FileList and DataTransfer (file_inputs.js). The native
//! side keeps the realm's private hooks: the embedder File capture and the `type`
//! attribute hook. Selection state lives in V8-private records that follow the objects.
use super::*;

struct FileHost(v8::Weak<v8::Function>);
struct TypeHook(v8::Weak<v8::Function>);

fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    match operation.as_str() {
        "record" => {
            if let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) {
                if let Some(value) = collection_records::get(scope, object) {
                    rv.set(value);
                }
            }
        }
        "set-record" => {
            if let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) {
                collection_records::set(scope, object, args.get(2));
            }
        }
        // KURL-style absolute URL parsing for text/uri-list lines.
        "url" => {
            let line = args.get(1).to_rust_string_lossy(scope);
            if let Some(url) = url::Url::parse(&line)
                .ok()
                .and_then(|url| v8::String::new(scope, url.as_str()))
            {
                rv.set(url.into());
            }
        }
        "schedule" => {
            if let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(1)) {
                html_popover::schedule(scope, callback);
            }
        }
        "type-hook" => {
            if let Ok(hook) = v8::Local::<v8::Function>::try_from(args.get(1)) {
                if let Some(hook) = dom_ownership::helper_function(scope, hook) {
                    scope
                        .get_current_context()
                        .set_slot(std::rc::Rc::new(TypeHook(hook)));
                }
            }
        }
        _ => {}
    }
}

/// A changed `type` attribute moves the input in or out of the file state.
pub(super) fn attribute_changed<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    element: v8::Local<'s, v8::Object>,
    name: &str,
) {
    if name != "type" {
        return;
    }
    let Some(hook) = scope
        .get_current_context()
        .get_slot::<TypeHook>()
        .and_then(|hook| hook.0.to_local(scope))
    else {
        return;
    };
    let receiver = v8::undefined(scope).into();
    hook.call(scope, receiver, &[element.into()]);
}

/// Hand the realm's `File` interface to the binding. The embedder calls this after
/// installing File and before any page script; a host without File keeps file
/// entry construction unavailable rather than fabricating File objects.
pub(super) fn capture_host(scope: &mut v8::PinScope) {
    let context = scope.get_current_context();
    let Some(capture) = context
        .get_slot::<FileHost>()
        .and_then(|host| host.0.to_local(scope))
    else {
        return;
    };
    let global = context.global(scope);
    let key = v8::String::new(scope, "File").unwrap();
    let Some(file) = global
        .get(scope, key.into())
        .filter(|value| value.is_function())
    else {
        return;
    };
    let receiver = v8::undefined(scope).into();
    capture.call(scope, receiver, &[file]);
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__fileInputNative", dispatch);
    run_js(scope, include_str!("file_inputs.js"));
    let key = v8::String::new(scope, "__fileInputHost").unwrap();
    if let Some(capture) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(capture) = dom_ownership::helper_function(scope, capture) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(FileHost(capture)));
        }
    }
    global.delete(scope, key.into());
    // Runners that define File before the binding (turbo-test's runtime) capture it now.
    capture_host(scope);
}

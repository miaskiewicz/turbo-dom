//! Same-document history for the binding's standalone window. Navigation-capable
//! embedders supply their own History and Location implementations.
use super::*;

fn can_rewrite(current: &url::Url, next: &url::Url) -> bool {
    current.origin() == next.origin()
        && current.username() == next.username()
        && current.password() == next.password()
        && current.scheme() == next.scheme()
}

fn resolve<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
) -> Option<v8::Local<'s, v8::Object>> {
    let current = url::Url::parse(&arg_str(scope, args, 0)).ok()?;
    let next = current.join(&arg_str(scope, args, 1)).ok()?;
    if !can_rewrite(&current, &next) {
        return None;
    }
    let value = v8::Object::new(scope);
    for name in [
        "href", "origin", "protocol", "host", "hostname", "port", "pathname", "search", "hash",
    ] {
        let text = if name == "href" {
            next.as_str().to_owned()
        } else {
            hyperlink_utils::component(&next, name)
        };
        let text = v8::String::new(scope, &text)?;
        set_prop(scope, value, name, text.into());
    }
    Some(value)
}

fn resolve_callback<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue<'s>,
) {
    if let Some(value) = resolve(scope, &args) {
        result.set(value.into());
    } else {
        result.set(v8::null(scope).into());
    }
}

fn schedule(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    if let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(0)) {
        result.set_bool(html_popover::schedule_after(scope, callback, 0));
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__historyURL", resolve_callback);
    bind_method(
        scope,
        global,
        "__historyClone",
        worker_scope::structured_clone,
    );
    bind_method(scope, global, "__historySchedule", schedule);
    run_js(scope, include_str!("window_history.js"));
}

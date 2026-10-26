//! Interest invokers: the native side of `interestfor`. Trusted focus/blur, hover
//! (mouseover/mouseout) and Escape keydown drive the private JS interest state; delays run
//! on the host-captured (virtual) timers; the shown state surfaces as `:interest-source` /
//! `:interest-target` and the target popover is shown/hidden with the invoker as source.
use super::*;

struct InterestHook(v8::Weak<v8::Function>);
/// Whether any interest is currently shown in this context (removal hooks stay cheap).
struct InterestEngaged(std::cell::Cell<bool>);

fn engaged(scope: &mut v8::PinScope) -> bool {
    scope
        .get_current_context()
        .get_slot::<InterestEngaged>()
        .is_some_and(|flag| flag.0.get())
}

fn call<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    args: &[v8::Local<'s, v8::Value>],
) -> Option<v8::Local<'s, v8::Value>> {
    let context = node.get_creation_context(scope)?;
    let hook = context.get_slot::<InterestHook>()?;
    let function = dom_ownership::local(scope, &hook.0);
    let receiver = v8::undefined(scope);
    v8::tc_scope!(let scope, scope);
    let result = function.call(scope, receiver.into(), args);
    if !scope.has_terminated() {
        if let Some(error) = scope.exception() {
            scope.reset();
            custom_elements::report_value(scope, error);
            return None;
        }
    }
    result.map(|value| v8::Local::new(scope, value))
}

/// After a trusted dispatch of `ty` at `target` (see `el_dispatch_event`).
pub(super) fn observe(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
    target: Handle,
    ty: &str,
) {
    if !matches!(ty, "focus" | "blur" | "mouseover" | "mouseout" | "keydown") {
        return;
    }
    if ty == "keydown" && !engaged(scope) {
        return;
    }
    let event = v8::Local::new(scope, event);
    let node = wrap(scope, target);
    let kind = v8::String::new(scope, ty).unwrap();
    call(scope, node, &[kind.into(), node.into(), event.into()]);
}

/// Removal steps: interest whose invoker or target leaves the tree ends silently.
pub(super) fn disconnect(scope: &mut v8::PinScope, root: Handle, include_root: bool) {
    if !engaged(scope) {
        return;
    }
    let node = wrap(scope, root);
    let kind = v8::String::new(scope, "disconnect").unwrap();
    let undefined = v8::undefined(scope);
    let include = v8::Boolean::new(scope, include_root);
    call(
        scope,
        node,
        &[kind.into(), node.into(), undefined.into(), include.into()],
    );
}

/// Changing `interestfor` ends the invoker's interest silently.
pub(super) fn attribute_changed<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    element: v8::Local<'s, v8::Object>,
) {
    if !engaged(scope) {
        return;
    }
    let kind = v8::String::new(scope, "attribute").unwrap();
    call(scope, element, &[kind.into(), element.into()]);
}

#[derive(Clone, Copy, PartialEq)]
enum Pseudo {
    Source,
    Target,
}
fn pseudo_ranges<'i, 't>(
    input: &mut cssparser::Parser<'i, 't>,
    ranges: &mut Vec<(usize, usize, Pseudo)>,
) -> Result<(), cssparser::ParseError<'i, ()>> {
    use cssparser::Token;
    let mut colon = None;
    let mut double_colon = false;
    while !input.is_exhausted() {
        let position = input.position().byte_index();
        let token = input.next_including_whitespace_and_comments()?.clone();
        match token {
            Token::Colon => {
                double_colon = colon.is_some();
                colon = if double_colon { None } else { Some(position) };
            }
            Token::Ident(name) => {
                let pseudo = if name.eq_ignore_ascii_case("interest-source") {
                    Some(Pseudo::Source)
                } else if name.eq_ignore_ascii_case("interest-target") {
                    Some(Pseudo::Target)
                } else {
                    None
                };
                if let (false, Some(start), Some(pseudo)) = (double_colon, colon, pseudo) {
                    ranges.push((start, input.position().byte_index(), pseudo));
                }
                colon = None;
                double_colon = false;
            }
            Token::Function(_) | Token::ParenthesisBlock => {
                colon = None;
                double_colon = false;
                input.parse_nested_block(|nested| pseudo_ranges(nested, ranges))?;
            }
            Token::Comment(_) => (),
            _ => {
                colon = None;
                double_colon = false;
            }
        }
    }
    Ok(())
}
/// Represent the private interest state as structural selectors within the anchor's root,
/// exactly as `:popover-open` does.
pub(super) fn selector(scope: &mut v8::PinScope, anchor: Handle, source: &str) -> Option<String> {
    if !source.to_ascii_lowercase().contains("interest-") {
        return Some(source.to_owned());
    }
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut ranges = Vec::new();
    if pseudo_ranges(&mut parser, &mut ranges).is_err() || ranges.is_empty() {
        return Some(source.to_owned());
    }
    let mut paths = |kind: &str| -> String {
        let mut handles = Vec::new();
        if engaged(scope) {
            let node = wrap(scope, anchor);
            let name = v8::String::new(scope, kind).unwrap();
            if let Some(list) = call(scope, node, &[name.into(), node.into()])
                .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
            {
                for index in 0..list.length() {
                    if let Some(handle) = list
                        .get_index(scope, index)
                        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                        .filter(|object| dom_id_of(scope, *object) == DOM_ID.with(|id| id.get()))
                        .and_then(|object| handle_of(scope, object))
                    {
                        handles.push(handle);
                    }
                }
            }
        }
        with_tree(|tree| {
            let root = html_popover::selector_root(tree, anchor);
            let paths = handles
                .iter()
                .filter(|&&node| html_popover::selector_root(tree, node) == root)
                .map(|&node| html_popover::selector_path(tree, node))
                .collect::<Vec<_>>();
            if paths.is_empty() {
                ":not(*)".to_owned()
            } else {
                format!(":is({})", paths.join(","))
            }
        })
        .unwrap_or_else(|| ":not(*)".to_owned())
    };
    let sources = paths("sources");
    let targets = paths("targets");
    let mut result = String::new();
    let mut start = 0;
    for (from, to, pseudo) in ranges {
        result.push_str(&source[start..from]);
        result.push_str(if pseudo == Pseudo::Source {
            &sources
        } else {
            &targets
        });
        start = to;
    }
    result.push_str(&source[start..]);
    Some(result)
}

fn native<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    match operation.as_str() {
        "engaged" => {
            let value = args.get(1).is_true();
            if let Some(flag) = scope.get_current_context().get_slot::<InterestEngaged>() {
                flag.0.set(value);
            }
            return;
        }
        "focused" => {
            let active = ACTIVE
                .with(|active| *active.borrow())
                .filter(|node| with_tree(|tree| is_connected(tree, *node)).unwrap_or(false));
            let value = wrap_opt(scope, active);
            rv.set(value);
            return;
        }
        "schedule" => {
            if let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(1)) {
                let delay = args.get(2).number_value(scope).unwrap_or(0.0);
                html_popover::schedule_after(scope, callback, delay.max(0.0) as u32);
            }
            return;
        }
        _ => {}
    }
    let Ok(target) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, target);
    let Some(node) = handle_of(scope, target) else {
        return;
    };
    match operation.as_str() {
        "disabled" => rv.set_bool(html_invokers::disabled_control(node)),
        "open" => rv.set_bool(html_popover::opened(node)),
        "dispatch" => {
            let Ok(event) = v8::Local::<v8::Object>::try_from(args.get(2)) else {
                return;
            };
            dispatch_host_event(scope, event, target);
            let continued = event_record(scope, event)
                .is_some_and(|record| !get_bool_prop(scope, record, "defaultPrevented"));
            rv.set_bool(continued);
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

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__interestNative", native);
    run_js(scope, include_str!("interest_invokers.js"));
    let key = v8::String::new(scope, "__captureInterestHook").unwrap();
    if let Some(function) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(function) = dom_ownership::helper_function(scope, function) {
            let context = scope.get_current_context();
            context.set_slot(std::rc::Rc::new(InterestHook(function)));
            context.set_slot(std::rc::Rc::new(InterestEngaged(std::cell::Cell::new(
                false,
            ))));
        }
    }
    global.delete(scope, key.into());
}

#[cfg(test)]
#[path = "interest_invokers_tests.rs"]
mod tests;

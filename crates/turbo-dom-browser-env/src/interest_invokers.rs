//! Interest invokers: the native side of `interestfor`. Trusted focus/blur, hover
//! (mouseover/mouseout) and Escape keydown drive the private JS interest state; delays run
//! on the host-captured (virtual) timers; the shown state surfaces as `:interest-source` /
//! `:interest-target` and the target popover is shown/hidden with the invoker as source.
use super::*;

struct InterestHook(v8::Weak<v8::Function>);
/// Whether any interest is currently shown in this context (removal hooks stay cheap).
struct InterestEngaged(std::cell::Cell<bool>);
impl context_rehydrate::FnSlot for InterestHook {
    const NAME: &'static str = concat!(module_path!(), "::InterestHook");
    fn wrap(function: v8::Weak<v8::Function>) -> Self {
        Self(function)
    }
    fn weak(&self) -> &v8::Weak<v8::Function> {
        &self.0
    }
}
/// The hook's engagement state starts disengaged with it.
pub(super) const SLOTS: &[context_rehydrate::Entry] = &[context_rehydrate::Entry {
    set: |scope, context, value| {
        let set = (context_rehydrate::Entry::of::<InterestHook>().set)(scope, context, value);
        if set {
            context.set_slot(std::rc::Rc::new(InterestEngaged(std::cell::Cell::new(
                false,
            ))));
        }
        set
    },
    ..context_rehydrate::Entry::of::<InterestHook>()
}];

fn engaged(scope: &mut v8::PinScope) -> bool {
    scope
        .get_current_context()
        .get_slot::<InterestEngaged>()
        .is_some_and(|flag| flag.0.get())
}

fn call_strict<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    args: &[v8::Local<'s, v8::Value>],
) -> Option<v8::Local<'s, v8::Value>> {
    let context = node.get_creation_context(scope)?;
    let function = context.get_slot::<InterestHook>()?.0.to_local(scope)?;
    let receiver = v8::undefined(scope);
    function.call(scope, receiver.into(), args)
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
    // invariant: engine-built name, far below V8's string length limit
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

/// Whether [`selector`] can change `source`: an interest pseudo-class range (a property of
/// the text, memoized by `selector_argument`).
pub(super) fn applies(source: &str) -> bool {
    if !source.contains('\\') && !source.to_ascii_lowercase().contains("interest-") {
        return false;
    }
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut ranges = Vec::new();
    pseudo_ranges(&mut parser, &mut ranges).is_ok() && !ranges.is_empty()
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
    selector_impl(scope, anchor, source, false)
}

pub(super) fn selector_strict(
    scope: &mut v8::PinScope,
    anchor: Handle,
    source: &str,
) -> Option<String> {
    selector_impl(scope, anchor, source, true)
}

fn selector_impl(
    scope: &mut v8::PinScope,
    anchor: Handle,
    source: &str,
    strict: bool,
) -> Option<String> {
    #[cfg(test)]
    SELECTOR_WORK.with(|work| work.transforms.set(work.transforms.get() + 1));
    if !source.contains('\\') && !source.to_ascii_lowercase().contains("interest-") {
        return Some(source.to_owned());
    }
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut ranges = Vec::new();
    if pseudo_ranges(&mut parser, &mut ranges).is_err() {
        return (!strict).then(|| source.to_owned());
    }
    if ranges.is_empty() {
        return Some(source.to_owned());
    }
    if strict {
        let context = scope.get_current_context();
        context.get_slot::<InterestEngaged>()?;
        context.get_slot::<InterestHook>()?.0.to_local(scope)?;
    }
    let mut paths = |kind: &str| -> Option<String> {
        let mut handles = Vec::new();
        if engaged(scope) {
            let node = wrap(scope, anchor);
            // invariant: engine-built name, far below V8's string length limit
            let name = v8::String::new(scope, kind).unwrap();
            let args = [name.into(), node.into()];
            let result = if strict {
                call_strict(scope, node, &args)
            } else {
                call(scope, node, &args)
            };
            let list = result.and_then(|value| v8::Local::<v8::Array>::try_from(value).ok());
            if strict && list.is_none() {
                return None;
            }
            if let Some(list) = list {
                for index in 0..list.length() {
                    let handle = list
                        .get_index(scope, index)
                        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                        .filter(|object| dom_id_of(scope, *object) == DOM_ID.with(|id| id.get()))
                        .and_then(|object| handle_of(scope, object));
                    if strict && handle.is_none() {
                        return None;
                    }
                    if let Some(handle) = handle {
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
        .or_else(|| (!strict).then(|| ":not(*)".to_owned()))
    };
    let sources = paths("sources")?;
    let targets = paths("targets")?;
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
    // interest_invokers.js passes an element target (and its InterestEvent) for the
    // remaining operations.
    let target = v8::Local::<v8::Object>::try_from(args.get(1)).expect("interest target");
    let _dom = DomScope::for_object(scope, target);
    let Some(node) = handle_of(scope, target) else {
        return;
    };
    match operation.as_str() {
        "disabled" => rv.set_bool(html_invokers::disabled_control(node)),
        "open" => rv.set_bool(html_popover::opened(node)),
        "dispatch" => {
            let event = v8::Local::<v8::Object>::try_from(args.get(2)).expect("interest event");
            dispatch_host_event(scope, event, target);
            let continued = event_record(scope, event)
                .is_some_and(|record| !get_bool_prop(scope, record, "defaultPrevented"));
            rv.set_bool(continued);
        }
        // "popover", the last operation interest_invokers.js calls.
        _ => {
            let action = args.get(2).to_rust_string_lossy(scope);
            if let Ok(source) = v8::Local::<v8::Object>::try_from(args.get(3)) {
                html_popover::invoke(scope, node, &action, source);
            }
        }
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
        if context_rehydrate::capture::<InterestHook>(scope, function).is_some() {
            let context = scope.get_current_context();
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

#[cfg(all(test, feature = "host-internals"))]
mod host_projection_tests {
    use super::*;
    use crate::host_internals::{prepare_selector, SelectorPreparationError};

    #[test]
    fn volatile_interest_projection_changes_without_a_revision_change() {
        crate::browser_env::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><button id=target></button>");
        let _owner = DomScope::enter(context.get_slot::<DomContextId>().unwrap().0);
        let (root, target) =
            with_tree(|tree| (tree.root(), tree.query_selector("#target").unwrap())).unwrap();
        let document = wrap(scope, root);
        let source = v8::String::new(scope, "(()=>[document.getElementById('target')])").unwrap();
        let callback = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let callback = v8::Local::<v8::Function>::try_from(callback).unwrap();
        context.set_slot(std::rc::Rc::new(InterestHook(v8::Weak::new(
            scope, callback,
        ))));
        for source in [":interest-source", ":is(:interest-s\\6furce)"] {
            context.get_slot::<InterestEngaged>().unwrap().0.set(false);
            let inactive = prepare_selector(scope, document, source).unwrap();
            assert!(inactive.is_volatile());
            assert!(!with_tree(|tree| tree.matches(target, inactive.match_selector())).unwrap());
            context.get_slot::<InterestEngaged>().unwrap().0.set(true);
            let active = prepare_selector(scope, document, source).unwrap();
            assert!(active.is_volatile());
            assert_eq!(inactive.stamp(), active.stamp());
            assert!(with_tree(|tree| tree.matches(target, active.match_selector())).unwrap());
        }
        for source in [
            "button[data-note='interest-source']",
            "button:focus",
            "button:invalid",
        ] {
            assert!(!prepare_selector(scope, document, source)
                .unwrap()
                .is_volatile());
        }
        reset();
    }

    #[test]
    fn strict_interest_projection_does_not_certify_hook_failure_as_empty() {
        crate::browser_env::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(
            scope,
            "<!doctype html><button interestfor=p></button><div id=p popover></div>",
        );
        let source = v8::String::new(scope, "document").unwrap();
        let document = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_object(scope)
            .unwrap();
        context.remove_slot::<InterestHook>();
        assert_eq!(
            prepare_selector(scope, document, ":interest-source").unwrap_err(),
            SelectorPreparationError::Unavailable
        );
        context.get_slot::<InterestEngaged>().unwrap().0.set(true);
        for body in ["()=>null", "()=>[{}]"] {
            let source = v8::String::new(scope, &format!("({body})")).unwrap();
            let callback = v8::Script::compile(scope, source, None)
                .unwrap()
                .run(scope)
                .unwrap();
            let callback = v8::Local::<v8::Function>::try_from(callback).unwrap();
            context.set_slot(std::rc::Rc::new(InterestHook(v8::Weak::new(
                scope, callback,
            ))));
            assert_eq!(
                prepare_selector(scope, document, ":interest-source").unwrap_err(),
                SelectorPreparationError::Unavailable
            );
        }
        let source =
            v8::String::new(scope, "(()=>{throw Error('interest classifier failure')})").unwrap();
        let callback = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let callback = v8::Local::<v8::Function>::try_from(callback).unwrap();
        context.set_slot(std::rc::Rc::new(InterestHook(v8::Weak::new(
            scope, callback,
        ))));
        v8::tc_scope!(let scope, scope);
        assert_eq!(
            prepare_selector(scope, document, ":interest-source").unwrap_err(),
            SelectorPreparationError::Exception
        );
        assert!(scope
            .exception()
            .unwrap()
            .to_rust_string_lossy(scope)
            .contains("interest classifier failure"));
        scope.reset();
        reset();
    }
}

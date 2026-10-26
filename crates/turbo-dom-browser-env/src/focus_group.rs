//! `focusgroup` arrow-key navigation: the default action of an unprevented trusted keydown
//! moves focus among the sequentially focusable items of the focused element's nearest
//! focusgroup, along the behavior's axes (writing-mode and direction aware), with wrapping.
use super::*;

struct FocusGroupHook(v8::Weak<v8::Function>);

const BEHAVIORS: [&str; 6] = [
    "toolbar",
    "tablist",
    "radiogroup",
    "listbox",
    "menu",
    "menubar",
];

#[derive(Clone, Copy)]
struct Group {
    inline: bool,
    block: bool,
    wrap: bool,
}
enum Attribute {
    None,
    OptOut,
    Owner(Group),
}
/// Tokens are ASCII case-insensitive and unknown ones are ignored: `none` anywhere opts out;
/// otherwise the first behavior token makes an owner, refined by axis and wrap modifiers.
fn attribute(tree: &Tree, node: Handle) -> Attribute {
    let Some(value) = tree.get_attribute_ns(node, None, "focusgroup") else {
        return Attribute::None;
    };
    let tokens = value
        .split(|c: char| c.is_ascii_whitespace())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let has = |name: &str| tokens.iter().any(|token| token == name);
    if has("none") {
        return Attribute::OptOut;
    }
    let Some(behavior) = tokens
        .iter()
        .find(|token| BEHAVIORS.contains(&token.as_str()))
    else {
        return Attribute::None;
    };
    let (mut inline, mut block, mut wrap) = match behavior.as_str() {
        "toolbar" => (true, false, false),
        "tablist" | "menubar" => (true, false, true),
        "radiogroup" => (true, true, true),
        "listbox" => (false, true, false),
        _ => (false, true, true),
    };
    if has("inline") || has("block") {
        inline = has("inline");
        block = has("block");
    }
    if has("nowrap") {
        wrap = false;
    } else if has("wrap") {
        wrap = true;
    }
    Attribute::Owner(Group {
        inline,
        block,
        wrap,
    })
}

fn element_of(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> Option<Handle> {
    if dom_id_of(scope, object) != DOM_ID.with(|id| id.get()) {
        return None;
    }
    handle_of(scope, object)
        .filter(|node| with_tree(|tree| tree.node_type_id(*node) == 1).unwrap_or(false))
}
fn flat_parent(scope: &mut v8::PinScope, node: Handle) -> Option<Handle> {
    let object = wrap(scope, node);
    let parent = shadow_slots::composed_parent(scope, object)?;
    element_of(scope, parent)
}
/// Flat-tree descendants of `owner` in order, skipping opted-out subtrees and the items of
/// nested focusgroups (whose owners themselves remain candidates).
fn candidates(scope: &mut v8::PinScope, owner: Handle) -> Vec<Handle> {
    let mut out = Vec::new();
    let root = wrap(scope, owner);
    let mut stack = shadow_slots::composed_children(scope, root).unwrap_or_default();
    stack.reverse();
    while let Some(object) = stack.pop() {
        let Some(node) = element_of(scope, object) else {
            continue;
        };
        let kind = with_tree(|tree| attribute(tree, node)).unwrap_or(Attribute::None);
        if matches!(kind, Attribute::OptOut) {
            continue;
        }
        out.push(node);
        if matches!(kind, Attribute::Owner(_)) {
            continue;
        }
        let mut children = shadow_slots::composed_children(scope, object).unwrap_or_default();
        children.reverse();
        stack.extend(children);
    }
    out
}
/// Focusable (rendered, enabled, not inert) and in sequential focus order.
fn item(
    tree: &Tree,
    node: Handle,
    owner: Handle,
    policy: &turbo_dom_parser::rtdom::cascade::StylePolicy,
) -> bool {
    if !html_dialog::focusable(tree, node, owner, policy) {
        return false;
    }
    let negative = tree.get_attribute(node, "tabindex").is_some_and(|value| {
        let value = value.trim_matches(|c: char| c.is_ascii_whitespace());
        value.starts_with('-')
            && value[1..].bytes().all(|byte| byte.is_ascii_digit())
            && value.len() > 1
    });
    !negative
}
/// Controls that handle arrow/Home/End keys themselves (text entry, selects, ranges, radio
/// groups, editing hosts) keep them.
fn consumes(tree: &Tree, node: Handle) -> bool {
    if tree.namespace_id(node) != 0 {
        return false;
    }
    match tree.local_name(node) {
        Some("textarea" | "select") => true,
        Some("input") => {
            let kind = tree
                .get_attribute(node, "type")
                .unwrap_or_default()
                .to_ascii_lowercase();
            !matches!(
                kind.as_str(),
                "button" | "submit" | "reset" | "image" | "checkbox" | "color" | "file" | "hidden"
            )
        }
        _ => tree
            .get_attribute(node, "contenteditable")
            .is_some_and(|value| {
                matches!(
                    value.to_ascii_lowercase().as_str(),
                    "" | "true" | "plaintext-only"
                )
            }),
    }
}

/// (vertical, block progression reversed) for a computed `writing-mode`.
fn writing_mode(mode: &str) -> (bool, bool) {
    let mode = mode.trim().to_ascii_lowercase();
    let vertical =
        mode.starts_with("vertical") || mode.starts_with("sideways") || mode.starts_with("tb");
    let reversed = matches!(
        mode.as_str(),
        "vertical-rl" | "sideways-rl" | "tb" | "tb-rl"
    );
    (vertical, reversed)
}
/// `dir=auto`: the first strong character of the element's own text, skipping script/style
/// content and descendants that set their own `dir`.
fn auto_rtl(tree: &Tree, element: Handle) -> bool {
    let mut pending = tree.children(element);
    pending.reverse();
    while let Some(node) = pending.pop() {
        match tree.node_type_id(node) {
            3 => {
                for unit in tree.node_value(node).unwrap_or_default().chars() {
                    let code = unit as u32;
                    if matches!(code, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF)
                    {
                        return true;
                    }
                    if unit.is_alphabetic() {
                        return false;
                    }
                }
            }
            1 if !matches!(tree.local_name(node), Some("script" | "style" | "textarea"))
                && tree.get_attribute_ns(node, None, "dir").is_none() =>
            {
                let mut children = tree.children(node);
                children.reverse();
                pending.extend(children);
            }
            _ => {}
        }
    }
    false
}
/// The focused item's direction: an authored CSS `rtl` from the native cascade, else the
/// nearest flat-tree inclusive ancestor's `dir` attribute (the UA's `[dir]` rules).
fn rtl(
    scope: &mut v8::PinScope,
    node: Handle,
    policy: &turbo_dom_parser::rtdom::cascade::StylePolicy,
) -> bool {
    use turbo_dom_parser::rtdom::cascade::{computed_style_with_policy, get_property_value};
    let css = with_tree(|tree| {
        let computed = computed_style_with_policy(tree, node, policy);
        get_property_value(&computed, "direction").eq_ignore_ascii_case("rtl")
    })
    .unwrap_or(false);
    if css {
        return true;
    }
    let mut cursor = Some(node);
    while let Some(element) = cursor {
        let found = with_tree(|tree| {
            let value = tree
                .get_attribute_ns(element, None, "dir")?
                .to_ascii_lowercase();
            match value.as_str() {
                "rtl" => Some(true),
                "ltr" => Some(false),
                "auto" => Some(auto_rtl(tree, element)),
                _ => None,
            }
        })
        .flatten();
        if let Some(found) = found {
            return found;
        }
        cursor = flat_parent(scope, element);
    }
    false
}

fn focused() -> Option<Handle> {
    ACTIVE
        .with(|active| *active.borrow())
        .filter(|node| with_tree(|tree| is_connected(tree, *node)).unwrap_or(false))
}
/// The focused element's nearest focusgroup owner, unless it keeps the keys or opted out.
fn owner(scope: &mut v8::PinScope, focused: Handle) -> Option<(Handle, Group)> {
    if with_tree(|tree| consumes(tree, focused)).unwrap_or(true) {
        return None;
    }
    let mut cursor = flat_parent(scope, focused);
    loop {
        let node = cursor?;
        match with_tree(|tree| attribute(tree, node)).unwrap_or(Attribute::None) {
            Attribute::OptOut => return None,
            Attribute::Owner(group) => return Some((node, group)),
            Attribute::None => cursor = flat_parent(scope, node),
        }
    }
}
/// The next item for `key`, given the focused item's computed `writing-mode`.
fn navigate(scope: &mut v8::PinScope, key: &str, mode: &str) -> Option<Handle> {
    let focused = focused()?;
    let (owner, group) = owner(scope, focused)?;
    let candidates = candidates(scope, owner);
    let policy = css_policy::cascade_policy(scope);
    let items = with_tree(|tree| {
        candidates
            .into_iter()
            .filter(|&node| item(tree, node, owner, &policy))
            .collect::<Vec<_>>()
    })?;
    let (vertical, block_reversed) = writing_mode(mode);
    let index = items.iter().position(|&node| node == focused)?;
    let rtl = rtl(scope, focused, &policy);
    // (inline?, forward?) for each arrow in this writing mode.
    let step = match (key, vertical) {
        ("ArrowRight", false) => Some((true, !rtl)),
        ("ArrowLeft", false) => Some((true, rtl)),
        ("ArrowDown", false) => Some((false, true)),
        ("ArrowUp", false) => Some((false, false)),
        ("ArrowDown", true) => Some((true, !rtl)),
        ("ArrowUp", true) => Some((true, rtl)),
        ("ArrowLeft", true) => Some((false, block_reversed)),
        ("ArrowRight", true) => Some((false, !block_reversed)),
        _ => None,
    };
    let next = match (key, step) {
        ("Home", _) => 0,
        ("End", _) => items.len() - 1,
        (_, Some((inline, forward))) => {
            if (inline && !group.inline) || (!inline && !group.block) {
                return None;
            }
            match (forward, index) {
                (true, index) if index + 1 < items.len() => index + 1,
                (true, _) if group.wrap => 0,
                (false, 0) if group.wrap => items.len() - 1,
                (false, index) if index > 0 => index - 1,
                _ => return None,
            }
        }
        _ => return None,
    };
    (items[next] != focused).then_some(items[next])
}

/// The default action of an unprevented trusted keydown (see `el_dispatch_event`).
pub(super) fn keydown(scope: &mut v8::PinScope, event: v8::Local<v8::Object>, target: Handle) {
    let node = wrap(scope, target);
    let Some(context) = node.get_creation_context(scope) else {
        return;
    };
    let Some(hook) = context.get_slot::<FocusGroupHook>() else {
        return;
    };
    let function = dom_ownership::local(scope, &hook.0);
    let receiver = v8::undefined(scope);
    let event = v8::Local::new(scope, event);
    v8::tc_scope!(let scope, scope);
    function.call(scope, receiver.into(), &[event.into()]);
    if !scope.has_terminated() {
        if let Some(error) = scope.exception() {
            scope.reset();
            custom_elements::report_value(scope, error);
        }
    }
}

fn native<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    match args.get(0).to_rust_string_lossy(scope).as_str() {
        // The focused element, when it is an item candidate of some focusgroup.
        "grouped" => {
            let grouped = focused().filter(|&node| owner(scope, node).is_some());
            let value = wrap_opt(scope, grouped);
            rv.set(value);
        }
        "navigate" => {
            let key = args.get(1).to_rust_string_lossy(scope);
            let mode = args.get(2).to_rust_string_lossy(scope);
            let target = navigate(scope, &key, &mode);
            let value = wrap_opt(scope, target);
            rv.set(value);
        }
        _ => {}
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__focusGroupNative", native);
    run_js(scope, include_str!("focus_group.js"));
    let key = v8::String::new(scope, "__captureFocusGroupHook").unwrap();
    if let Some(function) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(function) = dom_ownership::helper_function(scope, function) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(FocusGroupHook(function)));
        }
    }
    global.delete(scope, key.into());
}

#[cfg(test)]
#[path = "focus_group_tests.rs"]
mod tests;

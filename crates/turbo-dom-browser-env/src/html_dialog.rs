//! Private native dialog state: the is-modal flag in top-layer order, the dialog removing
//! steps, focus delegates, modal focus blocking and `:modal`/`:open` selector state.
use super::*;

#[derive(Default)]
pub(super) struct State {
    /// Modal dialogs in top-layer order; the last connected entry blocks focus elsewhere.
    modal: Vec<Handle>,
}

impl State {
    pub(super) fn sweep_storage(&mut self, tree: &Tree) {
        self.modal.retain(|node| tree.is_live(*node));
    }
}

fn modal(node: Handle) -> bool {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|dom| dom.dialog.modal.contains(&node))
    })
}

fn set_modal(node: Handle, value: bool) {
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let Some(dom) = dom.as_mut() else {
            return;
        };
        dom.dialog.modal.retain(|entry| *entry != node);
        if value {
            dom.dialog.modal.push(node);
        }
    });
}

/// Shadow-including inclusive descendant.
fn within(tree: &Tree, node: Handle, ancestor: Handle) -> bool {
    let mut current = Some(node);
    while let Some(node) = current {
        if node == ancestor {
            return true;
        }
        current = tree.parent(node).or_else(|| tree.shadow_host(node));
    }
    false
}

/// The dialog removing steps for every modal dialog leaving the document: leave the top
/// layer and clear the is-modal flag. `include_root` false removes only `root`'s children,
/// so `root` itself and its own shadow tree stay connected.
pub(super) fn disconnect(root: Handle, include_root: bool) {
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let Some(dom) = dom.as_mut() else {
            return;
        };
        let tree = &dom.tree;
        dom.dialog.modal.retain(|&entry| {
            let (mut previous, mut current) = (None, Some(entry));
            while let Some(node) = current {
                if node == root {
                    return !(include_root
                        || previous.is_some_and(|child| tree.parent(child) == Some(root)));
                }
                previous = Some(node);
                current = tree.parent(node).or_else(|| tree.shadow_host(node));
            }
            true
        });
    });
}

pub(super) fn is_modal(node: Handle) -> bool {
    modal(node)
}

/// The topmost connected modal dialog when `node` lies outside it (and so is inert).
pub(super) fn blocking_dialog(node: Handle) -> Option<Handle> {
    DOM.with(|dom| {
        let dom = dom.borrow();
        let dom = dom.as_ref()?;
        let top = dom
            .dialog
            .modal
            .iter()
            .rev()
            .find(|&&entry| is_connected(&dom.tree, entry))
            .copied()?;
        (!within(&dom.tree, node, top)).then_some(top)
    })
}

/// Modal dialogs make everything outside the topmost one inert, so focus() is a no-op there.
pub(super) fn blocks_focus(node: Handle) -> bool {
    DOM.with(|dom| {
        let dom = dom.borrow();
        let Some(dom) = dom.as_ref() else {
            return false;
        };
        dom.dialog
            .modal
            .iter()
            .rev()
            .find(|&&entry| is_connected(&dom.tree, entry))
            .is_some_and(|&top| !within(&dom.tree, node, top))
    })
}

fn html(tree: &Tree, node: Handle, name: &str) -> bool {
    tree.node_type_id(node) == 1
        && tree.namespace_id(node) == 0
        && tree.local_name(node) == Some(name)
}

/// Whether `node`'s own box is suppressed: the UA `[hidden]` rule, or a computed `display:none`
/// / `visibility:hidden` from the native cascade.
fn suppressed(
    tree: &Tree,
    node: Handle,
    policy: &turbo_dom_parser::rtdom::cascade::StylePolicy,
) -> bool {
    use turbo_dom_parser::rtdom::cascade::{computed_style_with_policy, get_property_value};
    if tree.namespace_id(node) == 0 && tree.has_attribute(node, "hidden") {
        return true;
    }
    let computed = computed_style_with_policy(tree, node, policy);
    get_property_value(&computed, "display").eq_ignore_ascii_case("none")
}

fn hidden_visibility(
    tree: &Tree,
    node: Handle,
    policy: &turbo_dom_parser::rtdom::cascade::StylePolicy,
) -> bool {
    use turbo_dom_parser::rtdom::cascade::{computed_style_with_policy, get_property_value};
    let computed = computed_style_with_policy(tree, node, policy);
    matches!(
        get_property_value(&computed, "visibility")
            .to_ascii_lowercase()
            .as_str(),
        "hidden" | "collapse"
    )
}

/// A focusable area as Chrome's dialog focusing steps measure it: a natively focusable,
/// enabled control or link, a valid tabindex, an editing host, an open dialog or a details
/// summary, which is rendered and not inert. Rendering is the native cascade plus the UA
/// rules for `[hidden]`, closed dialogs and closed details; there is no layout.
pub(super) fn focusable(
    tree: &Tree,
    node: Handle,
    dialog: Handle,
    policy: &turbo_dom_parser::rtdom::cascade::StylePolicy,
) -> bool {
    if tree.node_type_id(node) != 1 {
        return false;
    }
    let attribute = |name: &str| tree.get_attribute(node, name);
    let native = match (tree.namespace_id(node), tree.local_name(node)) {
        (0, Some("a")) => attribute("href").is_some(),
        (0, Some("button" | "select" | "textarea")) => true,
        (0, Some("input")) => {
            !attribute("type").is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
        }
        (0, Some("iframe")) => true,
        (0, Some("audio" | "video")) => attribute("controls").is_some(),
        (0, Some("dialog")) => attribute("open").is_some(),
        (0, Some("summary")) => tree.parent(node).is_some_and(|parent| {
            html(tree, parent, "details")
                && tree
                    .children(parent)
                    .into_iter()
                    .find(|&child| html(tree, child, "summary"))
                    == Some(node)
        }),
        (_, Some("a")) => {
            attribute("href").is_some()
                || tree
                    .get_attribute_ns(node, Some("http://www.w3.org/1999/xlink"), "href")
                    .is_some()
        }
        _ => false,
    };
    let tabindex = attribute("tabindex").is_some_and(|value| {
        let value = value.trim_matches(|c: char| c.is_ascii_whitespace());
        let digits = value.strip_prefix(['-', '+']).unwrap_or(value);
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    });
    let editable = tree.namespace_id(node) == 0
        && attribute("contenteditable").is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "" | "true" | "plaintext-only"
            )
        });
    if !(native || tabindex || editable) || html_invokers::disabled_control(node) {
        return false;
    }
    if hidden_visibility(tree, node, policy) {
        return false;
    }
    // Rendered and not inert up to the dialog: closed dialogs render nothing, closed details
    // render only their summary.
    let mut child = None;
    let mut current = Some(node);
    while let Some(element) = current {
        if tree.node_type_id(element) == 1 {
            if tree.namespace_id(element) == 0 && tree.has_attribute(element, "inert") {
                return false;
            }
            if suppressed(tree, element, policy) {
                return false;
            }
            if element != node
                && html(tree, element, "dialog")
                && element != dialog
                && !tree.has_attribute(element, "open")
            {
                return false;
            }
            if html(tree, element, "details") && !tree.has_attribute(element, "open") {
                let summary = tree
                    .children(element)
                    .into_iter()
                    .find(|&candidate| html(tree, candidate, "summary"));
                if child.is_some() && child != summary {
                    return false;
                }
            }
        }
        if element == dialog {
            break;
        }
        child = Some(element);
        current = tree.parent(element).or_else(|| tree.shadow_host(element));
    }
    true
}

/// Flat-tree-ish descendants: a shadow host's shadow tree precedes its light children.
fn descendants(tree: &Tree, root: Handle) -> Vec<Handle> {
    let mut out = Vec::new();
    let mut stack = Vec::new();
    let push_children = |stack: &mut Vec<Handle>, node: Handle| {
        let mut children = tree.children(node);
        if let Some(shadow) = tree.shadow_root(node) {
            let mut shadowed = tree.children(shadow);
            shadowed.append(&mut children);
            children = shadowed;
        }
        stack.extend(children.into_iter().rev());
    };
    push_children(&mut stack, root);
    while let Some(node) = stack.pop() {
        out.push(node);
        push_children(&mut stack, node);
    }
    out
}

/// The dialog focusing steps' control: the first focusable autofocus descendant, else the
/// first focusable descendant, else the dialog itself (Chrome ignores autofocus on the dialog).
fn focus_delegate(scope: &mut v8::PinScope, dialog: Handle) -> Handle {
    let policy = css_policy::cascade_policy(scope);
    with_tree(|tree| {
        let candidates = descendants(tree, dialog)
            .into_iter()
            .filter(|&node| tree.node_type_id(node) == 1)
            .collect::<Vec<_>>();
        candidates
            .iter()
            .copied()
            .find(|&node| {
                tree.has_attribute(node, "autofocus") && focusable(tree, node, dialog, &policy)
            })
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|&node| focusable(tree, node, dialog, &policy))
            })
            .unwrap_or(dialog)
    })
    .unwrap_or(dialog)
}

fn focused() -> Option<Handle> {
    ACTIVE
        .with(|active| *active.borrow())
        .filter(|&node| with_tree(|tree| is_connected(tree, node)).unwrap_or(false))
}

fn dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
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
        "modal" => rv.set_bool(modal(node)),
        "set-modal" => set_modal(node, args.get(2).is_true()),
        "popover" => rv.set_bool(html_popover::opened(node)),
        "active" => rv.set_bool(DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .is_some_and(|dom| dom.browsing_context)
        })),
        "hide-popovers" => html_popover::hide_unrelated(scope, node),
        "opened" => user_activation_gates::dialog_opened(node),
        "closed" => user_activation_gates::dialog_closed(node),
        "schedule" => {
            if let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(2)) {
                html_popover::schedule(scope, callback);
            }
        }
        "focused" => match focused() {
            Some(active) => rv.set(wrap(scope, active).into()),
            None => rv.set(v8::null(scope).into()),
        },
        "focus-within" => {
            let inside = focused().is_some_and(|active| {
                with_tree(|tree| within(tree, active, node)).unwrap_or(false)
            });
            rv.set_bool(inside);
        }
        "delegate" => {
            let delegate = focus_delegate(scope, node);
            rv.set(wrap(scope, delegate).into());
        }
        _ => {}
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__dialogNative", dispatch);
    run_js(scope, include_str!("html_dialog.js"));
}

#[derive(Clone, Copy, PartialEq)]
enum Pseudo {
    Modal,
    Open,
    /// Every element in the document's fullscreen stack.
    Fullscreen,
    /// A complex selector a forgiving `:is()`/`:where()` list drops for an invalid
    /// `:modal()`/`:open()`: it can never match.
    Never,
}

fn pseudo_kind(name: &str) -> Option<Pseudo> {
    if name.eq_ignore_ascii_case("modal") {
        Some(Pseudo::Modal)
    } else if name.eq_ignore_ascii_case("open") {
        Some(Pseudo::Open)
    } else if name.eq_ignore_ascii_case("fullscreen") {
        Some(Pseudo::Fullscreen)
    } else {
        None
    }
}

/// Each comma-separated item of a forgiving list is parsed on its own; an invalid item is
/// replaced rather than invalidating the whole selector.
fn forgiving_list<'i, 't>(
    input: &mut cssparser::Parser<'i, 't>,
    ranges: &mut Vec<(usize, usize, Pseudo)>,
) -> Result<(), cssparser::ParseError<'i, ()>> {
    loop {
        let start = input.position().byte_index();
        let mut item = Vec::new();
        let parsed = input.parse_until_before(cssparser::Delimiter::Comma, |nested| {
            pseudo_ranges(nested, &mut item)
        });
        match parsed {
            Ok(()) => ranges.append(&mut item),
            Err(_) => ranges.push((start, input.position().byte_index(), Pseudo::Never)),
        }
        if input.is_exhausted() {
            return Ok(());
        }
        input.next()?;
    }
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
            Token::Ident(ref name) => {
                if let (false, Some(start), Some(pseudo)) = (double_colon, colon, pseudo_kind(name))
                {
                    ranges.push((start, input.position().byte_index(), pseudo));
                }
                colon = None;
                double_colon = false;
            }
            // `:modal()` / `:open()` are invalid selectors.
            Token::Function(ref name) if colon.is_some() && pseudo_kind(name).is_some() => {
                return Err(input.new_custom_error(()));
            }
            Token::Function(ref name)
                if colon.is_some()
                    && (name.eq_ignore_ascii_case("is") || name.eq_ignore_ascii_case("where")) =>
            {
                colon = None;
                input.parse_nested_block(|nested| forgiving_list(nested, ranges))?;
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

/// CSS closes blocks left open at EOF; close them explicitly for selector validation.
fn close_blocks(source: &str) -> String {
    let mut open = Vec::new();
    let mut chars = source.chars();
    let mut quote = None;
    while let Some(c) = chars.next() {
        match (quote, c) {
            (_, '\\') => {
                chars.next();
            }
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => (),
            (None, '"' | '\'') => quote = Some(c),
            (None, '/') if chars.as_str().starts_with('*') => {
                let rest = chars.as_str();
                chars = rest[rest.find("*/").map_or(rest.len(), |end| end + 2)..].chars();
            }
            (None, '(') => open.push(')'),
            (None, '[') => open.push(']'),
            (None, ')' | ']') => {
                open.pop();
            }
            _ => (),
        }
    }
    let mut result = source.to_owned();
    result.extend(open.into_iter().rev());
    result
}

/// Represent private dialog state as structural selectors within this exact DOM root, as
/// `:popover-open` does. `:open` is the dialog/details open attribute (no select picker is
/// ever shown); `:modal` is the is-modal flag, which outlives a removed `open` attribute.
pub(super) fn selector(scope: &mut v8::PinScope, anchor: Handle, source: &str) -> Option<String> {
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut ranges = Vec::new();
    // Only an invalid `:modal()`/`:open()` outside every forgiving list fails the parse.
    if pseudo_ranges(&mut parser, &mut ranges).is_err() {
        throw_dom_error(scope, "SyntaxError", "Invalid selector");
        return None;
    }
    if ranges.is_empty() {
        return Some(source.to_owned());
    }
    let fullscreen_stack = user_activation_gates::fullscreen_elements();
    let (modal, open, fullscreen) = DOM.with(|dom| {
        let dom = dom.borrow();
        let dom = dom.as_ref().unwrap();
        let tree = &dom.tree;
        let root = html_popover::selector_root(tree, anchor);
        let replacement = |paths: Vec<String>| {
            if paths.is_empty() {
                ":not(*)".to_owned()
            } else {
                format!(":is({})", paths.join(","))
            }
        };
        let modal = dom
            .dialog
            .modal
            .iter()
            .filter(|&&entry| html_popover::selector_root(tree, entry) == root)
            .map(|&entry| html_popover::selector_path(tree, entry))
            .collect::<Vec<_>>();
        let mut open = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if (html(tree, node, "dialog") || html(tree, node, "details"))
                && tree.get_attribute_ns(node, None, "open").is_some()
            {
                open.push(html_popover::selector_path(tree, node));
            }
            stack.extend(tree.children(node));
        }
        let fullscreen = fullscreen_stack
            .iter()
            .filter(|&&entry| html_popover::selector_root(tree, entry) == root)
            .map(|&entry| html_popover::selector_path(tree, entry))
            .collect::<Vec<_>>();
        (
            replacement(modal),
            replacement(open),
            replacement(fullscreen),
        )
    });
    let mut result = String::new();
    let mut start = 0;
    for (from, to, pseudo) in ranges {
        result.push_str(&source[start..from]);
        result.push_str(match pseudo {
            Pseudo::Modal => &modal,
            Pseudo::Open => &open,
            Pseudo::Fullscreen => &fullscreen,
            Pseudo::Never => ":not(*)",
        });
        start = to;
    }
    result.push_str(&source[start..]);
    // Validate what the engine will match, after dropping forgiven invalid arguments.
    if !cssom_parser::valid_selector(&close_blocks(&result)) {
        throw_dom_error(scope, "SyntaxError", "Invalid selector");
        return None;
    }
    Some(result)
}

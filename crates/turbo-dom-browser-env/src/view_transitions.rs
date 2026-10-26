//! Native view-transition state: the transition roots that `:active-view-transition` and
//! `:active-view-transition-type()` match, the capture-time checks Chrome runs before taking
//! the old and new snapshots (a transition root needs a non-inline box, and rendered
//! `view-transition-name`s must be unique), and the host rendering seam the lifecycle runs on.
use super::*;
use turbo_dom_parser::rtdom::cascade::{
    computed_style_with_policy, get_property_value, StylePolicy,
};

#[derive(Default)]
pub(super) struct State {
    /// Transition roots of active transitions, each with its transition's current types.
    active: Vec<(Handle, Vec<String>)>,
    /// Host rendering capability `host(command, argument)`; see `capture_host`.
    host: Option<v8::Weak<v8::Function>>,
}

impl State {
    pub(super) fn sweep_storage(&mut self, tree: &Tree) {
        self.active.retain(|(node, _)| tree.is_live(*node));
    }
}

/// Capture the host's rendering seam. `host("frame", step)` runs `step(timestamp)` in the
/// next rendering update after that frame's animation frame callbacks; `host("suppress",
/// flag)` pauses or resumes the document's rendering updates (rendering suppression for view
/// transitions); `host("hidden")` reports whether the page is hidden. Bound to the host
/// function's own realm document, like the popover scheduler. Without a host, frames are
/// 16ms trusted tasks stamped with the realm clock, nothing suppresses rendering and the
/// page is visible.
pub(super) fn capture_host(scope: &mut v8::PinScope, host: v8::Local<v8::Function>) {
    // Bind to the seam's own realm: a frame bootstrap can run while its parent context is
    // current, and a parent-held child closure would keep a retired frame realm alive.
    let owner = host
        .get_creation_context(scope)
        .and_then(|context| context.get_slot::<DomContextId>().map(|id| id.0))
        .unwrap_or(0);
    let _dom = DomScope::enter(owner);
    let Some(host) = dom_ownership::helper_function(scope, host) else {
        return;
    };
    DOM.with(|dom| {
        if let Some(dom) = dom.borrow_mut().as_mut() {
            dom.view_transitions.host = Some(host);
        }
    });
}

fn host_command<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    command: v8::Local<'s, v8::Value>,
    argument: v8::Local<'s, v8::Value>,
) -> Option<v8::Local<'s, v8::Value>> {
    let host = DOM.with(|dom| dom.borrow().as_ref()?.view_transitions.host.clone())?;
    let host = dom_ownership::local(scope, &host);
    let receiver = v8::undefined(scope);
    host.call(scope, receiver.into(), &[command, argument])
}

fn html(tree: &Tree, node: Handle, name: &str) -> bool {
    tree.node_type_id(node) == 1
        && tree.namespace_id(node) == 0
        && tree.local_name(node) == Some(name)
}

#[derive(Clone, Copy, PartialEq)]
enum Display {
    None,
    Contents,
    Inline,
    /// An atomic inline: a replaced element or an inline-level formatting context root.
    Atomic,
    Ruby,
    /// table-row, row-group, header/footer group, column and column-group boxes.
    TableTrack,
    Block,
}

/// The UA stylesheet's display for an element without an authored one.
fn ua_display(tree: &Tree, node: Handle) -> Display {
    let Some(name) = tree.local_name(node) else {
        return Display::Inline;
    };
    match tree.namespace_id(node) {
        0 => {}
        // An outermost <svg> is a replaced element; SVG content has no CSS box of its own.
        _ if name == "svg" => {
            let parent_svg = tree.parent(node).is_some_and(|parent| {
                tree.node_type_id(parent) == 1 && tree.namespace_id(parent) != 0
            });
            return if parent_svg {
                Display::None
            } else {
                Display::Atomic
            };
        }
        _ => return Display::Inline,
    }
    if tree.has_attribute(node, "hidden") {
        return Display::None;
    }
    match name {
        "head" | "script" | "style" | "title" | "meta" | "link" | "base" | "template"
        | "noscript" | "datalist" | "param" | "area" | "source" | "track" | "noembed"
        | "noframes" | "rp" => Display::None,
        "dialog" if !tree.has_attribute(node, "open") => Display::None,
        "audio" if !tree.has_attribute(node, "controls") => Display::None,
        "input"
            if tree
                .get_attribute(node, "type")
                .is_some_and(|kind| kind.eq_ignore_ascii_case("hidden")) =>
        {
            Display::None
        }
        "slot" => Display::Contents,
        "html" | "body" | "address" | "article" | "aside" | "blockquote" | "center" | "dialog"
        | "div" | "figure" | "figcaption" | "footer" | "form" | "header" | "hgroup" | "hr"
        | "legend" | "listing" | "main" | "menu" | "nav" | "ol" | "p" | "plaintext" | "pre"
        | "search" | "section" | "ul" | "xmp" | "dl" | "dd" | "dt" | "fieldset" | "details"
        | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "optgroup" | "option" | "frameset" | "li"
        | "summary" | "table" | "caption" | "td" | "th" => Display::Block,
        "tr" | "tbody" | "thead" | "tfoot" | "col" | "colgroup" => Display::TableTrack,
        "ruby" => Display::Ruby,
        "img" | "canvas" | "video" | "audio" | "iframe" | "embed" | "object" | "input"
        | "button" | "select" | "textarea" | "meter" | "progress" | "marquee" => Display::Atomic,
        _ => Display::Inline,
    }
}

fn authored_display(value: &str) -> Option<Display> {
    let value = value.trim().to_ascii_lowercase();
    let keywords = value.split_ascii_whitespace().collect::<Vec<_>>();
    Some(match keywords.as_slice() {
        [] => return None,
        ["none"] => Display::None,
        ["contents"] => Display::Contents,
        ["inline"] | ["inline", "flow"] | ["flow", "inline"] => Display::Inline,
        ["ruby"] | ["inline", "ruby"] | ["ruby", "inline"] => Display::Ruby,
        ["table-row" | "table-row-group" | "table-header-group" | "table-footer-group"
        | "table-column" | "table-column-group"] => Display::TableTrack,
        ["ruby-base" | "ruby-text" | "ruby-base-container" | "ruby-text-container"] => {
            Display::Inline
        }
        [first, ..] if first.starts_with("inline") => Display::Atomic,
        _ => Display::Block,
    })
}

/// Boxes as the cascade lays them out: the authored or UA display, blockified for floats,
/// absolutely positioned boxes and flex/grid items.
struct Boxes<'a> {
    tree: &'a Tree,
    policy: &'a StylePolicy,
    /// The document element, whose box is always blockified.
    document_element: Option<Handle>,
}

impl Boxes<'_> {
    fn style(&self, node: Handle) -> HashMap<String, String> {
        computed_style_with_policy(self.tree, node, self.policy)
    }
    fn specified(&self, node: Handle, style: &HashMap<String, String>) -> Display {
        let value = get_property_value(style, "display");
        match value.trim().to_ascii_lowercase().as_str() {
            "inherit" => self
                .tree
                .parent(node)
                .filter(|&parent| self.tree.node_type_id(parent) == 1)
                .map_or(Display::Inline, |parent| {
                    let style = self.style(parent);
                    self.specified(parent, &style)
                }),
            "initial" | "unset" => Display::Inline,
            "revert" | "revert-layer" | "" => ua_display(self.tree, node),
            other => authored_display(other).unwrap_or_else(|| ua_display(self.tree, node)),
        }
    }
    fn display(&self, node: Handle, style: &HashMap<String, String>) -> Display {
        let display = self.specified(node, style);
        if !matches!(
            display,
            Display::Inline | Display::Ruby | Display::TableTrack
        ) {
            return display;
        }
        let positioned = matches!(
            get_property_value(style, "position")
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "absolute" | "fixed"
        );
        let floated = matches!(
            get_property_value(style, "float")
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "left" | "right" | "inline-start" | "inline-end"
        );
        let item = self
            .tree
            .parent(node)
            .filter(|&parent| self.tree.node_type_id(parent) == 1)
            .is_some_and(|parent| {
                let style = self.style(parent);
                let value = get_property_value(&style, "display").to_ascii_lowercase();
                value.contains("flex") || value.contains("grid")
            });
        if positioned || floated || item || Some(node) == self.document_element {
            Display::Block
        } else {
            display
        }
    }
    /// Whether `node` lies outside every display:none subtree, counting closed <details>
    /// (which render only their first summary).
    fn rendered(&self, node: Handle) -> bool {
        let mut child = None;
        let mut current = Some(node);
        while let Some(element) = current {
            if self.tree.node_type_id(element) != 1 {
                break;
            }
            let style = self.style(element);
            if self.specified(element, &style) == Display::None {
                return false;
            }
            if child.is_some()
                && html(self.tree, element, "details")
                && !self.tree.has_attribute(element, "open")
            {
                let summary = self
                    .tree
                    .children(element)
                    .into_iter()
                    .find(|&candidate| html(self.tree, candidate, "summary"));
                if child != summary {
                    return false;
                }
            }
            child = Some(element);
            current = self.tree.parent(element);
        }
        true
    }
}

/// Chrome's capture-time validation for a transition rooted at `root`: `"layout"` when an
/// element-scoped root has no box that can be captured, `"names"` when two rendered
/// elements in the scope share a `view-transition-name`, else `""`. Shadow-tree content
/// takes no part in a document-level capture.
fn capture(scope: &mut v8::PinScope, root: Handle, document_scope: bool) -> &'static str {
    let policy = css_policy::cascade_policy(scope);
    with_tree(|tree| {
        if !is_connected(tree, root) {
            return "layout";
        }
        let boxes = Boxes {
            tree,
            policy: &policy,
            document_element: tree
                .parent(root)
                .filter(|&parent| tree.node_type_id(parent) == 9)
                .map(|_| root),
        };
        if !boxes.rendered(root) {
            return "layout";
        }
        if !document_scope {
            let style = boxes.style(root);
            if matches!(
                boxes.display(root, &style),
                Display::None
                    | Display::Contents
                    | Display::Inline
                    | Display::Ruby
                    | Display::TableTrack
            ) {
                return "layout";
            }
        }
        let mut names = std::collections::HashSet::new();
        let mut stack = vec![(root, None::<Handle>)];
        while let Some((node, parent)) = stack.pop() {
            let style = boxes.style(node);
            let display = boxes.specified(node, &style);
            if display == Display::None {
                continue;
            }
            if let Some(parent) = parent.filter(|&parent| {
                html(tree, parent, "details") && !tree.has_attribute(parent, "open")
            }) {
                let summary = tree
                    .children(parent)
                    .into_iter()
                    .find(|&candidate| html(tree, candidate, "summary"));
                if summary != Some(node) {
                    continue;
                }
            }
            if display != Display::Contents {
                let mut name = get_property_value(&style, "view-transition-name")
                    .trim()
                    .to_owned();
                if name.is_empty() && document_scope && node == root {
                    name = "root".into();
                }
                if !matches!(name.as_str(), "" | "none" | "auto" | "match-element")
                    && !names.insert(name)
                {
                    return "names";
                }
            }
            for child in tree.children(node).into_iter().rev() {
                if tree.node_type_id(child) == 1 {
                    stack.push((child, Some(node)));
                }
            }
        }
        ""
    })
    .unwrap_or("layout")
}

fn set_active(node: Handle, types: Option<Vec<String>>) {
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let Some(dom) = dom.as_mut() else {
            return;
        };
        let active = &mut dom.view_transitions.active;
        active.retain(|(entry, _)| *entry != node);
        if let Some(types) = types {
            active.push((node, types));
        }
    });
}

fn strings(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<Vec<String>> {
    let array = v8::Local::<v8::Array>::try_from(value).ok()?;
    Some(
        (0..array.length())
            .filter_map(|index| array.get_index(scope, index))
            .map(|value| value.to_rust_string_lossy(scope))
            .collect(),
    )
}

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
        "inert" => rv.set_bool(!DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .is_some_and(|dom| dom.browsing_context)
        })),
        "capture" => {
            let status = capture(scope, node, args.get(2).is_true());
            if let Some(status) = v8::String::new(scope, status) {
                rv.set(status.into());
            }
        }
        "active" => {
            let types = strings(scope, args.get(2));
            set_active(node, types);
        }
        "exception" => {
            let name = args.get(2).to_rust_string_lossy(scope);
            let message = args.get(3).to_rust_string_lossy(scope);
            if let Some(error) = new_dom_exception(scope, &name, &message) {
                rv.set(error.into());
            }
        }
        "has-host" => rv.set_bool(DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .is_some_and(|dom| dom.view_transitions.host.is_some())
        })),
        "host" => {
            if let Some(value) = host_command(scope, args.get(2), args.get(3)) {
                rv.set(value);
            }
        }
        "schedule" => {
            if let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(2)) {
                let delay = args.get(3).uint32_value(scope).unwrap_or(0);
                rv.set_bool(html_popover::schedule_after(scope, callback, delay));
            }
        }
        _ => {}
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__viewTransitionNative", dispatch);
    run_js(scope, include_str!("view_transitions.js"));
}

#[derive(Clone, PartialEq)]
enum Pseudo {
    Active,
    Types(Vec<String>),
    /// A complex selector a forgiving list drops for an invalid argument: never matches.
    Never,
}

/// `:active-view-transition-type()` takes a non-empty comma list of custom idents.
fn type_list<'i, 't>(
    input: &mut cssparser::Parser<'i, 't>,
) -> Result<Vec<String>, cssparser::ParseError<'i, ()>> {
    input.parse_comma_separated(|item| {
        let location = item.current_source_location();
        let ident = item.expect_ident()?.to_string();
        if ident.eq_ignore_ascii_case("none")
            || [
                "initial",
                "inherit",
                "unset",
                "default",
                "revert",
                "revert-layer",
            ]
            .iter()
            .any(|keyword| ident.eq_ignore_ascii_case(keyword))
        {
            return Err(location.new_custom_error(()));
        }
        Ok(ident)
    })
}

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
                if let (false, Some(start)) = (double_colon, colon) {
                    if name.eq_ignore_ascii_case("active-view-transition") {
                        ranges.push((start, input.position().byte_index(), Pseudo::Active));
                    }
                }
                colon = None;
                double_colon = false;
            }
            Token::Function(ref name)
                if colon.is_some() && name.eq_ignore_ascii_case("active-view-transition") =>
            {
                return Err(input.new_custom_error(()));
            }
            Token::Function(ref name)
                if colon.is_some() && name.eq_ignore_ascii_case("active-view-transition-type") =>
            {
                let start = colon.take().unwrap_or(position);
                let types = input.parse_nested_block(|nested| type_list(nested))?;
                ranges.push((start, input.position().byte_index(), Pseudo::Types(types)));
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

/// Represent active-transition state as structural selectors within this exact DOM root,
/// as `:popover-open` and `:modal` do: `:active-view-transition` is every active transition
/// root, `:active-view-transition-type(a, b)` those whose types contain `a` or `b`.
pub(super) fn selector(scope: &mut v8::PinScope, anchor: Handle, source: &str) -> Option<String> {
    if !source
        .to_ascii_lowercase()
        .contains("active-view-transition")
    {
        return Some(source.to_owned());
    }
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut ranges = Vec::new();
    if pseudo_ranges(&mut parser, &mut ranges).is_err() {
        throw_dom_error(scope, "SyntaxError", "Invalid selector");
        return None;
    }
    if ranges.is_empty() {
        return Some(source.to_owned());
    }
    let replacement = |matching: &dyn Fn(&[String]) -> bool| {
        DOM.with(|dom| {
            let dom = dom.borrow();
            let dom = dom.as_ref().unwrap();
            let tree = &dom.tree;
            let root = html_popover::selector_root(tree, anchor);
            let paths = dom
                .view_transitions
                .active
                .iter()
                .filter(|(node, types)| {
                    matching(types) && html_popover::selector_root(tree, *node) == root
                })
                .map(|(node, _)| html_popover::selector_path(tree, *node))
                .collect::<Vec<_>>();
            if paths.is_empty() {
                ":not(*)".to_owned()
            } else {
                format!(":is({})", paths.join(","))
            }
        })
    };
    let mut result = String::new();
    let mut start = 0;
    for (from, to, pseudo) in ranges {
        result.push_str(&source[start..from]);
        result.push_str(&match pseudo {
            Pseudo::Active => replacement(&|_| true),
            Pseudo::Types(wanted) => {
                replacement(&|types| types.iter().any(|value| wanted.contains(value)))
            }
            Pseudo::Never => ":not(*)".to_owned(),
        });
        start = to;
    }
    result.push_str(&source[start..]);
    if !cssom_parser::valid_selector(&result) {
        throw_dom_error(scope, "SyntaxError", "Invalid selector");
        return None;
    }
    Some(result)
}

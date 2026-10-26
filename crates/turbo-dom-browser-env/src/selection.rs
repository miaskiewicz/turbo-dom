//! Selection API natives. Trusted `selectionchange`/`select` dispatch and task scheduling,
//! browsing-context and focus facts, live tracking of the frame selection's anchor/focus
//! (Blink `SelectionEditor` rules, see range_live.rs) and a layout-free visible-text model.
//!
//! The model stands in for LayoutNG's inline text content: per inline formatting context it
//! applies CSS white-space collapsing, block/inline flow, `<br>`/preserved newlines, atomic
//! replaced elements and display/visibility, then exposes each caret slot's canonical DOM
//! position (Blink's most-backward visually equivalent candidate). It drives caret movement,
//! visible-selection canonicalization and Selection.toString's TextIterator plain text.
//! Geometric line wrapping is not modelled: a line is a whole inline formatting context.
use super::*;
use std::collections::HashSet;

struct Hooks {
    focus: v8::Weak<v8::Function>,
    textarea: v8::Weak<v8::Function>,
    input_type: v8::Weak<v8::Function>,
    removed: v8::Weak<v8::Function>,
}

thread_local! {
    /// (DOM id, control) pairs with live value ranges (OpaqueRange) awaiting removal.
    static WATCHED: RefCell<HashSet<(u64, Handle)>> = RefCell::new(HashSet::new());
}

pub(super) fn clear() {
    // Teardown can run while thread-locals are being destroyed.
    let _ = WATCHED.try_with(|set| set.borrow_mut().clear());
}

/// `node` is about to leave its parent: disconnect value ranges of watched controls inside.
pub(super) fn removing(scope: &mut v8::PinScope, node: Handle) {
    let Ok(dom) = DOM_ID.try_with(|id| id.get()) else {
        return;
    };
    let watched: Vec<Handle> = WATCHED
        .try_with(|set| {
            set.borrow()
                .iter()
                .filter(|(id, _)| *id == dom)
                .map(|(_, handle)| *handle)
                .collect()
        })
        .unwrap_or_default();
    if watched.is_empty() {
        return;
    }
    let inside: Vec<Handle> = with_tree(|tree| {
        watched
            .into_iter()
            .filter(|&control| {
                let mut current = Some(control);
                while let Some(handle) = current {
                    if handle == node {
                        return true;
                    }
                    current = tree.parent(handle).or_else(|| tree.shadow_host(handle));
                }
                false
            })
            .collect()
    })
    .unwrap_or_default();
    if inside.is_empty() {
        return;
    }
    let Some(hooks) = scope.get_current_context().get_slot::<Hooks>() else {
        return;
    };
    for control in inside {
        let _ = WATCHED.try_with(|set| set.borrow_mut().remove(&(dom, control)));
        let Some(hook) = hooks.removed.to_local(scope) else {
            return;
        };
        let element = wrap(scope, control);
        let receiver = v8::undefined(scope);
        hook.call(scope, receiver.into(), &[element.into()]);
    }
}

/// An input's `type` attribute changed: its inner editor and cached selection are rebuilt.
pub(super) fn type_changed(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) {
    let Some(hooks) = scope.get_current_context().get_slot::<Hooks>() else {
        return;
    };
    let Some(hook) = hooks.input_type.to_local(scope) else {
        return;
    };
    let receiver = v8::undefined(scope);
    hook.call(scope, receiver.into(), &[element.into()]);
}

/// A non-dirty textarea's children are about to change: its selection clamps to the new
/// default value (`reset`: every child is replaced, so it first clamps to zero).
pub(super) fn textarea_children(scope: &mut v8::PinScope, parent: Handle, reset: bool) {
    let textarea = with_tree(|tree| {
        tree.node_type_id(parent) == 1
            && tree.namespace_id(parent) == 0
            && tree.local_name(parent) == Some("textarea")
    })
    .unwrap_or(false);
    if !textarea {
        return;
    }
    let Some(hooks) = scope.get_current_context().get_slot::<Hooks>() else {
        return;
    };
    let Some(hook) = hooks.textarea.to_local(scope) else {
        return;
    };
    let element = wrap(scope, parent);
    let reset = v8::Boolean::new(scope, reset);
    let receiver = v8::undefined(scope);
    hook.call(scope, receiver.into(), &[element.into(), reset.into()]);
}

/// Blink focusability for script focus of a form control: connected, enabled, rendered.
fn focusable(scope: &mut v8::PinScope, node: Handle) -> bool {
    let policy = css_policy::cascade_policy(scope);
    with_tree(|tree| {
        if !is_connected(tree, node) {
            return false;
        }
        let disabled = |element: Handle| {
            tree.namespace_id(element) == 0
                && matches!(
                    html_text::tag(tree, element),
                    "input" | "textarea" | "select" | "button" | "fieldset"
                )
                && tree.has_attribute(element, "disabled")
        };
        if disabled(node) {
            return false;
        }
        let mut current = tree.parent(node);
        let mut child = node;
        while let Some(ancestor) = current {
            if tree.node_type_id(ancestor) == 1
                && html_text::tag(tree, ancestor) == "fieldset"
                && disabled(ancestor)
            {
                let first_legend = tree.children(ancestor).into_iter().find(|&candidate| {
                    tree.node_type_id(candidate) == 1 && html_text::tag(tree, candidate) == "legend"
                });
                if first_legend != Some(child) {
                    return false;
                }
            }
            child = ancestor;
            current = tree.parent(ancestor).or_else(|| tree.shadow_host(ancestor));
        }
        let mut element = Some(node);
        while let Some(handle) = element {
            if tree.node_type_id(handle) == 1 {
                let style = turbo_dom_parser::rtdom::cascade::computed_style_with_policy(
                    tree, handle, &policy,
                );
                if display_of(tree, handle, &style) == "none" {
                    return false;
                }
            }
            element = tree.parent(handle).or_else(|| tree.shadow_host(handle));
        }
        true
    })
    .unwrap_or(false)
}

/// Run the selection focus steps (text-control selection restore) after `element` gained focus.
pub(super) fn focused(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) {
    let current = scope.get_current_context();
    let hooks = current.get_slot::<Hooks>().or_else(|| {
        element
            .get_creation_context(scope)
            .and_then(|context| context.get_slot::<Hooks>())
    });
    let Some(hooks) = hooks else {
        return;
    };
    let Some(hook) = hooks.focus.to_local(scope) else {
        return;
    };
    let receiver = v8::undefined(scope);
    hook.call(scope, receiver.into(), &[element.into()]);
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Block-level box: separate inline formatting context, newline in TextIterator.
    Block { body: bool, extra: bool },
    /// Table rows emit newlines and separate cells' contexts.
    Row,
    /// Tables: positions next to them stay outside (they are caret candidates).
    Table,
    /// Table cells are tab-delimited.
    Cell,
    /// Atomic inline-level boxes (inline-block, table sections) separate contexts silently.
    Silent,
}

#[derive(Clone, Copy)]
enum Piece {
    /// A rendered UTF-16 unit of a text node (collapsed white space is a single space).
    Text {
        node: Handle,
        offset: usize,
        unit: u16,
        collapsible: bool,
    },
    /// A replaced element (U+FFFC in text content).
    Atomic { node: Handle },
    /// A `<br>` (newline in text content).
    Break { node: Handle },
}

#[derive(Clone, Copy)]
enum Event {
    Start(Handle, Kind),
    End(Kind),
    Piece(Piece),
    Marker(usize),
    /// The caret position just before (false) or after (true) a table: its own segment.
    Edge(Handle, bool),
}

struct Flow<'t> {
    tree: &'t Tree,
    policy: &'t turbo_dom_parser::rtdom::cascade::StylePolicy,
    events: Vec<Event>,
    queries: Vec<(Handle, usize)>,
    /// Text-content state of the current inline formatting context.
    line_start: bool,
    space: bool,
    /// Hosts whose slot assignments were collected, and the light nodes assigned to slots.
    assigned: HashSet<Handle>,
    slots: HashSet<Handle>,
}

fn style(flow: &Flow, node: Handle) -> HashMap<String, String> {
    turbo_dom_parser::rtdom::cascade::computed_style_with_policy(flow.tree, node, flow.policy)
}

fn display_of(tree: &Tree, node: Handle, style: &HashMap<String, String>) -> String {
    if let Some(value) = style.get("display") {
        return value.clone();
    }
    match html_text::tag(tree, node) {
        "html" | "body" | "address" | "center" | "dir" | "menu" | "hgroup" | "search"
        | "listing" | "plaintext" | "xmp" => "block".into(),
        "thead" | "tbody" | "tfoot" => "table-row-group".into(),
        _ => html_text::display(tree, node, style),
    }
}

/// Case transforms that keep one UTF-16 unit (layout text for TextIterator).
fn transformed(unit: u16, transform: Option<&str>) -> u16 {
    let Some(character) = char::from_u32(u32::from(unit)) else {
        return unit;
    };
    let mapped: Vec<char> = match transform {
        Some("uppercase") => character.to_uppercase().collect(),
        Some("lowercase") => character.to_lowercase().collect(),
        _ => return unit,
    };
    match mapped.as_slice() {
        [single] if (*single as u32) < 0x10000 => *single as u16,
        _ => unit,
    }
}

fn atomic(tree: &Tree, node: Handle) -> bool {
    tree.namespace_id(node) == 0
        && matches!(
            html_text::tag(tree, node),
            "img"
                | "input"
                | "textarea"
                | "select"
                | "button"
                | "video"
                | "audio"
                | "canvas"
                | "iframe"
                | "object"
                | "embed"
                | "meter"
                | "progress"
        )
        || html_text::tag(tree, node) == "svg"
}

impl Flow<'_> {
    fn markers(&mut self, container: Handle, offset: usize) {
        for index in 0..self.queries.len() {
            if self.queries[index] == (container, offset) {
                self.events.push(Event::Marker(index));
            }
        }
    }
    /// Trailing collapsible spaces at the end of a line are not rendered.
    fn end_line(&mut self) {
        let mut index = self.events.len();
        while index > 0 {
            match self.events[index - 1] {
                Event::Marker(_) => index -= 1,
                Event::Piece(Piece::Text {
                    collapsible: true,
                    unit: 32,
                    ..
                }) => {
                    self.events.remove(index - 1);
                    index -= 1;
                }
                _ => break,
            }
        }
        self.line_start = true;
        self.space = false;
    }
    fn piece(&mut self, piece: Piece) {
        self.events.push(Event::Piece(piece));
    }
    /// DOM-tree iteration within one tree scope (Blink's PositionIterator never enters shadow
    /// roots): a shadow host renders only its slotted light children; a slot renders its
    /// fallback children only when nothing is assigned to it.
    fn children(&mut self, node: Handle, visible: bool) {
        let tree = self.tree;
        let kids = tree.children(node);
        let host = tree.shadow_root(node).is_some();
        let fallback_hidden = tree.local_name(node) == Some("slot")
            && tree.shadow_root_of(node).is_some()
            && !tree.assigned_nodes(node).is_empty();
        for (index, child) in kids.iter().enumerate() {
            self.markers(node, index);
            let rendered = visible && !fallback_hidden && (!host || self.slotted(node, *child));
            self.walk(*child, rendered);
        }
        self.markers(node, kids.len());
    }
    fn slotted(&mut self, host: Handle, child: Handle) -> bool {
        if !self.assigned.contains(&host) {
            self.assigned.insert(host);
            let tree = self.tree;
            if let Some(root) = tree.shadow_root(host) {
                for slot in tree.descendants(root) {
                    if tree.local_name(slot) == Some("slot") {
                        self.slots.extend(tree.assigned_nodes(slot));
                    }
                }
            }
        }
        self.slots.contains(&child)
    }
    fn walk(&mut self, node: Handle, visible: bool) {
        let tree = self.tree;
        match tree.node_type_id(node) {
            3 | 4 => self.text(node, visible),
            1 => self.element(node, visible),
            9 | 11 => self.children(node, visible),
            _ => {}
        }
    }
    fn text(&mut self, node: Handle, visible: bool) {
        let tree = self.tree;
        let units = tree.node_value_utf16(node).unwrap_or_default();
        let owner = tree.parent(node);
        let (shown, mode) = match owner.filter(|_| visible) {
            Some(owner) if tree.node_type_id(owner) == 1 => {
                let style = style(self, owner);
                let hidden = matches!(
                    style.get("visibility").map(String::as_str),
                    Some("hidden" | "collapse")
                );
                (!hidden, html_text::whitespace(tree, owner, &style))
            }
            Some(_) => (true, "normal".to_string()),
            None => (false, String::new()),
        };
        let transform = owner
            .filter(|&owner| visible && tree.node_type_id(owner) == 1)
            .and_then(|owner| style(self, owner).get("text-transform").cloned());
        let preserve_spaces = matches!(mode.as_str(), "pre" | "pre-wrap" | "break-spaces");
        let preserve_lines = preserve_spaces || mode == "pre-line";
        for (offset, &unit) in units.iter().enumerate() {
            self.markers(node, offset);
            if !shown {
                continue;
            }
            if unit == 10 && preserve_lines {
                if !preserve_spaces {
                    self.end_line();
                }
                self.piece(Piece::Text {
                    node,
                    offset,
                    unit,
                    collapsible: false,
                });
                self.line_start = true;
                self.space = false;
            } else if !preserve_spaces && matches!(unit, 9 | 10 | 12 | 13 | 32) {
                if !self.line_start && !self.space {
                    self.piece(Piece::Text {
                        node,
                        offset,
                        unit: 32,
                        collapsible: true,
                    });
                    self.space = true;
                }
            } else {
                let unit = transformed(unit, transform.as_deref());
                self.piece(Piece::Text {
                    node,
                    offset,
                    unit,
                    collapsible: false,
                });
                self.line_start = false;
                self.space = false;
            }
        }
        self.markers(node, units.len());
    }
    fn element(&mut self, node: Handle, visible: bool) {
        let tree = self.tree;
        if !visible {
            return self.children(node, false);
        }
        let style = style(self, node);
        let display = display_of(tree, node, &style);
        if display == "none" || html_text::closed_details_child(tree, node) {
            return self.children(node, false);
        }
        let name = html_text::tag(tree, node);
        if tree.namespace_id(node) == 0 && name == "br" {
            self.end_line();
            let hidden = matches!(
                style.get("visibility").map(String::as_str),
                Some("hidden" | "collapse")
            );
            if !hidden {
                self.piece(Piece::Break { node });
            }
            self.line_start = true;
            return self.children(node, false);
        }
        if atomic(tree, node) {
            self.piece(Piece::Atomic { node });
            self.line_start = false;
            self.space = false;
            return self.children(node, false);
        }
        let kind = match display.as_str() {
            "inline" | "contents" | "ruby" | "ruby-text" => None,
            "table-cell" => Some(Kind::Cell),
            "table-row" => Some(Kind::Row),
            "table" => Some(Kind::Table),
            "inline-block" | "inline-flex" | "inline-grid" | "inline-table" | "table-row-group"
            | "table-header-group" | "table-footer-group" | "table-column"
            | "table-column-group" => Some(Kind::Silent),
            _ => Some(Kind::Block {
                body: name == "body",
                extra: tree.namespace_id(node) == 0 && name == "p",
            }),
        };
        let floating = style.get("float").is_some_and(|value| value != "none")
            || style
                .get("position")
                .is_some_and(|value| value == "absolute" || value == "fixed");
        let kind = kind.map(|kind| match kind {
            Kind::Block { .. } if floating => Kind::Silent,
            kind => kind,
        });
        if let Some(kind) = kind {
            self.end_line();
            if kind == Kind::Table {
                self.events.push(Event::Edge(node, false));
            }
            self.events.push(Event::Start(node, kind));
            self.children(node, true);
            self.end_line();
            self.events.push(Event::End(kind));
            if kind == Kind::Table {
                self.events.push(Event::Edge(node, true));
            }
        } else {
            self.children(node, true);
        }
    }
}

fn flow(
    tree: &Tree,
    policy: &turbo_dom_parser::rtdom::cascade::StylePolicy,
    root: Handle,
    queries: Vec<(Handle, usize)>,
) -> Vec<Event> {
    let mut flow = Flow {
        tree,
        policy,
        events: Vec::new(),
        queries,
        line_start: true,
        space: false,
        assigned: HashSet::new(),
        slots: HashSet::new(),
    };
    flow.walk(root, true);
    flow.end_line();
    flow.events
}

/// Segments: maximal piece runs between block boundaries (inline formatting contexts), plus
/// one empty segment per table edge.
struct Segments {
    pieces: Vec<Vec<Piece>>,
    edges: Vec<Option<(Handle, bool)>>,
    /// Per query: (segment, slot) or None when it lies outside every non-empty segment.
    queries: Vec<Option<(usize, usize)>>,
    /// Per query: the next non-empty segment index at or after its position.
    after: Vec<usize>,
}

fn segments(events: &[Event], count: usize) -> Segments {
    let mut pieces: Vec<Vec<Piece>> = vec![Vec::new()];
    let mut edges: Vec<Option<(Handle, bool)>> = vec![None];
    let mut raw = vec![(0usize, 0usize); count];
    for event in events {
        match event {
            Event::Start(..) | Event::End(..) => {
                if !pieces.last().unwrap().is_empty() || edges.last().unwrap().is_some() {
                    pieces.push(Vec::new());
                    edges.push(None);
                }
            }
            Event::Edge(node, after) => {
                if !pieces.last().unwrap().is_empty() || edges.last().unwrap().is_some() {
                    pieces.push(Vec::new());
                    edges.push(None);
                }
                *edges.last_mut().unwrap() = Some((*node, *after));
                pieces.push(Vec::new());
                edges.push(None);
            }
            Event::Piece(piece) => pieces.last_mut().unwrap().push(*piece),
            Event::Marker(index) => {
                raw[*index] = (pieces.len() - 1, pieces.last().unwrap().len());
            }
        }
    }
    while pieces.last().is_some_and(Vec::is_empty) && edges.last().is_some_and(Option::is_none) {
        pieces.pop();
        edges.pop();
    }
    let queries: Vec<Option<(usize, usize)>> = raw
        .iter()
        .map(|&(segment, slot)| {
            (segment < pieces.len()
                && (!pieces[segment].is_empty() || edges[segment].is_some_and(|(_, after)| !after)))
            .then_some((segment, slot))
        })
        .collect();
    let after: Vec<usize> = raw.iter().map(|&(segment, _)| segment).collect();
    // Empty non-edge segments carry no caret position.
    let mut kept_pieces = Vec::new();
    let mut kept_edges = Vec::new();
    let mut renumber = Vec::with_capacity(pieces.len() + 1);
    for (piece, edge) in pieces.into_iter().zip(edges) {
        renumber.push(kept_pieces.len());
        if !piece.is_empty() || edge.is_some() {
            kept_pieces.push(piece);
            kept_edges.push(edge);
        }
    }
    renumber.push(kept_pieces.len());
    let queries = queries
        .into_iter()
        .map(|query| query.map(|(segment, slot)| (renumber[segment], slot)))
        .collect();
    let after = after
        .into_iter()
        .map(|segment| renumber[segment.min(renumber.len() - 1)])
        .collect();
    Segments {
        pieces: kept_pieces,
        edges: kept_edges,
        queries,
        after,
    }
}

fn index_in_parent(tree: &Tree, node: Handle) -> (Handle, usize) {
    let parent = tree.parent(node).unwrap_or(node);
    let index = tree
        .children(parent)
        .iter()
        .position(|&child| child == node)
        .unwrap_or(0);
    (parent, index)
}

/// Canonical DOM position of `slot` in a segment (Blink's most-backward candidate).
fn slot_position(tree: &Tree, pieces: &[Piece], slot: usize) -> (Handle, usize) {
    let before = slot.checked_sub(1).map(|index| pieces[index]);
    let after = pieces.get(slot).copied();
    match (before, after) {
        (Some(Piece::Text { node, offset, .. }), _) => (node, offset + 1),
        (Some(Piece::Break { .. }), Some(Piece::Text { node, offset, .. })) => (node, offset),
        (Some(Piece::Atomic { node } | Piece::Break { node }), _) => {
            let (parent, index) = index_in_parent(tree, node);
            (parent, index + 1)
        }
        (None, Some(Piece::Text { node, offset, .. })) => (node, offset),
        (None, Some(Piece::Atomic { node } | Piece::Break { node })) => index_in_parent(tree, node),
        (None, None) => (tree.root(), 0),
    }
}

/// The most-forward visually equivalent position of `slot` (before the next piece).
fn slot_forward(tree: &Tree, pieces: &[Piece], slot: usize) -> (Handle, usize) {
    match pieces.get(slot) {
        Some(Piece::Text { node, offset, .. }) if slot > 0 => (*node, *offset),
        Some(Piece::Atomic { node } | Piece::Break { node }) if slot > 0 => {
            index_in_parent(tree, *node)
        }
        _ => slot_position(tree, pieces, slot),
    }
}

fn unit_of(piece: &Piece) -> u16 {
    match piece {
        Piece::Text { unit, .. } => *unit,
        Piece::Atomic { .. } => 0xFFFC,
        Piece::Break { .. } => 10,
    }
}

/// TextIterator plain text between two markers (Selection.toString).
fn plain_text(
    tree: &Tree,
    events: &[Event],
    start: (Handle, usize),
    canonical_start: Option<(usize, usize)>,
) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    // The last emitted character; a replaced element resets it (TextIteratorTextState).
    let mut last: u16 = 0;
    let mut emitted = false;
    let mut text_node = false;
    let mut another = false;
    let mut edge = false;
    let begin = events
        .iter()
        .position(|event| matches!(event, Event::Marker(0)))
        .unwrap_or(events.len());
    let end = events
        .iter()
        .position(|event| matches!(event, Event::Marker(1)))
        .unwrap_or(events.len());
    let descendant = |node: Handle| {
        let mut current = Some(node);
        while let Some(handle) = current {
            if handle == start.0 {
                return true;
            }
            current = tree.parent(handle).or_else(|| tree.shadow_host(handle));
        }
        false
    };
    for event in events.iter().take(end).skip(begin) {
        if another {
            out.push(10);
            last = 10;
            another = false;
        }
        match *event {
            Event::Piece(piece) => {
                match piece {
                    Piece::Atomic { .. } => last = 0,
                    piece => {
                        last = unit_of(&piece);
                        out.push(last);
                        text_node = true;
                    }
                }
                emitted = true;
            }
            Event::Start(node, kind) => match kind {
                Kind::Block { body: false, .. } | Kind::Row | Kind::Table => {
                    let represent = if emitted {
                        last != 10
                    } else if node == start.0 || !descendant(node) {
                        node != start.0
                    } else if start.1 == 0 {
                        false
                    } else if kind == Kind::Row || kind == Kind::Table {
                        true
                    } else {
                        // Same line only when the start canonicalizes forward into this block.
                        canonical_start.is_some() || edge
                    };
                    if represent && (emitted || last != 10) {
                        out.push(10);
                        last = 10;
                    }
                }
                Kind::Cell => {
                    let previous_cell = tree.previous_sibling(node).is_some_and(|sibling| {
                        tree.node_type_id(sibling) == 1
                            && matches!(html_text::tag(tree, sibling), "td" | "th")
                    });
                    if emitted && previous_cell {
                        out.push(9);
                        last = 9;
                    }
                }
                _ => {}
            },
            Event::End(kind) => {
                if let Kind::Block { body: false, extra } = kind {
                    if emitted && text_node {
                        if last != 10 {
                            out.push(10);
                            another = extra;
                        } else if extra {
                            out.push(10);
                        }
                        last = 10;
                    }
                } else if (kind == Kind::Row || kind == Kind::Table)
                    && emitted
                    && text_node
                    && last != 10
                {
                    out.push(10);
                    last = 10;
                }
            }
            Event::Edge(..) => edge = true,
            Event::Marker(_) => {}
        }
    }
    if another {
        out.push(10);
    }
    out
}

fn scope_root(tree: &Tree, node: Handle) -> Handle {
    let mut current = node;
    while let Some(parent) = tree.parent(current) {
        current = parent;
    }
    current
}

fn node_arg(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<Handle> {
    let object = v8::Local::<v8::Object>::try_from(value).ok()?;
    handle_of(scope, object)
}

/// `model(root, n0, o0, n1, o1, ...)`: the visible-text segments of `root`'s flat tree and
/// each query position's canonical (segment, slot), as
/// `{texts: [string], slots: [[node, offset, ...]], queries: [[segment, slot] | [-1, next]]}`.
fn model<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
) -> Option<v8::Local<'s, v8::Value>> {
    let root = node_arg(scope, args.get(1))?;
    let mut queries = Vec::new();
    let mut index = 2;
    while index + 1 < args.length() {
        let node = node_arg(scope, args.get(index))?;
        let offset = args.get(index + 1).uint32_value(scope)? as usize;
        queries.push((node, offset));
        index += 2;
    }
    let policy = css_policy::cascade_policy(scope);
    let count = queries.len();
    let (texts, slots, forward, resolved, edges) = with_tree(|tree| {
        let events = flow(tree, &policy, root, queries);
        let segments = segments(&events, count);
        let texts: Vec<Vec<u16>> = segments
            .pieces
            .iter()
            .map(|pieces| pieces.iter().map(unit_of).collect())
            .collect();
        let slots: Vec<Vec<(Handle, usize)>> = segments
            .pieces
            .iter()
            .zip(&segments.edges)
            .map(|(pieces, edge)| match edge {
                Some((table, after)) => {
                    let (parent, index) = index_in_parent(tree, *table);
                    vec![(parent, index + usize::from(*after))]
                }
                None => (0..=pieces.len())
                    .map(|slot| slot_position(tree, pieces, slot))
                    .collect(),
            })
            .collect();
        let forward: Vec<Vec<(Handle, usize)>> = segments
            .pieces
            .iter()
            .zip(&segments.edges)
            .map(|(pieces, edge)| match edge {
                Some(_) => Vec::new(),
                None => (0..=pieces.len())
                    .map(|slot| slot_forward(tree, pieces, slot))
                    .collect(),
            })
            .collect();
        let resolved: Vec<(i64, usize)> = (0..count)
            .map(|index| match segments.queries[index] {
                Some((segment, slot)) => (segment as i64, slot),
                None => (-1, segments.after[index]),
            })
            .collect();
        let edges: Vec<u8> = segments
            .edges
            .iter()
            .map(|edge| match edge {
                None => 0,
                Some((_, false)) => 1,
                Some((_, true)) => 2,
            })
            .collect();
        (texts, slots, forward, resolved, edges)
    })?;
    let texts: Vec<v8::Local<v8::Value>> = texts
        .iter()
        .map(|text| unit_string(scope, text).into())
        .collect();
    let mut forward_arrays = Vec::new();
    for (index, segment) in forward.iter().enumerate() {
        let segment = if segment.is_empty() {
            &slots[index]
        } else {
            segment
        };
        let mut flat = Vec::with_capacity(segment.len() * 2);
        for &(node, offset) in segment {
            flat.push(wrap(scope, node).into());
            flat.push(v8::Number::new(scope, offset as f64).into());
        }
        forward_arrays.push(v8::Array::new_with_elements(scope, &flat).into());
    }
    let mut slot_arrays = Vec::new();
    for segment in &slots {
        let mut flat = Vec::with_capacity(segment.len() * 2);
        for &(node, offset) in segment {
            flat.push(wrap(scope, node).into());
            flat.push(v8::Number::new(scope, offset as f64).into());
        }
        slot_arrays.push(v8::Array::new_with_elements(scope, &flat).into());
    }
    let mut query_arrays = Vec::new();
    for &(segment, slot) in &resolved {
        let segment = v8::Number::new(scope, segment as f64);
        let slot = v8::Number::new(scope, slot as f64);
        query_arrays
            .push(v8::Array::new_with_elements(scope, &[segment.into(), slot.into()]).into());
    }
    let result = v8::Object::new(scope);
    let texts = v8::Array::new_with_elements(scope, &texts);
    let slot_arrays = v8::Array::new_with_elements(scope, &slot_arrays);
    let query_arrays = v8::Array::new_with_elements(scope, &query_arrays);
    set_prop(scope, result, "texts", texts.into());
    set_prop(scope, result, "slots", slot_arrays.into());
    let edges: Vec<v8::Local<v8::Value>> = edges
        .iter()
        .map(|&kind| v8::Integer::new(scope, i32::from(kind)).into())
        .collect();
    let edges = v8::Array::new_with_elements(scope, &edges);
    set_prop(scope, result, "edges", edges.into());
    let forward_arrays = v8::Array::new_with_elements(scope, &forward_arrays);
    set_prop(scope, result, "forward", forward_arrays.into());
    set_prop(scope, result, "queries", query_arrays.into());
    Some(result.into())
}

/// `text(document, startNode, startOffset, endNode, endOffset)`: TextIterator plain text of a range.
fn text<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
) -> Option<v8::Local<'s, v8::Value>> {
    let start = (
        node_arg(scope, args.get(2))?,
        args.get(3).uint32_value(scope)? as usize,
    );
    let end = (
        node_arg(scope, args.get(4))?,
        args.get(5).uint32_value(scope)? as usize,
    );
    let policy = css_policy::cascade_policy(scope);
    let units = with_tree(|tree| {
        let root = scope_root(tree, start.0);
        let events = flow(tree, &policy, root, vec![start, end]);
        let segments = segments(&events, 2);
        plain_text(tree, &events, start, segments.queries[0])
    })?;
    Some(unit_string(scope, &units).into())
}

/// The editing host of `node` (nearest contenteditable ancestor chain root) or null.
fn editing_host(tree: &Tree, node: Handle) -> Option<Handle> {
    let mut current = Some(if tree.node_type_id(node) == 1 {
        node
    } else {
        tree.parent(node)?
    });
    let mut host = None;
    while let Some(element) = current {
        if tree.node_type_id(element) != 1 {
            break;
        }
        match tree
            .get_attribute(element, "contenteditable")
            .map(|value| value.to_ascii_lowercase())
        {
            Some(value) if value.is_empty() || value == "true" || value == "plaintext-only" => {
                host = Some(element)
            }
            Some(value) if value == "false" => break,
            _ => {}
        }
        current = tree.parent(element);
    }
    host
}

fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    if operation == "schedule" {
        if let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(1)) {
            html_popover::schedule(scope, callback);
        }
        return;
    }
    let Ok(target) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, target);
    match operation.as_str() {
        "track" => {
            if let (Ok(record), Ok(notify)) = (
                v8::Local::<v8::Object>::try_from(args.get(2)),
                v8::Local::<v8::Function>::try_from(args.get(3)),
            ) {
                range_live::register_editor(scope, record, notify);
            }
        }
        "dispatch" => {
            if let Ok(event) = v8::Local::<v8::Object>::try_from(args.get(2)) {
                dispatch_host_event(scope, event, target);
            }
        }
        "active" => rv.set_bool(DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .is_some_and(|dom| dom.browsing_context)
        })),
        "focused" => {
            let active = ACTIVE
                .with(|active| *active.borrow())
                .filter(|&node| with_tree(|tree| is_connected(tree, node)).unwrap_or(false));
            match active {
                Some(active) => rv.set(wrap(scope, active).into()),
                None => rv.set(v8::null(scope).into()),
            }
        }
        "watch" => {
            if let Some(node) = handle_of(scope, target) {
                let dom = DOM_ID.with(|id| id.get());
                WATCHED.with(|set| set.borrow_mut().insert((dom, node)));
            }
        }
        "focusable" => {
            let focusable = handle_of(scope, target).is_some_and(|node| focusable(scope, node));
            rv.set_bool(focusable);
        }
        "host" => {
            let host = handle_of(scope, target)
                .and_then(|node| with_tree(|tree| editing_host(tree, node)).flatten());
            match host {
                Some(host) => rv.set(wrap(scope, host).into()),
                None => rv.set(v8::null(scope).into()),
            }
        }
        "model" => {
            if let Some(value) = model(scope, &args) {
                rv.set(value);
            }
        }
        "text" => {
            if let Some(value) = text(scope, &args) {
                rv.set(value);
            }
        }
        _ => {}
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let Some((record, static_record)) = range_contents::accessors(scope) else {
        return;
    };
    let Some(native) = v8::Function::new(scope, dispatch) else {
        return;
    };
    let Some(source) = v8::String::new(scope, include_str!("selection.js")) else {
        return;
    };
    let Some(initializer) = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    else {
        return;
    };
    let receiver = v8::undefined(scope);
    let Some(hooks) = initializer
        .call(
            scope,
            receiver.into(),
            &[native.into(), record.into(), static_record.into()],
        )
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
    else {
        return;
    };
    let focus = hooks
        .get_index(scope, 0)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());
    let textarea = hooks
        .get_index(scope, 2)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());
    let input_type = hooks
        .get_index(scope, 3)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());
    let removed = hooks
        .get_index(scope, 4)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());
    let sync = hooks
        .get_index(scope, 1)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());
    if let Some(sync) = sync {
        range_contents::set_selection_sync(scope, sync);
    }
    let focus = focus.and_then(|focus| dom_ownership::helper_function(scope, focus));
    let textarea = textarea.and_then(|textarea| dom_ownership::helper_function(scope, textarea));
    let input_type = input_type.and_then(|hook| dom_ownership::helper_function(scope, hook));
    let removed = removed.and_then(|hook| dom_ownership::helper_function(scope, hook));
    if let (Some(focus), Some(textarea), Some(input_type), Some(removed)) =
        (focus, textarea, input_type, removed)
    {
        scope
            .get_current_context()
            .set_slot(std::rc::Rc::new(Hooks {
                focus,
                textarea,
                input_type,
                removed,
            }));
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Run an async probe against the binding with one FIFO timer queue shared by the
    /// private selection/popover scheduler and the probe's own setTimeout; returns its JSON.
    pub(in super::super) fn run_probe(probe: &str, html: &str) -> serde_json::Value {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, html);
        run_js(
            scope,
            "globalThis.__tasks=[];globalThis.setTimeout=callback=>{__tasks.push(callback);return __tasks.length};globalThis.requestAnimationFrame=callback=>setTimeout(()=>callback(0));",
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
        for _ in 0..1_000_000 {
            scope.perform_microtask_checkpoint();
            if promise.state() != v8::PromiseState::Pending {
                break;
            }
            if step.run(scope).is_none() {
                panic!(
                    "task threw: {:?}",
                    scope
                        .exception()
                        .map(|error| error.to_rust_string_lossy(scope))
                );
            }
        }
        let value = promise.result(scope).to_rust_string_lossy(scope);
        assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
        reset();
        serde_json::from_str(&value).unwrap()
    }

    /// Caret moves that keep a horizontal position read real line-box geometry in Blink. This
    /// binding has no layout, so every caret's x is 0 and vertical moves land at line starts
    /// (KNOWN_GAP; turbo-surf's live layout closes it). Pinned to the deterministic values.
    fn known_gaps() -> Vec<(&'static str, &'static str, serde_json::Value)> {
        let lead = |offset: u64| serde_json::json!(["#  lead ", offset]);
        let dont = |offset: u64| serde_json::json!(["#don't stop-me now 3.14 e.g. foo", offset]);
        let accent = |offset: u64| serde_json::json!(["#a\u{301}b \u{1f44d}\u{1f3fd}c", offset]);
        let hello =
            |offset: u64| serde_json::json!(["#Hello,  big   world. It's  fine! Ok? x", offset]);
        let forward = serde_json::json!([lead(2), dont(0), accent(0), accent(9)]);
        vec![
            (
                "modify",
                "lineMidline",
                serde_json::json!([forward, [lead(2), hello(0), hello(0), hello(0)]]),
            ),
            ("modify", "paragraphMidline", forward),
            (
                "controls",
                "taLineMidline",
                serde_json::json!([
                    ["sc:ta", "select:ta:true:false:true"],
                    [
                        [9, 9, "none"],
                        ["Caret", "BODY", 2, "BODY", 2, 1, true, "none", "\"\""]
                    ]
                ]),
            ),
        ]
    }

    /// The control was captured by running the same probe in a fresh headless Chrome 156
    /// profile through scripts/invokers-chrome-control.mjs; every other case must match.
    #[test]
    fn selection_matches_chrome156_control() {
        use sha2::{Digest, Sha256};
        let probe = include_str!("selection_probe.js");
        let control: serde_json::Value =
            serde_json::from_str(include_str!("selection_chrome156.json")).unwrap();
        assert_eq!(control["browser"], "Chrome/156.0.8078.47");
        assert_eq!(control["v8"], "15.6.75.19");
        assert_eq!(
            control["probeSha256"],
            format!("{:x}", Sha256::digest(probe.as_bytes()))
        );
        let actual = run_probe(
            probe,
            "<!doctype html><html><head></head><body></body></html>",
        );
        let gaps = known_gaps();
        let mut differences = Vec::new();
        for (section, cases) in control["result"].as_object().unwrap() {
            let cases = cases.as_object().unwrap();
            assert_eq!(
                actual[section].as_object().map(|object| object.len()),
                Some(cases.len()),
                "{section} case count"
            );
            for (case, expected) in cases {
                if let Some((_, _, pinned)) =
                    gaps.iter().find(|(s, c, _)| s == section && c == case)
                {
                    assert_ne!(expected, pinned, "{section}.{case} is no longer a gap");
                    assert_eq!(&actual[section][case], pinned, "{section}.{case} gap value");
                    continue;
                }
                if &actual[section][case] != expected {
                    differences.push(format!(
                        "{section}.{case}\n  binding: {}\n  chrome:  {expected}",
                        actual[section][case]
                    ));
                }
            }
        }
        assert!(
            differences.is_empty(),
            "{} differences:\n{}",
            differences.len(),
            differences.join("\n")
        );
    }

    /// Selection and caret movement run on captured natives: guest replacements of DOM,
    /// Range, Event, iterator, regex and Intl primordials are never consulted, and trusted
    /// events still reach listeners registered before tampering. (Text controls read the
    /// binding's form value accessor, which itself consults Element.getAttribute.)
    #[test]
    fn selection_ignores_guest_primordial_replacements() {
        let probe = r#"(async () => {
          document.body.innerHTML = '<p id=p>Hello big world. Next one.</p><p id=q>tail</p>';
          const p = document.getElementById('p'), q = document.getElementById('q'), t = p.firstChild, u = q.firstChild;
          const events = [];
          document.addEventListener('selectionchange', () => events.push('doc'));
          let hooks = 0;
          const poison = () => { hooks++; throw new Error('guest hook'); };
          for (const name of ['setStart', 'setEnd', 'collapse', 'cloneRange', 'deleteContents', 'getClientRects'])
            Range.prototype[name] = poison;
          for (const [proto, names] of [[Node.prototype, ['parentNode', 'firstChild', 'nextSibling', 'nodeType', 'ownerDocument', 'isConnected', 'nodeName']], [CharacterData.prototype, ['data']], [Element.prototype, ['localName', 'namespaceURI']]])
            for (const name of names) Object.defineProperty(proto, name, {get: poison, configurable: true});
          Node.prototype.getRootNode = poison; Element.prototype.getAttribute = poison; HTMLElement.prototype.focus = poison;
         
          Array.prototype[Symbol.iterator] = poison; RegExp.prototype.exec = poison;
          Intl.Segmenter.prototype.segment = poison; EventTarget.prototype.dispatchEvent = poison;
          Object.defineProperty(Object.prototype, 'granularity', {get: poison, configurable: true});
          Object.defineProperty(Object.prototype, 'bubbles', {get: poison, configurable: true});
          globalThis.Event = poison; globalThis.Range = poison; globalThis.StaticRange = poison;
          const s = getSelection();
          s.setBaseAndExtent(t, 2, u, 2);
          const text = s.toString(), range = s.getRangeAt(0) === s.getRangeAt(0);
          s.collapse(t, 0); s.modify('extend', 'forward', 'word'); s.modify('extend', 'forward', 'sentence');
          const moved = [s.anchorOffset, s.focusOffset, s.direction, s.toString()];
          s.extend(u, 1); s.collapseToEnd();
          const contained = [s.containsNode(u), s.containsNode(p, true), s.getComposedRanges().length];
          await new Promise(resolve => setTimeout(resolve, 0));
          await new Promise(resolve => setTimeout(resolve, 0));
          delete Object.prototype.granularity; delete Object.prototype.bubbles;
          return {hooks, text, range, moved, contained, events};
        })()"#;
        let actual = run_probe(
            probe,
            "<!doctype html><html><head></head><body></body></html>",
        );
        assert_eq!(
            actual,
            serde_json::json!({
                "hooks": 0,
                "text": "llo big world. Next one.\n\nta",
                "range": true,
                "moved": [0, 17, "forward", "Hello big world. "],
                "contained": [true, false, 1],
                "events": ["doc"]
            })
        );
    }

    /// The embedder typing path edits like Blink's InsertText command: the caret follows the
    /// insertion, value ranges follow the edit and only selectionchange is queued.
    #[test]
    fn trusted_typing_replaces_the_selection_without_select_events() {
        let probe = r#"(async () => {
          document.body.innerHTML = '<input id=i value="hello world">';
          const i = document.getElementById('i'), log = [];
          for (const type of ['select', 'selectionchange']) i.addEventListener(type, () => log.push(type));
          i.focus(); i.setSelectionRange(2, 5);
          const range = i.createValueRange(6, 11);
          await new Promise(resolve => setTimeout(resolve, 0));
          log.length = 0;
          const replace = globalThis[Symbol.for('ts')].replaceControlSelection;
          const done = [replace(i, 'XY'), replace(document.body, 'z')];
          const state = [i.value, i.selectionStart, i.selectionEnd, range.startOffset, range.endOffset, getSelection().type];
          await new Promise(resolve => setTimeout(resolve, 0));
          return {done, state, log};
        })()"#;
        let actual = run_probe(
            probe,
            "<!doctype html><html><head></head><body></body></html>",
        );
        assert_eq!(
            actual,
            serde_json::json!({
                "done": [true, false],
                "state": ["heXY world", 4, 4, 5, 10, "Caret"],
                "log": ["selectionchange"]
            })
        );
    }

    /// Developer loop: `SELECTION_PROBE=path cargo test --lib selection::tests::dev -- --ignored`.
    #[test]
    #[ignore]
    fn dev() {
        let Ok(path) = std::env::var("SELECTION_PROBE") else {
            return;
        };
        let probe = std::fs::read_to_string(path).unwrap();
        let result = run_probe(
            &probe,
            "<!doctype html><html><head></head><body></body></html>",
        );
        println!("RESULT {result}");
    }
}

//! Shared parser core. No binding deps — html5ever in, plain nested tree out.
//! Both front-ends (napi-rs native, wasm-bindgen fallback) wrap this unchanged.
//!
//! Plan note: this is the "full marshaling" version — a *complete* tree.
//! The `SoA` flat-buffer (architecture Layer 1) is a later optimization, gated on
//! marshaling proving to be the actual cost. Do not SoA-ify here yet.

use html5ever::driver::ParseOpts;
use html5ever::tendril::StrTendril;
use html5ever::tendril::TendrilSink;
use html5ever::tokenizer::{BufferQueue, Token, TokenSink, TokenSinkResult, Tokenizer};
use html5ever::tree_builder::TreeBuilder;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::TokenizerResult;
use html5ever::{ns, parse_fragment, LocalName, QualName};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// DOM nodeType constants (subset html5ever can emit).
pub const ELEMENT_NODE: u8 = 1;
pub const TEXT_NODE: u8 = 3;
pub const PROCESSING_INSTRUCTION_NODE: u8 = 7;
pub const COMMENT_NODE: u8 = 8;
pub const DOCUMENT_NODE: u8 = 9;
pub const DOCUMENT_TYPE_NODE: u8 = 10;
pub const DOCUMENT_FRAGMENT_NODE: u8 = 11;

#[cfg_attr(
    feature = "wasm-bind",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    pub value: String,
    /// Namespace prefix for foreign-content attrs ("xlink", "xml", "xmlns"); empty otherwise.
    pub prefix: String,
}

/// One marshaled DOM node. Nested children = complete tree, no lazy indices.
#[cfg_attr(
    feature = "wasm-bind",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone)]
pub struct Node {
    /// DOM nodeType.
    pub node_type: u8,
    /// Tag name (element) lowercased, or "#text"/"#comment"/"#document"/doctype name.
    pub name: String,
    /// Text/comment/PI data; empty for elements.
    pub value: String,
    /// Element namespace, html5lib-style short form: "" (html), "svg", "math". Empty for non-elements.
    pub namespace: String,
    /// Doctype PUBLIC id; empty otherwise.
    pub public_id: String,
    /// Doctype SYSTEM id; empty otherwise.
    pub system_id: String,
    pub attrs: Vec<Attr>,
    pub children: Vec<Node>,
}

/// Parse options matching html5lib-tests defaults (scripting flag off, so
/// `<noscript>` content is parsed as markup rather than rawtext).
fn opts() -> ParseOpts {
    ParseOpts {
        tree_builder: TreeBuilderOpts {
            scripting_enabled: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Chrome 156 treats an empty DOCTYPE system identifier as missing when it picks the
/// document's quirks mode (miaskiewicz/turbo-dom#103): `<!DOCTYPE html PUBLIC "-//W3C//DTD
/// HTML 4.01 Transitional//EN" "">` is quirks (a `<table>` stays in an open `<p>`), where the
/// HTML spec (and html5ever) reads a present identifier. `doctype.systemId` is `""` both ways.
#[must_use]
pub fn chrome_doctype(token: Token) -> Token {
    match token {
        Token::DoctypeToken(mut doctype) if doctype.system_id.as_deref() == Some("") => {
            doctype.system_id = None;
            Token::DoctypeToken(doctype)
        }
        token => token,
    }
}

/// A token sink that applies [`chrome_doctype`] before the wrapped sink (html5ever's tree
/// builder) sees a token.
pub struct ChromeTokens<S>(pub S);

impl<S: TokenSink> TokenSink for ChromeTokens<S> {
    type Handle = S::Handle;
    fn process_token(&self, token: Token, line_number: u64) -> TokenSinkResult<S::Handle> {
        self.0.process_token(chrome_doctype(token), line_number)
    }
    // (html5ever's `TreeBuilder::end` only pops the open elements into the sink, and
    // `RcDom` ignores pops, so no tree here observes this delegation; it keeps the wrapper
    // transparent for any sink.)
    fn end(&self) {
        self.0.end();
    }
    fn adjusted_current_node_present_but_not_in_html_namespace(&self) -> bool {
        self.0
            .adjusted_current_node_present_but_not_in_html_namespace()
    }
}

/// `html5ever::parse_document` over a whole string, with [`ChromeTokens`].
fn parse_document_dom(html: &str, opts: ParseOpts) -> RcDom {
    let builder = TreeBuilder::new(RcDom::default(), opts.tree_builder);
    let tokenizer = Tokenizer::new(ChromeTokens(builder), opts.tokenizer);
    let input = BufferQueue::default();
    input.push_back(StrTendril::from_slice(html));
    while !matches!(tokenizer.feed(&input), TokenizerResult::Done) {}
    tokenizer.end();
    let dom = tokenizer.sink.0.sink;
    if mentions_selectedcontent(html) {
        fill_selectedcontent(&dom.document);
    }
    dom
}

/// Does `html` mention `<selectedcontent>` (ASCII case-insensitive)? Gates [`fill_selectedcontent`].
#[must_use]
pub fn mentions_selectedcontent(html: &str) -> bool {
    // Runs on every document parse: jump between candidate first letters with SIMD
    // `memchr2` instead of comparing a 15-byte window at every offset.
    const NEEDLE: &[u8; 15] = b"selectedcontent";
    let bytes = html.as_bytes();
    memchr::memchr2_iter(b's', b'S', bytes).any(|at| {
        bytes
            .get(at..at + NEEDLE.len())
            .is_some_and(|window| window.eq_ignore_ascii_case(NEEDLE))
    })
}

/// Chrome 156 fills every `<selectedcontent>` of a non-`multiple` `<select>` with a clone of
/// the selected option's content while it parses (the "maybe clone an option into
/// selectedcontent" step; miaskiewicz/turbo-dom#104): the last option with `selected`, else
/// the first option that is not disabled (itself or through its `<optgroup>`); with none,
/// the element keeps its content. `markup5ever_rcdom` 0.39's step never finds the element (its
/// descendant search compares the select's own name) and html5ever only runs it on an
/// explicit `</option>`, so the parsed tree is completed here — the final state Chrome's
/// per-option runs leave.
pub fn fill_selectedcontent(root: &Handle) {
    let mut pending = vec![root.clone()];
    while let Some(node) = pending.pop() {
        if html_element(&node, "select") && !has_attribute(&node, "multiple") {
            let (options, contents) = select_parts(&node);
            let selected = options
                .iter()
                .rev()
                .find(|(option, _)| has_attribute(option, "selected"))
                .or_else(|| options.iter().find(|(_, disabled)| !disabled));
            if let Some((option, _)) = selected {
                for content in &contents {
                    for old in content.children.take() {
                        old.parent.set(None);
                    }
                    for child in option.children.borrow().iter() {
                        append_handle(content, deep_clone(child));
                    }
                }
            }
        }
        pending.extend(node.children.borrow().iter().cloned());
    }
}

fn html_element(node: &Handle, local: &str) -> bool {
    matches!(&node.data, NodeData::Element { name, .. } if name.ns == ns!(html) && &*name.local == local)
}

fn has_attribute(node: &Handle, local: &str) -> bool {
    matches!(&node.data, NodeData::Element { attrs, .. }
        if attrs.borrow().iter().any(|a| a.name.ns == ns!() && &*a.name.local == local))
}

fn classify_select_child(
    node: &Handle,
    options: &mut Vec<(Handle, bool)>,
    blocked: &mut bool,
    in_option: &mut bool,
    grouped: &mut bool,
    group_disabled: &mut bool,
) {
    if html_element(node, "option") {
        if !*blocked {
            options.push((
                node.clone(),
                *group_disabled || has_attribute(node, "disabled"),
            ));
        }
        *blocked = true;
        *in_option = true;
    } else if html_element(node, "datalist") || html_element(node, "hr") {
        *blocked = true;
    } else if html_element(node, "optgroup") {
        *blocked |= *grouped;
        *grouped = true;
        *group_disabled |= has_attribute(node, "disabled");
    }
}

/// A select's options (with whether each is disabled) and selectedcontent elements, in tree
/// order: an option counts unless an option, datalist or hr, or a second optgroup, lies
/// between it and the select; a selectedcontent inside an option is not the select's
/// (Chrome 156 leaves it as parsed).
fn select_parts(select: &Handle) -> (Vec<(Handle, bool)>, Vec<Handle>) {
    let (mut options, mut contents) = (Vec::new(), Vec::new());
    // (node, blocked, inside an option, inside an optgroup, inside a disabled optgroup)
    let mut pending: Vec<(Handle, bool, bool, bool, bool)> = select
        .children
        .borrow()
        .iter()
        .rev()
        .map(|child| (child.clone(), false, false, false, false))
        .collect();
    while let Some((node, mut blocked, mut in_option, mut grouped, mut group_disabled)) =
        pending.pop()
    {
        if html_element(&node, "selectedcontent") && !in_option {
            contents.push(node);
            continue;
        }
        classify_select_child(
            &node,
            &mut options,
            &mut blocked,
            &mut in_option,
            &mut grouped,
            &mut group_disabled,
        );
        for child in node.children.borrow().iter().rev() {
            pending.push((child.clone(), blocked, in_option, grouped, group_disabled));
        }
    }
    (options, contents)
}

fn append_handle(parent: &Handle, child: Handle) {
    child.parent.set(Some(std::rc::Rc::downgrade(parent)));
    parent.children.borrow_mut().push(child);
}

/// A deep copy of a parsed node (element attributes and template contents included).
fn deep_clone(node: &Handle) -> Handle {
    let data = match &node.data {
        NodeData::Element {
            name,
            attrs,
            template_contents,
            mathml_annotation_xml_integration_point,
        } => NodeData::Element {
            name: name.clone(),
            attrs: std::cell::RefCell::new(attrs.borrow().clone()),
            template_contents: std::cell::RefCell::new(
                template_contents.borrow().as_ref().map(deep_clone),
            ),
            mathml_annotation_xml_integration_point: *mathml_annotation_xml_integration_point,
        },
        NodeData::Text { contents } => NodeData::Text {
            contents: std::cell::RefCell::new(contents.borrow().clone()),
        },
        NodeData::Comment { contents } => NodeData::Comment {
            contents: contents.clone(),
        },
        NodeData::Document | NodeData::Doctype { .. } | NodeData::ProcessingInstruction { .. } => {
            unreachable!("an option holds elements, text and comments only")
        }
    };
    let copy = markup5ever_rcdom::Node::new(data);
    for child in node.children.borrow().iter() {
        append_handle(&copy, deep_clone(child));
    }
    copy
}

/// Parse a full HTML document. Always yields a Document root (nodeType 9).
#[must_use]
pub fn parse_html_document(html: &str) -> Node {
    let dom = parse_document_dom(html, opts());
    walk(&dom.document)
}

// ============================ SoA flat buffer ============================
// Structure-of-Arrays: parser emits compact parallel arrays once, crossed over
// the boundary as cheap typed-array copies. JS reads tree structure straight from
// the arrays and inflates node objects only on access (no eager full-tree alloc).

/// Index-addressed flat tree. Node index IS its id; node 0 is the document.
#[cfg_attr(
    feature = "wasm-bind",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default, Debug, PartialEq)]
pub struct Soa {
    pub node_type: Vec<u8>,    // DOM nodeType
    pub ns: Vec<u8>,           // 0 html, 1 svg, 2 math
    pub tag_id: Vec<u32>,      // index into tag_names (elements); 0 otherwise
    pub parent: Vec<i32>,      // -1 for root
    pub first_child: Vec<i32>, // -1 if none
    pub next_sib: Vec<i32>,    // -1 if none
    pub text_id: Vec<i32>,     // index into strings for text/comment/doctype-name; -1
    pub pub_id: Vec<i32>,      // doctype PUBLIC id index into strings; -1
    pub sys_id: Vec<i32>,      // doctype SYSTEM id index into strings; -1
    pub attr_start: Vec<i32>,  // offset into attr_* tables; -1 if none
    pub attr_count: Vec<u16>,  // attrs for this node
    // flat attr tables — names/prefixes interned (highly repetitive), values pooled
    pub attr_name_id: Vec<u32>,   // index into attr_names
    pub attr_value_id: Vec<u32>,  // index into attr_values (interned, deduped)
    pub attr_prefix_id: Vec<u32>, // index into attr_prefixes
    // string tables (interned, deduped)
    pub tag_names: Vec<String>,
    pub attr_names: Vec<String>,
    pub attr_prefixes: Vec<String>,
    pub attr_values: Vec<String>, // interned attr value dictionary
    pub strings: Vec<String>,     // text/comment/doctype data, pooled
}

/// Why the marshalers have no processing-instruction arm: `markup5ever_rcdom` builds a
/// `NodeData::ProcessingInstruction` only from `TreeSink::create_pi`, which html5ever 0.39's
/// tree builder never calls (only xml5ever does); HTML's `<?x?>` is a bogus comment.
const NO_PI: &str = "html5ever never creates a processing instruction";

struct SoaBuilder {
    soa: Soa,
    tag_map: rustc_hash::FxHashMap<String, u32>,
    attr_name_map: rustc_hash::FxHashMap<String, u32>,
    attr_prefix_map: rustc_hash::FxHashMap<String, u32>,
    attr_value_map: rustc_hash::FxHashMap<String, u32>,
}

impl SoaBuilder {
    fn intern_tag(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.tag_map.get(name) {
            return id;
        }
        let id = self.soa.tag_names.len() as u32;
        self.soa.tag_names.push(name.to_string());
        self.tag_map.insert(name.to_string(), id);
        id
    }
    fn intern_attr_name(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.attr_name_map.get(name) {
            return id;
        }
        let id = self.soa.attr_names.len() as u32;
        self.soa.attr_names.push(name.to_string());
        self.attr_name_map.insert(name.to_string(), id);
        id
    }
    fn intern_attr_prefix(&mut self, prefix: &str) -> u32 {
        if let Some(&id) = self.attr_prefix_map.get(prefix) {
            return id;
        }
        let id = self.soa.attr_prefixes.len() as u32;
        self.soa.attr_prefixes.push(prefix.to_string());
        self.attr_prefix_map.insert(prefix.to_string(), id);
        id
    }
    fn intern_attr_value(&mut self, value: &str) -> u32 {
        if let Some(&id) = self.attr_value_map.get(value) {
            return id;
        }
        let id = self.soa.attr_values.len() as u32;
        self.soa.attr_values.push(value.to_string());
        self.attr_value_map.insert(value.to_string(), id);
        id
    }
    fn push_string(&mut self, s: &str) -> i32 {
        let id = self.soa.strings.len() as i32;
        self.soa.strings.push(s.to_string());
        id
    }

    /// Push one node's slots (placeholders the caller fills) and return its index.
    fn push_slots(&mut self, parent: i32) -> usize {
        let idx = self.soa.node_type.len();
        self.soa.node_type.push(0);
        self.soa.ns.push(0);
        self.soa.tag_id.push(0);
        self.soa.parent.push(parent);
        self.soa.first_child.push(-1);
        self.soa.next_sib.push(-1);
        self.soa.text_id.push(-1);
        self.soa.pub_id.push(-1);
        self.soa.sys_id.push(-1);
        self.soa.attr_start.push(-1);
        self.soa.attr_count.push(0);
        idx
    }

    /// Fill node `idx` as the element `name` with `attrs`.
    fn set_element(&mut self, idx: usize, name: &QualName, attrs: &[html5ever::Attribute]) {
        self.soa.node_type[idx] = ELEMENT_NODE;
        self.soa.ns[idx] = if name.ns == ns!(svg) {
            1
        } else if name.ns == ns!(mathml) {
            2
        } else {
            0
        };
        self.soa.tag_id[idx] = self.intern_tag(&name.local);
        if attrs.is_empty() {
            return;
        }
        self.soa.attr_start[idx] = self.soa.attr_name_id.len() as i32;
        self.soa.attr_count[idx] = attrs.len() as u16;
        for attr in attrs {
            let nid = self.intern_attr_name(&attr.name.local);
            self.soa.attr_name_id.push(nid);
            let vid = self.intern_attr_value(&attr.value);
            self.soa.attr_value_id.push(vid);
            let prefix = attr.name.prefix.as_deref().unwrap_or_default();
            let pid = self.intern_attr_prefix(prefix);
            self.soa.attr_prefix_id.push(pid);
        }
    }

    /// Link node `index` as `parent`'s next child (`last_child[parent]` tracks the
    /// previous one; -1 for none). A template's `content` fragment is a
    /// `DOCUMENT_FRAGMENT_NODE` named `content`.
    fn link(&mut self, last_child: &mut Vec<i32>, (index, parent, content): (i32, i32, bool)) {
        last_child.push(-1);
        if content {
            self.soa.node_type[index as usize] = DOCUMENT_FRAGMENT_NODE;
            self.soa.tag_id[index as usize] = self.intern_tag("content");
        }
        if parent < 0 {
            return;
        }
        let previous = last_child[parent as usize];
        // (-1: no child yet; index 0 is the root, never anyone's child)
        if previous < 0 {
            self.soa.first_child[parent as usize] = index;
        } else {
            self.soa.next_sib[previous as usize] = index;
        }
        last_child[parent as usize] = index;
    }

    // Allocate this node + descendants; return its index.
    fn alloc_node(&mut self, handle: &Handle, parent: i32) -> (i32, Option<Handle>) {
        let idx = self.push_slots(parent);
        let mut template_content: Option<Handle> = None;

        match &handle.data {
            NodeData::Document => self.soa.node_type[idx] = DOCUMENT_NODE,
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            } => {
                self.soa.node_type[idx] = DOCUMENT_TYPE_NODE;
                self.soa.text_id[idx] = self.push_string(name);
                self.soa.pub_id[idx] = self.push_string(public_id);
                self.soa.sys_id[idx] = self.push_string(system_id);
            }
            NodeData::Text { contents } => {
                self.soa.node_type[idx] = TEXT_NODE;
                self.soa.text_id[idx] = self.push_string(&contents.borrow());
            }
            NodeData::Comment { contents } => {
                self.soa.node_type[idx] = COMMENT_NODE;
                self.soa.text_id[idx] = self.push_string(contents);
            }
            NodeData::ProcessingInstruction { .. } => unreachable!("{NO_PI}"),
            NodeData::Element {
                name,
                attrs,
                template_contents,
                ..
            } => {
                self.set_element(idx, name, &attrs.borrow());
                if &*name.local == "template" {
                    if let Some(c) = &*template_contents.borrow() {
                        template_content = Some(c.clone());
                    }
                }
            }
        }

        (idx as i32, template_content)
    }

    fn alloc(&mut self, root: &Handle, parent: i32) -> i32 {
        let mut pending = vec![(root.clone(), parent, false)];
        let mut last_child = Vec::new();
        let first = self.soa.node_type.len() as i32;
        while let Some((handle, parent, content)) = pending.pop() {
            let (index, template) = self.alloc_node(&handle, parent);
            self.link(&mut last_child, (index, parent, content));
            if let Some(template) = template {
                pending.push((template, index, true));
            }
            for child in handle.children.borrow().iter().rev() {
                pending.push((child.clone(), index, false));
            }
        }
        first
    }
}

/// Parse a full document into the `SoA` flat buffer.
#[must_use]
pub fn parse_html_soa(html: &str) -> Soa {
    parse_html_soa_with_compat_mode(html).0
}

/// Parse once and retain html5ever's document compatibility mode.
///
/// html5ever builds straight into an index [`Arena`] whose nodes the `SoA` is read off
/// in one pass: no `Rc` node, `RefCell` child list or owned teardown per node. A
/// document that mentions `<selectedcontent>` keeps the `RcDom` path, because
/// [`fill_selectedcontent`] completes that tree through its handles.
#[must_use]
pub fn parse_html_soa_with_compat_mode(html: &str) -> (Soa, &'static str) {
    if mentions_selectedcontent(html) {
        return soa_from_rcdom(html);
    }
    soa_from_arena(html)
}

fn soa_builder(nodes: usize) -> SoaBuilder {
    let mut soa = Soa::default();
    // Every arena node but detached ones lands in the buffer: size the slots once.
    soa.node_type.reserve(nodes);
    soa.ns.reserve(nodes);
    soa.tag_id.reserve(nodes);
    soa.parent.reserve(nodes);
    soa.first_child.reserve(nodes);
    soa.next_sib.reserve(nodes);
    soa.text_id.reserve(nodes);
    soa.pub_id.reserve(nodes);
    soa.sys_id.reserve(nodes);
    soa.attr_start.reserve(nodes);
    soa.attr_count.reserve(nodes);
    soa.strings.reserve(nodes / 2);
    let fx = |capacity| {
        rustc_hash::FxHashMap::with_capacity_and_hasher(capacity, rustc_hash::FxBuildHasher)
    };
    SoaBuilder {
        soa,
        // pre-reserve typical intern-table sizes so the build loop doesn't rehash:
        // tag/attr-name sets are small + bounded; values vary more.
        tag_map: fx(32),
        attr_name_map: fx(64),
        attr_prefix_map: fx(8),
        attr_value_map: fx(128),
    }
}

fn compat_mode(mode: html5ever::tree_builder::QuirksMode) -> &'static str {
    if mode == html5ever::tree_builder::QuirksMode::Quirks {
        "BackCompat"
    } else {
        "CSS1Compat"
    }
}

/// The `RcDom` path: html5ever's reference-counted tree, completed by
/// [`fill_selectedcontent`], then walked into the `SoA`.
fn soa_from_rcdom(html: &str) -> (Soa, &'static str) {
    let dom = parse_document_dom(html, opts());
    let mut b = soa_builder(0);
    b.alloc(&dom.document, -1);
    (b.soa, compat_mode(dom.quirks_mode.get()))
}

/// The arena path: html5ever builds into an [`Arena`], read into the `SoA` in one pass.
fn soa_from_arena(html: &str) -> (Soa, &'static str) {
    let builder = TreeBuilder::new(Arena::new(), opts().tree_builder);
    let tokenizer = Tokenizer::new(ChromeTokens(builder), opts().tokenizer);
    let input = BufferQueue::default();
    input.push_back(StrTendril::from_slice(html));
    while !matches!(tokenizer.feed(&input), TokenizerResult::Done) {}
    tokenizer.end();
    let arena = html5ever::tree_builder::TreeSink::finish(tokenizer.sink.0.sink);
    let nodes = arena.nodes.borrow();
    let mut b = soa_builder(nodes.len());
    b.alloc_arena(&nodes);
    drop(nodes);
    (b.soa, compat_mode(arena.quirks.get()))
}

// ============================ Arena tree sink ============================
// The tree html5ever builds for `parse_html_soa`: nodes in one `Vec`, linked by index
// (`NIL` = none), so building is pushes and index writes and dropping it is one free.
// Node 0 is the document. Its sink semantics are `markup5ever_rcdom`'s: appended text
// merges into a text node just before the insertion point, and a node moved in front
// of a sibling leaves its old parent first.

const NIL: u32 = u32::MAX;

enum ArenaData {
    Document,
    Doctype {
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    },
    Text(StrTendril),
    Comment(StrTendril),
    Element {
        /// Boxed once by `create_element` and freed by `Arena`'s `Drop`, so
        /// `elem_name` can lend it to html5ever without cloning (see [`arena_name`]).
        name: std::ptr::NonNull<QualName>,
        attrs: Vec<html5ever::Attribute>,
        /// The template's content fragment (a detached `Document` node), else `NIL`.
        template: u32,
        annotation_xml_point: bool,
    },
}

struct ArenaNode {
    data: ArenaData,
    parent: u32,
    first: u32,
    last: u32,
    prev: u32,
    next: u32,
}

impl ArenaNode {
    fn new(data: ArenaData) -> Self {
        Self {
            data,
            parent: NIL,
            first: NIL,
            last: NIL,
            prev: NIL,
            next: NIL,
        }
    }
}

/// An element's name. html5ever asks for it on every open element at every start tag
/// (its in-scope checks), so lending it beats cloning three atoms each time (an 88%
/// slower `deep-nested` parse when cloned).
///
/// # Safety
/// `name` must come from an `ArenaData::Element` of a live `Arena`. Each name is boxed
/// once in `create_element` and freed only by `Arena`'s `Drop`; it is never moved or
/// mutated in between, so the reference is valid for as long as the arena is borrowed.
unsafe fn arena_name<'a>(name: std::ptr::NonNull<QualName>) -> &'a QualName {
    // SAFETY: guaranteed by the caller, as documented above.
    unsafe { name.as_ref() }
}

/// html5ever's tree sink for the `SoA` fast path (see the section comment).
struct Arena {
    nodes: std::cell::RefCell<Vec<ArenaNode>>,
    quirks: std::cell::Cell<html5ever::tree_builder::QuirksMode>,
}

impl Drop for Arena {
    fn drop(&mut self) {
        for node in self.nodes.get_mut() {
            if let ArenaData::Element { name, .. } = node.data {
                // SAFETY: `create_element` boxed this name; this is its only free.
                drop(unsafe { Box::from_raw(name.as_ptr()) });
            }
        }
    }
}

impl Arena {
    fn new() -> Self {
        Self {
            nodes: std::cell::RefCell::new(vec![ArenaNode::new(ArenaData::Document)]),
            quirks: std::cell::Cell::new(html5ever::tree_builder::QuirksMode::NoQuirks),
        }
    }
}

fn arena_push(nodes: &mut Vec<ArenaNode>, data: ArenaData) -> u32 {
    nodes.push(ArenaNode::new(data));
    (nodes.len() - 1) as u32
}

/// Append the parentless `child` as `parent`'s last child.
fn arena_link_last(nodes: &mut [ArenaNode], parent: u32, child: u32) {
    let last = nodes[parent as usize].last;
    let node = &mut nodes[child as usize];
    node.parent = parent;
    node.prev = last;
    node.next = NIL;
    if last == NIL {
        nodes[parent as usize].first = child;
    } else {
        nodes[last as usize].next = child;
    }
    nodes[parent as usize].last = child;
}

/// Insert the parentless `child` just before `sibling`.
fn arena_link_before(nodes: &mut [ArenaNode], sibling: u32, child: u32) {
    let (parent, prev) = (nodes[sibling as usize].parent, nodes[sibling as usize].prev);
    let node = &mut nodes[child as usize];
    node.parent = parent;
    node.prev = prev;
    node.next = sibling;
    nodes[sibling as usize].prev = child;
    if prev == NIL {
        nodes[parent as usize].first = child;
    } else {
        nodes[prev as usize].next = child;
    }
}

/// Detach `child` from its parent, if it has one.
fn arena_unlink(nodes: &mut [ArenaNode], child: u32) {
    let ArenaNode {
        parent, prev, next, ..
    } = nodes[child as usize];
    if parent == NIL {
        return;
    }
    if prev == NIL {
        nodes[parent as usize].first = next;
    } else {
        nodes[prev as usize].next = next;
    }
    if next == NIL {
        nodes[parent as usize].last = prev;
    } else {
        nodes[next as usize].prev = prev;
    }
    let node = &mut nodes[child as usize];
    node.parent = NIL;
    node.prev = NIL;
    node.next = NIL;
}

/// Merge `text` into node `at` when it is a text node (`NIL` is not).
fn arena_merge_text(nodes: &mut [ArenaNode], at: u32, text: &str) -> bool {
    match nodes.get_mut(at as usize).map(|node| &mut node.data) {
        Some(ArenaData::Text(contents)) => {
            contents.push_slice(text);
            true
        }
        _ => false,
    }
}

impl html5ever::tree_builder::TreeSink for Arena {
    type Handle = u32;
    type Output = Self;
    type ElemName<'a> = html5ever::ExpandedName<'a>;

    fn finish(self) -> Self {
        self
    }
    fn parse_error(&self, _msg: std::borrow::Cow<'static, str>) {}
    fn get_document(&self) -> u32 {
        0
    }
    fn elem_name<'a>(&'a self, target: &'a u32) -> html5ever::ExpandedName<'a> {
        let ArenaData::Element { name, .. } = self.nodes.borrow()[*target as usize].data else {
            unreachable!("html5ever names only elements")
        };
        // SAFETY: an element of this arena, which `&'a self` keeps alive.
        unsafe { arena_name(name) }.expanded()
    }
    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<html5ever::Attribute>,
        flags: html5ever::tree_builder::ElementFlags,
    ) -> u32 {
        let mut nodes = self.nodes.borrow_mut();
        let template = if flags.template {
            arena_push(&mut nodes, ArenaData::Document)
        } else {
            NIL
        };
        let element = ArenaData::Element {
            name: std::ptr::NonNull::from(Box::leak(Box::new(name))),
            attrs,
            template,
            annotation_xml_point: flags.mathml_annotation_xml_integration_point,
        };
        arena_push(&mut nodes, element)
    }
    fn create_comment(&self, text: StrTendril) -> u32 {
        arena_push(&mut self.nodes.borrow_mut(), ArenaData::Comment(text))
    }
    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> u32 {
        unreachable!("{NO_PI}")
    }
    fn append(&self, parent: &u32, child: html5ever::tree_builder::NodeOrText<u32>) {
        let nodes = &mut *self.nodes.borrow_mut();
        let child = match child {
            html5ever::tree_builder::AppendNode(node) => node,
            html5ever::tree_builder::AppendText(text) => {
                let last = nodes[*parent as usize].last;
                if arena_merge_text(nodes, last, &text) {
                    return;
                }
                arena_push(nodes, ArenaData::Text(text))
            }
        };
        arena_link_last(nodes, *parent, child);
    }
    fn append_based_on_parent_node(
        &self,
        element: &u32,
        _prev_element: &u32,
        child: html5ever::tree_builder::NodeOrText<u32>,
    ) {
        // html5ever fosters into the last open `<table>`, and nothing here can detach a
        // table: only a script could (scripting is off), and the one removal the parser
        // makes itself (`<frameset>` replacing `<body>`) is disabled by any `<table>`
        // (the frameset-ok flag). So the insertion is always in front of the table.
        assert_ne!(
            self.nodes.borrow()[*element as usize].parent,
            NIL,
            "a foster parent is attached"
        );
        self.append_before_sibling(element, child);
    }
    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        let nodes = &mut *self.nodes.borrow_mut();
        let doctype = ArenaData::Doctype {
            name,
            public_id,
            system_id,
        };
        let id = arena_push(nodes, doctype);
        arena_link_last(nodes, 0, id);
    }
    fn get_template_contents(&self, target: &u32) -> u32 {
        match &self.nodes.borrow()[*target as usize].data {
            ArenaData::Element { template, .. } => *template,
            _ => unreachable!("html5ever asks only templates for their contents"),
        }
    }
    fn same_node(&self, x: &u32, y: &u32) -> bool {
        x == y
    }
    fn set_quirks_mode(&self, mode: html5ever::tree_builder::QuirksMode) {
        self.quirks.set(mode);
    }
    fn append_before_sibling(
        &self,
        sibling: &u32,
        child: html5ever::tree_builder::NodeOrText<u32>,
    ) {
        let nodes = &mut *self.nodes.borrow_mut();
        let child = match child {
            html5ever::tree_builder::AppendNode(node) => {
                arena_unlink(nodes, node);
                node
            }
            html5ever::tree_builder::AppendText(text) => {
                let prev = nodes[*sibling as usize].prev;
                if arena_merge_text(nodes, prev, &text) {
                    return;
                }
                arena_push(nodes, ArenaData::Text(text))
            }
        };
        arena_link_before(nodes, *sibling, child);
    }
    fn add_attrs_if_missing(&self, target: &u32, attrs: Vec<html5ever::Attribute>) {
        let ArenaData::Element {
            attrs: existing, ..
        } = &mut self.nodes.borrow_mut()[*target as usize].data
        else {
            unreachable!("html5ever adds attributes only to elements")
        };
        // As `RcDom`: only the names present before this call are skipped.
        let present = existing.len();
        for attr in attrs {
            if !existing[..present].iter().any(|e| e.name == attr.name) {
                existing.push(attr);
            }
        }
    }
    fn remove_from_parent(&self, target: &u32) {
        arena_unlink(&mut self.nodes.borrow_mut(), *target);
    }
    fn reparent_children(&self, node: &u32, new_parent: &u32) {
        let nodes = &mut *self.nodes.borrow_mut();
        let mut child = nodes[*node as usize].first;
        while child != NIL {
            let next = nodes[child as usize].next;
            arena_unlink(nodes, child);
            arena_link_last(nodes, *new_parent, child);
            child = next;
        }
    }
    fn is_mathml_annotation_xml_integration_point(&self, handle: &u32) -> bool {
        matches!(
            self.nodes.borrow()[*handle as usize].data,
            ArenaData::Element {
                annotation_xml_point: true,
                ..
            }
        )
    }
}

impl SoaBuilder {
    /// Fill a fresh slot from arena node data; returns the slot and its template content.
    fn arena_node(&mut self, data: &ArenaData, parent: i32) -> (i32, u32) {
        let idx = self.push_slots(parent);
        let mut template_content = NIL;
        match data {
            ArenaData::Document => self.soa.node_type[idx] = DOCUMENT_NODE,
            ArenaData::Doctype {
                name,
                public_id,
                system_id,
            } => {
                self.soa.node_type[idx] = DOCUMENT_TYPE_NODE;
                self.soa.text_id[idx] = self.push_string(name);
                self.soa.pub_id[idx] = self.push_string(public_id);
                self.soa.sys_id[idx] = self.push_string(system_id);
            }
            ArenaData::Text(contents) => {
                self.soa.node_type[idx] = TEXT_NODE;
                self.soa.text_id[idx] = self.push_string(contents);
            }
            ArenaData::Comment(contents) => {
                self.soa.node_type[idx] = COMMENT_NODE;
                self.soa.text_id[idx] = self.push_string(contents);
            }
            ArenaData::Element {
                name,
                attrs,
                template,
                ..
            } => {
                // SAFETY: an element of the arena whose nodes `alloc_arena` borrows.
                self.set_element(idx, unsafe { arena_name(*name) }, attrs);
                template_content = *template;
            }
        }
        (idx as i32, template_content)
    }

    /// The arena's document in the same order as [`SoaBuilder::alloc`]: each node, its
    /// children's subtrees, then a template's `content` fragment.
    fn alloc_arena(&mut self, nodes: &[ArenaNode]) {
        let mut pending = vec![(0_u32, -1_i32, false)];
        let mut last_child = Vec::with_capacity(nodes.len());
        while let Some((id, parent, content)) = pending.pop() {
            let node = &nodes[id as usize];
            let (index, template) = self.arena_node(&node.data, parent);
            self.link(&mut last_child, (index, parent, content));
            if template != NIL {
                pending.push((template, index, true));
            }
            let mut child = node.last;
            while child != NIL {
                pending.push((child, index, false));
                child = nodes[child as usize].prev;
            }
        }
    }
}

/// Parse a document and return only the node count — no `core::Node` tree, no
/// marshaling. Isolates raw html5ever parse cost from tree-build + boundary cost.
#[must_use]
pub fn parse_html_document_count(html: &str) -> u32 {
    let dom = parse_document_dom(html, opts());
    count(&dom.document)
}

fn count(handle: &Handle) -> u32 {
    1 + handle.children.borrow().iter().map(count).sum::<u32>()
}

/// Parse an HTML fragment (e.g. an `innerHTML=` set) in `<body>` context.
/// Returns a synthetic Document-fragment-ish root whose children are the parsed nodes.
#[must_use]
pub fn parse_html_fragment(html: &str) -> Node {
    parse_html_fragment_in(html, "", "body")
}

/// Parse a fragment given an html5lib-style context string:
/// "" / "body" / "td" (html ns) or "svg path" / "math ms" (foreign ns).
#[must_use]
pub fn parse_html_fragment_context(html: &str, context: &str) -> Node {
    let (ns, local) = match context {
        "" => ("", "body"),
        s => match s.split_once(' ') {
            Some(("svg", local)) => ("svg", local),
            Some(("math", local)) => ("math", local),
            Some((_, local)) => ("", local),
            None => ("", s),
        },
    };
    parse_html_fragment_in(html, ns, local)
}

/// Parse a fragment in an explicit context element.
/// `context_ns`: "" (html), "svg", or "math"; `context_local`: the element local name.
#[must_use]
pub fn parse_html_fragment_in(html: &str, context_ns: &str, context_local: &str) -> Node {
    let ns = match context_ns {
        "svg" => ns!(svg),
        "math" => ns!(mathml),
        _ => ns!(html),
    };
    // Chrome 156 drops a literal U+0000 in an RCDATA-context (textarea / title) fragment,
    // where html5ever emits U+FFFD (miaskiewicz/turbo-dom#92). The whole input is text there
    // and U+0000 only ends a character reference, so each NUL-free segment is parsed alone
    // and the texts joined (the same rule as the rtdom crate's core).
    if ns == ns!(html) && matches!(context_local, "textarea" | "title") && html.contains('\0') {
        let value: String = html
            .split('\0')
            .flat_map(|segment| parse_html_fragment_in(segment, context_ns, context_local).children)
            .map(|text| text.value)
            .collect();
        let text = (!value.is_empty()).then(|| Node {
            node_type: TEXT_NODE,
            name: "#text".to_string(),
            value,
            namespace: String::new(),
            public_id: String::new(),
            system_id: String::new(),
            attrs: Vec::new(),
            children: Vec::new(),
        });
        return fragment_root(text.into_iter().collect());
    }
    let ctx = QualName::new(None, ns, LocalName::from(context_local));
    let dom = parse_fragment(RcDom::default(), opts(), ctx, vec![], false)
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("RcDom read_from is infallible over a byte slice");
    if mentions_selectedcontent(html) {
        fill_selectedcontent(&dom.document);
    }
    // parse_fragment wraps results under a synthetic <html> element, the Document's only
    // child: in the fragment case nothing is inserted into the Document itself (its comment
    // inserts are the "after after body / frameset" modes, entered only from `</html>` in
    // "after body" — ignored when fragment parsing — and from "after frameset", which
    // `</frameset>` never enters when fragment parsing).
    let root = walk(&dom.document);
    let [html] =
        <[Node; 1]>::try_from(root.children).expect("a parsed fragment has one <html> root");
    fragment_root(html.children)
}

/// The synthetic root a fragment parse returns (nodeType 9, `#document-fragment`).
fn fragment_root(children: Vec<Node>) -> Node {
    Node {
        node_type: DOCUMENT_NODE,
        name: "#document-fragment".to_string(),
        value: String::new(),
        namespace: String::new(),
        public_id: String::new(),
        system_id: String::new(),
        attrs: Vec::new(),
        children,
    }
}

fn shallow_node(handle: &Handle) -> Node {
    let node = handle;
    let mut public_id = String::new();
    let mut system_id = String::new();
    let (node_type, name, value, namespace, attrs) = match &node.data {
        NodeData::Document => (
            DOCUMENT_NODE,
            "#document".to_string(),
            String::new(),
            String::new(),
            Vec::new(),
        ),
        NodeData::Doctype {
            name,
            public_id: pub_id,
            system_id: sys_id,
        } => {
            public_id = pub_id.to_string();
            system_id = sys_id.to_string();
            (
                DOCUMENT_TYPE_NODE,
                name.to_string(),
                String::new(),
                String::new(),
                Vec::new(),
            )
        }
        NodeData::Text { contents } => (
            TEXT_NODE,
            "#text".to_string(),
            contents.borrow().to_string(),
            String::new(),
            Vec::new(),
        ),
        NodeData::Comment { contents } => (
            COMMENT_NODE,
            "#comment".to_string(),
            contents.to_string(),
            String::new(),
            Vec::new(),
        ),
        NodeData::ProcessingInstruction { .. } => unreachable!("{NO_PI}"),
        NodeData::Element { name, attrs, .. } => {
            let a = attrs
                .borrow()
                .iter()
                .map(|attr| Attr {
                    name: attr.name.local.to_string(),
                    value: attr.value.to_string(),
                    prefix: attr
                        .name
                        .prefix
                        .as_ref()
                        .map(std::string::ToString::to_string)
                        .unwrap_or_default(),
                })
                .collect();
            // html5lib short namespace form: "" (html), "svg", "math".
            let ns = if name.ns == ns!(svg) {
                "svg".to_string()
            } else if name.ns == ns!(mathml) {
                "math".to_string()
            } else {
                String::new()
            };
            (ELEMENT_NODE, name.local.to_string(), String::new(), ns, a)
        }
    };

    Node {
        node_type,
        name,
        value,
        namespace,
        public_id,
        system_id,
        attrs,
        children: Vec::new(),
    }
}

fn walk(handle: &Handle) -> Node {
    enum Task {
        Enter(Handle, bool),
        Finish(Node, usize),
    }
    let mut tasks = vec![Task::Enter(handle.clone(), false)];
    let mut complete = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Enter(handle, content) => {
                let mut node = shallow_node(&handle);
                if content {
                    node.node_type = DOCUMENT_FRAGMENT_NODE;
                    node.name = "content".into();
                }
                let children = handle.children.borrow();
                let template = match &handle.data {
                    NodeData::Element {
                        template_contents, ..
                    } => template_contents.borrow().clone(),
                    _ => None,
                };
                tasks.push(Task::Finish(
                    node,
                    children.len() + usize::from(template.is_some()),
                ));
                if let Some(template) = template {
                    tasks.push(Task::Enter(template, true));
                }
                for child in children.iter().rev() {
                    tasks.push(Task::Enter(child.clone(), false));
                }
            }
            Task::Finish(mut node, count) => {
                node.children = complete.split_off(complete.len() - count);
                complete.push(node);
            }
        }
    }
    complete.pop().expect("one parsed root")
}

#[cfg(test)]
mod conformance_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn first<'a>(n: &'a Node, pred: &dyn Fn(&Node) -> bool) -> Option<&'a Node> {
        if pred(n) {
            return Some(n);
        }
        for c in &n.children {
            if let Some(h) = first(c, pred) {
                return Some(h);
            }
        }
        None
    }

    #[test]
    fn document_has_html_head_body() {
        let doc = parse_html_document("<div>hi</div>");
        assert_eq!(doc.node_type, DOCUMENT_NODE);
        assert!(first(&doc, &|n| n.node_type == ELEMENT_NODE && n.name == "html").is_some());
        assert!(first(&doc, &|n| n.node_type == ELEMENT_NODE && n.name == "head").is_some());
        assert!(first(&doc, &|n| n.node_type == ELEMENT_NODE && n.name == "body").is_some());
    }

    #[test]
    fn attrs_marshaled_in_order() {
        let doc = parse_html_document("<div id=\"a\" class=\"x\"></div>");
        let div = first(&doc, &|n| n.name == "div").unwrap();
        assert_eq!(div.attrs[0].name, "id");
        assert_eq!(div.attrs[0].value, "a");
        assert_eq!(div.attrs[1].name, "class");
    }

    #[test]
    fn doctype_public_system_captured() {
        let doc = parse_html_document(
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01//EN\" \"http://x/strict.dtd\"><p>x",
        );
        let dt = first(&doc, &|n| n.node_type == DOCUMENT_TYPE_NODE).unwrap();
        assert_eq!(dt.name, "html");
        assert_eq!(dt.public_id, "-//W3C//DTD HTML 4.01//EN");
        assert_eq!(dt.system_id, "http://x/strict.dtd");
    }

    #[test]
    fn foster_parenting_table() {
        let doc = parse_html_document("<table>oops<tr><td>x</td></tr></table>");
        let body = first(&doc, &|n| n.name == "body").unwrap();
        let fostered = body
            .children
            .iter()
            .any(|c| c.node_type == TEXT_NODE && c.value.contains("oops"));
        assert!(fostered, "stray text must be fostered out of <table>");
    }

    #[test]
    fn svg_namespace_short_form() {
        let doc = parse_html_document("<svg><rect/></svg>");
        let svg = first(&doc, &|n| n.name == "svg").unwrap();
        assert_eq!(svg.namespace, "svg");
    }

    #[test]
    fn template_content_separated() {
        let frag = parse_html_fragment("<template>Hello</template>");
        let tmpl = first(&frag, &|n| n.name == "template").unwrap();
        let content = tmpl
            .children
            .iter()
            .find(|c| c.node_type == DOCUMENT_FRAGMENT_NODE)
            .expect("template has a content fragment");
        assert_eq!(content.name, "content");
        assert_eq!(content.children[0].value, "Hello");
    }

    #[test]
    fn rcdata_fragments_drop_nul_like_chrome() {
        // miaskiewicz/turbo-dom#92 (Chrome 156): a literal U+0000 is dropped, &#0; is U+FFFD
        let text = |html: &str, ctx: &str| {
            let frag = parse_html_fragment_context(html, ctx);
            assert_eq!(
                (frag.node_type, frag.name.as_str(), frag.value.as_str()),
                (DOCUMENT_NODE, "#document-fragment", "")
            );
            frag.children
                .iter()
                .map(|c| (c.node_type, c.value.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            text("a\0b</textarea>c\0d", "textarea"),
            [(TEXT_NODE, "ab</textarea>cd".into())]
        );
        assert_eq!(
            text("x&am\0p;&not\0in;", "title"),
            [(TEXT_NODE, "x&amp;\u{ac}in;".into())]
        );
        assert_eq!(
            text("a&#0;b", "textarea"),
            [(TEXT_NODE, "a\u{fffd}b".into())]
        );
        assert_eq!(text("\0", "title"), []);
        // other contexts keep html5ever's answer (data state: dropped; RAWTEXT: U+FFFD)
        assert_eq!(text("a\0b", "style"), [(TEXT_NODE, "a\u{fffd}b".into())]);
        assert_eq!(text("a\0b", "div"), [(TEXT_NODE, "ab".into())]);
        assert_eq!(text("a\0b", "svg title"), [(TEXT_NODE, "ab".into())]);
    }

    #[test]
    fn fragment_context_strings_pick_namespace_and_element() {
        let dump = |html: &str, ctx: &str| {
            conformance_tests::dump_node(&parse_html_fragment_context(html, ctx))
        };
        // body context ("" too): a stray <td> is dropped, its text kept
        assert_eq!(dump("<td>x", ""), "| \"x\"");
        assert_eq!(dump("<td>x", "body"), "| \"x\"");
        // a <tr> context (bare, or with any prefix other than svg / math: an HTML element)
        let cell = "| <td>\n|   \"x\"";
        assert_eq!(dump("<td>x", "tr"), cell);
        assert_eq!(dump("<td>x", "html tr"), cell);
        // foreign contexts: <mi> stays SVG under svg, is HTML in a MathML text
        // integration point (<ms>), and MathML under math <math>
        assert_eq!(dump("<mi/>", "svg path"), "| <svg mi>");
        assert_eq!(dump("<mi/>", "math ms"), "| <mi>");
        assert_eq!(dump("<mi/>", "math math"), "| <math mi>");
        let frag = parse_html_fragment("<b>y</b>");
        assert_eq!(
            (frag.node_type, frag.name.as_str(), frag.children.len()),
            (DOCUMENT_NODE, "#document-fragment", 1)
        );
        assert_eq!(conformance_tests::dump_node(&frag), "| <b>\n|   \"y\"");
    }

    #[test]
    fn soa_interns_names_values_and_prefixes_once() {
        let soa =
            parse_html_soa("<p class=a></p><p class=a id=b></p><svg><a xlink:href=b></a></svg>");
        assert_eq!(
            soa.tag_names,
            ["html", "head", "body", "p", "svg", "a"].map(String::from)
        );
        assert_eq!(soa.attr_names, ["class", "id", "href"].map(String::from));
        assert_eq!(soa.attr_values, ["a", "b"].map(String::from));
        assert_eq!(soa.attr_prefixes, ["", "xlink"].map(String::from));
        assert_eq!(soa.attr_name_id, [0, 0, 1, 2]);
        assert_eq!(soa.attr_value_id, [0, 0, 1, 1]);
        assert_eq!(soa.attr_prefix_id, [0, 0, 0, 1]);
        // document, html, head, body, p, p, svg, a
        assert_eq!(soa.node_type, [9, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(soa.ns, [0, 0, 0, 0, 0, 0, 1, 1]);
        assert_eq!(soa.tag_id, [0, 0, 1, 2, 3, 3, 4, 5]);
        assert_eq!(soa.parent, [-1, 0, 1, 1, 3, 3, 3, 6]);
        assert_eq!(soa.first_child, [1, 2, -1, 4, -1, -1, 7, -1]);
        assert_eq!(soa.next_sib, [-1, -1, 3, -1, 5, 6, -1, -1]);
        assert_eq!(soa.attr_start, [-1, -1, -1, -1, 0, 1, -1, 3]);
        assert_eq!(soa.attr_count, [0, 0, 0, 0, 1, 2, 0, 1]);
        assert_eq!(soa.text_id, [-1; 8]);
        assert_eq!(soa.strings.len(), 0);
    }

    #[test]
    fn soa_strings_doctype_text_comment_and_template_content() {
        let (soa, mode) = parse_html_soa_with_compat_mode(
            "<!DOCTYPE html PUBLIC \"p\" \"s\"><!--c--><template>t</template>",
        );
        assert_eq!(mode, "CSS1Compat");
        // document, doctype, comment, html, head, template, content, "t", body
        assert_eq!(soa.node_type, [9, 10, 8, 1, 1, 1, 11, 3, 1]);
        assert_eq!(soa.strings, ["html", "p", "s", "c", "t"].map(String::from));
        assert_eq!(soa.text_id, [-1, 0, 3, -1, -1, -1, -1, 4, -1]);
        assert_eq!(soa.pub_id, [-1, 1, -1, -1, -1, -1, -1, -1, -1]);
        assert_eq!(soa.sys_id, [-1, 2, -1, -1, -1, -1, -1, -1, -1]);
        assert_eq!(soa.tag_names[soa.tag_id[6] as usize], "content");
        assert_eq!(soa.parent[6..8], [5, 6]);
        assert_eq!(soa.first_child[5], 6);
        assert_eq!(parse_html_soa_with_compat_mode("<p>").1, "BackCompat");
        assert_eq!(parse_html_document_count("<!--c--><p>x"), 7);
    }

    #[test]
    fn selectedcontent_skips_options_in_a_nested_optgroup() {
        // Chrome 156 (headless DOMParser, recorded for this test): an option below a second
        // optgroup is not one of the select's options, so the next option fills
        // <selectedcontent>; Chrome's tree is the same and its innerHTML is "b".
        let html = "<select><optgroup><div><optgroup><option>a</option></optgroup></div></optgroup>\
                    <option>b</option><button><selectedcontent></selectedcontent></button></select>";
        let want = "| <html>\n|   <head>\n|   <body>\n|     <select>\n|       <optgroup>\n\
                    |         <div>\n|           <optgroup>\n|             <option>\n\
                    |               \"a\"\n|       <option>\n|         \"b\"\n|       <button>\n\
                    |         <selectedcontent>\n|           \"b\"";
        assert_eq!(
            conformance_tests::dump_node(&parse_html_document(html)),
            want
        );
        assert_eq!(conformance_tests::dump_soa(&parse_html_soa(html)), want);
    }

    #[test]
    fn selectedcontent_inside_an_option_is_not_filled() {
        // Chrome 156 (headless DOMParser): a <selectedcontent> inside the option it would
        // copy keeps its (empty) content instead of receiving a clone of the option.
        let html = "<select><option>a<selectedcontent></selectedcontent></option></select>";
        let want = "| <html>\n|   <head>\n|   <body>\n|     <select>\n|       <option>\n\
                    |         \"a\"\n|         <selectedcontent>";
        assert_eq!(
            conformance_tests::dump_node(&parse_html_document(html)),
            want
        );
        assert_eq!(conformance_tests::dump_soa(&parse_html_soa(html)), want);
    }

    #[test]
    fn mentions_selectedcontent_is_an_ascii_case_insensitive_substring_test() {
        assert!(mentions_selectedcontent("selectedcontent"));
        assert!(mentions_selectedcontent("<b></b><SelectedContent>"));
        assert!(mentions_selectedcontent("xSELECTEDCONTENTx"));
        assert!(!mentions_selectedcontent("selectedconten"));
        // A false start on an earlier `s` does not end the scan; a match may end the input.
        assert!(mentions_selectedcontent("<s>sselectedcontent"));
        assert!(mentions_selectedcontent("s s s SelectedContent"));
        assert!(!mentions_selectedcontent("<select><option>"));
        assert!(!mentions_selectedcontent(""));
    }

    #[test]
    fn fragment_context_namespace() {
        let frag = parse_html_fragment_context("<rect/>", "svg path");
        // context is svg path, so a bare <rect> parses in the SVG namespace
        let rect = first(&frag, &|n| n.name == "rect");
        assert!(rect.is_some_and(|n| n.namespace == "svg"));
    }
}

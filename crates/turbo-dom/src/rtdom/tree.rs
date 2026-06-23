//! Rust-native DOM tree (pure Rust API, no JS boundary) — Phase-0 core of the
//! pivoted runtime (RUST_PORT_PLAN §7 verdict: dual-runtime, Rust consumer gets
//! a native API). Mirrors the JS runtime's load-bearing model:
//!   * immutable parse buffer (`core::Soa`) is the read-only source of truth,
//!   * COW overlay: a node only allocates owned state when it is mutated,
//!   * handle (`u32` node index) is stable; identity is the handle itself,
//!   * `version` counter bumped on EVERY mutation = the cache-invalidation key.
//!
//! Reads come straight off the buffer (zero alloc) unless an overlay exists.

use crate::core::{self, Soa};
use std::cell::RefCell;
use std::collections::HashMap;

/// A DOM node type. `#[repr(u8)]` with the standard `nodeType` numeric values, so
/// it round-trips the SoA's compact `u8` column by a plain cast — no lookup table.
/// It enumerates exactly the node types the html5ever core can emit; nothing else
/// is representable, so a `match` over a node's type is exhaustive and compiler-checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeType {
    Element = 1,
    Text = 3,
    ProcessingInstruction = 7,
    Comment = 8,
    Document = 9,
    Doctype = 10,
    Fragment = 11,
}

impl NodeType {
    /// Read a node type back from the SoA's untyped `u8` column. Total over every
    /// value the parser core writes (see the node-type constants in `core`).
    fn from_u8(v: u8) -> NodeType {
        match v {
            1 => NodeType::Element,
            3 => NodeType::Text,
            7 => NodeType::ProcessingInstruction,
            8 => NodeType::Comment,
            9 => NodeType::Document,
            10 => NodeType::Doctype,
            11 => NodeType::Fragment,
            _ => unreachable!("SoA node_type column only holds DOM node-type values"),
        }
    }
}

/// An element's namespace — the only three an HTML parser produces. The SoA stores
/// it as `0`/`1`/`2`; this is the typed view at the API surface (`as u8` to store).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Namespace {
    Html = 0,
    Svg = 1,
    MathMl = 2,
}

impl Namespace {
    fn from_u8(v: u8) -> Namespace {
        match v {
            1 => Namespace::Svg,
            2 => Namespace::MathMl,
            _ => Namespace::Html,
        }
    }
}

/// A node handle. Index into the buffer, or (>= buf_len) into `new_nodes`.
pub type Handle = u32;

/// A node created at runtime (not read from the parse buffer). Each variant carries
/// exactly the data its node type needs — a text node cannot hold a namespace, an
/// element always has one — so the old `(node_type, name, ns)` triple's invalid
/// combinations (a `Text` with an `ns`, an element with no name) are unrepresentable.
enum NewNode {
    Element { name: String, ns: Namespace },
    Text(String),
    Comment(String),
    Fragment,
}

impl NewNode {
    fn node_type(&self) -> NodeType {
        match self {
            NewNode::Element { .. } => NodeType::Element,
            NewNode::Text(_) => NodeType::Text,
            NewNode::Comment(_) => NodeType::Comment,
            NewNode::Fragment => NodeType::Fragment,
        }
    }
    fn namespace(&self) -> Namespace {
        match self {
            NewNode::Element { ns, .. } => *ns,
            _ => Namespace::Html,
        }
    }
    /// The element's local name, if this is an element.
    fn element_name(&self) -> Option<&str> {
        match self {
            NewNode::Element { name, .. } => Some(name),
            _ => None,
        }
    }
    /// The character data, if this is a text or comment node.
    fn char_data(&self) -> Option<&str> {
        match self {
            NewNode::Text(s) | NewNode::Comment(s) => Some(s),
            _ => None,
        }
    }
}

pub struct Tree {
    buf: Soa,
    buf_len: u32,
    new_nodes: Vec<NewNode>,
    // --- COW overlays (present only for mutated/created nodes) ---
    // `Some(parent)` = re-parented; `None` = detached. No `-1` sentinel — a
    // detached node is `None`, never a handle that happens to read as negative.
    parent_ov: HashMap<Handle, Option<Handle>>,
    children_ov: HashMap<Handle, Vec<Handle>>,
    attrs_ov: HashMap<Handle, Vec<(String, String)>>,
    text_ov: HashMap<Handle, String>,
    /// host → shadow-root handle, and shadow-root → host (the two shadow maps).
    shadow_root_of_host: HashMap<Handle, Handle>,
    host_of_shadow_root: HashMap<Handle, Handle>,
    pub version: u64,
    /// version-keyed query result cache (mirrors JS cachedQSA / Document.__version).
    pub(crate) qcache: RefCell<QueryCache>,
    /// version-keyed computed-style cache (mirrors JS `__computedStyle`).
    pub(crate) css_cache: RefCell<crate::rtdom::cascade::CssCache>,
    /// Mutation-record buffer. `None` until an observer calls `start_recording`
    /// (the no-observer hot path stays zero-alloc, mirroring JS notifyMutation gating).
    mutation_log: Option<Vec<crate::rtdom::mutations::MutationRecord>>,
}

#[derive(Default)]
pub(crate) struct QueryCache {
    pub version: u64,
    /// Shared so a cache hit is a pointer bump, not a Vec copy (repeated qSA is hot).
    pub map: HashMap<String, std::rc::Rc<[Handle]>>,
}

impl Tree {
    pub fn parse(html: &str) -> Tree {
        let buf = core::parse_html_soa(html);
        let buf_len = buf.node_type.len() as u32;
        Tree {
            buf,
            buf_len,
            new_nodes: Vec::new(),
            parent_ov: HashMap::new(),
            children_ov: HashMap::new(),
            attrs_ov: HashMap::new(),
            text_ov: HashMap::new(),
            shadow_root_of_host: HashMap::new(),
            host_of_shadow_root: HashMap::new(),
            version: 0,
            qcache: RefCell::new(QueryCache::default()),
            css_cache: RefCell::new(Default::default()),
            mutation_log: None,
        }
    }

    #[inline]
    fn is_new(&self, h: Handle) -> bool {
        h >= self.buf_len
    }
    #[inline]
    fn new_ref(&self, h: Handle) -> &NewNode {
        &self.new_nodes[(h - self.buf_len) as usize]
    }

    pub fn node_count(&self) -> u32 {
        self.buf_len + self.new_nodes.len() as u32
    }

    /// The document root (node 0 in the SoA).
    pub fn root(&self) -> Handle {
        0
    }

    // ----------------------------------------------------------------- reads
    pub fn node_type(&self, h: Handle) -> NodeType {
        if self.is_new(h) {
            self.new_ref(h).node_type()
        } else {
            NodeType::from_u8(self.buf.node_type[h as usize])
        }
    }

    pub fn namespace(&self, h: Handle) -> Namespace {
        if self.is_new(h) {
            self.new_ref(h).namespace()
        } else {
            Namespace::from_u8(self.buf.ns[h as usize])
        }
    }

    /// localName (lowercase as parsed). None for non-elements.
    pub fn local_name(&self, h: Handle) -> Option<&str> {
        if self.node_type(h) != NodeType::Element {
            return None;
        }
        if self.is_new(h) {
            self.new_ref(h).element_name()
        } else {
            Some(&self.buf.tag_names[self.buf.tag_id[h as usize] as usize])
        }
    }

    /// tagName: uppercased for HTML-namespace elements (DOM contract), as-is otherwise.
    pub fn tag_name(&self, h: Handle) -> Option<String> {
        let ln = self.local_name(h)?;
        if self.namespace(h) == Namespace::Html {
            Some(ln.to_ascii_uppercase())
        } else {
            Some(ln.to_string())
        }
    }

    pub fn parent(&self, h: Handle) -> Option<Handle> {
        if let Some(&p) = self.parent_ov.get(&h) {
            return p;
        }
        if self.is_new(h) {
            return None; // detached unless set via overlay
        }
        let p = self.buf.parent[h as usize];
        if p < 0 {
            None
        } else {
            Some(p as Handle)
        }
    }

    /// Live child list. Reads overlay if the node was structurally mutated,
    /// else walks the buffer first-child/next-sib chain (zero overlay alloc).
    pub fn children(&self, h: Handle) -> Vec<Handle> {
        if let Some(c) = self.children_ov.get(&h) {
            return c.clone();
        }
        if self.is_new(h) {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut c = self.buf.first_child[h as usize];
        while c >= 0 {
            out.push(c as Handle);
            c = self.buf.next_sib[c as usize];
        }
        out
    }

    pub fn first_child(&self, h: Handle) -> Option<Handle> {
        self.children(h).first().copied()
    }
    pub fn last_child(&self, h: Handle) -> Option<Handle> {
        self.children(h).last().copied()
    }

    pub fn next_sibling(&self, h: Handle) -> Option<Handle> {
        let p = self.parent(h)?;
        let sibs = self.children(p);
        let i = sibs.iter().position(|&x| x == h)?;
        sibs.get(i + 1).copied()
    }
    pub fn previous_sibling(&self, h: Handle) -> Option<Handle> {
        let p = self.parent(h)?;
        let sibs = self.children(p);
        let i = sibs.iter().position(|&x| x == h)?;
        if i == 0 {
            None
        } else {
            sibs.get(i - 1).copied()
        }
    }

    // ----------------------------------------------------------------- attrs
    /// getAttribute. Reads overlay if attrs were mutated, else the buffer slice
    /// (lazy — no owned Vec built for unmutated nodes).
    pub fn get_attribute(&self, h: Handle, name: &str) -> Option<&str> {
        if let Some(ov) = self.attrs_ov.get(&h) {
            return ov.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str());
        }
        if self.is_new(h) {
            return None;
        }
        let i = h as usize;
        let start = self.buf.attr_start[i];
        if start < 0 {
            return None;
        }
        let start = start as usize;
        let count = self.buf.attr_count[i] as usize;
        for j in start..start + count {
            let nid = self.buf.attr_name_id[j] as usize;
            if self.buf.attr_names[nid] == name {
                return Some(&self.buf.attr_values[self.buf.attr_value_id[j] as usize]);
            }
        }
        None
    }

    pub fn has_attribute(&self, h: Handle, name: &str) -> bool {
        self.get_attribute(h, name).is_some()
    }

    /// All (name, value) pairs for a node (owned copy).
    pub fn attributes(&self, h: Handle) -> Vec<(String, String)> {
        if let Some(ov) = self.attrs_ov.get(&h) {
            return ov.clone();
        }
        if self.is_new(h) {
            return Vec::new();
        }
        let i = h as usize;
        let start = self.buf.attr_start[i];
        if start < 0 {
            return Vec::new();
        }
        let start = start as usize;
        let count = self.buf.attr_count[i] as usize;
        let mut out = Vec::with_capacity(count);
        for j in start..start + count {
            let nid = self.buf.attr_name_id[j] as usize;
            let vid = self.buf.attr_value_id[j] as usize;
            out.push((self.buf.attr_names[nid].clone(), self.buf.attr_values[vid].clone()));
        }
        out
    }

    /// Visit each (name, value) pair WITHOUT allocating — yields borrowed `&str`
    /// straight from the overlay or the SoA tables. Hot-path alternative to
    /// `attributes()` (which clones a `Vec<(String, String)>`).
    pub fn for_each_attr(&self, h: Handle, mut f: impl FnMut(&str, &str)) {
        if let Some(ov) = self.attrs_ov.get(&h) {
            for (n, v) in ov {
                f(n, v);
            }
            return;
        }
        if self.is_new(h) {
            return;
        }
        let i = h as usize;
        let start = self.buf.attr_start[i];
        if start < 0 {
            return;
        }
        let start = start as usize;
        let count = self.buf.attr_count[i] as usize;
        for j in start..start + count {
            let nid = self.buf.attr_name_id[j] as usize;
            let vid = self.buf.attr_value_id[j] as usize;
            f(&self.buf.attr_names[nid], &self.buf.attr_values[vid]);
        }
    }

    /// Like `for_each_attr` but also yields the foreign-content attr `prefix`
    /// ("xlink"/"xml"/"xmlns", else ""). Needed by the html5lib dump. Overlay
    /// (created/mutated) attrs have no stored prefix → "".
    pub fn for_each_attr_full(&self, h: Handle, mut f: impl FnMut(&str, &str, &str)) {
        if let Some(ov) = self.attrs_ov.get(&h) {
            for (n, v) in ov {
                f("", n, v);
            }
            return;
        }
        if self.is_new(h) {
            return;
        }
        let i = h as usize;
        let start = self.buf.attr_start[i];
        if start < 0 {
            return;
        }
        let start = start as usize;
        let count = self.buf.attr_count[i] as usize;
        for j in start..start + count {
            let prefix = &self.buf.attr_prefixes[self.buf.attr_prefix_id[j] as usize];
            let nid = self.buf.attr_name_id[j] as usize;
            let vid = self.buf.attr_value_id[j] as usize;
            f(prefix, &self.buf.attr_names[nid], &self.buf.attr_values[vid]);
        }
    }

    /// The interned tag/label name for an element OR document-fragment node (the
    /// `<template>` synthetic `content` fragment carries the label "content").
    /// None for nodes without a tag id (text/comment/doctype/document).
    pub fn node_label(&self, h: Handle) -> Option<&str> {
        let nt = self.node_type(h);
        if nt != NodeType::Element && nt != NodeType::Fragment {
            return None;
        }
        if self.is_new(h) {
            return if nt == NodeType::Element { self.new_ref(h).element_name() } else { None };
        }
        Some(&self.buf.tag_names[self.buf.tag_id[h as usize] as usize])
    }

    /// DOCTYPE name / public id / system id (buffer-backed doctype nodes). The
    /// SoA stores all three as pooled strings (public/system "" when absent).
    pub fn doctype_name(&self, h: Handle) -> Option<&str> {
        self.doctype_field(h, &self.buf.text_id)
    }
    pub fn doctype_public_id(&self, h: Handle) -> Option<&str> {
        self.doctype_field(h, &self.buf.pub_id)
    }
    pub fn doctype_system_id(&self, h: Handle) -> Option<&str> {
        self.doctype_field(h, &self.buf.sys_id)
    }
    fn doctype_field<'a>(&'a self, h: Handle, col: &[i32]) -> Option<&'a str> {
        if self.node_type(h) != NodeType::Doctype || self.is_new(h) {
            return None;
        }
        let id = col[h as usize];
        Some(self.buf.strings.get(id as usize).map_or("", |s| s.as_str()))
    }

    // ----------------------------------------------------------------- text
    pub fn node_value(&self, h: Handle) -> Option<String> {
        let nt = self.node_type(h);
        if nt != NodeType::Text && nt != NodeType::Comment {
            return None;
        }
        if let Some(t) = self.text_ov.get(&h) {
            return Some(t.clone());
        }
        if self.is_new(h) {
            return self.new_ref(h).char_data().map(|s| s.to_string());
        }
        // buffer text/comment nodes always have text_id >= 0 (core pools every
        // string); .get + default keeps this total and coverable.
        let tid = self.buf.text_id[h as usize];
        Some(self.buf.strings.get(tid as usize).cloned().unwrap_or_default())
    }

    /// textContent: concatenated descendant text.
    pub fn text_content(&self, h: Handle) -> String {
        let nt = self.node_type(h);
        if nt == NodeType::Text || nt == NodeType::Comment {
            return self.node_value(h).unwrap_or_default();
        }
        let mut out = String::new();
        self.collect_text(h, &mut out);
        out
    }
    fn collect_text(&self, h: Handle, out: &mut String) {
        for c in self.children(h) {
            match self.node_type(c) {
                NodeType::Text => out.push_str(&self.node_value(c).unwrap_or_default()),
                NodeType::Comment => {}
                _ => self.collect_text(c, out),
            }
        }
    }

    // ------------------------------------------------------------- mutations
    fn bump(&mut self) {
        self.version += 1;
    }

    /// Push a mutation record IF recording is on (an observer is attached). The
    /// no-observer path skips this entirely — zero alloc.
    fn record(&mut self, rec: crate::rtdom::mutations::MutationRecord) {
        if let Some(log) = self.mutation_log.as_mut() {
            log.push(rec);
        }
    }

    /// Begin buffering mutation records (called by `MutationObserver::observe`).
    pub fn start_recording(&mut self) {
        self.mutation_log.get_or_insert_with(Vec::new);
    }
    /// Stop buffering and drop any pending records.
    pub fn stop_recording(&mut self) {
        self.mutation_log = None;
    }
    pub fn is_recording(&self) -> bool {
        self.mutation_log.is_some()
    }
    /// Drain the buffered records (keeps recording on). Empty if not recording.
    pub fn take_mutation_records(&mut self) -> Vec<crate::rtdom::mutations::MutationRecord> {
        self.mutation_log.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// Ensure a node's attr overlay exists (copy buffer attrs in on first write).
    fn ensure_attrs(&mut self, h: Handle) -> &mut Vec<(String, String)> {
        if !self.attrs_ov.contains_key(&h) {
            let init = self.attributes(h);
            self.attrs_ov.insert(h, init);
        }
        self.attrs_ov.get_mut(&h).unwrap()
    }

    pub fn set_attribute(&mut self, h: Handle, name: &str, value: &str) {
        let old = if self.is_recording() {
            self.get_attribute(h, name).map(|s| s.to_string())
        } else {
            None
        };
        let ov = self.ensure_attrs(h);
        if let Some(slot) = ov.iter_mut().find(|(n, _)| n == name) {
            slot.1 = value.to_string();
        } else {
            ov.push((name.to_string(), value.to_string()));
        }
        self.bump();
        self.record(crate::rtdom::mutations::MutationRecord::attributes(h, name, old));
    }

    pub fn remove_attribute(&mut self, h: Handle, name: &str) {
        let old = if self.is_recording() {
            self.get_attribute(h, name).map(|s| s.to_string())
        } else {
            None
        };
        let ov = self.ensure_attrs(h);
        ov.retain(|(n, _)| n != name);
        self.bump();
        self.record(crate::rtdom::mutations::MutationRecord::attributes(h, name, old));
    }

    /// Ensure a node's child overlay exists (copy buffer children in on first mutate).
    fn ensure_children(&mut self, h: Handle) -> &mut Vec<Handle> {
        if !self.children_ov.contains_key(&h) {
            let init = self.children(h);
            self.children_ov.insert(h, init);
        }
        self.children_ov.get_mut(&h).unwrap()
    }

    fn detach(&mut self, child: Handle) {
        if let Some(p) = self.parent(child) {
            let list = self.ensure_children(p);
            list.retain(|&x| x != child);
        }
    }

    pub fn append_child(&mut self, parent: Handle, child: Handle) {
        self.detach(child);
        self.ensure_children(parent).push(child);
        self.parent_ov.insert(child, Some(parent));
        self.bump();
        self.record(crate::rtdom::mutations::MutationRecord::child_list(parent, vec![child], vec![]));
    }

    pub fn insert_before(&mut self, parent: Handle, child: Handle, reference: Option<Handle>) {
        self.detach(child);
        let list = self.ensure_children(parent);
        match reference.and_then(|r| list.iter().position(|&x| x == r)) {
            Some(idx) => list.insert(idx, child),
            None => list.push(child),
        }
        self.parent_ov.insert(child, Some(parent));
        self.bump();
        self.record(crate::rtdom::mutations::MutationRecord::child_list(parent, vec![child], vec![]));
    }

    pub fn remove_child(&mut self, parent: Handle, child: Handle) {
        self.ensure_children(parent).retain(|&x| x != child);
        self.parent_ov.insert(child, None);
        self.bump();
        self.record(crate::rtdom::mutations::MutationRecord::child_list(parent, vec![], vec![child]));
    }

    pub fn set_text_content(&mut self, h: Handle, text: &str) {
        let nt = self.node_type(h);
        if nt == NodeType::Text || nt == NodeType::Comment {
            let old = if self.is_recording() { self.node_value(h) } else { None };
            self.text_ov.insert(h, text.to_string());
            self.bump();
            self.record(crate::rtdom::mutations::MutationRecord::character_data(h, old));
        } else {
            // replace children with a single text node — but per the DOM spec, the EMPTY string
            // produces NO child node (the element ends up empty). React's createRoot clears the
            // container via `textContent = ''` and relies on it leaving zero children.
            let removed = self.children(h);
            let added = if text.is_empty() {
                Vec::new()
            } else {
                let t = self.create_text_node(text);
                self.parent_ov.insert(t, Some(h));
                vec![t]
            };
            self.children_ov.insert(h, added.clone());
            self.bump();
            self.record(crate::rtdom::mutations::MutationRecord::child_list(h, added, removed));
        }
    }

    // ---- CharacterData mutation (text/comment nodes) -------------------------------------------
    // The DOM's CharacterData methods, operating on the node's text. Offsets/counts are in chars
    // (BMP-correct; matches JS string indexing for the ASCII/typed-text these drive). Records a
    // characterData mutation so a MutationObserver (Lexical's editor model) sees the edit.
    /// Set the text of a text/comment node directly (`CharacterData.data` / `nodeValue` setter).
    pub fn set_node_value(&mut self, h: Handle, text: &str) {
        let nt = self.node_type(h);
        if nt != NodeType::Text && nt != NodeType::Comment {
            return;
        }
        let old = if self.is_recording() { self.node_value(h) } else { None };
        self.text_ov.insert(h, text.to_string());
        self.bump();
        self.record(crate::rtdom::mutations::MutationRecord::character_data(h, old));
    }
    pub fn insert_data(&mut self, h: Handle, offset: usize, data: &str) {
        let chars: Vec<char> = self.node_value(h).unwrap_or_default().chars().collect();
        let off = offset.min(chars.len());
        let mut out: String = chars[..off].iter().collect();
        out.push_str(data);
        out.extend(chars[off..].iter());
        self.set_node_value(h, &out);
    }
    pub fn delete_data(&mut self, h: Handle, offset: usize, count: usize) {
        let chars: Vec<char> = self.node_value(h).unwrap_or_default().chars().collect();
        let off = offset.min(chars.len());
        let end = off.saturating_add(count).min(chars.len());
        let mut out: String = chars[..off].iter().collect();
        out.extend(chars[end..].iter());
        self.set_node_value(h, &out);
    }
    pub fn append_data(&mut self, h: Handle, data: &str) {
        let mut s = self.node_value(h).unwrap_or_default();
        s.push_str(data);
        self.set_node_value(h, &s);
    }
    pub fn replace_data(&mut self, h: Handle, offset: usize, count: usize, data: &str) {
        let chars: Vec<char> = self.node_value(h).unwrap_or_default().chars().collect();
        let off = offset.min(chars.len());
        let end = off.saturating_add(count).min(chars.len());
        let mut out: String = chars[..off].iter().collect();
        out.push_str(data);
        out.extend(chars[end..].iter());
        self.set_node_value(h, &out);
    }
    pub fn substring_data(&self, h: Handle, offset: usize, count: usize) -> String {
        let chars: Vec<char> = self.node_value(h).unwrap_or_default().chars().collect();
        let off = offset.min(chars.len());
        let end = off.saturating_add(count).min(chars.len());
        chars[off..end].iter().collect()
    }
    /// Split a text node at `offset`: truncate this node to the head, create a sibling holding the
    /// tail (inserted right after), and return the new node's handle.
    pub fn split_text(&mut self, h: Handle, offset: usize) -> Handle {
        let chars: Vec<char> = self.node_value(h).unwrap_or_default().chars().collect();
        let off = offset.min(chars.len());
        let head: String = chars[..off].iter().collect();
        let tail: String = chars[off..].iter().collect();
        self.set_node_value(h, &head);
        let new = self.create_text_node(&tail);
        if let Some(p) = self.parent(h) {
            let next = self.next_sibling(h);
            self.insert_before(p, new, next);
        }
        new
    }

    // ------------------------------------------------------------- creation
    fn push_node(&mut self, node: NewNode) -> Handle {
        let h = self.node_count();
        self.new_nodes.push(node);
        h
    }

    pub fn create_element(&mut self, tag: &str) -> Handle {
        let h = self.push_node(NewNode::Element { name: tag.to_ascii_lowercase(), ns: Namespace::Html });
        self.children_ov.insert(h, Vec::new());
        self.attrs_ov.insert(h, Vec::new());
        self.bump();
        h
    }

    pub fn create_element_ns(&mut self, tag: &str, ns: Namespace) -> Handle {
        let h = self.push_node(NewNode::Element { name: tag.to_string(), ns });
        self.children_ov.insert(h, Vec::new());
        self.attrs_ov.insert(h, Vec::new());
        self.bump();
        h
    }

    pub fn create_text_node(&mut self, data: &str) -> Handle {
        let h = self.push_node(NewNode::Text(data.to_string()));
        self.bump();
        h
    }

    pub fn create_comment(&mut self, data: &str) -> Handle {
        let h = self.push_node(NewNode::Comment(data.to_string()));
        self.bump();
        h
    }

    /// Parse `html` as a fragment in the context of element `h` and replace its
    /// children (innerHTML setter). Imported nodes become owned (new) nodes.
    pub fn set_inner_html(&mut self, h: Handle, html: &str) {
        let ctx = self.local_name(h).unwrap_or("body").to_string();
        let frag = core::parse_html_fragment_context(html, &ctx);
        let mut kids = Vec::with_capacity(frag.children.len());
        for c in &frag.children {
            kids.push(self.import_node(c));
        }
        for &k in &kids {
            self.parent_ov.insert(k, Some(h));
        }
        self.children_ov.insert(h, kids);
        self.bump();
    }

    // ---- ChildNode / ParentNode manipulation (spec methods) -----------------------------------
    /// Insert `nodes` (in order) immediately before `h` within `h`'s parent. No-op if detached.
    pub fn before(&mut self, h: Handle, nodes: &[Handle]) {
        let Some(p) = self.parent(h) else { return };
        for &n in nodes {
            self.insert_before(p, n, Some(h));
        }
    }
    /// Insert `nodes` (in order) immediately after `h` within `h`'s parent.
    pub fn after(&mut self, h: Handle, nodes: &[Handle]) {
        let Some(p) = self.parent(h) else { return };
        let next = self.next_sibling(h);
        for &n in nodes {
            self.insert_before(p, n, next);
        }
    }
    /// Replace `h` with `nodes` (in order) in `h`'s parent.
    pub fn replace_with(&mut self, h: Handle, nodes: &[Handle]) {
        let Some(p) = self.parent(h) else { return };
        let next = self.next_sibling(h);
        // Insert before h's next sibling so order is preserved, then remove h. (If a node in `nodes`
        // is h itself it is first detached by insert_before — harmless; spec dedups similarly.)
        for &n in nodes {
            if n != h {
                self.insert_before(p, n, next);
            }
        }
        self.remove_child(p, h);
    }
    /// Replace all children of `h` with `nodes` (in order).
    pub fn replace_children(&mut self, h: Handle, nodes: &[Handle]) {
        for c in self.children(h) {
            self.remove_child(h, c);
        }
        for &n in nodes {
            self.append_child(h, n);
        }
    }
    /// `insertAdjacentElement(position, element)` — position is beforebegin | afterbegin | beforeend
    /// | afterend (case-insensitive). Returns true if inserted.
    pub fn insert_adjacent_element(&mut self, h: Handle, position: &str, el: Handle) -> bool {
        match position.to_ascii_lowercase().as_str() {
            "beforebegin" => { if self.parent(h).is_some() { self.before(h, &[el]); true } else { false } }
            "afterend" => { if self.parent(h).is_some() { self.after(h, &[el]); true } else { false } }
            "afterbegin" => { let first = self.first_child(h); self.insert_before(h, el, first); true }
            "beforeend" => { self.append_child(h, el); true }
            _ => false,
        }
    }
    /// `insertAdjacentHTML(position, html)` — parse `html` as a fragment (in `h`'s context for
    /// beforeend/afterbegin, else the parent's) and insert the resulting nodes at `position`.
    pub fn insert_adjacent_html(&mut self, h: Handle, position: &str, html: &str) {
        let pos = position.to_ascii_lowercase();
        let ctx_h = match pos.as_str() {
            "beforebegin" | "afterend" => self.parent(h).unwrap_or(h),
            _ => h,
        };
        let ctx = self.local_name(ctx_h).unwrap_or("body").to_string();
        let frag = core::parse_html_fragment_context(html, &ctx);
        let kids: Vec<Handle> = frag.children.iter().map(|c| self.import_node(c)).collect();
        match pos.as_str() {
            "beforebegin" => self.before(h, &kids),
            "afterend" => self.after(h, &kids),
            "afterbegin" => { let first = self.first_child(h); for &k in &kids { self.insert_before(h, k, first); } }
            _ => for &k in &kids { self.append_child(h, k); }, // beforeend (default)
        }
    }
    /// `toggleAttribute(name, force?)` — add when absent (or force=Some(true)), remove when present
    /// (or force=Some(false)). Returns whether the attribute is present afterward.
    pub fn toggle_attribute(&mut self, h: Handle, name: &str, force: Option<bool>) -> bool {
        let present = self.has_attribute(h, name);
        let want = force.unwrap_or(!present);
        if want {
            if !present { self.set_attribute(h, name, ""); }
            true
        } else {
            if present { self.remove_attribute(h, name); }
            false
        }
    }
    /// `getAttributeNS(namespace, localName)` — our attrs are keyed by qualified name; match the
    /// local name directly, then any `prefix:localName` form (covers SVG `xlink:href` etc.).
    pub fn get_attribute_ns(&self, h: Handle, _ns: Option<&str>, local: &str) -> Option<String> {
        if let Some(v) = self.get_attribute(h, local) { return Some(v.to_string()); }
        let suffix = format!(":{local}");
        for (n, v) in self.attributes(h) {
            if n == local || n.ends_with(&suffix) { return Some(v); }
        }
        None
    }

    /// Recursively import a parsed `core::Node` as owned nodes. Returns its handle.
    fn import_node(&mut self, n: &core::Node) -> Handle {
        match NodeType::from_u8(n.node_type) {
            NodeType::Element => {
                let ns = ns_id(&n.namespace);
                let h = self.create_element_ns(&n.name, ns);
                for a in &n.attrs {
                    self.set_attribute(h, &a.name, &a.value);
                }
                let mut kids = Vec::with_capacity(n.children.len());
                for c in &n.children {
                    kids.push(self.import_node(c));
                }
                for &k in &kids {
                    self.parent_ov.insert(k, Some(h));
                }
                self.children_ov.insert(h, kids);
                h
            }
            NodeType::Comment => self.create_comment(&n.value),
            _ => self.create_text_node(&n.value),
        }
    }

    /// Force every buffer-backed node into the owned overlay (attrs + children).
    /// Defeats laziness on purpose — only for the lazy-vs-eager A/B bench. After
    /// this, reads go through the HashMap overlay instead of the zero-alloc buffer.
    pub fn force_inflate_all(&mut self) {
        for h in 0..self.buf_len {
            if self.node_type(h) == NodeType::Element {
                self.ensure_attrs(h);
            }
            self.ensure_children(h);
        }
    }

    // --------------------------------------------------------------- shadow DOM
    /// Attach a shadow root to `host` and return its handle (a FRAGMENT node).
    /// The shadow root is NOT a light child of the host — `children(host)` never
    /// includes it (encapsulation is free: queries over the light tree skip it).
    /// Populate it via `set_inner_html(shadow_root, ...)` or append_child.
    pub fn attach_shadow(&mut self, host: Handle) -> Handle {
        let sr = self.push_node(NewNode::Fragment);
        self.children_ov.insert(sr, Vec::new());
        self.shadow_root_of_host.insert(host, sr);
        self.host_of_shadow_root.insert(sr, host);
        self.bump();
        sr
    }

    pub fn shadow_root(&self, host: Handle) -> Option<Handle> {
        self.shadow_root_of_host.get(&host).copied()
    }
    pub fn shadow_host(&self, shadow_root: Handle) -> Option<Handle> {
        self.host_of_shadow_root.get(&shadow_root).copied()
    }
    pub fn is_shadow_root(&self, h: Handle) -> bool {
        self.host_of_shadow_root.contains_key(&h)
    }

    /// The shadow root that contains `h`, if any (walk to the top of `h`'s tree;
    /// a node inside a shadow tree tops out at the shadow-root fragment).
    pub fn shadow_root_of(&self, h: Handle) -> Option<Handle> {
        let mut top = h;
        while let Some(p) = self.parent(top) {
            top = p;
        }
        if self.is_shadow_root(top) {
            Some(top)
        } else {
            None
        }
    }

    /// DFS document-order descendants of `root` (excluding `root`).
    pub fn descendants(&self, root: Handle) -> Vec<Handle> {
        let mut out = Vec::new();
        let mut stack: Vec<Handle> = self.children(root).into_iter().rev().collect();
        while let Some(h) = stack.pop() {
            out.push(h);
            for &c in self.children(h).iter().rev() {
                stack.push(c);
            }
        }
        out
    }

    /// Light-DOM nodes assigned to `slot` (a `<slot>` inside a shadow tree):
    /// the host's light element children whose `slot` attr equals the slot's
    /// `name` (default "" ↔ unnamed slot). Mirrors `assignedNodes`.
    pub fn assigned_nodes(&self, slot: Handle) -> Vec<Handle> {
        // shadow_root_of yields a real shadow root (is_shadow_root ⇒ host exists),
        // so the only reachable miss is "slot not in any shadow tree".
        let host = match self.shadow_root_of(slot).and_then(|sr| self.shadow_host(sr)) {
            Some(host) => host,
            None => return Vec::new(),
        };
        let slot_name = self.get_attribute(slot, "name").unwrap_or("");
        self.children(host)
            .into_iter()
            .filter(|&c| self.node_type(c) == NodeType::Element)
            .filter(|&c| self.get_attribute(c, "slot").unwrap_or("") == slot_name)
            .collect()
    }
}

fn ns_id(namespace: &str) -> Namespace {
    match namespace {
        "svg" => Namespace::Svg,
        "math" => Namespace::MathMl,
        _ => Namespace::Html,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(html: &str) -> Tree {
        Tree::parse(html)
    }

    #[test]
    fn reads_structure_from_buffer() {
        let tree = t("<div id=a><span class=x>hi</span><p>yo</p></div>");
        // find the div
        let mut div = None;
        for h in 0..tree.node_count() {
            if tree.local_name(h) == Some("div") {
                div = Some(h);
            }
        }
        let div = div.unwrap();
        assert_eq!(tree.tag_name(div).as_deref(), Some("DIV"));
        assert_eq!(tree.get_attribute(div, "id"), Some("a"));
        let kids = tree.children(div);
        assert_eq!(kids.len(), 2);
        assert_eq!(tree.local_name(kids[0]), Some("span"));
        assert_eq!(tree.local_name(kids[1]), Some("p"));
        assert_eq!(tree.text_content(div), "hiyo");
    }

    #[test]
    fn set_attribute_is_cow_and_bumps_version() {
        let mut tree = t("<div id=a></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        let v0 = tree.version;
        tree.set_attribute(div, "class", "card");
        assert!(tree.version > v0);
        assert_eq!(tree.get_attribute(div, "class"), Some("card"));
        // original buffer attr still readable through overlay
        assert_eq!(tree.get_attribute(div, "id"), Some("a"));
    }

    #[test]
    fn append_and_remove_child() {
        let mut tree = t("<div></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        let span = tree.create_element("span");
        tree.append_child(div, span);
        assert_eq!(tree.children(div), vec![span]);
        assert_eq!(tree.parent(span), Some(div));
        assert_eq!(tree.tag_name(span).as_deref(), Some("SPAN"));
        tree.remove_child(div, span);
        assert!(tree.children(div).is_empty());
        assert_eq!(tree.parent(span), None);
    }

    #[test]
    fn create_text_and_set_text_content() {
        let mut tree = t("<p>old</p>");
        let p = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("p")).unwrap();
        tree.set_text_content(p, "new");
        assert_eq!(tree.text_content(p), "new");
    }

    #[test]
    fn dump_accessors_doctype_prefix_label() {
        // doctype name + public/system ids
        let dt = t("<!DOCTYPE html PUBLIC \"-//W3C//DTD\" \"sys.dtd\"><html></html>");
        let doctype = (0..dt.node_count()).find(|&h| dt.node_type(h) == NodeType::Doctype).unwrap();
        assert_eq!(dt.doctype_name(doctype), Some("html"));
        assert_eq!(dt.doctype_public_id(doctype), Some("-//W3C//DTD"));
        assert_eq!(dt.doctype_system_id(doctype), Some("sys.dtd"));
        // plain doctype: empty ids
        let dt2 = t("<!DOCTYPE html><html></html>");
        let d2 = (0..dt2.node_count()).find(|&h| dt2.node_type(h) == NodeType::Doctype).unwrap();
        assert_eq!(dt2.doctype_public_id(d2), Some(""));
        // doctype accessors return None for non-doctype + new nodes
        let html = dt2.get_elements_by_tag_name("html")[0];
        assert_eq!(dt2.doctype_name(html), None);
        let mut m = t("<div></div>");
        let txt = m.create_text_node("x");
        assert_eq!(m.doctype_name(txt), None);

        // node_label: element, template-content fragment, and None for text
        let tpl = t("<template><i></i></template>");
        let frag = (0..tpl.node_count()).find(|&h| tpl.node_type(h) == NodeType::Fragment).unwrap();
        assert_eq!(tpl.node_label(frag), Some("content"));
        let i = (0..tpl.node_count()).find(|&h| tpl.local_name(h) == Some("i")).unwrap();
        assert_eq!(tpl.node_label(i), Some("i"));
        let txt2 = (0..tpl.node_count()).find(|&h| tpl.node_type(h) == NodeType::Text);
        if let Some(tx) = txt2 {
            assert_eq!(tpl.node_label(tx), None);
        }
        // node_label for a created (is_new) element + a created fragment-less text
        let fresh = m.create_element("span");
        assert_eq!(m.node_label(fresh), Some("span"));
        assert_eq!(m.node_label(txt), None);

        // for_each_attr_full yields foreign-content prefix from the buffer
        let svg = t("<svg><a xlink:href=\"u\" id=\"k\"></a></svg>");
        let a = (0..svg.node_count()).find(|&h| svg.local_name(h) == Some("a")).unwrap();
        let mut got = Vec::new();
        svg.for_each_attr_full(a, |p, n, v| got.push(format!("{p}|{n}={v}")));
        assert!(got.contains(&"xlink|href=u".to_string()));
        assert!(got.contains(&"|id=k".to_string()));
        // overlay + is_new paths → empty prefix / no calls
        let mut o = t("<b></b>");
        let b = (0..o.node_count()).find(|&h| o.local_name(h) == Some("b")).unwrap();
        o.set_attribute(b, "class", "c");
        let mut ov = Vec::new();
        o.for_each_attr_full(b, |p, n, v| ov.push(format!("{p}|{n}={v}")));
        assert_eq!(ov, vec!["|class=c".to_string()]);
        let fresh_txt = o.create_text_node("z");
        o.for_each_attr_full(fresh_txt, |_, _, _| panic!("text has no attrs"));
    }

    #[test]
    fn for_each_attr_borrows_both_sources() {
        let tree = t("<a href=/x data-k=v>hi</a>");
        let a = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("a")).unwrap();
        let mut buf = Vec::new();
        tree.for_each_attr(a, |n, v| buf.push(format!("{n}={v}")));
        assert_eq!(buf, vec!["href=/x".to_string(), "data-k=v".to_string()]);
        // overlay source after a mutation
        let mut t2 = t("<b>x</b>");
        let b = (0..t2.node_count()).find(|&h| t2.local_name(h) == Some("b")).unwrap();
        t2.set_attribute(b, "class", "y");
        let mut ov = Vec::new();
        t2.for_each_attr(b, |n, v| ov.push(format!("{n}={v}")));
        assert_eq!(ov, vec!["class=y".to_string()]);
        // buffer element with NO attributes (start < 0) → no calls
        let t3 = t("<i>x</i>");
        let i = (0..t3.node_count()).find(|&h| t3.local_name(h) == Some("i")).unwrap();
        let mut count = 0;
        t3.for_each_attr(i, |_, _| count += 1);
        assert_eq!(count, 0);
        // created text node (is_new, no attrs overlay) → hits the is_new return
        let mut t4 = t("<div></div>");
        let txt = t4.create_text_node("hello");
        t4.for_each_attr(txt, |_, _| count += 1);
        assert_eq!(count, 0);
    }

    #[test]
    fn set_inner_html_imports_fragment() {
        let mut tree = t("<div></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        tree.set_inner_html(div, "<span class=a>hi</span><b>x</b>");
        let kids = tree.children(div);
        assert_eq!(kids.len(), 2);
        assert_eq!(tree.local_name(kids[0]), Some("span"));
        assert_eq!(tree.get_attribute(kids[0], "class"), Some("a"));
        assert_eq!(tree.local_name(kids[1]), Some("b"));
        assert_eq!(tree.text_content(div), "hix");
        assert_eq!(tree.parent(kids[0]), Some(div));
    }

    #[test]
    fn shadow_root_encapsulation_and_slots() {
        let mut tree = t("<my-card><span slot=title>Hi</span><p>light</p></my-card>");
        let host = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("my-card")).unwrap();
        let sr = tree.attach_shadow(host);
        tree.set_inner_html(sr, "<style>:host{color:red}</style><slot name=title></slot><slot></slot>");
        // encapsulation: shadow content is NOT in the light tree
        assert!(tree.shadow_root_of_host.get(&host).is_some());
        assert_eq!(tree.query_selector_all("slot").len(), 0); // light query skips shadow
        // shadow_root_of resolves for shadow-internal nodes
        let slots = tree.descendants(sr).into_iter().filter(|&h| tree.local_name(h) == Some("slot")).collect::<Vec<_>>();
        assert_eq!(slots.len(), 2);
        assert_eq!(tree.shadow_root_of(slots[0]), Some(sr));
        assert_eq!(tree.shadow_host(sr), Some(host));
        // assignedNodes: the named slot gets the span[slot=title]; default slot gets <p>
        let named = slots.iter().find(|&&s| tree.get_attribute(s, "name") == Some("title")).copied().unwrap();
        let default = slots.iter().find(|&&s| tree.get_attribute(s, "name").is_none()).copied().unwrap();
        let an = tree.assigned_nodes(named);
        assert_eq!(an.len(), 1);
        assert_eq!(tree.local_name(an[0]), Some("span"));
        assert_eq!(tree.assigned_nodes(default).len(), 1); // the <p>
    }

    #[test]
    fn siblings() {
        let tree = t("<ul><li>1</li><li>2</li><li>3</li></ul>");
        let lis: Vec<_> = (0..tree.node_count()).filter(|&h| tree.local_name(h) == Some("li")).collect();
        assert_eq!(tree.next_sibling(lis[0]), Some(lis[1]));
        assert_eq!(tree.previous_sibling(lis[2]), Some(lis[1]));
        assert_eq!(tree.next_sibling(lis[2]), None);
    }

    #[test]
    fn first_and_last_child() {
        let mut tree = t("<ul><li>1</li><li>2</li><li>3</li></ul>");
        let ul = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("ul")).unwrap();
        let kids = tree.children(ul);
        assert_eq!(tree.first_child(ul), Some(kids[0]));
        assert_eq!(tree.last_child(ul), Some(kids[2]));
        // previous_sibling of the first child is None (index 0 branch)
        assert_eq!(tree.previous_sibling(kids[0]), None);
        // empty element: no first/last child
        let empty = tree.create_element("div");
        assert_eq!(tree.first_child(empty), None);
        assert_eq!(tree.last_child(empty), None);
    }

    #[test]
    fn create_element_ns_keeps_case_and_namespace() {
        let mut tree = t("<div></div>");
        // SVG-namespace element: mixed-case local name preserved,
        // tag_name returns as-is (non-HTML namespace branch).
        let g = tree.create_element_ns("linearGradient", Namespace::Svg);
        assert_eq!(tree.namespace(g), Namespace::Svg);
        assert_eq!(tree.local_name(g), Some("linearGradient"));
        assert_eq!(tree.tag_name(g).as_deref(), Some("linearGradient"));
        // math namespace
        let m = tree.create_element_ns("mi", Namespace::MathMl);
        assert_eq!(tree.namespace(m), Namespace::MathMl);
        assert_eq!(tree.tag_name(m).as_deref(), Some("mi"));
    }

    #[test]
    fn new_node_reads_are_detached_and_empty() {
        let mut tree = t("<div></div>");
        let span = tree.create_element("span");
        // a freshly created (new) node with no parent overlay → detached
        assert_eq!(tree.parent(span), None);
        // get_attribute / has_attribute on an attr-less new node
        assert_eq!(tree.get_attribute(span, "id"), None);
        assert!(!tree.has_attribute(span, "id"));
        tree.set_attribute(span, "id", "x");
        assert!(tree.has_attribute(span, "id"));
        // attributes() on a brand-new text node (is_new, no attrs_ov) → empty
        let txt = tree.create_text_node("hi");
        assert!(tree.attributes(txt).is_empty());
        // get_attribute on a new node lacking an attr overlay (text node) → None
        assert_eq!(tree.get_attribute(txt, "foo"), None);
    }

    #[test]
    fn node_value_text_comment_and_none() {
        let mut tree = t("<p>hi</p><!--c-->");
        // node_value on an element → None (not text/comment)
        let p = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("p")).unwrap();
        assert_eq!(tree.node_value(p), None);
        // freshly created text node reads its data via is_new branch
        let txt = tree.create_text_node("hello");
        assert_eq!(tree.node_value(txt).as_deref(), Some("hello"));
        // a comment node from the buffer
        let comment = (0..tree.node_count()).find(|&h| tree.node_type(h) == NodeType::Comment).unwrap();
        assert_eq!(tree.node_value(comment).as_deref(), Some("c"));
        // text_content of a comment goes through the text/comment branch
        assert_eq!(tree.text_content(comment), "c");
        // text_ov branch: mutate text content of the text node, read back
        tree.set_text_content(txt, "changed");
        assert_eq!(tree.node_value(txt).as_deref(), Some("changed"));
    }

    #[test]
    fn empty_buffer_text_node_value() {
        // An empty text node (text_id < 0) → node_value is empty string.
        let tree = t("<p></p>");
        // collect_text skips comments: a comment inside an element is ignored.
        let p = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("p")).unwrap();
        assert_eq!(tree.text_content(p), "");
    }

    #[test]
    fn text_content_skips_comments() {
        // The NodeType::Comment arm of collect_text is exercised: comment text is NOT
        // concatenated into an element's text_content.
        let tree = t("<div>a<!--ignored-->b</div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        assert_eq!(tree.text_content(div), "ab");
    }

    #[test]
    fn set_attribute_updates_existing_slot() {
        let mut tree = t("<div id=a></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        // overwrite an existing attribute (the find-and-update slot branch)
        tree.set_attribute(div, "id", "b");
        assert_eq!(tree.get_attribute(div, "id"), Some("b"));
        assert_eq!(tree.attributes(div).len(), 1);
    }

    #[test]
    fn remove_attribute_works() {
        let mut tree = t("<div id=a class=x></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        let v0 = tree.version;
        tree.remove_attribute(div, "id");
        assert!(tree.version > v0);
        assert!(!tree.has_attribute(div, "id"));
        assert_eq!(tree.get_attribute(div, "class"), Some("x"));
    }

    #[test]
    fn insert_before_with_reference_and_append() {
        let mut tree = t("<ul><li>1</li><li>2</li></ul>");
        let ul = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("ul")).unwrap();
        let kids = tree.children(ul);
        let (a, b) = (kids[0], kids[1]);
        // insert before a reference node (the Some(idx) branch)
        let x = tree.create_element("li");
        tree.insert_before(ul, x, Some(b));
        assert_eq!(tree.children(ul), vec![a, x, b]);
        assert_eq!(tree.parent(x), Some(ul));
        // insert with None reference → append (the None branch)
        let y = tree.create_element("li");
        tree.insert_before(ul, y, None);
        assert_eq!(tree.children(ul), vec![a, x, b, y]);
        // detach-with-parent: re-inserting x moves it (detach removes from parent)
        tree.insert_before(ul, x, Some(a));
        assert_eq!(tree.children(ul), vec![x, a, b, y]);
        // reference that is not a child → falls through to append
        let z = tree.create_element("li");
        tree.insert_before(ul, z, Some(9999));
        assert_eq!(tree.children(ul).last().copied(), Some(z));
    }

    #[test]
    fn set_text_content_on_element_replaces_children() {
        let mut tree = t("<div><span>old</span></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        // element branch: replaced with a single text node
        tree.set_text_content(div, "new");
        assert_eq!(tree.text_content(div), "new");
        let kids = tree.children(div);
        assert_eq!(kids.len(), 1);
        assert_eq!(tree.node_type(kids[0]), NodeType::Text);
        assert_eq!(tree.parent(kids[0]), Some(div));
    }

    #[test]
    fn create_comment_node() {
        let mut tree = t("<div></div>");
        let v0 = tree.version;
        let c = tree.create_comment("note");
        assert!(tree.version > v0);
        assert_eq!(tree.node_type(c), NodeType::Comment);
        assert_eq!(tree.node_value(c).as_deref(), Some("note"));
    }

    #[test]
    fn set_inner_html_imports_comment_and_svg() {
        let mut tree = t("<div></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        // imports an element, a comment (NodeType::Comment arm), and an svg element (ns).
        tree.set_inner_html(div, "<p>hi</p><!--c--><svg><circle r=5></circle></svg>");
        let kids = tree.children(div);
        // p, comment, svg
        let comment = kids.iter().copied().find(|&k| tree.node_type(k) == NodeType::Comment).unwrap();
        assert_eq!(tree.node_value(comment).as_deref(), Some("c"));
        let svg = kids.iter().copied().find(|&k| tree.local_name(k) == Some("svg")).unwrap();
        assert_eq!(tree.namespace(svg), Namespace::Svg);
        // svg child carries the svg namespace too
        let circle = tree.children(svg).into_iter().find(|&k| tree.local_name(k) == Some("circle")).unwrap();
        assert_eq!(tree.namespace(circle), Namespace::Svg);
    }

    #[test]
    fn force_inflate_all_preserves_reads() {
        let mut tree = t("<div id=a><span class=x>hi</span></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        tree.force_inflate_all();
        // after inflation reads go through overlays but stay correct
        assert_eq!(tree.get_attribute(div, "id"), Some("a"));
        let kids = tree.children(div);
        assert_eq!(tree.local_name(kids[0]), Some("span"));
        assert_eq!(tree.get_attribute(kids[0], "class"), Some("x"));
        assert_eq!(tree.text_content(div), "hi");
    }

    #[test]
    fn assigned_nodes_without_shadow_is_empty() {
        let tree = t("<div><span>x</span></div>");
        // a node not inside any shadow tree → shadow_root_of None → empty
        let span = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("span")).unwrap();
        assert!(tree.assigned_nodes(span).is_empty());
    }
}

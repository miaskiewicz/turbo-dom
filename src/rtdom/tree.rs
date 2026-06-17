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

pub const ELEMENT_NODE: u8 = 1;
pub const TEXT_NODE: u8 = 3;
pub const COMMENT_NODE: u8 = 8;
pub const DOCUMENT_NODE: u8 = 9;
pub const DOCTYPE_NODE: u8 = 10;
pub const FRAGMENT_NODE: u8 = 11;

/// A node handle. Index into the buffer, or (>= buf_len) into `new_nodes`.
pub type Handle = u32;

struct NewNode {
    node_type: u8,
    name: String, // local name (elements) or data (text/comment)
    ns: u8,
}

pub struct Tree {
    buf: Soa,
    buf_len: u32,
    new_nodes: Vec<NewNode>,
    // --- COW overlays (present only for mutated/created nodes) ---
    parent_ov: HashMap<Handle, i32>,
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
}

#[derive(Default)]
pub(crate) struct QueryCache {
    pub version: u64,
    pub map: HashMap<String, Vec<Handle>>,
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
    pub fn node_type(&self, h: Handle) -> u8 {
        if self.is_new(h) {
            self.new_ref(h).node_type
        } else {
            self.buf.node_type[h as usize]
        }
    }

    pub fn namespace_id(&self, h: Handle) -> u8 {
        if self.is_new(h) {
            self.new_ref(h).ns
        } else {
            self.buf.ns[h as usize]
        }
    }

    /// localName (lowercase as parsed). None for non-elements.
    pub fn local_name(&self, h: Handle) -> Option<&str> {
        if self.node_type(h) != ELEMENT_NODE {
            return None;
        }
        if self.is_new(h) {
            Some(&self.new_ref(h).name)
        } else {
            Some(&self.buf.tag_names[self.buf.tag_id[h as usize] as usize])
        }
    }

    /// tagName: uppercased for HTML-namespace elements (DOM contract), as-is otherwise.
    pub fn tag_name(&self, h: Handle) -> Option<String> {
        let ln = self.local_name(h)?;
        if self.namespace_id(h) == 0 {
            Some(ln.to_ascii_uppercase())
        } else {
            Some(ln.to_string())
        }
    }

    pub fn parent(&self, h: Handle) -> Option<Handle> {
        if let Some(&p) = self.parent_ov.get(&h) {
            return if p < 0 { None } else { Some(p as Handle) };
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

    // ----------------------------------------------------------------- text
    pub fn node_value(&self, h: Handle) -> Option<String> {
        let nt = self.node_type(h);
        if nt != TEXT_NODE && nt != COMMENT_NODE {
            return None;
        }
        if let Some(t) = self.text_ov.get(&h) {
            return Some(t.clone());
        }
        if self.is_new(h) {
            return Some(self.new_ref(h).name.clone());
        }
        // buffer text/comment nodes always have text_id >= 0 (core pools every
        // string); .get + default keeps this total and coverable.
        let tid = self.buf.text_id[h as usize];
        Some(self.buf.strings.get(tid as usize).cloned().unwrap_or_default())
    }

    /// textContent: concatenated descendant text.
    pub fn text_content(&self, h: Handle) -> String {
        let nt = self.node_type(h);
        if nt == TEXT_NODE || nt == COMMENT_NODE {
            return self.node_value(h).unwrap_or_default();
        }
        let mut out = String::new();
        self.collect_text(h, &mut out);
        out
    }
    fn collect_text(&self, h: Handle, out: &mut String) {
        for c in self.children(h) {
            match self.node_type(c) {
                TEXT_NODE => out.push_str(&self.node_value(c).unwrap_or_default()),
                COMMENT_NODE => {}
                _ => self.collect_text(c, out),
            }
        }
    }

    // ------------------------------------------------------------- mutations
    fn bump(&mut self) {
        self.version += 1;
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
        let ov = self.ensure_attrs(h);
        if let Some(slot) = ov.iter_mut().find(|(n, _)| n == name) {
            slot.1 = value.to_string();
        } else {
            ov.push((name.to_string(), value.to_string()));
        }
        self.bump();
    }

    pub fn remove_attribute(&mut self, h: Handle, name: &str) {
        let ov = self.ensure_attrs(h);
        ov.retain(|(n, _)| n != name);
        self.bump();
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
        self.parent_ov.insert(child, parent as i32);
        self.bump();
    }

    pub fn insert_before(&mut self, parent: Handle, child: Handle, reference: Option<Handle>) {
        self.detach(child);
        let list = self.ensure_children(parent);
        match reference.and_then(|r| list.iter().position(|&x| x == r)) {
            Some(idx) => list.insert(idx, child),
            None => list.push(child),
        }
        self.parent_ov.insert(child, parent as i32);
        self.bump();
    }

    pub fn remove_child(&mut self, parent: Handle, child: Handle) {
        self.ensure_children(parent).retain(|&x| x != child);
        self.parent_ov.insert(child, -1);
        self.bump();
    }

    pub fn set_text_content(&mut self, h: Handle, text: &str) {
        let nt = self.node_type(h);
        if nt == TEXT_NODE || nt == COMMENT_NODE {
            self.text_ov.insert(h, text.to_string());
        } else {
            // replace children with a single text node
            let t = self.create_text_node(text);
            self.children_ov.insert(h, vec![t]);
            self.parent_ov.insert(t, h as i32);
        }
        self.bump();
    }

    // ------------------------------------------------------------- creation
    fn push_new(&mut self, node_type: u8, name: String, ns: u8) -> Handle {
        let h = self.node_count();
        self.new_nodes.push(NewNode { node_type, name, ns });
        h
    }

    pub fn create_element(&mut self, tag: &str) -> Handle {
        let h = self.push_new(ELEMENT_NODE, tag.to_ascii_lowercase(), 0);
        self.children_ov.insert(h, Vec::new());
        self.attrs_ov.insert(h, Vec::new());
        self.bump();
        h
    }

    pub fn create_element_ns(&mut self, tag: &str, ns: u8) -> Handle {
        let h = self.push_new(ELEMENT_NODE, tag.to_string(), ns);
        self.children_ov.insert(h, Vec::new());
        self.attrs_ov.insert(h, Vec::new());
        self.bump();
        h
    }

    pub fn create_text_node(&mut self, data: &str) -> Handle {
        let h = self.push_new(TEXT_NODE, data.to_string(), 0);
        self.bump();
        h
    }

    pub fn create_comment(&mut self, data: &str) -> Handle {
        let h = self.push_new(COMMENT_NODE, data.to_string(), 0);
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
            self.parent_ov.insert(k, h as i32);
        }
        self.children_ov.insert(h, kids);
        self.bump();
    }

    /// Recursively import a parsed `core::Node` as owned nodes. Returns its handle.
    fn import_node(&mut self, n: &core::Node) -> Handle {
        match n.node_type {
            ELEMENT_NODE => {
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
                    self.parent_ov.insert(k, h as i32);
                }
                self.children_ov.insert(h, kids);
                h
            }
            COMMENT_NODE => self.create_comment(&n.value),
            _ => self.create_text_node(&n.value),
        }
    }

    /// Force every buffer-backed node into the owned overlay (attrs + children).
    /// Defeats laziness on purpose — only for the lazy-vs-eager A/B bench. After
    /// this, reads go through the HashMap overlay instead of the zero-alloc buffer.
    pub fn force_inflate_all(&mut self) {
        for h in 0..self.buf_len {
            if self.node_type(h) == ELEMENT_NODE {
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
        let sr = self.push_new(FRAGMENT_NODE, String::new(), 0);
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
            .filter(|&c| self.node_type(c) == ELEMENT_NODE)
            .filter(|&c| self.get_attribute(c, "slot").unwrap_or("") == slot_name)
            .collect()
    }
}

fn ns_id(namespace: &str) -> u8 {
    match namespace {
        "svg" => 1,
        "math" => 2,
        _ => 0,
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
        // SVG-namespace element: ns_id 1, mixed-case local name preserved,
        // tag_name returns as-is (non-HTML namespace branch).
        let g = tree.create_element_ns("linearGradient", 1);
        assert_eq!(tree.namespace_id(g), 1);
        assert_eq!(tree.local_name(g), Some("linearGradient"));
        assert_eq!(tree.tag_name(g).as_deref(), Some("linearGradient"));
        // math namespace
        let m = tree.create_element_ns("mi", 2);
        assert_eq!(tree.namespace_id(m), 2);
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
        let comment = (0..tree.node_count()).find(|&h| tree.node_type(h) == COMMENT_NODE).unwrap();
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
        // The COMMENT_NODE arm of collect_text is exercised: comment text is NOT
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
        assert_eq!(tree.node_type(kids[0]), TEXT_NODE);
        assert_eq!(tree.parent(kids[0]), Some(div));
    }

    #[test]
    fn create_comment_node() {
        let mut tree = t("<div></div>");
        let v0 = tree.version;
        let c = tree.create_comment("note");
        assert!(tree.version > v0);
        assert_eq!(tree.node_type(c), COMMENT_NODE);
        assert_eq!(tree.node_value(c).as_deref(), Some("note"));
    }

    #[test]
    fn set_inner_html_imports_comment_and_svg() {
        let mut tree = t("<div></div>");
        let div = (0..tree.node_count()).find(|&h| tree.local_name(h) == Some("div")).unwrap();
        // imports an element, a comment (COMMENT_NODE arm), and an svg element (ns).
        tree.set_inner_html(div, "<p>hi</p><!--c--><svg><circle r=5></circle></svg>");
        let kids = tree.children(div);
        // p, comment, svg
        let comment = kids.iter().copied().find(|&k| tree.node_type(k) == COMMENT_NODE).unwrap();
        assert_eq!(tree.node_value(comment).as_deref(), Some("c"));
        let svg = kids.iter().copied().find(|&k| tree.local_name(k) == Some("svg")).unwrap();
        assert_eq!(tree.namespace_id(svg), 1);
        // svg child carries the svg namespace too
        let circle = tree.children(svg).into_iter().find(|&k| tree.local_name(k) == Some("circle")).unwrap();
        assert_eq!(tree.namespace_id(circle), 1);
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

//! Shared parser core. No binding deps — html5ever in, plain nested tree out.
//! Both front-ends (napi-rs native, wasm-bindgen fallback) wrap this unchanged.
//!
//! Plan note: this is the "full marshaling" version — a *complete* tree.
//! The SoA flat-buffer (architecture Layer 1) is a later optimization, gated on
//! marshaling proving to be the actual cost. Do not SoA-ify here yet.

use html5ever::driver::ParseOpts;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{parse_document, parse_fragment, local_name, namespace_url, ns, LocalName, QualName};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// DOM nodeType constants (subset html5ever can emit).
pub const ELEMENT_NODE: u8 = 1;
pub const TEXT_NODE: u8 = 3;
pub const PROCESSING_INSTRUCTION_NODE: u8 = 7;
pub const COMMENT_NODE: u8 = 8;
pub const DOCUMENT_NODE: u8 = 9;
pub const DOCUMENT_TYPE_NODE: u8 = 10;
pub const DOCUMENT_FRAGMENT_NODE: u8 = 11;

#[cfg_attr(feature = "wasm-bind", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    pub value: String,
    /// Namespace prefix for foreign-content attrs ("xlink", "xml", "xmlns"); empty otherwise.
    pub prefix: String,
}

/// One marshaled DOM node. Nested children = complete tree, no lazy indices.
#[cfg_attr(feature = "wasm-bind", derive(serde::Serialize), serde(rename_all = "camelCase"))]
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
        tree_builder: TreeBuilderOpts { scripting_enabled: false, ..Default::default() },
        ..Default::default()
    }
}

/// Parse a full HTML document. Always yields a Document root (nodeType 9).
pub fn parse_html_document(html: &str) -> Node {
    let dom = parse_document(RcDom::default(), opts())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("RcDom read_from is infallible over a byte slice");
    walk(&dom.document)
}

// ============================ SoA flat buffer ============================
// Structure-of-Arrays: parser emits compact parallel arrays once, crossed over
// the boundary as cheap typed-array copies. JS reads tree structure straight from
// the arrays and inflates node objects only on access (no eager full-tree alloc).

/// Index-addressed flat tree. Node index IS its id; node 0 is the document.
#[cfg_attr(feature = "wasm-bind", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[derive(Default)]
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
    pub attr_name_id: Vec<u32>, // index into attr_names
    pub attr_value_id: Vec<u32>, // index into attr_values (interned, deduped)
    pub attr_prefix_id: Vec<u32>, // index into attr_prefixes
    // string tables (interned, deduped)
    pub tag_names: Vec<String>,
    pub attr_names: Vec<String>,
    pub attr_prefixes: Vec<String>,
    pub attr_values: Vec<String>, // interned attr value dictionary
    pub strings: Vec<String>,   // text/comment/doctype data, pooled
}

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

    // Allocate this node + descendants; return its index.
    fn alloc(&mut self, handle: &Handle, parent: i32) -> i32 {
        let idx = self.soa.node_type.len();
        // push placeholders; fill scalars below
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

        let mut template_content: Option<Handle> = None;

        match &handle.data {
            NodeData::Document => self.soa.node_type[idx] = DOCUMENT_NODE,
            NodeData::Doctype { name, public_id, system_id } => {
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
            NodeData::ProcessingInstruction { target, contents } => {
                self.soa.node_type[idx] = PROCESSING_INSTRUCTION_NODE;
                let combined = format!("{} {}", target, contents);
                self.soa.text_id[idx] = self.push_string(&combined);
            }
            NodeData::Element { name, attrs, template_contents, .. } => {
                self.soa.node_type[idx] = ELEMENT_NODE;
                self.soa.ns[idx] = if name.ns == ns!(svg) { 1 } else if name.ns == ns!(mathml) { 2 } else { 0 };
                self.soa.tag_id[idx] = self.intern_tag(&name.local);
                let borrowed = attrs.borrow();
                if !borrowed.is_empty() {
                    self.soa.attr_start[idx] = self.soa.attr_name_id.len() as i32;
                    self.soa.attr_count[idx] = borrowed.len() as u16;
                    for attr in borrowed.iter() {
                        let nid = self.intern_attr_name(&attr.name.local);
                        self.soa.attr_name_id.push(nid);
                        let vid = self.intern_attr_value(&attr.value);
                        self.soa.attr_value_id.push(vid);
                        let pfx = attr.name.prefix.as_ref().map(|p| p.to_string()).unwrap_or_default();
                        let pid = self.intern_attr_prefix(&pfx);
                        self.soa.attr_prefix_id.push(pid);
                    }
                }
                if &*name.local == "template" {
                    if let Some(c) = &*template_contents.borrow() {
                        template_content = Some(c.clone());
                    }
                }
            }
        }

        // children, linking first_child / next_sib (inline; no closure to avoid borrow churn)
        let mut prev = -1i32;
        for child in handle.children.borrow().iter() {
            let cidx = self.alloc(child, idx as i32);
            if prev == -1 { self.soa.first_child[idx] = cidx; } else { self.soa.next_sib[prev as usize] = cidx; }
            prev = cidx;
        }
        // <template> content as a synthetic document-fragment child named "content"
        if let Some(content) = template_content {
            let cidx = self.soa.node_type.len() as i32;
            self.soa.node_type.push(DOCUMENT_FRAGMENT_NODE);
            self.soa.ns.push(0);
            let content_tag = self.intern_tag("content");
            self.soa.tag_id.push(content_tag);
            self.soa.parent.push(idx as i32);
            self.soa.first_child.push(-1);
            self.soa.next_sib.push(-1);
            self.soa.text_id.push(-1);
            self.soa.pub_id.push(-1);
            self.soa.sys_id.push(-1);
            self.soa.attr_start.push(-1);
            self.soa.attr_count.push(0);
            let mut cprev = -1i32;
            for gc in content.children.borrow().iter() {
                let gcidx = self.alloc(gc, cidx);
                if cprev == -1 { self.soa.first_child[cidx as usize] = gcidx; } else { self.soa.next_sib[cprev as usize] = gcidx; }
                cprev = gcidx;
            }
            // link the content fragment as a child of the template
            if prev == -1 { self.soa.first_child[idx] = cidx; } else { self.soa.next_sib[prev as usize] = cidx; }
            prev = cidx;
        }
        let _ = prev;

        idx as i32
    }
}

/// Parse a full document into the SoA flat buffer.
pub fn parse_html_soa(html: &str) -> Soa {
    let dom = parse_document(RcDom::default(), opts())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("RcDom read_from is infallible over a byte slice");
    let mut b = SoaBuilder {
        soa: Soa::default(),
        // pre-reserve typical intern-table sizes so the build loop doesn't rehash:
        // tag/attr-name sets are small + bounded; values vary more.
        tag_map: rustc_hash::FxHashMap::with_capacity_and_hasher(32, Default::default()),
        attr_name_map: rustc_hash::FxHashMap::with_capacity_and_hasher(64, Default::default()),
        attr_prefix_map: rustc_hash::FxHashMap::with_capacity_and_hasher(8, Default::default()),
        attr_value_map: rustc_hash::FxHashMap::with_capacity_and_hasher(128, Default::default()),
    };
    b.alloc(&dom.document, -1);
    b.soa
}

/// Parse a document and return only the node count — no `core::Node` tree, no
/// marshaling. Isolates raw html5ever parse cost from tree-build + boundary cost.
pub fn parse_html_document_count(html: &str) -> u32 {
    let dom = parse_document(RcDom::default(), opts())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("RcDom read_from is infallible over a byte slice");
    count(&dom.document)
}

fn count(handle: &Handle) -> u32 {
    1 + handle.children.borrow().iter().map(count).sum::<u32>()
}

/// Parse an HTML fragment (e.g. an `innerHTML=` set) in `<body>` context.
/// Returns a synthetic Document-fragment-ish root whose children are the parsed nodes.
pub fn parse_html_fragment(html: &str) -> Node {
    parse_html_fragment_in(html, "", "body")
}

/// Parse a fragment given an html5lib-style context string:
/// "" / "body" / "td" (html ns) or "svg path" / "math ms" (foreign ns).
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
pub fn parse_html_fragment_in(html: &str, context_ns: &str, context_local: &str) -> Node {
    let ns = match context_ns {
        "svg" => ns!(svg),
        "math" => ns!(mathml),
        _ => ns!(html),
    };
    let ctx = QualName::new(None, ns, LocalName::from(context_local));
    let dom = parse_fragment(RcDom::default(), opts(), ctx, vec![], false)
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("RcDom read_from is infallible over a byte slice");
    // parse_fragment wraps results under a synthetic <html> element; unwrap it.
    let root = walk(&dom.document);
    let mut frag = Node {
        node_type: DOCUMENT_NODE,
        name: "#document-fragment".to_string(),
        value: String::new(),
        namespace: String::new(),
        public_id: String::new(),
        system_id: String::new(),
        attrs: Vec::new(),
        children: Vec::new(),
    };
    for child in root.children {
        if child.node_type == ELEMENT_NODE && child.name == "html" {
            frag.children.extend(child.children);
        } else {
            frag.children.push(child);
        }
    }
    frag
}

fn walk(handle: &Handle) -> Node {
    let node = handle;
    let mut public_id = String::new();
    let mut system_id = String::new();
    let (node_type, name, value, namespace, attrs) = match &node.data {
        NodeData::Document => {
            (DOCUMENT_NODE, "#document".to_string(), String::new(), String::new(), Vec::new())
        }
        NodeData::Doctype { name, public_id: pub_id, system_id: sys_id } => {
            public_id = pub_id.to_string();
            system_id = sys_id.to_string();
            (DOCUMENT_TYPE_NODE, name.to_string(), String::new(), String::new(), Vec::new())
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
        NodeData::ProcessingInstruction { target, contents } => (
            PROCESSING_INSTRUCTION_NODE,
            target.to_string(),
            contents.to_string(),
            String::new(),
            Vec::new(),
        ),
        NodeData::Element { name, attrs, .. } => {
            let a = attrs
                .borrow()
                .iter()
                .map(|attr| Attr {
                    name: attr.name.local.to_string(),
                    value: attr.value.to_string(),
                    prefix: attr.name.prefix.as_ref().map(|p| p.to_string()).unwrap_or_default(),
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

    let mut children: Vec<Node> = node.children.borrow().iter().map(walk).collect();

    // <template> content lives in a separate document fragment, not in children.
    // html5lib prints it as a synthetic `content` fragment node holding the parsed tree.
    if let NodeData::Element { template_contents, .. } = &node.data {
        if let Some(contents) = &*template_contents.borrow() {
            let inner = contents.children.borrow().iter().map(walk).collect();
            children.push(Node {
                node_type: DOCUMENT_FRAGMENT_NODE,
                name: "content".to_string(),
                value: String::new(),
                namespace: String::new(),
                public_id: String::new(),
                system_id: String::new(),
                attrs: Vec::new(),
                children: inner,
            });
        }
    }

    Node { node_type, name, value, namespace, public_id, system_id, attrs, children }
}

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
    fn fragment_context_namespace() {
        let frag = parse_html_fragment_context("<rect/>", "svg path");
        // context is svg path, so a bare <rect> parses in the SVG namespace
        let rect = first(&frag, &|n| n.name == "rect");
        assert!(rect.map(|n| n.namespace == "svg").unwrap_or(false));
    }
}

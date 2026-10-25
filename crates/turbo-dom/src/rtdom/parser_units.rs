//! An injective, tokenizer-local transport for UTF-16 DOM strings. Only the
//! scalar tokenizer sees escapes: native nodes immediately store original units.
use super::unit_tokenizer::{BufferQueue, Tokenizer, TokenizerOpts};
use super::{
    core, DoctypeData, DomString, Handle, Namespace, NewNode, NodeType, ProcessingInstructionData,
    Tree,
};
use html5ever::tree_builder::{QuirksMode, TreeBuilder, TreeBuilderOpts};
use html5ever::{LocalName, QualName};
use markup5ever_rcdom::RcDom;
pub(super) const ESCAPE: char = '\u{f0000}';
const UNIT_BASE: u32 = 0xf0800;
pub(super) fn encode(units: &[u16]) -> String {
    let mut source = String::new();
    for value in char::decode_utf16(units.iter().copied()) {
        match value {
            Ok(c) => {
                source.push(c);
                if c == ESCAPE {
                    source.push(c);
                }
            }
            Err(error) => {
                source.push(ESCAPE);
                source.push(
                    char::from_u32(UNIT_BASE + u32::from(error.unpaired_surrogate() - 0xd800))
                        .unwrap(),
                );
            }
        }
    }
    source
}
pub(super) fn decode(source: &str) -> Vec<u16> {
    let mut chars = source.chars();
    let mut units = Vec::new();
    while let Some(c) = chars.next() {
        let c = if c == ESCAPE {
            let escaped = chars.next().expect("well formed internal unit transport");
            if escaped != ESCAPE {
                let offset = u32::from(escaped)
                    .checked_sub(UNIT_BASE)
                    .filter(|&offset| offset < 2048)
                    .expect("internal lone unit escape");
                units.push(0xd800 + offset as u16);
                continue;
            }
            escaped
        } else {
            c
        };
        let mut buffer = [0; 2];
        units.extend_from_slice(c.encode_utf16(&mut buffer));
    }
    units
}
fn parsed(units: &[u16], context: Option<(QualName, Vec<html5ever::Attribute>)>) -> RcDom {
    parsed_with_form(units, context, false, false, QuirksMode::NoQuirks)
}
// The iframe srcdoc document flag: a missing or legacy doctype never selects quirks.
fn parsed_srcdoc(units: &[u16]) -> RcDom {
    let options = TreeBuilderOpts {
        iframe_srcdoc: true,
        ..Default::default()
    };
    parsed_with_options(units, None, false, options)
}
fn parsed_with_form(
    units: &[u16],
    context: Option<(QualName, Vec<html5ever::Attribute>)>,
    has_form_ancestor: bool,
    scripting_enabled: bool,
    quirks_mode: QuirksMode,
) -> RcDom {
    let options = TreeBuilderOpts {
        scripting_enabled,
        quirks_mode,
        ..Default::default()
    };
    parsed_with_options(units, context, has_form_ancestor, options)
}
fn parsed_with_options(
    units: &[u16],
    context: Option<(QualName, Vec<html5ever::Attribute>)>,
    has_form_ancestor: bool,
    options: TreeBuilderOpts,
) -> RcDom {
    let sink = RcDom::default();
    let fragment = context.is_some();
    let builder = if let Some((name, attributes)) = context {
        let element = html5ever::tree_builder::create_element(&sink, name, attributes);
        let form = has_form_ancestor.then(|| {
            html5ever::tree_builder::create_element(
                &sink,
                QualName::new(
                    None,
                    html5ever::Namespace::from("http://www.w3.org/1999/xhtml"),
                    LocalName::from("form"),
                ),
                Vec::new(),
            )
        });
        TreeBuilder::new_for_fragment(sink, element, form, options)
    } else {
        TreeBuilder::new(sink, options)
    };
    let initial_state = fragment.then(|| builder.tokenizer_state_for_context_elem(false));
    let tokenizer = Tokenizer::new(
        builder,
        TokenizerOpts {
            discard_bom: false,
            initial_state,
            ..Default::default()
        },
    );
    let queue = BufferQueue::default();
    queue.push_back(encode(units).into());
    // Parser indicators need no host work: this API never executes scripts.
    while !matches!(tokenizer.feed(&queue), html5ever::TokenizerResult::Done) {}
    tokenizer.end();
    tokenizer.sink.sink
}
impl Tree {
    fn fragment_context(
        &self,
        node: Handle,
        transport: bool,
    ) -> (QualName, Vec<html5ever::Attribute>) {
        let scalar = |units: &[u16]| {
            if transport {
                encode(units)
            } else {
                String::from_utf16_lossy(units)
            }
        };
        let local = self
            .local_name_utf16(node)
            .unwrap_or_else(|| "body".encode_utf16().collect());
        let namespace = self
            .namespace_uri_utf16(node)
            .unwrap_or_else(|| "http://www.w3.org/1999/xhtml".encode_utf16().collect());
        let name = QualName::new(
            None,
            html5ever::Namespace::from(scalar(&namespace)),
            LocalName::from(scalar(&local)),
        );
        let attributes = self
            .attributes_units(node)
            .into_iter()
            .map(|data| html5ever::Attribute {
                name: QualName::new(
                    data.prefix
                        .as_deref()
                        .map(|units| html5ever::Prefix::from(scalar(units))),
                    html5ever::Namespace::from(
                        data.namespace.as_deref().map_or_else(String::new, scalar),
                    ),
                    LocalName::from(scalar(&data.local_name)),
                ),
                value: scalar(&data.value).into(),
            })
            .collect();
        (name, attributes)
    }
    pub(super) fn parse_scalar_fragment(&self, node: Handle, html: &str) -> core::Node {
        let (name, attributes) = self.fragment_context(node, false);
        core::parse_html_fragment_with_context(html, name, attributes)
    }
    /// Parse a detached HTML fragment using the actual context element's name,
    /// namespace, attributes and nearest form ancestor. Existing context children
    /// and pending mutation records are preserved. The returned fragment is an
    /// ordinary collectable component: callers retain its handle until adoption.
    /// `scripting_enabled` selects the HTML tokenizer's `noscript` behavior;
    /// this method never executes scripts or invokes custom-element callbacks.
    #[must_use]
    pub fn create_contextual_fragment_utf16(
        &mut self,
        context: Handle,
        units: &[u16],
        scripting_enabled: bool,
    ) -> Handle {
        let mut ancestor = Some(context);
        let mut has_form_ancestor = false;
        while let Some(node) = ancestor {
            if self.namespace(node) == Namespace::Html && self.local_name(node) == Some("form") {
                has_form_ancestor = true;
                break;
            }
            ancestor = self.parent(node);
        }
        let dom = parsed_with_form(
            units,
            Some(self.fragment_context(context, true)),
            has_form_ancestor,
            scripting_enabled,
            if self.compat_mode == "BackCompat" {
                QuirksMode::Quirks
            } else {
                QuirksMode::NoQuirks
            },
        );
        let document = core::walk(&dom.document);
        let parsed_root = document.children.first().unwrap_or(&document);
        let fragment = self.create_document_fragment();
        let recording = self.mutation_log.take();
        let children = parsed_root
            .children
            .iter()
            .map(|child| self.import_units_node(child))
            .collect::<Vec<_>>();
        for &child in &children {
            self.parent_ov.insert(child, Some(fragment));
        }
        self.children_ov.insert(fragment, children.into());
        self.mutation_log = recording;
        self.bump();
        core::discard_node(document);
        fragment
    }

    /// Parse a DOM string without repairing its original code units. UTF-8
    /// inputs retain the existing lazy buffer path.
    #[must_use]
    pub fn parse_utf16(units: &[u16]) -> Self {
        if let Some(source) = String::from_utf16(units)
            .ok()
            .filter(|_| !units.contains(&0xfeff))
        {
            return Self::parse(&source);
        }
        let dom = parsed(units, None);
        Self::from_units_dom(&dom)
    }
    /// Parse an iframe srcdoc document: identical to [`Tree::parse_utf16`] except
    /// that the document is never in quirks mode.
    #[must_use]
    pub fn parse_srcdoc_utf16(units: &[u16]) -> Self {
        Self::from_units_dom(&parsed_srcdoc(units))
    }
    pub(super) fn from_units_dom(dom: &RcDom) -> Self {
        let node = core::walk(&dom.document);
        let mut tree = Self::parse("");
        tree.compat_mode = if dom.quirks_mode.get() == QuirksMode::Quirks {
            "BackCompat"
        } else {
            "CSS1Compat"
        };
        let root = tree.root();
        let children = node
            .children
            .iter()
            .map(|child| tree.import_units_node(child))
            .collect::<Vec<_>>();
        tree.replace_children(root, &children);
        core::discard_node(node);
        tree
    }
    fn import_units_node(&mut self, node: &core::Node) -> Handle {
        let root = self.import_units_shallow(node);
        let mut pending = vec![(node, root)];
        while let Some((source, target)) = pending.pop() {
            let mut children = super::ChildList::new();
            for child in &source.children {
                let handle = self.import_units_shallow(child);
                self.parent_ov.insert(handle, Some(target));
                children.push(handle);
                pending.push((child, handle));
            }
            if !children.is_empty() {
                self.children_ov.insert(target, children);
            }
        }
        root
    }
    fn import_units_shallow(&mut self, node: &core::Node) -> Handle {
        let target = match NodeType::from_u8(node.node_type) {
            NodeType::Element => {
                let target = self
                    .create_element_ns_units(&decode(&node.name), super::ns_id(&node.namespace));
                for attribute in &node.attrs {
                    let namespace = if self.namespace(target) == Namespace::Html {
                        None
                    } else {
                        super::parsed_attribute_namespace(&attribute.prefix, &attribute.name)
                    };
                    let qualified = if attribute.prefix.is_empty() {
                        attribute.name.clone()
                    } else {
                        format!("{}:{}", attribute.prefix, attribute.name)
                    };
                    self.set_attribute_ns_units(
                        target,
                        namespace
                            .map(|uri| uri.encode_utf16().collect::<Vec<_>>())
                            .as_deref(),
                        &decode(&qualified),
                        &decode(&attribute.value),
                    );
                }
                target
            }
            NodeType::Doctype => self.create_document_type_units(
                &decode(&node.name),
                &decode(&node.public_id),
                &decode(&node.system_id),
            ),
            NodeType::Comment => self.create_comment_utf16(&decode(&node.value)),
            NodeType::ProcessingInstruction => {
                self.create_processing_instruction_units(&decode(&node.name), &decode(&node.value))
            }
            NodeType::Fragment => self.create_document_fragment(),
            _ => self.create_text_node_utf16(&decode(&node.value)),
        };
        target
    }
    pub fn set_inner_html_utf16(&mut self, node: Handle, units: &[u16]) {
        if let Some(source) = String::from_utf16(units)
            .ok()
            .filter(|_| !units.contains(&0xfeff))
        {
            self.set_inner_html(node, &source);
            return;
        }
        let dom = parsed(units, Some(self.fragment_context(node, true)));
        let document = core::walk(&dom.document);
        // html5ever fragment parsing creates an artificial HTML root.
        let fragment = document.children.first().unwrap_or(&document);
        let children = fragment
            .children
            .iter()
            .map(|child| self.import_units_node(child))
            .collect::<Vec<_>>();
        let removed = if self.is_recording() {
            self.children(node)
        } else {
            Vec::new()
        };
        self.orphan_children(node);
        for &child in &children {
            self.parent_ov.insert(child, Some(node));
        }
        self.children_ov.insert(node, children.clone().into());
        self.bump();
        if self.is_recording() && (!children.is_empty() || !removed.is_empty()) {
            self.record_child_list_batch(node, children, removed);
        }
        core::discard_node(document);
    }
    pub fn insert_adjacent_html_utf16(&mut self, node: Handle, position: &str, units: &[u16]) {
        if let Some(source) = String::from_utf16(units)
            .ok()
            .filter(|_| !units.contains(&0xfeff))
        {
            self.insert_adjacent_html(node, position, &source);
            return;
        }
        let position = position.to_ascii_lowercase();
        let context = if matches!(position.as_str(), "beforebegin" | "afterend") {
            self.parent(node).unwrap_or(node)
        } else {
            node
        };
        let dom = parsed(units, Some(self.fragment_context(context, true)));
        let document = core::walk(&dom.document);
        let fragment = document.children.first().unwrap_or(&document);
        let children = fragment
            .children
            .iter()
            .map(|child| self.import_units_node(child))
            .collect::<Vec<_>>();
        match position.as_str() {
            "beforebegin" => self.before(node, &children),
            "afterend" => self.after(node, &children),
            "afterbegin" => {
                let first = self.first_child(node);
                for child in children {
                    self.insert_before(node, child, first);
                }
            }
            _ => {
                for child in children {
                    self.append_child(node, child);
                }
            }
        }
        core::discard_node(document);
    }
    pub fn create_processing_instruction_units(&mut self, target: &[u16], data: &[u16]) -> Handle {
        let node = self.push_node(NewNode::ProcessingInstruction(Box::new(
            ProcessingInstructionData {
                target: DomString::from_units(target),
                data: DomString::from_units(data),
            },
        )));
        self.bump();
        node
    }
    pub fn create_document_type_units(
        &mut self,
        name: &[u16],
        public: &[u16],
        system: &[u16],
    ) -> Handle {
        let node = self.push_node(NewNode::Doctype(Box::new(DoctypeData {
            name: DomString::from_units(name),
            public_id: DomString::from_units(public),
            system_id: DomString::from_units(system),
        })));
        self.bump();
        node
    }
    #[must_use]
    pub fn processing_instruction_target_utf16(&self, node: Handle) -> Option<Vec<u16>> {
        if self.node_type(node) != NodeType::ProcessingInstruction {
            return None;
        }
        if self.is_new(node) {
            if let NewNode::ProcessingInstruction(data) = self.new_ref(node) {
                return Some(data.target.units());
            }
        }
        self.processing_instruction_target(node)
            .map(|target| target.encode_utf16().collect())
    }
    #[must_use]
    pub fn doctype_name_utf16(&self, node: Handle) -> Option<Vec<u16>> {
        if self.node_type(node) != NodeType::Doctype {
            return None;
        }
        if self.is_new(node) {
            if let NewNode::Doctype(data) = self.new_ref(node) {
                return Some(data.name.units());
            }
        }
        self.doctype_name(node)
            .map(|name| name.encode_utf16().collect())
    }
    #[must_use]
    pub fn doctype_public_id_utf16(&self, node: Handle) -> Option<Vec<u16>> {
        if self.node_type(node) != NodeType::Doctype {
            return None;
        }
        if self.is_new(node) {
            if let NewNode::Doctype(data) = self.new_ref(node) {
                return Some(data.public_id.units());
            }
        }
        self.doctype_public_id(node)
            .map(|name| name.encode_utf16().collect())
    }
    #[must_use]
    pub fn doctype_system_id_utf16(&self, node: Handle) -> Option<Vec<u16>> {
        if self.node_type(node) != NodeType::Doctype {
            return None;
        }
        if self.is_new(node) {
            if let NewNode::Doctype(data) = self.new_ref(node) {
                return Some(data.system_id.units());
            }
        }
        self.doctype_system_id(node)
            .map(|name| name.encode_utf16().collect())
    }
}

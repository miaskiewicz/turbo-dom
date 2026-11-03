//! XML documents (turbo-dom#186): the XML parser behind an embedder's `DOMParser` XML
//! types (`parse_xml_document_units`) and the `XMLSerializer` interface.
//!
//! The XML is read by the xml-rs (`xml`) crate; its well-formedness error messages and
//! positions match Chrome 156's control (`xml_documents_chrome156.json`). Chrome's
//! document shape is reproduced on top: a character run becomes its Text node when the run starts and receives its data
//! when the next event arrives (an error in between leaves the node empty); an element's
//! namespace declarations come first, ordered by prefix; a well-formedness error inserts
//! Chrome's `<parsererror>` as the root element's first child, or as
//! `<html><body><parsererror>` when there is no root element yet.
use super::*;
use turbo_dom_parser::rtdom::tree::NodeType;
use xml::common::Position;
use xml::name::OwnedName;
use xml::namespace::Namespace as XmlNamespace;
use xml::reader::{DoctypeRef, ParserConfig, XmlEvent};

const XHTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
const PARSERERROR_STYLE: &str = "display: block; white-space: pre; border: 2px solid #c77; padding: 0 1em 0 1em; margin: 1em; background-color: #fdd; color: black";

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

/// A parsed XML document: its tree and, when the source had one, the XML declaration
/// `XMLSerializer` writes back (Chrome keeps the encoding and standalone pseudo-attributes
/// and always writes version 1.0).
pub(crate) struct ParsedXml {
    pub(crate) tree: Tree,
    pub(crate) declaration: Option<String>,
}

/// The namespace a fragment parse binds an unbound prefix to; Chrome's fragment parser
/// accepts `<z:a/>` as a no-namespace element whose local name is `z:a`.
const UNBOUND_NAMESPACE: &str = "urn:turbo-dom:unbound-prefix";

struct Builder<'a, 't> {
    tree: &'t mut Tree,
    tags: StartTags<'a>,
    open: Vec<Handle>,
    pending: Option<(Handle, String)>,
}

impl Builder<'_, '_> {
    fn parent(&self) -> Handle {
        // invariant: the document stays at the bottom of the open-element stack (xml-rs
        // never ends more elements than it started)
        *self.open.last().unwrap()
    }
    fn append(&mut self, node: Handle) {
        let parent = self.parent();
        self.tree.append_child(parent, node);
    }
    /// The pending character run receives its data now that another event arrived.
    fn flush(&mut self) {
        if let Some((node, data)) = self.pending.take() {
            self.tree.set_text_content_utf16(node, &units(&data));
        }
    }
    fn text(&mut self, data: String) {
        let node = self.tree.create_text_node("");
        self.append(node);
        self.pending = Some((node, data));
    }
    fn doctype(&mut self, ids: Option<DoctypeRef<'_>>) {
        // invariant: xml-rs records the DOCTYPE ids before it emits the Doctype event
        let ids = ids.unwrap();
        let node = self.tree.create_document_type(
            ids.name(),
            ids.public_id().unwrap_or_default(),
            ids.system_id().unwrap_or_default(),
        );
        self.append(node);
    }
    fn start(
        &mut self,
        name: &OwnedName,
        attributes: Vec<xml::attribute::OwnedAttribute>,
        namespace: &XmlNamespace,
    ) {
        let element = if name.namespace.as_deref() == Some(UNBOUND_NAMESPACE) {
            self.tree.create_element_ns_units(
                &units(&qualified(name)),
                turbo_dom_parser::rtdom::tree::Namespace::Other,
            )
        } else {
            self.tree.create_element_namespace_units(
                name.namespace.as_deref().map(units).as_deref(),
                &units(&qualified(name)),
            )
        };
        let xmlns = units(XMLNS_NAMESPACE);
        let mut declared = self.tags.next_declarations(&qualified(name));
        declared.sort_unstable();
        for prefix in declared {
            let qualified = if prefix.is_empty() {
                "xmlns".to_owned()
            } else {
                format!("xmlns:{prefix}")
            };
            let value = namespace.get(prefix).unwrap_or_default();
            self.tree.set_attribute_ns_units(
                element,
                Some(&xmlns),
                &units(&qualified),
                &units(value),
            );
        }
        for attribute in attributes {
            if attribute.name.namespace.as_deref() == Some(UNBOUND_NAMESPACE) {
                let name = units(&qualified(&attribute.name));
                self.tree
                    .set_attribute_units(element, &name, &units(&attribute.value));
                continue;
            }
            self.tree.set_attribute_ns_units(
                element,
                attribute.name.namespace.as_deref().map(units).as_deref(),
                &units(&qualified(&attribute.name)),
                &units(&attribute.value),
            );
        }
        self.append(element);
        self.open.push(element);
    }
    fn event<R: std::io::Read>(&mut self, event: XmlEvent, reader: &xml::EventReader<R>) {
        match event {
            XmlEvent::Doctype { .. } => self.doctype(reader.doctype_ids()),
            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => self.start(&name, attributes, &namespace),
            XmlEvent::EndElement { .. } => {
                self.open.pop();
            }
            XmlEvent::CData(data) => {
                let node = self.tree.create_cdata_section_utf16(&units(&data));
                self.append(node);
            }
            XmlEvent::Comment(data) => {
                let node = self.tree.create_comment_utf16(&units(&data));
                self.append(node);
            }
            XmlEvent::ProcessingInstruction { name, data } => {
                let data = data.unwrap_or_default();
                let node = self
                    .tree
                    .create_processing_instruction_units(&units(&name), &units(data.trim_start()));
                self.append(node);
            }
            // whitespace_to_characters folds Whitespace into the character run
            XmlEvent::Characters(data) | XmlEvent::Whitespace(data) => self.text(data),
            // StartDocument carries nothing the tree keeps (the declaration is read from
            // the source); EndDocument stops the loop before it gets here.
            XmlEvent::StartDocument { .. } | XmlEvent::EndDocument => {}
        }
    }
}

fn qualified(name: &OwnedName) -> String {
    match &name.prefix {
        Some(prefix) => format!("{prefix}:{}", name.local_name),
        None => name.local_name.clone(),
    }
}

/// The source's start tags in order, for the namespace declarations xml-rs folds into
/// its scope map (Chrome keeps each element's own `xmlns` attributes). The source up to
/// a start-element event is well-formed, so a light scan finds the tags. An internal
/// entity's reference is followed into its replacement text, which xml-rs reparses, so
/// the elements an entity spells take their own tags (and declarations) in order.
struct StartTags<'a> {
    rest: &'a str,
    /// The text after each entity reference being expanded, innermost last.
    outer: Vec<&'a str>,
    /// The internal DTD subset's general entities with a literal replacement text.
    entities: std::collections::HashMap<&'a str, &'a str>,
}

/// Entity references followed at once: xml-rs stops a reparse deeper than 10 (as Chrome
/// 156 does, "Entity too big"), so the scan never needs more.
const ENTITY_SCAN_DEPTH: usize = 64;

impl<'a> StartTags<'a> {
    fn new(source: &'a str) -> Self {
        StartTags {
            rest: source,
            outer: Vec::new(),
            entities: std::collections::HashMap::new(),
        }
    }
    /// Skip past `end`, or to the end of the current text.
    fn skip(&mut self, end: &str) {
        self.rest = self
            .rest
            .find(end)
            .map_or("", |index| &self.rest[index + end.len()..]);
    }
    /// Skip past the first of `stops` outside a quoted literal and return it (None at
    /// the end of the current text).
    fn skip_quoted(&mut self, stops: &[char]) -> Option<char> {
        let mut quote = None;
        for (index, c) in self.rest.char_indices() {
            match quote {
                Some(open) if c == open => quote = None,
                Some(_) => {}
                None if c == '"' || c == '\'' => quote = Some(c),
                None if stops.contains(&c) => {
                    self.rest = &self.rest[index + 1..];
                    return Some(c);
                }
                None => {}
            }
        }
        self.rest = "";
        None
    }
    /// The next start tag's name and the prefixes it declares (`""` for `xmlns`).
    fn next_tag(&mut self) -> Option<(&'a str, Vec<&'a str>)> {
        loop {
            let Some(index) = self.rest.find(['<', '&']) else {
                self.rest = self.outer.pop()?;
                continue;
            };
            let reference = self.rest.as_bytes()[index] == b'&';
            self.rest = &self.rest[index + 1..];
            let markup = self.rest;
            if reference {
                self.reference();
            } else if markup.starts_with("!--") {
                self.skip("-->");
            } else if markup.starts_with("![CDATA[") {
                self.skip("]]>");
            } else if markup.starts_with('?') {
                self.skip("?>");
            } else if markup.starts_with("!DOCTYPE") {
                self.doctype();
            } else if markup.starts_with('/') {
                self.skip(">");
            } else {
                return Some(self.attributes());
            }
        }
    }
    /// After `&`: continue in a declared entity's replacement text and then after the
    /// reference (character and predefined references spell no markup).
    fn reference(&mut self) {
        let Some(end) = self.rest.find(';') else {
            return;
        };
        let after = &self.rest[end + 1..];
        match self.entities.get(&self.rest[..end]) {
            Some(&text) if self.outer.len() < ENTITY_SCAN_DEPTH => {
                self.outer.push(after);
                self.rest = text;
            }
            _ => self.rest = after,
        }
    }
    /// After `<`: skip `!DOCTYPE …>`, recording the internal subset's entities. A `]` or
    /// `>` in a quoted literal, a comment or a processing instruction does not end it.
    fn doctype(&mut self) {
        if self.skip_quoted(&['[', '>']) != Some('[') {
            return;
        }
        loop {
            self.rest = self.rest.trim_start();
            let markup = self.rest;
            if markup.is_empty() {
                return;
            }
            if markup.starts_with(']') {
                self.skip(">");
                return;
            }
            if markup.starts_with("<!--") {
                self.skip("-->");
            } else if markup.starts_with("<?") {
                self.skip("?>");
            } else if let Some(declaration) = markup.strip_prefix("<!ENTITY") {
                self.rest = declaration;
                self.entity();
            } else if markup.starts_with('%') {
                self.skip(";");
            } else {
                self.skip_quoted(&['>']);
            }
        }
    }
    /// After `<!ENTITY`: record a general entity's literal replacement text (the first
    /// declaration of a name binds it), then skip the rest of the declaration.
    fn entity(&mut self) {
        let declaration = self.rest.trim_start();
        let (name, value) =
            declaration.split_at(declaration.find(char::is_whitespace).unwrap_or(0));
        let value = value.trim_start();
        let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''));
        if let (Some(quote), false) = (quote, name == "%") {
            if let Some(close) = value[1..].find(quote) {
                self.entities.entry(name).or_insert(&value[1..=close]);
            }
        }
        self.rest = declaration;
        self.skip_quoted(&['>']);
    }
    fn attributes(&mut self) -> (&'a str, Vec<&'a str>) {
        let end = self
            .rest
            .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .unwrap_or(self.rest.len());
        let name = &self.rest[..end];
        let mut prefixes = Vec::new();
        let mut rest = &self.rest[end..];
        loop {
            rest = rest.trim_start();
            let (Some(equals), false) = (rest.find('='), rest.starts_with(['/', '>'])) else {
                self.rest = rest;
                return (name, prefixes);
            };
            let attribute = rest[..equals].trim_end();
            let value = rest[equals + 1..].trim_start();
            let quote = value.chars().next().unwrap_or('"');
            let close = value[1..]
                .find(quote)
                .map_or(value.len(), |index| index + 2);
            if attribute == "xmlns" {
                prefixes.push("");
            } else if let Some(prefix) = attribute.strip_prefix("xmlns:") {
                prefixes.push(prefix);
            }
            rest = &value[close..];
        }
    }
    /// The prefixes the next start tag declares, when it is the element xml-rs reported
    /// (an entity's replacement text can hold elements the source does not spell).
    fn next_declarations(&mut self, qualified: &str) -> Vec<&'a str> {
        self.next_tag()
            .filter(|(name, _)| *name == qualified)
            .map(|(_, prefixes)| prefixes)
            .unwrap_or_default()
    }
}

/// `<?xml version="1.0"[ encoding="…"][ standalone="yes|no"]?>` when the source starts
/// with an XML declaration (Chrome writes version 1.0 whatever was declared).
fn declaration(source: &str, standalone: Option<bool>) -> Option<String> {
    let rest = source.strip_prefix("<?xml")?;
    if !rest.starts_with([' ', '\t', '\n', '\r']) {
        return None;
    }
    let head = &rest[..rest.find("?>")?];
    let mut text = String::from("<?xml version=\"1.0\"");
    if let Some(encoding) = pseudo_attribute(head, "encoding") {
        text.push_str(&format!(" encoding=\"{encoding}\""));
    }
    match standalone {
        Some(true) => text.push_str(" standalone=\"yes\""),
        Some(false) => text.push_str(" standalone=\"no\""),
        None => {}
    }
    text.push_str("?>");
    Some(text)
}

fn pseudo_attribute<'s>(head: &'s str, name: &str) -> Option<&'s str> {
    let after = head[head.find(name)? + name.len()..]
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let quote = after.chars().next()?;
    let value = &after[quote.len_utf8()..];
    Some(&value[..value.find(quote)?])
}

fn xhtml_element(tree: &mut Tree, name: &str) -> Handle {
    tree.create_element_namespace_units(Some(&units(XHTML_NAMESPACE)), &units(name))
}

fn styled_text(tree: &mut Tree, name: &str, style: Option<&str>, text: &str) -> Handle {
    let element = xhtml_element(tree, name);
    if let Some(style) = style {
        tree.set_attribute_ns_units(element, None, &units("style"), &units(style));
    }
    let text = tree.create_text_node_utf16(&units(text));
    tree.append_child(element, text);
    element
}

/// Chrome's `<parsererror>` for the first well-formedness error.
fn report(tree: &mut Tree, error: &xml::reader::Error) {
    let position = error.position();
    // Display is "row:column message"
    let text = error.to_string();
    let message = text.split_once(' ').map_or("", |(_, message)| message);
    let parsererror = xhtml_element(tree, "parsererror");
    tree.set_attribute_ns_units(
        parsererror,
        None,
        &units("style"),
        &units(PARSERERROR_STYLE),
    );
    let line = format!(
        "error on line {} at column {}: {message}",
        position.row + 1,
        position.column + 1
    );
    for (name, style, text) in [
        ("h3", None, "This page contains the following errors:"),
        (
            "div",
            Some("font-family:monospace;font-size:12px"),
            line.as_str(),
        ),
        (
            "h3",
            None,
            "Below is a rendering of the page up to the first error.",
        ),
    ] {
        let child = styled_text(tree, name, style, text);
        tree.append_child(parsererror, child);
    }
    let root = tree.root();
    if let Some(element) = tree
        .children(root)
        .into_iter()
        .find(|&child| tree.node_type_id(child) == 1)
    {
        let first = tree.children(element).first().copied();
        tree.insert_before(element, parsererror, first);
    } else {
        let html = xhtml_element(tree, "html");
        let body = xhtml_element(tree, "body");
        tree.append_child(body, parsererror);
        tree.append_child(html, body);
        tree.append_child(root, html);
    }
}

fn reader_config() -> ParserConfig {
    ParserConfig::new()
        .ignore_comments(false)
        .whitespace_to_characters(true)
        .cdata_to_characters(false)
        .coalesce_characters(true)
        .ignore_end_of_stream(true)
        .allow_multiple_root_elements(false)
        .override_encoding(Some(xml::Encoding::Utf8))
        .ignore_invalid_encoding_declarations(true)
}

/// Build `source`'s nodes under `parent`, recording the XML declaration to keep (also
/// when a later well-formedness error stops the parse).
fn build(
    tree: &mut Tree,
    parent: Handle,
    source: &str,
    parsed_declaration: &mut Option<String>,
) -> Result<(), xml::reader::Error> {
    let mut reader = reader_config().create_reader(source.as_bytes());
    let mut builder = Builder {
        tree,
        tags: StartTags::new(source),
        open: vec![parent],
        pending: None,
    };
    loop {
        let event = reader.next()?;
        builder.flush();
        match event {
            XmlEvent::EndDocument => return Ok(()),
            XmlEvent::StartDocument { standalone, .. } => {
                *parsed_declaration = declaration(source, standalone);
            }
            event => builder.event(event, &reader),
        }
    }
}

/// Parse an XML document into a fresh tree (no browsing context).
pub(crate) fn parse(source: &str) -> ParsedXml {
    // A no-quirks document (XML documents report CSS1Compat) without children.
    let mut tree = Tree::parse("<!doctype html>");
    let root = tree.root();
    for child in tree.children(root) {
        tree.remove_child(root, child);
    }
    let mut declaration = None;
    if let Err(error) = build(&mut tree, root, source, &mut declaration) {
        report(&mut tree, &error);
    }
    ParsedXml { tree, declaration }
}

/// The prefix bindings in scope at `element` (its ancestors' and its own `xmlns`
/// attributes, and each element's own prefix), outermost first.
fn scope_at(tree: &Tree, element: Handle) -> Vec<(String, String)> {
    let mut chain = vec![element];
    while let Some(parent) = tree.parent(*chain.last().unwrap()) {
        chain.push(parent);
    }
    let mut bindings = Vec::new();
    for node in chain
        .into_iter()
        .rev()
        .filter(|&node| tree.node_type_id(node) == 1)
    {
        if let Some(namespace) = tree.namespace_uri(node) {
            let prefix = tree.prefix(node).unwrap_or_default();
            bindings.push((prefix.to_owned(), namespace.to_owned()));
        }
        for attribute in tree.attributes_units(node) {
            if attribute.namespace.as_deref() == Some(units(XMLNS_NAMESPACE).as_slice()) {
                let prefix = attribute
                    .prefix
                    .is_some()
                    .then(|| String::from_utf16_lossy(&attribute.local_name))
                    .unwrap_or_default();
                bindings.push((prefix, String::from_utf16_lossy(&attribute.value)));
            }
        }
    }
    bindings
}

/// The prefix an "Element p:x prefix is unbound" / "Attribute p:x prefix is unbound"
/// error names.
fn unbound_prefix(error: &xml::reader::Error) -> Option<String> {
    let text = error.to_string();
    let name = text
        .strip_suffix(" prefix is unbound")?
        .rsplit(' ')
        .next()?;
    Some(name.split(':').next()?.to_owned())
}

/// Parse `markup` as an XML fragment in `context`'s namespace scope (the innerHTML /
/// outerHTML setters of an XML document). None when the markup is not well-formed.
pub(crate) fn parse_fragment(
    tree: &mut Tree,
    context: Handle,
    markup: &str,
) -> Option<Vec<Handle>> {
    let mut bindings = scope_at(tree, context);
    let mut unbound = Vec::new();
    loop {
        let mut wrapper = String::from("<turbo-fragment");
        for (prefix, namespace) in bindings
            .iter()
            .map(|(p, n)| (p, n.as_str()))
            .chain(unbound.iter().map(|prefix| (prefix, UNBOUND_NAMESPACE)))
        {
            let mut value = String::new();
            escape(&mut value, namespace, true);
            if prefix.is_empty() {
                wrapper.push_str(&format!(" xmlns=\"{value}\""));
            } else if prefix != "xml" {
                wrapper.push_str(&format!(" xmlns:{prefix}=\"{value}\""));
            }
        }
        wrapper.push('>');
        wrapper.push_str(markup);
        wrapper.push_str("</turbo-fragment>");
        let fragment = tree.create_document_fragment();
        match build(tree, fragment, &wrapper, &mut None) {
            Ok(_) => {
                // invariant: a well-formed wrapper document has exactly its one element
                let element = tree.children(fragment)[0];
                return Some(tree.children(element));
            }
            Err(error) => {
                let prefix = unbound_prefix(&error).filter(|prefix| !unbound.contains(prefix))?;
                unbound.push(prefix);
                bindings.retain(|(bound, _)| !unbound.contains(bound));
            }
        }
    }
}

fn record_key<'s>(scope: &mut v8::PinScope<'s, '_>, name: &str) -> v8::Local<'s, v8::String> {
    // invariant: engine-built name, far below V8's string length limit
    v8::String::new(scope, name).unwrap()
}

fn document_record<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    doc: v8::Local<'s, v8::Object>,
) -> v8::Local<'s, v8::Object> {
    if let Some(record) = collection_records::get(scope, doc)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    {
        return record;
    }
    let record = v8::Object::new(scope);
    let null = v8::null(scope);
    record.set_prototype(scope, null.into());
    collection_records::set(scope, doc, record.into());
    record
}

fn prototype_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Private> {
    let name = record_key(scope, "turbo.native.xml-document.prototype");
    v8::Private::for_api(scope, Some(name))
}

/// Parse `units` as an XML document of `content_type` (`application/xml`, `text/xml`,
/// `image/svg+xml` or `application/xhtml+xml`) into an inert `XMLDocument`, as Chrome
/// 156's `DOMParser.parseFromString` does: a well-formedness error yields Chrome's
/// `<parsererror>` document instead of throwing. Like [`parse_html_document_units`] the
/// embedder's inert-document initializer is not called; the embedder registers the
/// document itself.
pub fn parse_xml_document_units<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    units: &[u16],
    content_type: &str,
) -> v8::Local<'s, v8::Object> {
    let parsed = parse(&String::from_utf16_lossy(units));
    let doc = parse_html_tree_with(scope, parsed.tree, false, None);
    let record = document_record(scope, doc);
    let content = record_key(scope, "nativeContentType");
    let content_value = record_key(scope, content_type);
    record.create_data_property(scope, content.into(), content_value.into());
    if let Some(declaration) = parsed.declaration {
        let key = record_key(scope, "nativeXmlDeclaration");
        let value = record_key(scope, &declaration);
        record.create_data_property(scope, key.into(), value.into());
    }
    let global = scope.get_current_context().global(scope);
    let key = prototype_key(scope);
    if let Some(prototype) = global.get_private(scope, key) {
        doc.set_prototype(scope, prototype);
    }
    doc
}

/// The XML declaration a parsed document keeps for `XMLSerializer`.
fn stored_declaration<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    doc: v8::Local<'s, v8::Object>,
) -> Option<String> {
    let record = collection_records::get(scope, doc)?.to_object(scope)?;
    let key = record_key(scope, "nativeXmlDeclaration");
    if record.has_own_property(scope, key.into()) != Some(true) {
        return None;
    }
    Some(record.get(scope, key.into())?.to_rust_string_lossy(scope))
}

// ---- XMLSerializer ------------------------------------------------------------------------

/// The in-scope prefix bindings (`""` is the default namespace).
#[derive(Clone)]
struct Scope(Vec<(String, String)>);

impl Scope {
    fn lookup(&self, prefix: &str) -> Option<&str> {
        self.0
            .iter()
            .rev()
            .find(|(bound, _)| bound == prefix)
            .map(|(_, namespace)| namespace.as_str())
    }
    fn bind(&mut self, prefix: &str, namespace: &str) {
        self.0.push((prefix.to_owned(), namespace.to_owned()));
    }
    /// A non-default prefix currently bound to `namespace`.
    fn prefix_for(&self, namespace: &str) -> Option<String> {
        self.0
            .iter()
            .rev()
            .find(|(prefix, bound)| {
                !prefix.is_empty() && bound == namespace && self.lookup(prefix) == Some(namespace)
            })
            .map(|(prefix, _)| prefix.clone())
    }
}

enum Task {
    Node(Handle, Scope),
    Close(String),
}

struct Serializer<'t> {
    tree: &'t Tree,
    out: String,
    generated: usize,
    pending: Vec<Task>,
}

fn escape(out: &mut String, value: &str, attribute: bool) {
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            '\t' if attribute => out.push_str("&#9;"),
            '\n' if attribute => out.push_str("&#10;"),
            '\r' if attribute => out.push_str("&#13;"),
            _ => out.push(character),
        }
    }
}

impl Serializer<'_> {
    fn declare(&mut self, scope: &mut Scope, prefix: &str, namespace: &str) {
        if prefix.is_empty() {
            self.out.push_str(" xmlns=\"");
        } else {
            self.out.push_str(" xmlns:");
            self.out.push_str(prefix);
            self.out.push_str("=\"");
        }
        escape(&mut self.out, namespace, true);
        self.out.push('"');
        scope.bind(prefix, namespace);
    }
    fn children(&mut self, node: Handle, scope: &Scope) {
        for child in self.tree.children(node).into_iter().rev() {
            self.pending.push(Task::Node(child, scope.clone()));
        }
    }
    /// An attribute's serialized name, declaring (or generating) its prefix when needed.
    fn attribute_name(&mut self, scope: &mut Scope, attribute: &AttributeDataUtf16) -> String {
        let local = String::from_utf16_lossy(&attribute.local_name);
        let Some(namespace) = attribute.namespace.as_deref().map(String::from_utf16_lossy) else {
            return local;
        };
        if namespace == XMLNS_NAMESPACE {
            return String::from_utf16_lossy(&attribute.qualified_name);
        }
        if namespace == XML_NAMESPACE {
            return format!("xml:{local}");
        }
        let prefix = attribute
            .prefix
            .as_deref()
            .map(String::from_utf16_lossy)
            .filter(|prefix| scope.lookup(prefix).is_none_or(|bound| bound == namespace))
            .or_else(|| scope.prefix_for(&namespace))
            .unwrap_or_else(|| {
                self.generated += 1;
                format!("ns{}", self.generated)
            });
        if scope.lookup(&prefix) != Some(namespace.as_str()) {
            self.declare(scope, &prefix, &namespace);
        }
        format!("{prefix}:{local}")
    }
    fn element(&mut self, node: Handle, mut scope: Scope) {
        let tree = self.tree;
        let namespace = tree.namespace_uri(node).unwrap_or_default();
        let prefix = tree.prefix(node).unwrap_or_default();
        // invariant: every element has a qualified name
        let name = tree.qualified_name(node).unwrap().to_owned();
        let attributes = tree.attributes_units(node);
        // The element's own declarations are in scope for its name and attributes.
        for attribute in &attributes {
            if attribute.namespace.as_deref() == Some(units(XMLNS_NAMESPACE).as_slice()) {
                let declared = if attribute.prefix.is_some() {
                    String::from_utf16_lossy(&attribute.local_name)
                } else {
                    String::new()
                };
                scope.bind(&declared, &String::from_utf16_lossy(&attribute.value));
            }
        }
        self.out.push('<');
        self.out.push_str(&name);
        if scope.lookup(prefix) != Some(namespace) {
            self.declare(&mut scope, prefix, namespace);
        }
        for attribute in &attributes {
            let attribute_name = self.attribute_name(&mut scope, attribute);
            self.out.push(' ');
            self.out.push_str(&attribute_name);
            self.out.push_str("=\"");
            escape(
                &mut self.out,
                &String::from_utf16_lossy(&attribute.value),
                true,
            );
            self.out.push('"');
        }
        let empty = tree.children(node).is_empty();
        let html = namespace == XHTML_NAMESPACE;
        if empty && !html {
            self.out.push_str("/>");
        } else if empty && turbo_dom_parser::rtdom::serialize::is_void(&name) {
            self.out.push_str(" />");
        } else {
            self.out.push('>');
            self.pending.push(Task::Close(name));
            self.children(node, &scope);
        }
    }
    fn doctype(&mut self, node: Handle) {
        let tree = self.tree;
        let public = tree.doctype_public_id(node).unwrap_or_default();
        let system = tree.doctype_system_id(node).unwrap_or_default();
        self.out.push_str("<!DOCTYPE ");
        self.out
            .push_str(tree.doctype_name(node).unwrap_or_default());
        if !public.is_empty() {
            self.out.push_str(" PUBLIC \"");
            self.out.push_str(public);
            self.out.push('"');
        } else if !system.is_empty() {
            self.out.push_str(" SYSTEM");
        }
        if !system.is_empty() {
            self.out.push_str(" \"");
            self.out.push_str(system);
            self.out.push('"');
        }
        self.out.push('>');
    }
    fn node(&mut self, node: Handle, scope: Scope) {
        let tree = self.tree;
        let data = tree.node_value(node).unwrap_or_default();
        match tree.node_type(node) {
            NodeType::Element => self.element(node, scope),
            NodeType::Document | NodeType::Fragment => self.children(node, &scope),
            NodeType::Text => escape(&mut self.out, &data, false),
            NodeType::CDataSection => {
                self.out.push_str("<![CDATA[");
                self.out.push_str(&data);
                self.out.push_str("]]>");
            }
            NodeType::Comment => {
                self.out.push_str("<!--");
                self.out.push_str(&data);
                self.out.push_str("-->");
            }
            NodeType::ProcessingInstruction => {
                self.out.push_str("<?");
                self.out
                    .push_str(tree.processing_instruction_target(node).unwrap_or_default());
                self.out.push(' ');
                self.out.push_str(&data);
                self.out.push_str("?>");
            }
            NodeType::Doctype => self.doctype(node),
            // Chrome serializes an Attr node as nothing.
            NodeType::Attribute => {}
        }
    }
}

/// Chrome's XML serialization of `node` (`XMLSerializer.serializeToString`), with a
/// parsed document's XML declaration first.
pub(crate) fn serialize(tree: &Tree, node: Handle, declaration: Option<&str>) -> String {
    let root = Scope(vec![
        ("xml".to_owned(), XML_NAMESPACE.to_owned()),
        (String::new(), String::new()),
    ]);
    let mut serializer = Serializer {
        tree,
        out: declaration.unwrap_or_default().to_owned(),
        generated: 0,
        pending: vec![Task::Node(node, root)],
    };
    while let Some(task) = serializer.pending.pop() {
        match task {
            Task::Node(node, scope) => serializer.node(node, scope),
            Task::Close(name) => {
                serializer.out.push_str("</");
                serializer.out.push_str(&name);
                serializer.out.push('>');
            }
        }
    }
    serializer.out
}

fn invoke<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
) -> Option<v8::Local<'s, v8::Value>> {
    let operation = args.get(0).to_rust_string_lossy(scope);
    let target = v8::Local::<v8::Object>::try_from(args.get(1)).ok()?;
    if operation == "prototype" {
        let global = scope.get_current_context().global(scope);
        let key = prototype_key(scope);
        global.set_private(scope, key, target.into());
        return Some(v8::undefined(scope).into());
    }
    let _dom = DomScope::for_object(scope, target);
    let handle = handle_of(scope, target)?;
    let declaration = if with_tree(|tree| tree.node_type_id(handle)) == Some(9) {
        stored_declaration(scope, target)
    } else {
        None
    };
    let text = with_tree(|tree| serialize(tree, handle, declaration.as_deref()))?;
    Some(unit_string(scope, &units(&text)).into())
}

fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    if let Some(value) = invoke(scope, &args) {
        rv.set(value);
    }
}

pub(crate) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__xmlDocumentsNative", dispatch);
    run_js(scope, include_str!("xml_documents.js"));
}

// ---- innerHTML / outerHTML in XML documents -------------------------------------------------

fn current_document_is_xml(scope: &mut v8::PinScope) -> bool {
    let Some(root) = with_tree(Tree::root) else {
        return false;
    };
    let doc = wrap(scope, root);
    dom_implementation::is_xml_document(scope, doc)
}

/// The innerHTML / outerHTML getters of a node of an XML document: the XML serialization
/// (each child of innerHTML starts with no prefixes in scope, like Chrome). None for
/// HTML documents.
pub(crate) fn markup(scope: &mut v8::PinScope, node: Handle, outer: bool) -> Option<Vec<u16>> {
    if !current_document_is_xml(scope) {
        return None;
    }
    let text = with_tree(|tree| {
        if outer {
            serialize(tree, node, None)
        } else {
            tree.children(node)
                .into_iter()
                .map(|child| serialize(tree, child, None))
                .collect::<String>()
        }
    })?;
    Some(units(&text))
}

/// The innerHTML / outerHTML setters of an XML document: the markup parses as an XML
/// fragment in the context element's namespace scope; ill-formed markup throws Chrome's
/// SyntaxError. False (nothing done) for HTML documents.
pub(crate) fn set_markup(
    scope: &mut v8::PinScope,
    node: Handle,
    context: Handle,
    html: &[u16],
    outer: bool,
) -> bool {
    if !current_document_is_xml(scope) {
        return false;
    }
    let markup = String::from_utf16_lossy(html);
    let Some(nodes) = with_tree_mut(|tree| parse_fragment(tree, context, &markup)).flatten() else {
        let property = if outer { "outerHTML" } else { "innerHTML" };
        throw_dom_error(
            scope,
            "SyntaxError",
            &format!("Failed to set the '{property}' property on 'Element': The provided markup is invalid XML, and therefore cannot be inserted into an XML document."),
        );
        return true;
    };
    if !outer {
        range_live::replacing_children(scope, node);
    }
    with_tree_mut(|tree| {
        if outer {
            for child in nodes {
                tree.insert_before(context, child, Some(node));
            }
            tree.remove_child(context, node);
        } else {
            for child in tree.children(node) {
                tree.remove_child(node, child);
            }
            for child in nodes {
                tree.append_child(node, child);
            }
        }
    });
    true
}

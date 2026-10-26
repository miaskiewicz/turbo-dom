//! HTML Sanitizer API core (`Sanitizer`, `setHTML`, `Document.parseHTML[Unsafe]`).
//! Configuration canonicalization and the Web IDL surface live in sanitizer.js;
//! this module walks freshly parsed native trees with a validated configuration.
use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const HTML: &str = "http://www.w3.org/1999/xhtml";
const SVG: &str = "http://www.w3.org/2000/svg";
const MATHML: &str = "http://www.w3.org/1998/Math/MathML";
const XLINK: &str = "http://www.w3.org/1999/xlink";

/// A canonical `{name, namespace}` pair compared by exact code units.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Name {
    name: Vec<u16>,
    namespace: Option<Vec<u16>>,
}
impl Name {
    fn new(name: &str, namespace: Option<&str>) -> Self {
        Self {
            name: name.encode_utf16().collect(),
            namespace: namespace.map(|namespace| namespace.encode_utf16().collect()),
        }
    }
    fn is(&self, name: &str, namespace: Option<&str>) -> bool {
        self.name.iter().copied().eq(name.encode_utf16())
            && match (&self.namespace, namespace) {
                (None, None) => true,
                (Some(actual), Some(expected)) => {
                    actual.iter().copied().eq(expected.encode_utf16())
                }
                _ => false,
            }
    }
}
#[derive(Default)]
struct ElementRules {
    attributes: Option<HashSet<Name>>,
    remove_attributes: Option<HashSet<Name>>,
}
/// A valid canonical SanitizerConfig (see "configuration is valid").
#[derive(Default)]
pub(super) struct Config {
    elements: Option<HashMap<Name, ElementRules>>,
    remove_elements: Option<HashSet<Name>>,
    replace_with_children: Option<HashSet<Name>>,
    attributes: Option<HashSet<Name>>,
    remove_attributes: Option<HashSet<Name>>,
    processing_instructions: Option<HashSet<Vec<u16>>>,
    remove_processing_instructions: Option<HashSet<Vec<u16>>>,
    comments: bool,
    data_attributes: bool,
}

fn property<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let key = v8::String::new(scope, name)?;
    object
        .get(scope, key.into())
        .filter(|value| !value.is_undefined())
}
fn units(scope: &v8::PinScope, value: v8::Local<v8::Value>) -> Option<Vec<u16>> {
    let string = v8::Local::<v8::String>::try_from(value).ok()?;
    Some(string_code_units(scope, string))
}
fn list<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    name: &str,
) -> Option<Vec<v8::Local<'s, v8::Object>>> {
    let array = v8::Local::<v8::Array>::try_from(property(scope, object, name)?).ok()?;
    let mut items = Vec::with_capacity(array.length() as usize);
    for index in 0..array.length() {
        items.push(array.get_index(scope, index)?.to_object(scope)?);
    }
    Some(items)
}
fn name_of<'s>(scope: &mut v8::PinScope<'s, '_>, item: v8::Local<'s, v8::Object>) -> Option<Name> {
    let name = property(scope, item, "name")?;
    let name = units(scope, name)?;
    let namespace = match property(scope, item, "namespace") {
        Some(value) if value.is_string() => Some(units(scope, value)?),
        _ => None,
    };
    Some(Name { name, namespace })
}
fn names<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    key: &str,
) -> Option<Option<HashSet<Name>>> {
    let Some(items) = list(scope, object, key) else {
        return Some(None);
    };
    let mut set = HashSet::with_capacity(items.len());
    for item in items {
        set.insert(name_of(scope, item)?);
    }
    Some(Some(set))
}
fn targets<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    key: &str,
) -> Option<Option<HashSet<Vec<u16>>>> {
    let Some(items) = list(scope, object, key) else {
        return Some(None);
    };
    let mut set = HashSet::with_capacity(items.len());
    for item in items {
        let target = property(scope, item, "target")?;
        set.insert(units(scope, target)?);
    }
    Some(Some(set))
}
fn flag<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    key: &str,
) -> bool {
    property(scope, object, key).is_some_and(|value| value.is_true())
}

impl Config {
    /// Read the binding's private canonical record (null-prototype data objects
    /// produced by sanitizer.js; no guest code runs while it is read).
    pub(super) fn read<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        record: v8::Local<'s, v8::Object>,
    ) -> Option<Self> {
        let elements = match list(scope, record, "elements") {
            None => None,
            Some(items) => {
                let mut map = HashMap::with_capacity(items.len());
                for item in items {
                    let name = name_of(scope, item)?;
                    let rules = ElementRules {
                        attributes: names(scope, item, "attributes")?,
                        remove_attributes: names(scope, item, "removeAttributes")?,
                    };
                    map.insert(name, rules);
                }
                Some(map)
            }
        };
        Some(Self {
            elements,
            remove_elements: names(scope, record, "removeElements")?,
            replace_with_children: names(scope, record, "replaceWithChildrenElements")?,
            attributes: names(scope, record, "attributes")?,
            remove_attributes: names(scope, record, "removeAttributes")?,
            processing_instructions: targets(scope, record, "processingInstructions")?,
            remove_processing_instructions: targets(scope, record, "removeProcessingInstructions")?,
            comments: flag(scope, record, "comments"),
            data_attributes: flag(scope, record, "dataAttributes"),
        })
    }
    /// The built-in safe default configuration (Chrome's `new Sanitizer().get()`).
    pub(super) fn builtin_default() -> &'static Self {
        static DEFAULT: OnceLock<Config> = OnceLock::new();
        DEFAULT.get_or_init(|| {
            let data: serde_json::Value =
                serde_json::from_str(DEFAULT_JSON).expect("recorded default sanitizer policy");
            let config = &data["configuration"];
            let names = |values: &serde_json::Value| {
                values
                    .as_array()
                    .expect("attribute list")
                    .iter()
                    .map(|item| {
                        Name::new(item["name"].as_str().unwrap(), item["namespace"].as_str())
                    })
                    .collect::<HashSet<_>>()
            };
            Config {
                elements: Some(
                    config["elements"]
                        .as_array()
                        .expect("element list")
                        .iter()
                        .map(|item| {
                            (
                                Name::new(
                                    item["name"].as_str().unwrap(),
                                    item["namespace"].as_str(),
                                ),
                                ElementRules {
                                    attributes: Some(names(&item["attributes"])),
                                    remove_attributes: None,
                                },
                            )
                        })
                        .collect(),
                ),
                attributes: Some(names(&config["attributes"])),
                processing_instructions: Some(HashSet::new()),
                comments: config["comments"].as_bool() == Some(true),
                data_attributes: config["dataAttributes"].as_bool() == Some(true),
                ..Default::default()
            }
        })
    }
}
pub(super) const DEFAULT_JSON: &str = include_str!("browser_sanitizer_default.json");
const BASELINE_JSON: &str = include_str!("sanitizer_safe_baseline.json");

/// The URL parser's scheme for `value` is "javascript": leading/trailing C0
/// control-or-space is stripped, ASCII tab/newline removed, and a scheme is an
/// ASCII alpha followed by alphanumerics, "+", "-" or "." up to ":".
fn javascript_url(value: &[u16]) -> bool {
    let mut scheme = Vec::new();
    let mut started = false;
    for &unit in value {
        if matches!(unit, 0x09 | 0x0a | 0x0d) {
            continue;
        }
        if !started {
            if unit <= 0x20 {
                continue;
            }
            started = true;
            if !(unit as u8).is_ascii_alphabetic() || unit > 0x7f {
                return false;
            }
        }
        if unit == u16::from(b':') {
            return scheme.eq_ignore_ascii_case(b"javascript");
        }
        if unit > 0x7f || !((unit as u8).is_ascii_alphanumeric() || b"+-.".contains(&(unit as u8)))
        {
            return false;
        }
        scheme.push(unit as u8);
    }
    false
}
fn navigating(element: &Name, attribute: &Name) -> bool {
    (element.is("a", Some(HTML))
        || element.is("area", Some(HTML))
        || element.is("base", Some(HTML)))
        && attribute.is("href", None)
        || (element.is("button", Some(HTML)) || element.is("input", Some(HTML)))
            && attribute.is("formaction", None)
        || element.is("form", Some(HTML)) && attribute.is("action", None)
        || element.is("a", Some(SVG))
            && (attribute.is("href", None) || attribute.is("href", Some(XLINK)))
}
fn animating(element: &Name, attribute: &Name) -> bool {
    (element.is("animate", Some(SVG))
        || element.is("animateTransform", Some(SVG))
        || element.is("set", Some(SVG)))
        && attribute.is("attributeName", None)
}
fn in_namespace(element: &Name, namespace: &str) -> bool {
    element
        .namespace
        .as_ref()
        .is_some_and(|actual| actual.iter().copied().eq(namespace.encode_utf16()))
}
fn data_attribute(attribute: &Name) -> bool {
    attribute
        .name
        .starts_with(&"data-".encode_utf16().collect::<Vec<_>>())
}

/// One pending node of the sanitizer walk: its parser parent element (where a
/// `<template shadowrootmode>` would attach) and whether that parent was
/// replaced with its children.
struct Pending {
    node: Handle,
    parser_parent: Handle,
    parent_replaced: bool,
}
fn children_of(
    tree: &Tree,
    parent: Handle,
    parser_parent: Handle,
    parent_replaced: bool,
) -> impl Iterator<Item = Pending> {
    tree.children(parent).into_iter().map(move |node| Pending {
        node,
        parser_parent,
        parent_replaced,
    })
}

/// "Sanitize core" over the children of `root` in a freshly parsed tree whose
/// template contents are still parser fragment children. Chrome sanitizes each
/// node as the parser inserts it, so a `<template>` keeps its declarative
/// shadow root only if it survives with a valid `shadowrootmode` under a kept
/// valid host; one under a host that was replaced with its children attaches
/// to that discarded host. The returned templates are the ones the
/// declarative conversion may attach. Removal and attribute filtering depend
/// only on each node, so a work stack visits nodes without recursion.
pub(super) fn sanitize(
    tree: &mut Tree,
    root: Handle,
    config: &Config,
    safe: bool,
) -> HashSet<Handle> {
    let empty = ElementRules::default();
    let template = Name::new("template", Some(HTML));
    let mut declarative = HashSet::new();
    let mut discarded_hosts = HashSet::new();
    let mut pending = children_of(tree, root, root, false).collect::<Vec<_>>();
    while let Some(Pending {
        node,
        parser_parent,
        parent_replaced,
    }) = pending.pop()
    {
        let Some(parent) = tree.parent(node) else {
            continue;
        };
        match tree.node_type_id(node) {
            // Template contents (a parser fragment child of its template).
            11 => pending.extend(children_of(tree, node, parser_parent, false)),
            // Chrome keeps comments that are children of a Document.
            8 if !config.comments && tree.node_type_id(parent) != 9 => {
                tree.remove_child(parent, node)
            }
            7 => {
                let target = tree
                    .processing_instruction_target(node)
                    .unwrap_or_default()
                    .encode_utf16()
                    .collect::<Vec<_>>();
                let allowed = config
                    .processing_instructions
                    .as_ref()
                    .is_none_or(|list| list.contains(&target))
                    && !config
                        .remove_processing_instructions
                        .as_ref()
                        .is_some_and(|list| list.contains(&target));
                if !allowed {
                    tree.remove_child(parent, node);
                }
            }
            1 => {
                let element = Name {
                    name: tree.local_name_utf16(node).unwrap_or_default(),
                    namespace: tree.namespace_uri_utf16(node),
                };
                if config
                    .replace_with_children
                    .as_ref()
                    .is_some_and(|list| list.contains(&element))
                {
                    for child in tree.children(node) {
                        // A replaced template contributes its contents.
                        let hoisted = if tree.node_type_id(child) == 11 {
                            tree.children(child)
                        } else {
                            vec![child]
                        };
                        for child in hoisted {
                            tree.insert_before(parent, child, Some(node));
                            pending.push(Pending {
                                node: child,
                                parser_parent: node,
                                parent_replaced: true,
                            });
                        }
                    }
                    tree.remove_child(parent, node);
                    continue;
                }
                let rules = match &config.elements {
                    Some(elements) => elements.get(&element),
                    None => Some(&empty),
                };
                let Some(rules) = rules.filter(|_| {
                    !config
                        .remove_elements
                        .as_ref()
                        .is_some_and(|list| list.contains(&element))
                }) else {
                    tree.remove_child(parent, node);
                    continue;
                };
                filter_attributes(tree, node, &element, rules, config, safe);
                let shadow_template = element == template
                    && tree
                        .get_attribute(node, "shadowrootmode")
                        .is_some_and(|mode| {
                            mode.eq_ignore_ascii_case("open") || mode.eq_ignore_ascii_case("closed")
                        })
                    && shadow_html::valid_host(tree, parser_parent);
                if shadow_template && parent_replaced {
                    // Attached to the discarded host: dropped with it, once.
                    if discarded_hosts.insert(parser_parent) {
                        tree.remove_child(parent, node);
                        continue;
                    }
                } else if shadow_template {
                    declarative.insert(node);
                }
                pending.extend(children_of(tree, node, node, false));
                if let Some(shadow) = tree.shadow_root(node) {
                    pending.extend(children_of(tree, shadow, shadow, false));
                }
            }
            _ => {}
        }
    }
    declarative
}
fn filter_attributes(
    tree: &mut Tree,
    node: Handle,
    element: &Name,
    rules: &ElementRules,
    config: &Config,
    safe: bool,
) {
    let mathml = in_namespace(element, MATHML);
    for attribute in tree.attributes_units(node) {
        let name = Name {
            name: attribute.local_name,
            namespace: attribute.namespace,
        };
        let contains =
            |list: &Option<HashSet<Name>>| list.as_ref().is_some_and(|list| list.contains(&name));
        let mut remove = contains(&rules.remove_attributes);
        if !remove {
            remove = if config.attributes.is_some() {
                !contains(&config.attributes)
                    && !contains(&rules.attributes)
                    && !(config.data_attributes && data_attribute(&name))
            } else if rules.attributes.is_some() {
                !contains(&rules.attributes)
            } else {
                contains(&config.remove_attributes)
            };
        }
        if !remove && safe {
            let value = &attribute.value;
            remove = navigating(element, &name) && javascript_url(value)
                || mathml
                    && (name.is("href", None) || name.is("href", Some(XLINK)))
                    && javascript_url(value)
                || animating(element, &name)
                    && (value.iter().copied().eq("href".encode_utf16())
                        || value.iter().copied().eq("xlink:href".encode_utf16()));
        }
        if remove {
            tree.remove_attribute_ns_units(node, name.namespace.as_deref(), &name.name);
        }
    }
}

fn throw_type_error(scope: &mut v8::PinScope, message: &str) {
    if let Some(message) = v8::String::new(scope, message) {
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
    }
}
fn record_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Private>> {
    // Isolate-wide private name: a Sanitizer from any realm brands in every realm.
    let name = v8::String::new(scope, "turbo.sanitizer.configuration")?;
    Some(v8::Private::for_api(scope, Some(name)))
}
/// The current realm's document, for Trusted Types enforcement of static sinks.
fn realm_document<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Object>> {
    let owner = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map_or(DOM_ID.with(|id| id.get()), |id| id.0);
    let _owner = DomScope::enter(owner);
    let root = with_tree(Tree::root)?;
    Some(wrap(scope, root))
}
fn script_context(scope: &mut v8::PinScope, receiver: v8::Local<v8::Object>) -> bool {
    let _owner = DomScope::for_object(scope, receiver);
    handle_of(scope, receiver)
        .and_then(|node| {
            with_tree(|tree| {
                tree.node_type_id(node) == 1
                    && tree.local_name(node) == Some("script")
                    && matches!(tree.namespace_uri(node), Some(HTML | SVG))
            })
        })
        .unwrap_or(false)
}
fn html_target(scope: &mut v8::PinScope, receiver: v8::Local<v8::Object>) -> bool {
    let _owner = DomScope::for_object(scope, receiver);
    handle_of(scope, receiver)
        .and_then(|node| {
            with_tree(|tree| tree.node_type_id(node) == 1 || tree.is_shadow_root(node))
        })
        .unwrap_or(false)
}

fn native<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    if scope.is_execution_terminating() {
        return;
    }
    let operation = args.get(0).to_rust_string_lossy(scope);
    match operation.as_str() {
        // The private configuration record of a Sanitizer (brand + state).
        "record" => {
            let (Ok(object), Some(key)) = (
                v8::Local::<v8::Object>::try_from(args.get(1)),
                record_key(scope),
            ) else {
                return;
            };
            if object.is_proxy() {
                return;
            }
            if args.length() > 2 {
                object.set_private(scope, key, args.get(2));
            } else if let Some(value) = object.get_private(scope, key) {
                rv.set(value);
            }
        }
        "trustedHTML.is" => {
            let trusted = trusted_types::is_trusted(scope, args.get(1), "TrustedHTML");
            rv.set_bool(trusted);
        }
        "scriptContext" => {
            let script = v8::Local::<v8::Object>::try_from(args.get(1))
                .is_ok_and(|receiver| script_context(scope, receiver));
            rv.set_bool(script);
        }
        // Element/ShadowRoot setHTML: arguments are converted and the safe
        // configuration (remove-unsafe applied) is resolved by the caller.
        "setHTML" => {
            let Ok(receiver) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
                throw_type_error(scope, "Illegal invocation");
                return;
            };
            if !html_target(scope, receiver) {
                throw_type_error(scope, "Illegal invocation");
                return;
            }
            let (Some(html), Ok(record)) = (
                units(scope, args.get(2)),
                v8::Local::<v8::Object>::try_from(args.get(3)),
            ) else {
                return;
            };
            let Some(config) = Config::read(scope, record) else {
                return;
            };
            shadow_html::set_html_unsafe(scope, receiver, &html, Some((&config, true)));
        }
        // (TrustedHTML or DOMString) through the Trusted Types check of `sink`,
        // run by the receiver's (or this realm's) document after option
        // conversion and before the sanitizer configuration is validated.
        "trustedHTML.convert" => {
            let node = match v8::Local::<v8::Object>::try_from(args.get(1)) {
                Ok(node) => node,
                Err(_) => {
                    let Some(document) = realm_document(scope) else {
                        return;
                    };
                    document
                }
            };
            let sink = args.get(3).to_rust_string_lossy(scope);
            let prefix = args.get(4).to_rust_string_lossy(scope);
            if let Some(html) = trusted_types::units(
                scope,
                node,
                args.get(2),
                "TrustedHTML",
                &sink,
                &prefix,
                false,
            ) {
                let html = unit_string(scope, &html);
                rv.set(html.into());
            }
        }
        // Both markups arrive converted (parseHTMLUnsafe's already passed its
        // Trusted Types check).
        "parse" => {
            let safe = args.get(1).is_true();
            let Some(html) = units(scope, args.get(2)) else {
                return;
            };
            let config = v8::Local::<v8::Object>::try_from(args.get(3))
                .ok()
                .and_then(|record| Config::read(scope, record));
            let tree = Tree::parse_utf16(&html);
            let document = parse_html_tree_with(
                scope,
                tree,
                true,
                config.as_ref().map(|config| (config, safe)),
            );
            if dom_implementation::initialize(scope, document, "text/html").is_some() {
                rv.set(document.into());
            }
        }
        _ => throw_type_error(scope, "Illegal invocation"),
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__sanitizerNative", native);
    for (name, json) in [
        ("__sanitizerDefault", DEFAULT_JSON),
        ("__sanitizerBaseline", BASELINE_JSON),
    ] {
        let Some(value) = v8::String::new(scope, json) else {
            return;
        };
        set_prop(scope, global, name, value.into());
    }
    run_js(scope, include_str!("sanitizer.js"));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn javascript_scheme_follows_url_parser_prefix_rules() {
        let check = |value: &str| javascript_url(&value.encode_utf16().collect::<Vec<_>>());
        for value in [
            "javascript:x",
            " \u{1}JaVaScRiPt:x",
            "java\tscr\nipt:x",
            "javascript:",
        ] {
            assert!(check(value), "{value}");
        }
        for value in [
            "javascript",
            "javascript :x",
            "\u{a0}javascript:x",
            "java\u{fffd}script:x",
            "vbscript:x",
            "data:text/html,x",
            "1javascript:x",
            "",
        ] {
            assert!(!check(value), "{value}");
        }
    }
}

//! Host-only style authorization keeps authored attributes separate from applied CSS.
use super::*;
use turbo_dom_parser::rtdom::cascade::StylePolicy;

#[derive(Default)]
pub(super) struct PolicyState {
    callback: Option<v8::Weak<v8::Function>>,
    sheets: HashMap<Handle, (String, bool)>,
    inline: HashMap<Handle, Inline>,
    cascade: Option<(u64, StylePolicy)>,
    shadow_cascade: Option<(u64, StylePolicy)>,
}
#[derive(Clone)]
struct Inline {
    source: String,
    applied: String,
}
fn authorize(
    scope: &mut v8::PinScope,
    node: Handle,
    kind: &str,
    text: &str,
    initial: Option<&[String]>,
) -> bool {
    let callback = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .and_then(|dom| dom.css_policy.callback.clone())
    });
    let Some(callback) = callback else {
        return true;
    };
    let callback = dom_ownership::local(scope, callback);
    let root = with_tree(Tree::root).unwrap();
    let document = wrap(scope, root);
    let element = wrap(scope, node);
    let units = with_tree(|tree| {
        if kind == "style-src-attr" {
            tree.get_attribute_utf16(node, "style").unwrap_or_default()
        } else {
            tree.text_content_utf16(node)
        }
    })
    .unwrap_or_else(|| text.encode_utf16().collect());
    authorize_element(scope, document, element, callback, kind, &units, initial)
}
fn authorize_element<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    element: v8::Local<'s, v8::Object>,
    callback: v8::Local<'s, v8::Function>,
    kind: &str,
    units: &[u16],
    initial: Option<&[String]>,
) -> bool {
    let parser_position = initial.and_then(|_| {
        node_parser_position(
            scope,
            element,
            (kind == "style-src-attr").then_some("style"),
        )
    });
    let owner_window = node_has_window_owner(scope, element);
    let source = v8::Object::new(scope);
    let null = v8::null(scope);
    source.set_prototype(scope, null.into());
    let key = v8::String::new(scope, "ownerWindow").unwrap();
    let value = v8::Boolean::new(scope, owner_window);
    source.set(scope, key.into(), value.into());
    if let Some(position) = parser_position {
        for (name, value) in [
            ("offset", position.utf16_offset),
            ("line", position.line),
            ("column", position.column),
        ] {
            let key = v8::String::new(scope, name).unwrap();
            let value = v8::Number::new(scope, value as f64);
            source.set(scope, key.into(), value.into());
        }
    }
    let text = unit_string(scope, units);
    let kind = v8::String::new(scope, kind).unwrap();
    let metas: v8::Local<v8::Value> = if let Some(initial) = initial {
        let array = v8::Array::new(scope, initial.len() as i32);
        for (index, value) in initial.iter().enumerate() {
            let value = v8::String::new(scope, value).unwrap();
            array.set_index(scope, index as u32, value.into());
        }
        array.into()
    } else {
        v8::undefined(scope).into()
    };
    let receiver = v8::undefined(scope);
    callback
        .call(
            scope,
            receiver.into(),
            &[
                document.into(),
                element.into(),
                kind.into(),
                text.into(),
                metas,
                source.into(),
            ],
        )
        .is_some_and(|value| value.is_true())
}
fn source(node: Handle, sheet: bool) -> String {
    with_tree(|tree| {
        if sheet {
            tree.text_content(node)
        } else {
            tree.get_attribute(node, "style").unwrap_or("").to_owned()
        }
    })
    .unwrap_or_default()
}
fn inline_changed(scope: &mut v8::PinScope, node: Handle, initial: Option<&[String]>) {
    let source = source(node, false);
    let present = with_tree(|tree| tree.has_attribute(node, "style")).unwrap_or(false);
    let allowed = authorize(scope, node, "style-src-attr", &source, initial);
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let policy = &mut dom.as_mut().unwrap().css_policy;
        let applied = if !present {
            String::new()
        } else if allowed {
            source.clone()
        } else {
            policy
                .inline
                .get(&node)
                .map(|old| old.applied.clone())
                .unwrap_or_default()
        };
        policy.inline.insert(node, Inline { source, applied });
    });
}
pub(super) fn attribute_changed(scope: &mut v8::PinScope, node: Handle) {
    inline_changed(scope, node, None);
}
fn applied_inline(scope: &mut v8::PinScope, node: Handle) -> String {
    let text = source(node, false);
    let cached = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .unwrap()
            .css_policy
            .inline
            .get(&node)
            .cloned()
    });
    if cached.as_ref().is_none_or(|entry| entry.source != text) {
        inline_changed(scope, node, None);
    }
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .unwrap()
            .css_policy
            .inline
            .get(&node)
            .unwrap()
            .applied
            .clone()
    })
}
fn sheet_allowed(scope: &mut v8::PinScope, node: Handle, initial: Option<&[String]>) -> bool {
    let element = wrap(scope, node);
    if !node_has_window_owner(scope, element) {
        return false;
    }
    let text = source(node, true);
    let cached = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .unwrap()
            .css_policy
            .sheets
            .get(&node)
            .cloned()
    });
    if initial.is_none() {
        if let Some((source, allowed)) = cached {
            if source == text {
                return allowed;
            }
        }
    }
    let allowed = authorize(scope, node, "style-src-elem", &text, initial);
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .css_policy
            .sheets
            .insert(node, (text, allowed))
    });
    allowed
}
pub(super) fn invalidate_sheet(node: Handle) {
    DOM.with(|dom| {
        if let Some(dom) = dom.borrow_mut().as_mut() {
            dom.css_policy.sheets.remove(&node);
        }
    });
}
pub fn install(scope: &mut v8::PinScope, callback: v8::Local<v8::Function>) {
    let Some(callback) = dom_ownership::helper_function(scope, callback) else {
        return;
    };
    DOM.with(|dom| {
        dom.borrow_mut().as_mut().unwrap().css_policy = PolicyState {
            callback: Some(callback.clone()),
            ..Default::default()
        }
    });
    let callback = dom_ownership::local(scope, callback);
    // Native preorder preserves parser policy position. Later meta policies never
    // retroactively revoke a style that was already authorized.
    let document = wrap(scope, with_tree(Tree::root).unwrap());
    let nodes = template_contents::parser_policy_nodes(scope, document);
    let mut metas = Vec::new();
    for element in nodes {
        let _owner = DomScope::for_object(scope, element);
        let Some(node) = handle_of(scope, element) else {
            continue;
        };
        let owner_window = node_has_window_owner(scope, element);
        let (tag, style, meta) = with_tree(|tree| {
            (
                tree.local_name(node).unwrap_or("").to_owned(),
                tree.has_attribute(node, "style"),
                if owner_window
                    && tree.local_name(node) == Some("meta")
                    && tree
                        .parent(node)
                        .is_some_and(|parent| tree.local_name(parent) == Some("head"))
                    && tree
                        .get_attribute(node, "http-equiv")
                        .is_some_and(|value| value.eq_ignore_ascii_case("content-security-policy"))
                {
                    tree.get_attribute(node, "content").map(str::to_owned)
                } else {
                    None
                },
            )
        })
        .unwrap();
        if style {
            if owner_window {
                inline_changed(scope, node, Some(&metas));
            } else {
                // The parser checks attributes against its active document policy,
                // even after template children move into their inert owner storage.
                let units =
                    with_tree(|tree| tree.get_attribute_utf16(node, "style").unwrap_or_default())
                        .unwrap_or_default();
                authorize_element(
                    scope,
                    document,
                    element,
                    callback,
                    "style-src-attr",
                    &units,
                    Some(&metas),
                );
            }
        }
        if tag == "style" && owner_window {
            sheet_allowed(scope, node, Some(&metas));
        }
        if let Some(meta) = meta {
            metas.push(meta);
        }
    }
}
pub(super) fn cascade_policy(scope: &mut v8::PinScope) -> StylePolicy {
    let version = with_tree(|tree| tree.version).unwrap_or_default();
    if let Some(policy) = DOM.with(|dom| {
        let dom = dom.borrow();
        let state = &dom.as_ref().unwrap().css_policy;
        if state.callback.is_none() {
            return Some(StylePolicy::default());
        }
        state
            .cascade
            .as_ref()
            .filter(|(cached, _)| *cached == version)
            .map(|(_, policy)| policy.clone())
    }) {
        return policy;
    }
    let mut policy = StylePolicy::default();
    let nodes = with_tree(|tree| {
        tree.handles()
            .filter(|&node| tree.node_type_id(node) == 1)
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    for node in nodes {
        if with_tree(|tree| tree.local_name(node) == Some("style")).unwrap_or(false)
            && !sheet_allowed(scope, node, None)
        {
            policy.blocked_stylesheets.insert(node);
        }
        let text = source(node, false);
        let applied = applied_inline(scope, node);
        if text != applied {
            policy.inline_overrides.insert(node, applied);
        }
    }
    DOM.with(|dom| {
        dom.borrow_mut().as_mut().unwrap().css_policy.cascade = Some((version, policy.clone()))
    });
    policy
}
/// Whether native shadow-scoped rules (shadow `<style>`, `:host`, `::slotted`)
/// can reach `node` or a flattened ancestor. Only those rules are missing from
/// the JS CSSOM cascade.
pub(super) fn shadow_scoped(tree: &Tree, node: Handle) -> bool {
    let mut current = Some(node);
    while let Some(node) = current {
        if tree.shadow_root(node).is_some() || tree.is_shadow_root(node) {
            return true;
        }
        current = tree.parent(node);
    }
    false
}
/// The native cascade restricted to shadow-scoped rules. Document `<style>`
/// sheets belong to the JS CSSOM, whose fresh per-read snapshots resolve their
/// inheritance; native text would keep serving rules CSSOM already edited.
pub(super) fn shadow_cascade_policy(scope: &mut v8::PinScope) -> StylePolicy {
    let version = with_tree(|tree| tree.version).unwrap_or_default();
    if let Some(policy) = DOM.with(|dom| {
        let dom = dom.borrow();
        let state = &dom.as_ref().unwrap().css_policy;
        state
            .shadow_cascade
            .as_ref()
            .filter(|(cached, _)| *cached == version)
            .map(|(_, policy)| policy.clone())
    }) {
        return policy;
    }
    let mut policy = cascade_policy(scope);
    with_tree(|tree| {
        policy
            .blocked_stylesheets
            .extend(tree.get_elements_by_tag_name("style"))
    });
    DOM.with(|dom| {
        dom.borrow_mut().as_mut().unwrap().css_policy.shadow_cascade =
            Some((version, policy.clone()))
    });
    policy
}
fn read_inline(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let text = applied_inline(scope, node);
    rv.set(v8::String::new(scope, &text).unwrap().into());
}
fn write_inline(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    let text = arg_str(scope, &args, 0);
    with_tree_mut(|tree| tree.set_attribute(node, "style", &text));
    DOM.with(|dom| {
        dom.borrow_mut().as_mut().unwrap().css_policy.inline.insert(
            node,
            Inline {
                source: text.clone(),
                applied: text,
            },
        )
    });
}
fn read_sheet(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = receiver_handle(scope, &args, &[1]) else {
        return;
    };
    rv.set_bool(sheet_allowed(scope, node, None));
}
pub(super) fn register(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__readAppliedInlineStyle", read_inline);
    bind_method(scope, global, "__writeAppliedInlineStyle", write_inline);
    bind_method(scope, global, "__isInlineSheetAllowed", read_sheet);
}
impl PolicyState {
    pub(super) fn sweep_storage(&mut self, tree: &Tree) {
        self.sheets.retain(|node, _| tree.is_live(*node));
        self.inline.retain(|node, _| tree.is_live(*node));
        self.cascade = None;
        self.shadow_cascade = None;
    }
}

fn own_object<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Object>> {
    let key = v8::String::new(scope, name)?;
    let descriptor = object.get_own_property_descriptor(scope, key.into())?;
    let descriptor = v8::Local::<v8::Object>::try_from(descriptor).ok()?;
    let value = v8::String::new(scope, "value")?;
    if descriptor.has_own_property(scope, value.into()) != Some(true) {
        return None;
    }
    v8::Local::<v8::Object>::try_from(descriptor.get(scope, value.into())?).ok()
}

pub(super) fn clear_adopted_sheets<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    shadow: v8::Local<'s, v8::Object>,
) {
    if scope.is_execution_terminating() {
        return;
    }
    let Some(record) = get_interface_record(scope, shadow)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return;
    };
    let Some(target) = own_object(scope, record, "css")
        .and_then(|css| own_object(scope, css, "adoption"))
        .and_then(|adoption| own_object(scope, adoption, "target"))
        .and_then(|target| v8::Local::<v8::Array>::try_from(target).ok())
    else {
        return;
    };
    // Mutate the private backing array, preserving every retained ObservableArray
    // identity without consulting page-overridden getters or array methods.
    let length = v8::String::new(scope, "length").unwrap();
    let zero = v8::Integer::new(scope, 0);
    target.set(scope, length.into(), zero.into());
}

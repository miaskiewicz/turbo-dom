//! Authored HTML handler bodies are compiled by V8 with native DOM scope extensions.
use super::*;
type SourceKey = (Handle, String);
type SourceValue = Option<Vec<u16>>;
type AdoptedSource = (SourceKey, SourceValue);
#[derive(Default)]
pub(super) struct State {
    policy: Option<v8::Weak<v8::Function>>,
    base: String,
    sources: HashMap<SourceKey, SourceValue>,
}
fn source(node: Handle, name: &str) -> Option<Vec<u16>> {
    with_tree(|tree| tree.get_attribute_utf16(node, name)).flatten()
}
fn note(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = handle_of(scope, args.this()) else {
        return;
    };
    let name = arg_str(scope, &args, 0);
    let source = source(node, &name);
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .event_handlers
            .sources
            .insert((node, name), source);
    });
}
fn prepare(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = handle_of(scope, args.this()) else {
        return;
    };
    let name = arg_str(scope, &args, 0);
    refresh(scope, node, &name);
}
pub(super) fn changed(scope: &mut v8::PinScope, node: Handle, name: &str) {
    if !name.starts_with("on") {
        return;
    }
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .event_handlers
            .sources
            .remove(&(node, name.to_owned()));
    });
    refresh(scope, node, name);
}
pub(super) fn refresh(scope: &mut v8::PinScope, node: Handle, name: &str) {
    if with_tree(|tree| tree.node_type_id(node)) != Some(1)
        || !element_members::compiles_content_attribute(node, name)
    {
        return;
    }
    let source = source(node, name);
    let (same, policy, base) = DOM.with(|dom| {
        let dom = dom.borrow();
        let state = &dom.as_ref().unwrap().event_handlers;
        (
            state
                .sources
                .get(&(node, name.to_owned()))
                .is_some_and(|old| old == &source),
            state.policy.clone(),
            state.base.clone(),
        )
    });
    if same {
        return;
    }
    let element = wrap(scope, node);
    let key = v8::String::new(scope, name).unwrap();
    if element.has(scope, key.into()) != Some(true) {
        return;
    }
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .event_handlers
            .sources
            .insert((node, name.to_owned()), source.clone());
    });
    let root = with_tree(Tree::root).unwrap();
    let document = wrap(scope, root);
    let active = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|state| state.browsing_context)
    });
    let function = source.filter(|_| active).and_then(|text| {
        if let Some(policy) = policy {
            let callback = dom_ownership::local(scope, policy);
            let kind = v8::String::new(scope, "script-src-attr").unwrap();
            let code = v8::String::new_from_two_byte(scope, &text, v8::NewStringType::Normal)?;
            let receiver = v8::undefined(scope);
            if !callback
                .call(
                    scope,
                    receiver.into(),
                    &[document.into(), element.into(), kind.into(), code.into()],
                )
                .is_some_and(|value| value.is_true())
            {
                return None;
            }
        }
        let code = v8::String::new_from_two_byte(scope, &text, v8::NewStringType::Normal)?;
        let resource = v8::String::new(
            scope,
            if base.is_empty() {
                "about:blank"
            } else {
                &base
            },
        )
        .unwrap();
        let origin = v8::ScriptOrigin::new(
            scope,
            resource.into(),
            0,
            0,
            false,
            0,
            None,
            false,
            false,
            false,
            None,
        );
        let mut source = v8::script_compiler::Source::new(code, Some(&origin));
        let event = v8::String::new(scope, element_members::handler_parameter(node)).unwrap();
        let mut extensions = vec![document];
        let form = with_tree(|tree| {
            let mut parent = tree.parent(node);
            while let Some(current) = parent {
                if tree.tag_name(current).is_some_and(|name| name == "form") {
                    return Some(current);
                }
                parent = tree.parent(current);
            }
            None
        })
        .flatten();
        if let Some(form) = form {
            extensions.push(wrap(scope, form));
        }
        extensions.push(element);
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut catch.init();
        let function = v8::script_compiler::compile_function(
            scope,
            &mut source,
            &[event],
            &extensions,
            v8::script_compiler::CompileOptions::NoCompileOptions,
            v8::script_compiler::NoCacheReason::NoReason,
        );
        if function.is_none() {
            scope.reset();
        }
        if let Some(function) = function {
            function.set_name(key);
        }
        function
    });
    let value = function.map_or_else(|| v8::null(scope).into(), Into::into);
    element.set(scope, key.into(), value);
}
pub(super) fn register(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__prepareInlineHandler", prepare);
    bind_method(scope, global, "__noteInlineHandler", note);
}
pub(super) fn install(
    scope: &mut v8::PinScope,
    policy: Option<v8::Local<v8::Function>>,
    base: &str,
) {
    let policy = match policy {
        Some(callback) => {
            let Some(callback) = dom_ownership::helper_function(scope, callback) else {
                return;
            };
            Some(callback)
        }
        None => None,
    };
    DOM.with(|dom| {
        dom.borrow_mut().as_mut().unwrap().event_handlers = State {
            policy,
            base: base.to_owned(),
            sources: HashMap::new(),
        };
    });
    let attrs = with_tree(|tree| {
        tree.query_selector_all("*")
            .iter()
            .flat_map(|&node| {
                tree.attributes(node)
                    .into_iter()
                    .filter(|(name, _)| name.starts_with("on"))
                    .map(move |(name, _)| (node, name.to_owned()))
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    for (node, name) in attrs {
        refresh(scope, node, &name);
    }
}

/// Keep the native source marker with an adopted node. Losing this marker makes
/// the destination's first refresh overwrite a previously assigned on* callback.
pub(super) fn take_adopted_sources(mapping: &[(Handle, Handle)]) -> Vec<AdoptedSource> {
    let mapping = mapping.iter().copied().collect::<HashMap<_, _>>();
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let sources = &mut dom.as_mut().unwrap().event_handlers.sources;
        let keys = sources
            .keys()
            .filter(|(node, _)| mapping.contains_key(node))
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter()
            .map(|key| {
                let value = sources.remove(&key).unwrap();
                ((mapping[&key.0], key.1), value)
            })
            .collect()
    })
}
pub(super) fn restore_adopted_sources(sources: Vec<AdoptedSource>) {
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .event_handlers
            .sources
            .extend(sources)
    });
}
impl State {
    pub(super) fn sweep_storage(&mut self, tree: &Tree) {
        self.sources.retain(|(node, _), _| tree.is_live(*node));
    }
}

/// Host-installed document URL, never read through guest location accessors.
pub(super) fn document_url() -> String {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .map(|dom| dom.event_handlers.base.clone())
            .filter(|url| !url.is_empty())
            .unwrap_or_else(|| "about:blank".to_owned())
    })
}

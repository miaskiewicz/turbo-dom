//! Script preparation flags live with native nodes, independently of wrapper GC.
use super::*;
use turbo_dom_parser::rtdom::mutations::{MutationEvent, MutationRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptState {
    pub already_started: bool,
    pub parser_inserted: bool,
    pub force_async: bool,
    parser_document: Option<u64>,
}
impl Default for ScriptState {
    fn default() -> Self {
        Self {
            already_started: false,
            parser_inserted: false,
            force_async: true,
            parser_document: None,
        }
    }
}

#[derive(Debug)]
pub struct PreparedScript {
    pub source: String,
    /// Original inline source units for direct V8 compilation.
    pub source_units: Vec<u16>,
    pub src: Option<String>,
    pub kind: String,
    pub asynchronous: bool,
    pub parser_inserted: bool,
    pub attributes: Vec<(String, String)>,
    pub in_shadow_tree: bool,
}

fn script(tree: &Tree, node: Handle) -> bool {
    tree.node_type_id(node) == 1
        && tree.namespace_id(node) == 0
        && tree.local_name(node) == Some("script")
}

pub(super) fn observe(node: Handle) {
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        if let Some(dom) = dom.as_mut() {
            if script(&dom.tree, node) {
                dom.scripts.entry(node).or_default();
            }
        }
    });
}

pub(super) fn recording() -> bool {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|dom| !dom.scripts.is_empty())
    })
}

pub(super) fn initial(tree: &Tree) -> HashMap<Handle, ScriptState> {
    let mut states = HashMap::new();
    let mut pending = vec![(tree.root(), false)];
    while let Some((node, inherited_template)) = pending.pop() {
        let template = inherited_template
            || tree.node_type_id(node) == 1 && tree.local_name(node) == Some("template");
        if script(tree, node) {
            states.insert(
                node,
                if template {
                    ScriptState::default()
                } else {
                    ScriptState {
                        already_started: false,
                        parser_inserted: true,
                        force_async: false,
                        parser_document: Some(DOM_ID.with(std::cell::Cell::get)),
                    }
                },
            );
        }
        pending.extend(
            tree.children(node)
                .into_iter()
                .rev()
                .map(|child| (child, template)),
        );
        // Declarative shadow roots of the parsed document hold parser scripts.
        if let Some(shadow) = tree.shadow_root(node) {
            pending.push((shadow, template));
        }
    }
    states
}

/// Script elements (as `querySelectorAll("script")` selects them) of a freshly
/// parsed tree in parse order, taken before declarative shadow roots attach:
/// a root's scripts keep the position of the template that declared them.
pub(super) fn parse_order(tree: &Tree) -> Vec<Handle> {
    let mut order = Vec::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if tree.node_type_id(node) == 1 && tree.local_name(node) == Some("script") {
            order.push(node);
        }
        pending.extend(tree.children(node).into_iter().rev());
    }
    order
}

/// The document's parser scripts in parse order, including those inside
/// declarative shadow roots, that are still shadow-including connected to it.
pub fn parser_scripts<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
) -> Vec<v8::Local<'s, v8::Object>> {
    let _owner = DomScope::for_object(scope, document);
    let handles = DOM.with(|dom| {
        dom.borrow().as_ref().map_or_else(Vec::new, |dom| {
            dom.parser_scripts
                .iter()
                .copied()
                .filter(|node| dom.tree.is_live(*node) && is_connected(&dom.tree, *node))
                .collect::<Vec<_>>()
        })
    });
    handles.into_iter().map(|node| wrap(scope, node)).collect()
}

pub(super) fn initialize_inert() {
    let handles = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .map(|dom| dom.scripts.keys().copied().collect::<Vec<_>>())
            .unwrap_or_default()
    });
    for handle in handles {
        prepared(handle, false);
    }
}

pub fn script_state(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) -> Option<ScriptState> {
    let _owner = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    if !with_tree(|tree| script(tree, handle)).unwrap_or(false) {
        return None;
    }
    DOM.with(|dom| {
        Some(
            *dom.borrow_mut()
                .as_mut()?
                .scripts
                .entry(handle)
                .or_default(),
        )
    })
}

/// Resolve ownership from native fields even after cross-document adoption.
pub fn script_document<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let _owner = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    let root = with_tree(|tree| script(tree, handle).then(|| tree.root())).flatten()?;
    Some(wrap(scope, root))
}

/// Retirement affects resource execution without changing retained document identity.
pub fn set_document_resource_active(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    active: bool,
) -> bool {
    let _owner = DomScope::for_object(scope, document);
    let Some(handle) = handle_of(scope, document) else {
        return false;
    };
    if with_tree(|tree| tree.node_type_id(handle)) != Some(9) {
        return false;
    }
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let Some(state) = dom.as_mut() else {
            return false;
        };
        state.resource_active = active;
        true
    })
}

fn prepared(handle: Handle, claim_active: bool) -> Option<PreparedScript> {
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let dom = dom.as_mut()?;
        let tree = &dom.tree;
        let mut root = handle;
        while let Some(parent) = tree.parent(root) {
            root = parent;
        }
        let in_shadow_tree = tree.is_shadow_root(root);
        let mut ancestor = tree.parent(handle);
        while let Some(node) = ancestor {
            if tree.local_name(node) == Some("template") {
                return None;
            }
            ancestor = tree.parent(node);
        }
        if !script(tree, handle) {
            return None;
        }
        let state = dom.scripts.entry(handle).or_default();
        if state.already_started {
            return None;
        }
        let active = dom.browsing_context && dom.resource_active;
        if active != claim_active {
            return None;
        }
        let parser_document = state.parser_document.take();
        state.parser_inserted = false;
        if parser_document.is_some() && !tree.has_attribute(handle, "async") {
            state.force_async = true;
        }
        let src = tree.get_attribute(handle, "src").map(str::to_owned);
        let source_units = tree
            .children(handle)
            .into_iter()
            .filter(|node| matches!(tree.node_type_id(*node), 3 | 4))
            .flat_map(|node| tree.node_value_utf16(node).unwrap_or_default())
            .collect::<Vec<_>>();
        let source = String::from_utf16_lossy(&source_units);
        if src.is_none() && source.is_empty() || !is_connected(tree, handle) {
            return None;
        }
        let kind = match tree.get_attribute(handle, "type") {
            Some(value) => value
                .trim_matches(|c| matches!(c, '\t' | '\n' | '\r' | '\x0c' | ' '))
                .to_ascii_lowercase(),
            None => tree
                .get_attribute(handle, "language")
                .filter(|value| !value.is_empty())
                .map_or_else(String::new, |value| {
                    format!("text/{}", value.to_ascii_lowercase())
                }),
        };
        if !matches!(
            kind.as_str(),
            "" | "text/javascript"
                | "application/javascript"
                | "text/ecmascript"
                | "application/ecmascript"
                | "application/x-ecmascript"
                | "application/x-javascript"
                | "text/javascript1.0"
                | "text/javascript1.1"
                | "text/javascript1.2"
                | "text/javascript1.3"
                | "text/javascript1.4"
                | "text/javascript1.5"
                | "text/jscript"
                | "text/livescript"
                | "text/x-ecmascript"
                | "text/x-javascript"
                | "module"
                | "importmap"
                | "speculationrules"
        ) {
            return None;
        }
        if let Some(document) = parser_document {
            state.parser_document = Some(document);
            state.parser_inserted = true;
            state.force_async = false;
        }
        state.already_started = true;
        if !active
            || parser_document.is_some_and(|document| document != DOM_ID.with(std::cell::Cell::get))
        {
            return None;
        }
        let attributes = tree
            .attributes_namespaced(handle)
            .into_iter()
            .filter(|attribute| attribute.namespace.is_none())
            .map(|attribute| {
                (
                    attribute.local_name.to_string(),
                    attribute.value.to_string(),
                )
            })
            .collect();
        let parser_inserted = state.parser_inserted;
        let asynchronous = state.force_async || tree.has_attribute(handle, "async");
        Some(PreparedScript {
            source,
            source_units,
            src,
            kind,
            asynchronous,
            parser_inserted,
            attributes,
            in_shadow_tree,
        })
    })
}

/// Host-only admission. Guest properties, prototypes and expando markers are never read.
pub fn prepare_script(
    scope: &mut v8::PinScope,
    node: v8::Local<v8::Object>,
) -> Option<PreparedScript> {
    let _owner = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    prepared(handle, true)
}

fn script_subtree(tree: &Tree, node: Handle) -> Vec<Handle> {
    let mut scripts = Vec::new();
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if script(tree, node) {
            scripts.push(node);
        }
        if let Some(shadow) = tree.shadow_root(node) {
            pending.push(shadow);
        }
        pending.extend(tree.children(node).into_iter().rev());
    }
    scripts
}
fn subtree(node: Handle) -> Vec<Handle> {
    with_tree(|tree| script_subtree(tree, node)).unwrap_or_default()
}

pub(super) fn parsed(node: Handle, inert: bool) {
    let nodes = subtree(node);
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let Some(dom) = dom.as_mut() else {
            return;
        };
        for node in nodes {
            dom.scripts.insert(
                node,
                ScriptState {
                    already_started: inert,
                    parser_inserted: false,
                    force_async: !inert,
                    parser_document: None,
                },
            );
        }
    });
}

pub(super) fn attribute_changed(node: Handle, name: &str) {
    if name != "async"
        || !with_tree(|tree| script(tree, node) && tree.has_attribute(node, "async"))
            .unwrap_or(false)
    {
        return;
    }
    DOM.with(|dom| {
        if let Some(dom) = dom.borrow_mut().as_mut() {
            dom.scripts.entry(node).or_default().force_async = false;
        }
    });
}

pub(super) fn candidates(event: &MutationEvent) -> Vec<Handle> {
    let mut nodes = Vec::new();
    match &event.record {
        MutationRecord::ChildList { target, added, .. } => {
            if with_tree(|tree| script(tree, *target)).unwrap_or(false) {
                nodes.push(*target);
            }
            for node in added {
                nodes.extend(subtree(*node));
            }
        }
        MutationRecord::Attributes { target, .. } => {
            if event.attribute_namespace.is_none()
                && event.record.attribute_name() == Some("src")
                && with_tree(|tree| script(tree, *target) && tree.has_attribute(*target, "src"))
                    .unwrap_or(false)
            {
                nodes.push(*target);
            }
        }
        MutationRecord::CharacterData { .. } => {}
    }
    nodes
}

pub(super) fn inactive_mutation(event: &MutationEvent) {
    for handle in candidates(event) {
        prepared(handle, false);
    }
}

fn flatten_script_copy(tree: &Tree, node: Handle, deep: bool, out: &mut Vec<Handle>) {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        out.push(node);
        if deep {
            pending.extend(tree.children(node).into_iter().rev());
        }
    }
}

pub(super) fn copy(
    scope: &mut v8::PinScope,
    source: v8::Local<v8::Object>,
    target: v8::Local<v8::Object>,
    deep: bool,
) {
    let values = {
        let _owner = DomScope::for_object(scope, source);
        let Some(handle) = handle_of(scope, source) else {
            return;
        };
        let mut nodes = Vec::new();
        with_tree(|tree| flatten_script_copy(tree, handle, deep, &mut nodes));
        DOM.with(|dom| {
            let dom = dom.borrow();
            let dom = dom.as_ref().unwrap();
            nodes
                .into_iter()
                .map(|node| {
                    if script(&dom.tree, node) {
                        Some(
                            dom.scripts
                                .get(&node)
                                .copied()
                                .unwrap_or_default()
                                .already_started,
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
    };
    let _owner = DomScope::for_object(scope, target);
    let Some(handle) = handle_of(scope, target) else {
        return;
    };
    let mut nodes = Vec::new();
    with_tree(|tree| flatten_script_copy(tree, handle, deep, &mut nodes));
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let dom = dom.as_mut().unwrap();
        for (node, started) in nodes.into_iter().zip(values) {
            if let Some(already_started) = started {
                dom.scripts.insert(
                    node,
                    ScriptState {
                        already_started,
                        parser_inserted: false,
                        force_async: !dom.tree.has_attribute(node, "async"),
                        parser_document: None,
                    },
                );
            }
        }
    });
}

fn async_access(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let _owner = DomScope::for_object(scope, args.this());
    let Some(state) = script_state(scope, args.this()) else {
        let message = v8::String::new(scope, "Illegal invocation").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    };
    let handle = handle_of(scope, args.this()).unwrap();
    if args.data().is_true() {
        DOM.with(|dom| {
            dom.borrow_mut()
                .as_mut()
                .unwrap()
                .scripts
                .entry(handle)
                .or_default()
                .force_async = false
        });
        let enabled = args.get(0).boolean_value(scope);
        with_tree_mut(|tree| {
            if enabled {
                tree.set_attribute(handle, "async", "");
            } else {
                tree.remove_attribute(handle, "async");
            }
        });
    } else {
        rv.set_bool(
            state.force_async
                || with_tree(|tree| tree.has_attribute(handle, "async")).unwrap_or(false),
        );
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    let ctor_name = v8::String::new(scope, "HTMLScriptElement").unwrap();
    let Some(constructor) = global
        .get(scope, ctor_name.into())
        .and_then(|value| value.to_object(scope))
    else {
        return;
    };
    let prototype_name = v8::String::new(scope, "prototype").unwrap();
    let Some(prototype) = constructor
        .get(scope, prototype_name.into())
        .and_then(|value| value.to_object(scope))
    else {
        return;
    };
    let mut functions = Vec::new();
    for setter in [false, true] {
        let data = v8::Boolean::new(scope, setter);
        let template = v8::FunctionTemplate::builder(async_access)
            .data(data.into())
            .length(i32::from(setter))
            .constructor_behavior(v8::ConstructorBehavior::Throw)
            .build(scope);
        template.remove_prototype();
        let name = v8::String::new(scope, if setter { "set async" } else { "get async" }).unwrap();
        template.set_class_name(name);
        functions.push(template.get_function(scope).unwrap());
    }
    let descriptor =
        v8::PropertyDescriptor::new_from_get_set(functions[0].into(), functions[1].into());
    let mut descriptor = descriptor;
    descriptor.set_enumerable(true);
    descriptor.set_configurable(true);
    let name = v8::String::new(scope, "async").unwrap();
    prototype.define_property(scope, name.into(), &descriptor);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_inert(
        scope: &mut v8::PinScope,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) {
        let html = args.get(0).to_rust_string_lossy(scope);
        rv.set(parse_html_document(scope, &html, false).into());
    }

    fn host<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        args: v8::FunctionCallbackArguments<'s>,
        mut rv: v8::ReturnValue,
    ) {
        let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
            rv.set_bool(false);
            return;
        };
        match args.get(0).to_rust_string_lossy(scope).as_str() {
            "state" => {
                if let Some(state) = script_state(scope, node) {
                    let value = format!(
                        "{},{},{}",
                        state.already_started, state.parser_inserted, state.force_async
                    );
                    rv.set(v8::String::new(scope, &value).unwrap().into());
                } else {
                    rv.set(v8::null(scope).into());
                }
            }
            "owner" => {
                if let Some(document) = script_document(scope, node) {
                    rv.set(document.into());
                }
            }
            "import" => {
                let document = wrap(scope, with_tree(Tree::root).unwrap());
                if let Ok(clone) = import_document_node(scope, document, node, true) {
                    rv.set(clone.into());
                }
            }
            "claim" => {
                rv.set_bool(prepare_script(scope, node).is_some());
            }
            "retire" => {
                rv.set_bool(set_document_resource_active(scope, node, false));
            }
            _ => {}
        }
    }

    fn consumer(
        scope: &mut v8::PinScope,
        args: v8::FunctionCallbackArguments,
        _rv: v8::ReturnValue,
    ) {
        let event = args.get(0).to_object(scope).unwrap();
        let name = v8::String::new(scope, "target").unwrap();
        let target = event
            .get(scope, name.into())
            .unwrap()
            .to_object(scope)
            .unwrap();
        let _owner = DomScope::for_object(scope, target);
        let name = v8::String::new(scope, "scriptCandidates").unwrap();
        let nodes = event
            .get(scope, name.into())
            .unwrap()
            .to_object(scope)
            .unwrap();
        let length = v8::String::new(scope, "length").unwrap();
        let count = nodes
            .get(scope, length.into())
            .unwrap()
            .uint32_value(scope)
            .unwrap();
        for index in 0..count {
            let node = nodes
                .get_index(scope, index)
                .unwrap()
                .to_object(scope)
                .unwrap();
            let handle = handle_of(scope, node).unwrap();
            let node = wrap(scope, handle);
            if let Some(script) = prepare_script(scope, node) {
                if script.src.is_none()
                    && !matches!(
                        script.kind.as_str(),
                        "module" | "importmap" | "speculationrules"
                    )
                {
                    let source = v8::String::new(scope, &script.source).unwrap();
                    if let Some(script) = v8::Script::compile(scope, source, None) {
                        script.run(scope);
                    }
                }
            }
        }
    }

    fn check(source: &str, load: bool) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><head><script id=parser>0</script><template><script id=template-script>0</script></template></head><body></body>");
        let global = context.global(scope);
        bind_method(scope, global, "hostScript", host);
        bind_method(scope, global, "parseInert", parse_inert);
        if load {
            let root = with_tree(Tree::root).unwrap();
            let document = wrap(scope, root);
            let consumer = v8::Function::new(scope, consumer).unwrap();
            assert!(install_mutation_consumer(scope, document, consumer));
            // Initial parser scripts are executed by the embedder's parser lifecycle.
            for handle in subtree(root) {
                let node = wrap(scope, handle);
                prepare_script(scope, node);
            }
        }
        v8::tc_scope!(let scope, scope);
        let source = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        assert!(
            result.is_some_and(|value| value.is_true()),
            "script contract: {:?}",
            scope
                .exception()
                .map(|error| error.to_rust_string_lossy(scope))
        );
        reset();
    }

    #[test]
    fn script_async_reflection_distinguishes_parser_force_async_and_attribute_mutations() {
        check(
            r#"(()=>{
          const s=document.createElement('script'),get=Object.getOwnPropertyDescriptor(HTMLScriptElement.prototype,'async').get;
          if(!s.async||s.hasAttribute('async'))throw Error('dynamic default');
          s.removeAttribute('async');if(!s.async)throw Error('absent removal');
          s.async=false;if(s.async||s.hasAttribute('async'))throw Error('IDL false');
          s.setAttribute('async','');if(!s.async)throw Error('attribute true');s.removeAttribute('async');if(s.async)throw Error('force async cleared');
          const a=document.createElement('script');a.toggleAttribute('async',true);a.toggleAttribute('async',false);if(a.async)throw Error('toggle');
          const parser=document.getElementById('parser');if(parser.async||!parser.cloneNode(true).async)throw Error('parser clone');
          const inert=parseInert('<script>0</script>').querySelector('script');if(inert.async||!inert.cloneNode(true).async)throw Error('inert parser');
          for(const html of ['<script></script>','<script type=application/json>0</script>'])if(!parseInert(html).querySelector('script').async)throw Error('failed parser preparation');
          for(const invalid of [document.createElement('div'),Object.create(HTMLScriptElement.prototype)]){let threw=false;try{get.call(invalid)}catch(e){threw=e instanceof TypeError}if(!threw)throw Error('brand')}
          return true;
        })()"#,
            false,
        );
    }

    #[test]
    fn native_preparation_claims_real_identity_and_survives_clones_adoption_and_guest_markers() {
        check(
            r#"(()=>{
          const s=document.createElement('script');s.textContent='0';s.__tcDone=true;document.head.append(s);
          Object.defineProperties(s,{src:{get(){throw Error('guest src')}},type:{get(){throw Error('guest type')}},textContent:{get(){throw Error('guest text')}}});
          if(!hostScript('claim',s)||hostScript('claim',s))throw Error('native claim');
          const c=s.cloneNode(true);document.head.append(c);if(hostScript('claim',c))throw Error('started clone');
          const f=document.implementation.createHTMLDocument('inert'),u=f.createElement('script');u.textContent='0';document.head.append(u);if(u.ownerDocument!==document||!hostScript('claim',u))throw Error('detached inert adoption');Object.defineProperty(u,'ownerDocument',{get(){throw Error('guest owner')}});if(hostScript('owner',u)!==document)throw Error('native adopted owner');
          const v=f.createElement('script');v.textContent='0';f.head.append(v);document.head.append(v);if(hostScript('claim',v))throw Error('connected inert adoption');
          const empty=document.createElement('script');document.head.append(empty);if(hostScript('claim',empty))throw Error('empty');empty.textContent='0';if(!hostScript('claim',empty))throw Error('filled empty');
          hostScript('retire',document);const retired=document.createElement('script');retired.textContent='0';document.head.append(retired);if(hostScript('claim',retired)||hostScript('state',retired)!=='true,false,true')throw Error('retired preparation');
          return true;
        })()"#,
            false,
        );
    }

    #[test]
    fn parsed_fragment_scripts_are_inert_but_contextual_fragment_scripts_prepare() {
        check(
            r#"(()=>{
          const box=document.createElement('div');box.innerHTML='<script>0</script>';const s=box.firstChild;document.head.append(s);
          if(hostScript('claim',s)||hostScript('state',s)!=='true,false,false')throw Error('innerHTML');
          const range=document.createRange(),fragment=range.createContextualFragment('<script>0</script>'),active=fragment.firstChild;
          if(!active.async||hostScript('state',active)!=='false,false,true')throw Error('context reset');document.head.append(fragment);if(!hostScript('claim',active))throw Error('context claim');
          const p=parseInert('<script>0</script>').querySelector('script'), imported=hostScript('import',p);document.head.append(imported);if(hostScript('claim',imported))throw Error('inert imported');
          box.insertAdjacentHTML('beforeend','<script>0</script>');const adjacent=box.lastChild;document.head.append(adjacent);if(hostScript('claim',adjacent))throw Error('adjacent');
          const old=document.createElement('span');box.append(old);old.outerHTML='<script>0</script>';const outer=box.lastChild;document.head.append(outer);if(hostScript('claim',outer))throw Error('outer');
          return true;
        })()"#,
            false,
        );
    }

    #[test]
    fn preparation_mime_admission_matches_the_authored_chrome_control() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("script_preparation_chrome155.json")).unwrap();
        let rows = &fixture["result"]["mime"];
        check(
            &format!(
                r#"(()=>{{const rows={rows};for(const [type,expected] of rows){{const node=document.createElement('script');node.type=type;node.textContent='0';document.head.append(node);if(hostScript('claim',node)!==expected)throw Error(type);node.remove()}}return true}})()"#
            ),
            false,
        );
    }

    #[test]
    fn legacy_language_admission_matches_the_authored_chrome_control() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("script_preparation_chrome155.json")).unwrap();
        let rows = &fixture["result"]["languages"];
        check(
            &format!(
                r#"(()=>{{const rows={rows};for(const [language,expected] of rows){{const node=document.createElement('script');node.setAttribute('language',language);node.textContent='0';document.head.append(node);if(hostScript('claim',node)!==expected)throw Error(language);node.remove()}}return true}})()"#
            ),
            false,
        );
    }

    #[test]
    fn template_script_preparation_matches_independent_chrome_control() {
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("template_lifecycle_chrome155.json")).unwrap();
        check(
            &format!(
                r#"(()=>{{
          globalThis.__templateScriptRuns=[];
          const scripted=document.createElement('template');scripted.innerHTML='<script>__templateScriptRuns.push("inner")</script>';document.body.append(scripted.content);
          const cloned=document.createElement('template');cloned.innerHTML='<script>__templateScriptRuns.push("clone")</script>';document.body.append(cloned.content.cloneNode(true));
          const contextual=document.createRange().createContextualFragment('<template><script>__templateScriptRuns.push("contextual")</script></template>');document.body.append(contextual.firstChild.content);
          const parser=parseInert('<template><script>__templateScriptRuns.push("parser")</script></template>');document.body.append(parser.querySelector('template').content);
          if(JSON.stringify(__templateScriptRuns)!==JSON.stringify({}))throw Error(JSON.stringify(__templateScriptRuns));return true;
        }})()"#,
                oracle["result"]["scriptRuns"]
            ),
            true,
        );
    }

    #[test]
    fn script_text_reflection_matches_independent_chrome_source_control() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("script_text_chrome155.json")).unwrap();
        check(&format!("(()=>{{const actual={},expected={};const sorted=value=>Array.isArray(value)?value.map(sorted):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(key=>[key,sorted(value[key])])):value;if(JSON.stringify(sorted(actual))!==JSON.stringify(sorted(expected)))throw Error(JSON.stringify(actual));return true}})()", include_str!("script_text_probe.js"), fixture["result"]), false);
    }

    #[test]
    fn script_text_setter_prepares_actual_native_source_and_preserves_failure_atomicity() {
        check(
            r#"(()=>{
          globalThis.signals=[];const s=document.createElement('script');s.text='signals.push("text")';document.head.append(s);if(signals.join(',')!=='text')throw Error('text source not executed');
          const empty=document.createElement('script');document.head.append(empty);empty.text='signals.push("connected")';if(signals.join(',')!=='text,connected')throw Error('connected setter did not prepare');
          const range=document.createRange();range.selectNodeContents(s.firstChild);let mutations=[];const observer=new MutationObserver(records=>mutations.push(...records));observer.observe(s,{childList:true});
          const old=s.firstChild;try{s.text=Symbol()}catch(e){if(e.name!=='TypeError')throw e}if(s.firstChild!==old||range.toString()!==old.data||observer.takeRecords().length)throw Error('failed conversion changed native tree');
          s.text='replacement';const record=observer.takeRecords();if(record.length!==1||record[0].removedNodes[0]!==old||record[0].addedNodes[0]!==s.firstChild||range.startContainer!==s||range.startOffset!==0||range.endOffset!==0)throw Error('native range or observer seam');
          if(signals.join(',')!=='text,connected')throw Error('already-started script re-executed');
          return true;
        })()"#,
            true,
        );
    }

    #[test]
    fn document_private_consumer_runs_inline_script_postconnection_synchronously() {
        check(
            r#"(()=>{
          globalThis.signals=[];const s=document.createElement('script');s.textContent='signals.push("single")';document.head.append(s);if(signals.join(',')!=='single')throw Error('not synchronous');
          s.remove();document.head.append(s);document.head.append(s.cloneNode(true));if(signals.length!==1)throw Error('repeated');
          const empty=document.createElement('script');document.head.append(empty);empty.appendChild(document.createTextNode('signals.push("filled")'));if(signals.join(',')!=='single,filled')throw Error('text addition');
          const inert=document.implementation.createHTMLDocument(''),x=inert.createElement('script');x.textContent='signals.push("wrong")';inert.head.append(x);document.head.append(x);if(signals.length!==2)throw Error('inert');
          const shadowHost=document.createElement('div'),shadow=shadowHost.attachShadow({mode:'open'}),shadowScript=document.createElement('script');shadowScript.textContent='signals.push("shadow")';shadow.append(shadowScript);if(signals.length!==2)throw Error('detached shadow');document.body.append(shadowHost);if(signals.join(',')!=='single,filled,shadow')throw Error('shadow connection');signals.pop();
          const outer=document.createElement('script'),inner=document.createElement('script');inner.textContent='signals.push("inner")';document.head.append(outer);outer.append(document.createTextNode('signals.push("outer-first");'),inner,document.createTextNode('signals.push("outer-last")'));if(signals.slice(-3).join(',')!=='outer-first,outer-last,inner')throw Error('nested direct text and order');signals.splice(-3);
          const data=document.createElement('script');data.type='application/json';data.textContent='signals.push("changed")';document.head.append(data);data.type='text/javascript';data.firstChild.data+='\n';if(signals.length!==2)throw Error('wrong preparation trigger');data.appendChild(document.createTextNode(''));if(signals.join(',')!=='single,filled,changed')throw Error('child trigger:'+JSON.stringify(signals)+':'+hostScript('state',data)+':'+data.childNodes.length);
          return true;
        })()"#,
            true,
        );
    }
}

#[cfg(test)]
mod depth_tests {
    use super::*;
    #[test]
    fn deep_script_preparation_scans_preserve_template_flags_and_order() {
        std::thread::Builder::new().stack_size(512*1024).spawn(|| {
            let source=format!("{}<script>A</script><template><script>B</script></template><script>C</script>{}","<div>".repeat(8192),"</div>".repeat(8192));
            let tree=Tree::parse(&source);
            let ordered=script_subtree(&tree,tree.root());
            assert_eq!(ordered.iter().map(|node| tree.text_content(*node)).collect::<Vec<_>>(),["A","B","C"]);
            let states=initial(&tree);
            assert!(states[&ordered[0]].parser_inserted);
            assert!(!states[&ordered[1]].parser_inserted);
            assert!(states[&ordered[2]].parser_inserted);
            let mut all=Vec::new();flatten_script_copy(&tree,tree.root(),true,&mut all);
            assert_eq!(all.into_iter().filter(|node| script(&tree,*node)).collect::<Vec<_>>(),ordered);
            let mut shallow=Vec::new();flatten_script_copy(&tree,tree.root(),false,&mut shallow);
            assert_eq!(shallow,vec![tree.root()]);
        }).unwrap().join().unwrap();
    }
}

fn text_type_error(scope: &mut v8::PinScope) {
    if let Some(message) = v8::String::new(scope, "Illegal invocation") {
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
    }
}
fn text<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = handle_of(scope, args.this()) else {
        return text_type_error(scope);
    };
    if !with_tree(|tree| script(tree, node)).unwrap_or(false) {
        return text_type_error(scope);
    }
    if args.length() > 0 {
        let Some(units) =
            trusted_types::script_units(scope, args.this(), args.get(0), "text", false)
        else {
            return;
        };
        html_popover::disconnect(scope, node, false);
        range_live::replacing_children(scope, node);
        with_tree_mut(|tree| tree.set_text_content_utf16(node, &units));
        return;
    }
    let units = with_tree(|tree| {
        tree.children(node)
            .into_iter()
            .filter(|child| matches!(tree.node_type_id(*child), 3 | 4))
            .flat_map(|child| tree.node_value_utf16(child).unwrap_or_default())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    if let Some(value) = v8::String::new_from_two_byte(scope, &units, v8::NewStringType::Normal) {
        result.set(value.into());
    }
}
pub(super) fn install_text(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__scriptTextNative", text);
    run_js(scope, include_str!("script_text.js"));
}

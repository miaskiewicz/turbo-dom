//! Native flat-tree facts for CSS renderedness; no style or guest getter reads.
use super::shadow_slots::ComposedParent;
use super::*;

const MAX_SAFE_VERSION: u64 = 9_007_199_254_740_991;
struct Projection {
    version: u64,
    root: Handle,
    nodes: Option<Vec<Handle>>,
}
enum Step<'s> {
    Root,
    Parent(Handle, v8::Local<'s, v8::Object>),
    NotInFlatTree,
}
fn step<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    root: Handle,
    owner: u64,
) -> Option<Step<'s>> {
    if scope.is_execution_terminating() || dom_id_of(scope, node) != owner {
        return None;
    }
    let handle = handle_of(scope, node)?;
    let kind =
        with_tree(|tree| tree.is_live(handle).then(|| tree.node_type_id(handle))).flatten()?;
    if handle == root {
        return (kind == 9).then_some(Step::Root);
    }
    if kind != 1 {
        return None;
    }
    match shadow_slots::composed_parent_status(scope, node) {
        ComposedParent::Parent(parent) => Some(Step::Parent(handle, parent)),
        ComposedParent::NotInFlatTree => Some(Step::NotInFlatTree),
        ComposedParent::Unavailable => None,
    }
}
fn project<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    element: v8::Local<'s, v8::Object>,
) -> Option<Projection> {
    let _owner = DomScope::for_object(scope, element);
    let owner = dom_id_of(scope, element);
    let (root, count, version) = with_tree(|tree| {
        (tree.is_live(tree.root()) && tree.node_type_id(tree.root()) == 9)
            .then(|| (tree.root(), tree.node_count(), tree.version))
    })
    .flatten()?;
    let mut nodes = Vec::new();
    let mut current = element;
    for _ in 0..count {
        match step(scope, current, root, owner)? {
            Step::Root => {
                return Some(Projection {
                    version,
                    root,
                    nodes: Some(nodes),
                })
            }
            Step::NotInFlatTree => {
                return Some(Projection {
                    version,
                    root,
                    nodes: None,
                })
            }
            Step::Parent(handle, parent) => {
                nodes.push(handle);
                current = parent;
            }
        }
    }
    None
}
fn current(scope: &mut v8::PinScope, element: v8::Local<v8::Object>, version: u64) -> bool {
    if scope.is_execution_terminating() {
        return false;
    }
    let _owner = DomScope::for_object(scope, element);
    let Some(handle) = handle_of(scope, element) else {
        return false;
    };
    with_tree(|tree| tree.is_live(handle) && tree.version == version).unwrap_or(false)
}
pub(super) fn path<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    rv: &mut v8::ReturnValue,
) {
    rv.set(v8::null(scope).into());
    if scope.is_execution_terminating() {
        return;
    }
    let Ok(element) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let Some(projection) = project(scope, element) else {
        return;
    };
    if projection.version > MAX_SAFE_VERSION || !current(scope, element, projection.version) {
        return;
    }
    let _owner = DomScope::for_object(scope, element);
    let mut values = vec![
        v8::Number::new(scope, projection.version as f64).into(),
        v8::Boolean::new(scope, projection.nodes.is_some()).into(),
        wrap(scope, projection.root).into(),
    ];
    if let Some(nodes) = projection.nodes {
        for node in nodes {
            values.push(wrap(scope, node).into());
        }
    }
    if !scope.is_execution_terminating() {
        rv.set(v8::Array::new_with_elements(scope, &values).into());
    }
}
fn data<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    array: v8::Local<'s, v8::Array>,
    index: u32,
) -> Option<v8::Local<'s, v8::Value>> {
    if scope.is_execution_terminating() {
        return None;
    }
    let key = v8::String::new(scope, &index.to_string())?;
    let descriptor = array.get_own_property_descriptor(scope, key.into())?;
    let descriptor = v8::Local::<v8::Object>::try_from(descriptor).ok()?;
    let value = v8::String::new(scope, "value")?;
    if !descriptor.has_own_property(scope, value.into())? {
        return None;
    }
    descriptor.get(scope, value.into())
}
fn same_node<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<v8::Value>,
    handle: Handle,
) -> bool {
    value.strict_equals(wrap(scope, handle).into())
}
fn metadata_matches<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    array: v8::Local<'s, v8::Array>,
    projection: &Projection,
) -> Option<bool> {
    let version = v8::Local::<v8::Number>::try_from(data(scope, array, 0)?)
        .ok()?
        .value();
    let status = data(scope, array, 1)?;
    if version != projection.version as f64
        || !status.is_boolean()
        || status.is_true() != projection.nodes.is_some()
    {
        return Some(false);
    }
    let root = data(scope, array, 2)?;
    Some(same_node(scope, root, projection.root))
}
fn nodes_match<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    array: v8::Local<'s, v8::Array>,
    projection: &Projection,
) -> Option<bool> {
    if let Some(nodes) = &projection.nodes {
        for (index, node) in nodes.iter().enumerate() {
            let value = data(scope, array, index as u32 + 3)?;
            if !same_node(scope, value, *node) {
                return Some(false);
            }
        }
    }
    Some(true)
}
fn matches<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    array: v8::Local<'s, v8::Array>,
    projection: &Projection,
) -> Option<bool> {
    let count = projection.nodes.as_ref().map_or(0, Vec::len);
    if projection.version > MAX_SAFE_VERSION || array.length() as usize != count + 3 {
        return Some(false);
    }
    if !metadata_matches(scope, array, projection)? {
        return Some(false);
    }
    nodes_match(scope, array, projection)
}
pub(super) fn validate<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    rv: &mut v8::ReturnValue,
) {
    rv.set_bool(false);
    let Ok(element) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let value = args.get(2);
    if value.is_proxy() {
        return;
    }
    let Ok(array) = v8::Local::<v8::Array>::try_from(value) else {
        return;
    };
    let Some(projection) = project(scope, element) else {
        return;
    };
    let _owner = DomScope::for_object(scope, element);
    let matched = matches(scope, array, &projection).unwrap_or(false);
    rv.set_bool(matched && current(scope, element, projection.version));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
        let source = v8::String::new(scope, source).unwrap();
        v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
    }
    fn path_callback<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        args: v8::FunctionCallbackArguments<'s>,
        mut rv: v8::ReturnValue,
    ) {
        path(scope, args, &mut rv);
    }
    fn validate_callback<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        args: v8::FunctionCallbackArguments<'s>,
        mut rv: v8::ReturnValue,
    ) {
        validate(scope, args, &mut rv);
    }
    fn with_dom(test: impl FnOnce(&mut v8::PinScope)) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, "<!doctype html><div id=target></div>");
        let path = v8::Function::new(scope, path_callback).unwrap();
        let validate = v8::Function::new(scope, validate_callback).unwrap();
        let key = v8::String::new(scope, "__path").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), path.into());
        let key = v8::String::new(scope, "__validate").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), validate.into());
        eval(scope, "globalThis.target=document.getElementById('target')");
        let document = v8::Local::<v8::Object>::try_from(eval(scope, "document")).unwrap();
        let owner = DomScope::for_object(scope, document);
        test(scope);
        drop(owner);
        reset();
    }
    #[test]
    fn private_rendering_evidence_does_not_coerce_or_invoke_accessor_and_proxy_traps() {
        with_dom(|scope| {
            assert!(eval(scope, "globalThis.p=__path('renderingPath',target);__validate('renderingPathCurrent',target,p)").is_true());
            for mutation in ["Object.defineProperty(p,'0',{get(){traps++;throw Error('getter')}})","p[0]={valueOf(){traps++;throw Error('coercion')}}","p[2]=new Proxy(document,{get(){traps++;throw Error('root proxy')}})","p=new Proxy(p,{getOwnPropertyDescriptor(){traps++;throw Error('descriptor proxy')}})"]{
                eval(scope,&format!("globalThis.traps=0;p=__path('renderingPath',target);{mutation}"));
                assert!(eval(scope,"__validate('renderingPathCurrent',target,p)===false&&traps===0").is_true());
            }
            assert!(eval(scope,"__path('renderingPath',new Proxy(target,{get(){traps++;throw Error('target proxy')}}))===null&&traps===0").is_true());
        });
    }
    #[test]
    fn rendering_evidence_rejects_invalid_receivers_and_metadata() {
        with_dom(|scope| {
            for receiver in ["null", "0", "document.createTextNode('x')"] {
                assert!(
                    eval(scope, &format!("__path('renderingPath',{receiver})===null")).is_true()
                );
                assert!(eval(
                    scope,
                    &format!("__validate('renderingPathCurrent',{receiver},[])===false")
                )
                .is_true());
            }
            for evidence in ["null", "{}", "[]", "[0,true,document]"] {
                assert!(eval(
                    scope,
                    &format!("__validate('renderingPathCurrent',target,{evidence})===false")
                )
                .is_true());
            }
            eval(
                scope,
                "globalThis.p=__path('renderingPath',target);p[1]='true'",
            );
            assert!(eval(scope, "__validate('renderingPathCurrent',target,p)===false").is_true());
        });
    }

    #[test]
    fn rendering_evidence_reprojects_private_slot_status_and_owner() {
        with_dom(|scope| {
            eval(scope,"globalThis.host=document.createElement('div');document.body.appendChild(host);globalThis.root=host.attachShadow({mode:'closed',slotAssignment:'manual'});root.innerHTML='<slot id=a></slot><slot id=b></slot>';globalThis.a=root.querySelector('#a');globalThis.b=root.querySelector('#b');host.appendChild(target);a.assign(target);globalThis.p=__path('renderingPath',target)");
            let version = with_tree(|tree| tree.version).unwrap();
            eval(scope, "b.assign(target)");
            assert_eq!(with_tree(|tree| tree.version), Some(version));
            assert!(eval(scope, "__validate('renderingPathCurrent',target,p)===false").is_true());
            assert!(eval(
                scope,
                "p=__path('renderingPath',target);__validate('renderingPathCurrent',target,p)"
            )
            .is_true());
            eval(
                scope,
                "b.assign();p=__path('renderingPath',target);a.assign(target)",
            );
            assert_eq!(with_tree(|tree| tree.version), Some(version));
            assert!(eval(scope, "__validate('renderingPathCurrent',target,p)===false").is_true());
            eval(scope,"p=__path('renderingPath',target);globalThis.other=document.implementation.createHTMLDocument('other');other.body.appendChild(target)");
            assert!(eval(scope, "__validate('renderingPathCurrent',target,p)===false").is_true());
        });
    }
    #[test]
    fn terminating_rendering_evidence_keeps_native_termination_intact() {
        with_dom(|scope| {
            let target = v8::Local::<v8::Object>::try_from(eval(scope, "target")).unwrap();
            assert!(project(scope, target).is_some());
            let source = v8::String::new(scope, "1").unwrap();
            let script = v8::Script::compile(scope, source, None).unwrap();
            scope.terminate_execution();
            {
                v8::tc_scope!(let catch,scope);
                assert!(script.run(catch).is_none());
                assert!(catch.has_terminated());
                assert!(project(catch, target).is_none());
                assert!(catch.is_execution_terminating());
                catch.cancel_terminate_execution();
                catch.reset();
            }
            assert!(project(scope, target).is_some());
        });
    }
}

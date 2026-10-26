//! Bounded public-rtdom snapshots for the interruptible guest-isolate XPath provider.
use super::*;
use serde_json::json;

const NODE_LIMIT: usize = 4096;
const DEPTH_LIMIT: usize = 128;
const STRING_LIMIT: usize = 1024 * 1024;

fn record_key<'s>(scope: &mut v8::PinScope<'s, '_>, family: &str) -> v8::Local<'s, v8::Private> {
    let name = v8::String::new(scope, &format!("turbo-dom.XPath.{family}")).unwrap();
    v8::Private::for_api(scope, Some(name))
}
/// Genuine non-cloneable XPath platform objects, including objects from other realms.
pub fn is_native_xpath_object(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> bool {
    ["expression", "result", "evaluator"]
        .into_iter()
        .any(|family| {
            let key = record_key(scope, family);
            object
                .get_private(scope, key)
                .is_some_and(|record| record.is_object())
        })
}

struct Snapshot {
    nodes: Vec<Handle>,
    rows: Vec<serde_json::Value>,
    context: usize,
    stamp: String,
}
fn snapshot(tree: &mut Tree, context: Handle) -> Result<Snapshot, &'static str> {
    let mut root = tree.attribute_owner(context).unwrap_or(context);
    for depth in 0..=DEPTH_LIMIT {
        if tree.shadow_host(root).is_some() || tree.node_type_id(root) == 11 {
            return Err("XPath fragment/shadow-root contexts are unsupported");
        }
        match tree.parent(root) {
            Some(parent) if depth < DEPTH_LIMIT => root = parent,
            Some(_) => return Err("XPath context depth exceeds its budget"),
            None => break,
        }
    }
    let mut stack = vec![(tree.root(), None, 0usize)];
    if root != tree.root() {
        stack.push((root, None, 0));
    }
    let mut nodes = Vec::new();
    let mut rows = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut bytes = 0usize;
    while let Some((node, parent, depth)) = stack.pop() {
        if !visited.insert(node) || depth > DEPTH_LIMIT || nodes.len() >= NODE_LIMIT {
            return Err("XPath tree exceeds its node/depth budget");
        }
        let kind = tree.node_type_id(node);
        if kind == 10 {
            continue;
        }
        let index = nodes.len();
        nodes.push(node);
        rows.push(serde_json::Value::Null);
        let name = (if kind == 7 {
            tree.processing_instruction_target(node)
        } else {
            tree.qualified_name(node)
        })
        .unwrap_or(match kind {
            9 => "#document",
            3 => "#text",
            8 => "#comment",
            _ => "",
        })
        .to_owned();
        let local = tree.local_name(node).unwrap_or("").to_owned();
        let namespace = tree.namespace_uri(node).map(str::to_owned);
        let prefix = tree.prefix(node).map(str::to_owned);
        let value = tree.node_value(node).unwrap_or_default();
        bytes += name.len() + local.len() + namespace.as_ref().map_or(0, String::len) + value.len();
        if bytes > STRING_LIMIT {
            return Err("XPath text exceeds its byte budget");
        }
        let mut attrs = Vec::new();
        for attribute in tree.attributes_namespaced(node) {
            if nodes.len() >= NODE_LIMIT {
                return Err("XPath attributes exceed the node budget");
            }
            // Namespace declarations are resolved separately, not XPath attributes.
            if attribute.namespace.as_deref() == Some("http://www.w3.org/2000/xmlns/") {
                continue;
            }
            bytes += attribute.value.len()
                + attribute.qualified_name.len()
                + attribute.local_name.len()
                + attribute.namespace.as_ref().map_or(0, |value| value.len())
                + attribute.prefix.as_ref().map_or(0, |value| value.len());
            if bytes > STRING_LIMIT {
                return Err("XPath attributes exceed the byte budget");
            }
            let handle = tree
                .attribute_node(node, attribute.namespace.as_deref(), &attribute.local_name)
                .ok_or("XPath attribute identity is unavailable")?;
            attrs.push(nodes.len());
            nodes.push(handle);
            rows.push(json!([
                2,
                attribute.qualified_name.to_string(),
                attribute.local_name.to_string(),
                attribute.namespace.as_ref().map(ToString::to_string),
                attribute.prefix.as_ref().map(ToString::to_string),
                attribute.value.to_string(),
                null,
                [],
                index
            ]));
        }
        rows[index] = json!([kind, name, local, namespace, prefix, value, parent, attrs, null]);
        let children = tree.children(node);
        if nodes.len() + stack.len() + children.len() > NODE_LIMIT {
            return Err("XPath tree exceeds its node budget");
        }
        for child in children.into_iter().rev() {
            stack.push((child, Some(index), depth + 1));
        }
    }
    let context = nodes
        .iter()
        .position(|&node| node == context)
        .ok_or("XPath context is outside the supported tree")?;
    Ok(Snapshot {
        nodes,
        rows,
        context,
        stamp: format!("{}:{}", DOM_ID.with(|id| id.get()), tree.version),
    })
}
fn dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let operation = args.get(0).to_rust_string_lossy(scope);
    for family in ["expression", "result", "evaluator"] {
        if operation == format!("{family}Get") || operation == format!("{family}Set") {
            let key = record_key(scope, family);
            if operation.ends_with("Set") {
                object.set_private(scope, key, args.get(2));
            }
            if let Some(value) = object.get_private(scope, key) {
                rv.set(value);
            }
            return;
        }
    }
    if operation == "interfaces" || operation == "registerInterfaces" {
        let owner = dom_id_of(scope, object);
        let context = DOM_CONTEXTS.with(|contexts| contexts.borrow().get(&owner).cloned());
        if let Some(context) = context {
            let context = dom_ownership::local(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            let global = context.global(scope);
            let key = record_key(scope, "interfaces");
            if operation == "registerInterfaces" {
                global.set_private(scope, key, args.get(2));
            }
            if let Some(value) = global.get_private(scope, key) {
                rv.set(value);
            }
        }
        return;
    }
    let _dom = DomScope::for_object(scope, object);
    let Some(node) = handle_of(scope, object) else {
        return;
    };
    if operation == "stamp" {
        let stamp = with_tree(|tree| format!("{}:{}", DOM_ID.with(|id| id.get()), tree.version))
            .unwrap_or_default();
        rv.set(v8::String::new(scope, &stamp).unwrap().into());
        return;
    }
    let Some(result) = with_tree_mut(|tree| snapshot(tree, node)) else {
        return;
    };
    let snapshot = match result {
        Ok(snapshot) => snapshot,
        Err(message) => {
            throw_dom_error(scope, "NotSupportedError", message);
            return;
        }
    };
    let data =
        json!({"rows":snapshot.rows,"context":snapshot.context,"stamp":snapshot.stamp}).to_string();
    let data = v8::String::new(scope, &data).unwrap();
    let Some(result) = v8::json::parse(scope, data).and_then(|value| value.to_object(scope)) else {
        return;
    };
    let nodes = v8::Array::new(scope, snapshot.nodes.len() as i32);
    for (index, node) in snapshot.nodes.into_iter().enumerate() {
        let node = wrap(scope, node);
        nodes.set_index(scope, index as u32, node.into());
    }
    set_prop(scope, result, "nodes", nodes.into());
    rv.set(result.into());
}
pub(super) fn install(scope: &mut v8::PinScope) {
    // Keep provider intrinsics and snapshots separate from mutable page builtins.
    // Only native facade callables are exported; this context is never page-visible.
    let public = scope.get_current_context();
    let target = public.global(scope);
    let context = v8::Context::new(scope, Default::default());
    context.set_security_token(public.get_security_token(scope));
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    set_prop(scope, global, "__xpathTarget", target.into());
    let name = v8::String::new(scope, "DOMException").unwrap();
    if let Some(constructor) = target.get(scope, name.into()) {
        let key = dom_exception_key(scope);
        global.set_private(scope, key, constructor);
    }
    bind_method(scope, global, "__xpathNative", dispatch);
    run_js(
        scope,
        concat!(
            "(function(){const exports={};const document=undefined;",
            include_str!("xpath_provider/xpath.js"),
            ";globalThis.__xpathProvider=exports;})();"
        ),
    );
    run_js(scope, include_str!("document_xpath.js"));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(source: &str) {
        prepared_fixture(source, |_| {});
    }
    fn prepared_fixture(source: &str, prepare: impl FnOnce(&mut Tree)) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        super::super::install(scope);
        with_tree_mut(prepare).unwrap();
        let parsed = parse_html_document(scope, "<!doctype html><body><section id=scope><p data-key=one>alpha</p><p data-key=two>beta<b>nested</b></p></section><p>outside</p>", false);
        let global = context.global(scope);
        let key = v8::String::new(scope, "inert").unwrap();
        global.set(scope, key.into(), parsed.into());
        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();
        let source = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        if result.is_none() {
            panic!(
                "XPath fixture: {:?}",
                scope.exception().map(|error| {
                    let object = error.to_object(scope).unwrap();
                    let key = v8::String::new(scope, "stack").unwrap();
                    object
                        .get(scope, key.into())
                        .unwrap()
                        .to_rust_string_lossy(scope)
                })
            );
        }
        assert!(result.unwrap().is_true());
        reset();
    }
    #[test]
    fn xpath_matches_authored_chrome_paths_axes_scalars_and_native_attributes() {
        fixture(
            r#"(()=>{
          if(Object.hasOwn(document,'evaluate')||Object.hasOwn(document,'createExpression'))throw new Error('provider escaped private wrapper');const scope=inert.getElementById('scope'),evaluate=(source,node=scope,type=7)=>inert.evaluate(source,node,null,type,null);
          const rows=value=>Array.from({length:value.snapshotLength},(_,index)=>{const node=value.snapshotItem(index);return [node.nodeType,node.nodeName,node.textContent]});
          const expected=[['absolute','/html/body/section/p',inert,[[1,'P','alpha'],[1,'P','betanested']]],['relative','p',scope,[[1,'P','alpha'],[1,'P','betanested']]],['position','p[2]',scope,[[1,'P','betanested']]],['attribute','p/@data-key',scope,[[2,'data-key','one'],[2,'data-key','two']]],['union','p[1] | p[2]/b',scope,[[1,'P','alpha'],[1,'B','nested']]],['axis','p[2]/preceding-sibling::p',scope,[[1,'P','alpha']]],['following','p[1]/following::p',scope,[[1,'P','betanested'],[1,'P','outside']]],['preceding','p[2]/b/preceding::p',scope,[[1,'P','alpha']]]];
          for(const [name,source,node,wanted] of expected){const actual=rows(evaluate(source,node));if(JSON.stringify(actual)!==JSON.stringify(wanted))throw new Error(name+' '+JSON.stringify(actual));}
          if(evaluate('count(p)',scope,1).numberValue!==2||evaluate('string(p[2])',scope,2).stringValue!=='betanested'||!evaluate("boolean(p[@data-key='two'])",scope,3).booleanValue)throw new Error('scalar mismatch');
          const attr=evaluate('p/@data-key').snapshotItem(0),p=scope.firstChild;
          if(attr!==p.getAttributeNode('data-key')||attr.ownerDocument!==inert||attr.ownerElement!==p)throw new Error('attribute identity');
          const text=evaluate('p[1]/text()',scope,9).singleNodeValue;if(text!==p.firstChild)throw new Error('text identity');
          const expression=inert.createExpression('p[1]',null);if(!(expression instanceof XPathExpression)||expression.evaluate(scope,9,null).singleNodeValue!==p)throw new Error('compiled expression');
          return true;
        })()"#,
        );
    }
    #[test]
    fn xpath_snapshots_are_stable_and_iterators_invalidate_on_native_mutations() {
        fixture(
            r#"(()=>{
          const scope=inert.getElementById('scope'),iterator=inert.evaluate('p',scope,null,5),snapshot=inert.evaluate('p',scope,null,7),p=scope.firstChild;
          if(iterator.invalidIteratorState||iterator.iterateNext()!==p)throw new Error('fresh iterator');
          p.setAttribute('data-key','changed');if(!iterator.invalidIteratorState)throw new Error('attribute change did not invalidate');
          let error;try{iterator.iterateNext()}catch(caught){error=caught.name}if(error!=='InvalidStateError')throw new Error('invalid iterator accepted');
          p.remove();if(snapshot.snapshotLength!==2||snapshot.snapshotItem(0)!==p||snapshot.snapshotItem(2)!==null||snapshot.snapshotItem(-1)!==null||snapshot.invalidIteratorState)throw new Error('snapshot changed');
          const second=inert.evaluate('p',scope,null,4);second.iterateNext().firstChild.data='changed';if(!second.invalidIteratorState)throw new Error('character mutation');
          const third=inert.evaluate('p',scope,null,4);document.body.append(document.createElement('div'));if(third.invalidIteratorState)throw new Error('foreign document invalidated iterator');
          const fourth=inert.evaluate('p',scope,null,4);document.adoptNode(scope.firstChild);if(!fourth.invalidIteratorState)throw new Error('adoption did not invalidate source');
          return true;
        })()"#,
        );
    }
    #[test]
    fn xpath_resolves_prefixes_at_compile_time_and_preserves_foreign_case() {
        fixture(
            r#"(()=>{
          const root=inert.body,upper=inert.createElementNS('urn:Case','x:Upper'),lower=inert.createElementNS('urn:Case','x:upper');root.append(upper,lower);upper.setAttributeNS('urn:Attr','a:Key','value');
          let calls=0;const expression=inert.createExpression('x:Upper/@a:Key',prefix=>{calls++;return prefix==='x'?'urn:Case':'urn:Attr'});
          if(calls!==2||expression.evaluate(root,9).singleNodeValue!==upper.getAttributeNodeNS('urn:Attr','Key')||calls!==2)throw new Error('namespace/callback phase');
          if(inert.evaluate('x:upper',root,()=> 'urn:Case',7).snapshotItem(0)!==lower||inert.evaluate('Upper',root,null,7).snapshotLength!==0)throw new Error('foreign namespace/case');
          const htmlPrefix=inert.createExpression('h:section',()=> 'http://www.w3.org/1999/xhtml');if(htmlPrefix.evaluate(root,9).singleNodeValue!==root.firstChild)throw new Error('explicit HTML namespace');
          let error;try{inert.createExpression('unknown:node',null)}catch(caught){error=caught.name}if(error!=='NamespaceError')throw new Error('unresolved namespace');
          return true;
        })()"#,
        );
    }
    #[test]
    fn xpath_enforces_native_brands_result_types_and_explicit_unsupported_cases() {
        fixture(
            r#"(()=>{
          const scope=inert.getElementById('scope'),snap=inert.evaluate('p',scope,null,7),expression=inert.createExpression('p');
          for(const [run,wanted] of [[()=>inert.evaluate('p[',scope,null,7),'SyntaxError'],[()=>inert.evaluate('p',scope,null,42),'TypeError'],[()=>inert.evaluate('count(p)',scope,null,7),'TypeError'],[()=>Document.prototype.evaluate.call({},'p',scope),'TypeError'],[()=>inert.evaluate('p',{nodeType:1}),'TypeError'],[()=>XPathExpression.prototype.evaluate.call({},scope),'TypeError'],[()=>XPathResult.prototype.snapshotItem.call({},0),'TypeError'],[()=>snap.numberValue,'TypeError'],[()=>snap.iterateNext(),'TypeError'],[()=>snap.snapshotItem(),'TypeError'],[()=>new XPathResult(),'TypeError'],[()=>expression.evaluate(inert.createDocumentFragment()),'NotSupportedError'],[()=>inert.createExpression('namespace::*'),'NotSupportedError'],[()=>inert.createExpression('$variable'),'SyntaxError'],[()=>inert.createExpression('unknown()'),'SyntaxError']]){let error;try{run()}catch(caught){error=caught.name}if(error!==wanted)throw new Error(wanted+' got '+error);}
          return true;
        })()"#,
        );
    }
    #[test]
    fn xpath_private_intrinsics_and_exception_brands_survive_page_overrides() {
        fixture(
            r#"(()=>{
          const scope=inert.getElementById('scope');for(const node of inert.querySelectorAll('*')){void node.childNodes;void node.attributes;}
          const html=inert.documentElement.outerHTML,originalCall=Function.prototype.call;
          Array.prototype.map=()=>{throw new Error('page Array.map')};Reflect.get=()=>{throw new Error('page Reflect.get')};Function.prototype.call=()=>{throw new Error('page Function.call')};
          const result=Document.prototype.evaluate.apply(inert,['count(p)',scope,null,1]);if(result.numberValue!==2)throw new Error('private provider used page builtins');
          let error;try{inert.evaluate('p[',scope)}catch(caught){error=caught}if(!(error instanceof DOMException)||error.name!=='SyntaxError'||error.code!==12)throw new Error('wrong syntax exception realm');
          try{inert.evaluate('p',scope,null,42)}catch(caught){error=caught}if(!(error instanceof TypeError))throw new Error('wrong type exception realm');
          Function.prototype.call=originalCall;if(inert.documentElement.outerHTML!==html)throw new Error('XPath mutated DOM');
          return true;
        })()"#,
        );
    }

    #[test]
    fn xpath_evaluator_is_real_and_opaque_objects_have_private_native_brands() {
        fixture(
            r#"(()=>{
          const scope=inert.getElementById('scope'),evaluator=new XPathEvaluator();
          if(Object.prototype.toString.call(evaluator)!=='[object XPathEvaluator]'||Object.getPrototypeOf(Object.getPrototypeOf(evaluator))!==Object.prototype)throw new Error('evaluator prototype');
          if(evaluator.evaluate('count(p)',scope,null,1).numberValue!==2||evaluator.createExpression('p').evaluate(scope,9).singleNodeValue!==scope.firstChild||evaluator.createNSResolver(scope)!==scope)throw new Error('evaluator behavior');
          let error;try{XPathEvaluator.prototype.evaluate.call({},'p',scope)}catch(caught){error=caught}if(!(error instanceof TypeError))throw new Error('evaluator brand');
          try{inert.evaluate('p',scope,null,Symbol())}catch(caught){error=caught}if(!(error instanceof TypeError))throw new Error('numeric type realm');
          return true;
        })()"#,
        );
    }

    #[test]
    fn xpath_borrowed_interfaces_accept_foreign_native_records_and_keep_result_realms() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        super::super::install(scope);
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(parent.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<!doctype html><body><p>child</p>");
            run_js(scope,"globalThis.expression=document.createExpression('//p');globalThis.result=expression.evaluate(document,7);globalThis.evaluator=new XPathEvaluator();");
        }
        let global = parent.global(scope);
        let child_global = child.global(scope);
        set_prop(scope, global, "child", child_global.into());
        let source=v8::String::new(scope,r#"(()=>{
          const result=XPathExpression.prototype.evaluate.call(child.expression,child.document,7);
          if(result instanceof XPathResult||!(result instanceof child.XPathResult)||result.snapshotItem(0)!==child.document.body.firstChild)throw new Error('foreign expression/result realm');
          if(Document.prototype.evaluate.call(child.document,'//p',child.document,null,7) instanceof XPathResult||Document.prototype.createExpression.call(child.document,'//p') instanceof XPathExpression||!(document.evaluate('//p',child.document,null,7) instanceof XPathResult)||XPathEvaluator.prototype.createExpression.call(child.evaluator,'//p') instanceof XPathExpression||XPathEvaluator.prototype.evaluate.call(child.evaluator,'//p',child.document,null,7) instanceof XPathResult)throw new Error('borrowed interface owner realm');
          if(XPathResult.prototype.snapshotItem.call(child.result,0)!==child.document.body.firstChild||Object.getOwnPropertyDescriptor(XPathResult.prototype,'snapshotLength').get.call(child.result)!==1)throw new Error('foreign result brand');
          if(XPathEvaluator.prototype.evaluate.call(child.evaluator,'//p',child.document,null,9).singleNodeValue!==child.document.body.firstChild)throw new Error('foreign evaluator brand');
          return [result,child.expression,child.evaluator,Object.create(XPathResult.prototype),{}];
        })()"#).unwrap();
        let values = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let values = v8::Local::<v8::Array>::try_from(values).unwrap();
        for index in 0..5 {
            let value = values
                .get_index(scope, index)
                .unwrap()
                .to_object(scope)
                .unwrap();
            assert_eq!(is_native_xpath_object(scope, value), index < 3);
        }
        reset();
    }

    #[test]
    fn xpath_tree_expression_and_operation_budgets_fail_without_mutating_documents() {
        prepared_fixture(
            r#"(()=>{
          const scope=inert.getElementById('scope'),before=scope.outerHTML;
          for(const source of ['('.repeat(40)+'1'+')'.repeat(40),'p|'.repeat(300)+'p','p'.repeat(4097)]){let error;try{inert.createExpression(source)}catch(caught){error=caught.name}if(error!=='NotSupportedError')throw new Error('expression budget '+error);}
          let error;try{document.evaluate('//i',document,null,7)}catch(caught){error=caught.name}if(error!=='NotSupportedError'||scope.outerHTML!==before)throw new Error('tree budget');
          if(inert.evaluate('count(p)',scope,null,1).numberValue!==2)throw new Error('session unusable after budget');
          return true;
        })()"#,
            |tree| {
                let body = tree.query_selector("body").unwrap();
                tree.set_inner_html(body, &"<i></i>".repeat(4096));
            },
        );
        prepared_fixture(
            r#"(()=>{
          const html=document.body.innerHTML;let error;try{document.evaluate("//i[following::i[preceding::i[following::i]]]",document,null,7)}catch(caught){error=caught.name}if(error!=='NotSupportedError'||document.body.innerHTML!==html)throw new Error('operation budget '+error);
          if(inert.evaluate('count(p)',inert.getElementById('scope'),null,1).numberValue!==2)throw new Error('session unusable after operation budget');
          return true;
        })()"#,
            |tree| {
                let body = tree.query_selector("body").unwrap();
                tree.set_inner_html(body, &"<i><b></b></i>".repeat(1000));
            },
        );
    }
}

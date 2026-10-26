use super::*;

#[test]
fn draining_observer_events_does_not_dirty_native_ownership_graphs() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body>sentinel");
        let source = v8::String::new(
            scope,
            "new MutationObserver(()=>{}).observe(document.body,{childList:true});",
        )
        .unwrap();
        v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        DOM.with(|dom| dom.borrow_mut().as_mut().unwrap().ownership_dirty = false);
        mutation_observers::flush(scope);
        assert!(!DOM.with(|dom| dom.borrow().as_ref().unwrap().ownership_dirty));
        reset();
    }
    clear_interface_records(isolate);
}

#[test]
fn trusted_document_consumer_runs_after_observer_fanout_and_supports_nested_mutations() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(
            scope,
            "<!doctype html><html><body><div id=resources></div></body></html>",
        );
        v8::tc_scope!(let scope, scope);
        let source = v8::String::new(scope, r#"(()=>{
          const root=document.getElementById('resources'),observer=new MutationObserver(()=>{});
          observer.observe(root,{childList:true,attributes:true,subtree:true});
          const rows=[];globalThis.resourceRows=rows;globalThis.resourceRoot=root;globalThis.resourceObserver=observer;
          return function(event){
            const pending=observer.takeRecords();rows.push([event.type,event.target===root,pending.length,event.addedNodes.length]);
            if(event.type==='childList'&&event.addedNodes.length)event.addedNodes[0].setAttribute('data-prepared','true');
          };
        })()"#).unwrap();
        let callback = v8::Script::compile(scope, source, None)
            .and_then(|script| script.run(scope))
            .unwrap();
        let callback = v8::Local::<v8::Function>::try_from(callback).unwrap();
        let id = context.get_slot::<DomContextId>().unwrap().0;
        let _owner = DomScope::enter(id);
        let document = wrap(scope, with_tree(Tree::root).unwrap());
        assert!(install_mutation_consumer(scope, document, callback));
        let source = v8::String::new(scope, "resourceRoot.appendChild(document.createElement('script'));JSON.stringify(resourceRows)").unwrap();
        let result = v8::Script::compile(scope, source, None)
            .and_then(|script| script.run(scope))
            .unwrap();
        assert_eq!(
            result.to_rust_string_lossy(scope),
            "[[\"childList\",true,1,1],[\"attributes\",false,1,0]]"
        );
        remove_mutation_consumer(scope, document);
        let source = v8::String::new(scope, "resourceRoot.setAttribute('data-after','true');resourceObserver.disconnect();resourceRows.length").unwrap();
        let result = v8::Script::compile(scope, source, None)
            .and_then(|script| script.run(scope))
            .unwrap();
        assert_eq!(result.int32_value(scope), Some(2));
        let inert = parse_html_document(scope, "<body></body>", false);
        assert!(!install_mutation_consumer(scope, inert, callback));
        reset();
    }
    clear_interface_records(isolate);
}

#[test]
fn observer_records_and_static_lists_bypass_page_intrinsic_hooks() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(
            scope,
            "<!doctype html><html><body><div id='watched'></div></body></html>",
        );
        v8::tc_scope!(let scope, scope);
        let source = v8::String::new(scope, r#"(()=>{
          const root=document.getElementById('watched'),observer=new MutationObserver(()=>{});
          observer.observe(root,{childList:true,attributes:true,subtree:true});
          const define=Object.defineProperty,apply=Reflect.apply;
          const originals=[WeakMap.prototype.get,WeakMap.prototype.set,WeakRef.prototype.deref,
            Object.defineProperty,Reflect.apply,Array.prototype.push,Array.prototype[Symbol.iterator]];
          let hooks=0,records;const poison=()=>{hooks++;throw Error('page observer intrinsic');};
          try{
            WeakMap.prototype.get=WeakMap.prototype.set=WeakRef.prototype.deref=poison;
            Object.defineProperty=Reflect.apply=Array.prototype.push=Array.prototype[Symbol.iterator]=poison;
            define(Array.prototype,'0',{configurable:true,set:poison});
            root.innerHTML='<b>native</b>';
            root.setAttribute('data-observer','yes');
            records=observer.takeRecords();
          } finally {
            WeakMap.prototype.get=originals[0];WeakMap.prototype.set=originals[1];WeakRef.prototype.deref=originals[2];
            Object.defineProperty=originals[3];Reflect.apply=originals[4];Array.prototype.push=originals[5];Array.prototype[Symbol.iterator]=originals[6];delete Array.prototype['0'];
          }
          observer.disconnect();
          if(hooks!==0||records.length!==2||records[0].type!=='childList'||records[1].type!=='attributes')throw Error('observer interception '+hooks);
          const list=records[0].addedNodes;
          if(list.length!==1||list.item(0)!==root.firstChild||list[0]!==root.firstChild||Object.keys(list).join()!=='0')throw Error('static node list');
          const item=NodeList.prototype.item;
          let forged=false;try{apply(item,Object.create(NodeList.prototype),[0]);}catch(error){forged=error.name==='TypeError';}
          if(!forged||Reflect.ownKeys(observer).length||Reflect.ownKeys(records[0]).length)throw Error('private brands');
          return true;
        })()"#).unwrap();
        let value = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        assert!(
            value.is_some_and(|value| value.is_true()),
            "{:?}",
            scope
                .exception()
                .map(|value| value.to_rust_string_lossy(scope))
        );
        reset();
    }
    clear_interface_records(isolate);
}

#[test]
fn native_mutation_observers_match_authored_chrome_control() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
        );
        v8::tc_scope!(let scope, scope);
        let probe = include_str!("mutation_observers_probe.js");
        let source = v8::String::new(scope, probe).unwrap();
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        let result = result.unwrap_or_else(|| {
            panic!(
                "{:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        });
        let promise = v8::Local::<v8::Promise>::try_from(result).unwrap();
        for _ in 0..12 {
            mutation_observers::flush(scope);
            scope.perform_microtask_checkpoint();
            if promise.state() != v8::PromiseState::Pending {
                break;
            }
        }
        assert_eq!(
            promise.state(),
            v8::PromiseState::Fulfilled,
            "{}",
            promise.result(scope).to_rust_string_lossy(scope)
        );
        let value = promise.result(scope);
        let value = v8::json::stringify(scope, value)
            .unwrap()
            .to_rust_string_lossy(scope);
        let actual: serde_json::Value = serde_json::from_str(&value).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("mutation_observers_chrome155.json")).unwrap();
        let differences = expected["result"]
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(name, value)| (actual[name] != *value).then_some(name.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(differences, Vec::<&str>::new(), "Native result: {actual}");
        reset();
    }
    clear_interface_records(isolate);
}

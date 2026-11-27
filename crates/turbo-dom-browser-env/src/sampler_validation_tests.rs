//! Bounded operation counts, not elapsed-time or cross-call reuse permissions.
use super::sampler_diagnostics as diag;
use super::*;
use crate::host_internals as host;

fn fixture(depth: usize, count: usize) -> String {
    let mut html = "<!doctype html><style>.ancestor{font:16px/20px serif;color:black;display:block}.target{display:block;width:100px;letter-spacing:inherit}</style>".to_owned();
    for _ in 0..depth {
        html.push_str("<div class=ancestor>");
    }
    for i in 0..count {
        html.push_str(&format!("<div class=target id=t{i}></div>"));
    }
    for _ in 0..depth {
        html.push_str("</div>");
    }
    html
}
fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}
fn world() -> (u64, u64, u64, u64) {
    let (version, selector) = host::with_tree(|t| (t.version, t.selector_version)).unwrap();
    let (cssom, live) = host_style_revisions();
    (version, selector, cssom, live)
}
fn with_scene(
    depth: usize,
    count: usize,
    retained: usize,
    recording: bool,
    inherited: bool,
    test: impl FnOnce(&mut v8::PinScope, v8::Local<v8::Object>, &[host::Handle]),
) {
    tests::init_v8();
    let isolate = &mut tests::plain_isolate();
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let html = fixture(depth, count);
    let html = if inherited {
        html
    } else {
        html.replace("letter-spacing:inherit", "letter-spacing:normal")
    };
    install_with_html(scope, &html);
    run_engine_script(scope,"__tsNs.installAnimationHost({now:()=>1000,frame(){},task(){}});globalThis.__providers=0;__tsNs.layoutUsedSize=()=>{__providers++;throw Error('geometry requested')};").unwrap();
    let mut kept = Vec::new();
    for _ in 0..retained {
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(context.get_security_token(scope));
        {
            let inner = &mut v8::ContextScope::new(scope, child);
            install_independent_html(inner, "<!doctype html><body><p>retained</p></body>");
        }
        if kept.is_empty() {
            let key = v8::String::new(scope, "__foreignWindow").unwrap();
            let foreign = child.global(scope);
            context.global(scope).set(scope, key.into(), foreign.into());
        }
        kept.push(v8::Global::new(scope, child));
    }
    if recording {
        eval(scope,"globalThis.__records=0;globalThis.__observer=new MutationObserver(rows=>{__records+=rows.length});__observer.observe(document.body,{subtree:true,childList:true,attributes:true})");
    }
    let (root, targets) = host::with_tree(|tree| {
        (
            tree.root(),
            (0..count)
                .map(|i| tree.query_selector(&format!("#t{i}")).unwrap())
                .collect::<Vec<_>>(),
        )
    })
    .unwrap();
    let document = host::wrap(scope, root);
    let owner = host::enter_document_of(scope, document);
    host::sample_presented_effects(scope, document, &targets).unwrap();
    test(scope, document, &targets);
    drop(owner);
    drop(kept);
    reset();
}
fn sample(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    targets: &[host::Handle],
) -> (host::PresentedEffectsSnapshot, diag::Report) {
    diag::begin(scope);
    let sample = host::sample_presented_effects(scope, document, targets).unwrap();
    let report = diag::finish(scope);
    assert_eq!(sample.sample_time, Some(1000.0));
    assert_eq!(sample.targets.len(), targets.len());
    assert!(sample.targets.iter().all(|target|matches!(&target.presentation,host::PresentedTargetState::Certified{animation,transition}if animation.is_empty()&&transition.is_empty())));
    (sample, report)
}
#[test]
fn validation_origin_counter_proves_adjacent_same_entry_revalidation() {
    let mut rows = Vec::new();
    for (depth, count) in [(4, 16), (4, 64), (16, 16), (16, 64)] {
        with_scene(depth, count, 0, false, true, |scope, document, targets| {
            let before = world();
            let (first, a) = sample(scope, document, targets);
            let middle = world();
            let (second, b) = sample(scope, document, targets);
            let after = world();
            assert_eq!(before, middle);
            assert_eq!(middle, after);
            assert_eq!(format!("{first:?}"), format!("{second:?}"));
            assert_eq!(eval(scope, "__providers"), "0");
            rows.push(serde_json::json!({"D":depth,"N":count,"first":a,"second":b}));
        });
    }
    eprintln!(
        "VALIDATION_ORIGIN_ROWS={}",
        serde_json::to_string(&rows).unwrap()
    );
    assert!(
        rows.iter()
            .all(|r| r["first"]["adjacent_import_visits"] == 0
                && r["second"]["adjacent_import_visits"] == 0),
        "adjacent same-entry import must not repeat an already complete guard operation: {rows:?}"
    );
}
#[test]
fn unrelated_retained_document_counter_proves_empty_read_fanout() {
    let mut rows = Vec::new();
    for recording in [false, true] {
        for retained in [0, 1, 8] {
            for (depth, count) in [(4, 16), (4, 64), (16, 16), (16, 64)] {
                with_scene(
                    depth,
                    count,
                    retained,
                    recording,
                    false,
                    |scope, document, targets| {
                        let before = world();
                        let (first, a) = sample(scope, document, targets);
                        let middle = world();
                        let (second, b) = sample(scope, document, targets);
                        let after = world();
                        assert_eq!(before, middle);
                        assert_eq!(middle, after);
                        assert_eq!(format!("{first:?}"), format!("{second:?}"));
                        assert_eq!(eval(scope, "__providers"), "0");
                        let row = serde_json::json!({"D":depth,"N":count,"M":retained,"recording":recording,"first":a,"second":b});
                        eprintln!("EMPTY_FANOUT_ROW={row}");
                        rows.push(row);
                    },
                );
            }
        }
    }
    eprintln!(
        "EMPTY_FANOUT_ROWS={}",
        serde_json::to_string(&rows).unwrap()
    );
    assert!(
        rows.iter().all(|row| {
            ["first", "second"]
                .into_iter()
                .all(|phase| row[phase]["claim_scope_switches"] == 0)
        }),
        "claim phase must not swap document state: {rows:?}"
    );
    for row in &rows {
        for phase in ["first", "second"] {
            let report = &row[phase];
            let foreign = report["foreign_empty_documents"].as_u64().unwrap();
            if !row["recording"].as_bool().unwrap() {
                assert_eq!(foreign, 0);
            } else {
                assert_eq!(
                    foreign,
                    report["flushes"].as_u64().unwrap() * row["M"].as_u64().unwrap()
                );
            }
        }
    }
}

#[test]
fn raw_diagnostic_hook_preserves_cross_document_callbacks_and_ownership() {
    with_scene(4, 1, 1, true, false, |scope, document, targets| {
        eval(
            scope,
            r#"__foreignWindow.eval(`globalThis.__ce=[];customElements.define('x-foreign',class extends HTMLElement{connectedCallback(){__ce.push(['connected',this.ownerDocument===document])}adoptedCallback(oldDoc,newDoc){__ce.push(['adopted',this.ownerDocument===newDoc])}});globalThis.__foreignRecords=0;globalThis.__otherObserver=new MutationObserver(rows=>{__foreignRecords+=rows.length});__otherObserver.observe(document.body,{subtree:true,childList:true});`)"#,
        );
        run_engine_script(
            scope,
            r#"document.styleSheets[0].insertRule('@media(max-width:2000px){.ancestor{color:rgb(1,2,3)}}',0);const ns=__tsNs,original=ns.mediaTextMatches;let did=false;ns.mediaTextMatches=function(...args){if(!did){did=true;const other=__foreignWindow.document,node=other.createElement('x-foreign');other.body.appendChild(node);document.body.appendChild(node);}return Reflect.apply(original,ns,args);};"#,
        );
        let (_, report) = sample(scope, document, targets);
        assert!(report.nonempty_documents > 0, "{report:?}");
        assert_eq!(
            eval(scope, "JSON.stringify(__foreignWindow.__ce)"),
            "[[\"connected\",true],[\"adopted\",true],[\"connected\",false]]"
        );
        assert_eq!(
            eval(
                scope,
                "document.querySelector('x-foreign').ownerDocument===document"
            ),
            "true"
        );
        assert_eq!(
            eval(scope, "__records===0&&__foreignWindow.__foreignRecords===0"),
            "true"
        );
        assert_eq!(eval(scope, "__providers"), "0");
        scope.perform_microtask_checkpoint();
        assert_eq!(
            eval(scope, "__records>0&&__foreignWindow.__foreignRecords>0"),
            "true"
        );
        eprintln!(
            "CROSS_DOCUMENT_DIAGNOSTIC={}",
            serde_json::to_string(&report).unwrap()
        );
    });
}

fn claim_consumer<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Function> {
    let source = v8::String::new(scope, r#"globalThis.__claimObserverRows=[];
      globalThis.__claimObserver=new MutationObserver(rows=>{for(const row of rows)__claimObserverRows.push([row.attributeName,row.oldValue===null?null:row.oldValue.charCodeAt(0)]);});
      __claimObserver.observe(document.body,{attributes:true,attributeOldValue:true});
      (function(event){
        __claimLog.push([document===__claimMain?'main':'foreign',event.attributeName,event.oldValue===null?null:event.oldValue.charCodeAt(0),event.target.ownerDocument===document]);
        if(event.attributeName==='data-a')document.body.setAttribute('data-b','nested');
      })"#).unwrap();
    v8::Local::<v8::Function>::try_from(
        v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn claimed_utf16_batches_keep_current_last_and_survive_reentrant_consumers() {
    with_scene(4, 1, 1, true, false, |scope, document, _| {
        let context = scope.get_current_context();
        let foreign_key = v8::String::new(scope, "__foreignWindow").unwrap();
        let foreign = v8::Local::<v8::Object>::try_from(
            context
                .global(scope)
                .get(scope, foreign_key.into())
                .unwrap(),
        )
        .unwrap();
        let foreign_context = foreign.get_creation_context(scope).unwrap();
        let contexts = [foreign_context, context];
        let log = v8::Array::new(scope, 0);
        let mut documents = Vec::new();
        for realm in contexts {
            let inner = &mut v8::ContextScope::new(scope, realm);
            let id = realm.get_slot::<DomContextId>().map_or(0, |id| id.0);
            let _owner = DomScope::enter(id);
            let root = with_tree(Tree::root).unwrap();
            let own_document = wrap(inner, root);
            for (name, value) in [("__claimLog", log.into()), ("__claimMain", document.into())] {
                let key = v8::String::new(inner, name).unwrap();
                realm.global(inner).set(inner, key.into(), value);
            }
            let callback = claim_consumer(inner);
            assert!(mutation_observers::install_mutation_consumer(
                inner,
                own_document,
                callback
            ));
            with_tree_mut(|tree| {
                let body = tree.query_selector("body").unwrap();
                tree.set_attribute_utf16(body, "data-a", &[0xd800]);
                tree.take_mutation_events_utf16();
            });
            documents.push((id, own_document));
        }
        for (id, _) in &documents {
            let _owner = DomScope::enter(*id);
            with_tree_mut(|tree| {
                let body = tree.query_selector("body").unwrap();
                tree.set_attribute_utf16(body, "data-a", &[0xd801]);
            });
        }
        let original = DOM_ID.with(std::cell::Cell::get);
        diag::begin(scope);
        mutation_observers::flush(scope);
        let report = diag::finish(scope);
        assert_eq!(DOM_ID.with(std::cell::Cell::get), original);
        assert_eq!(
            eval(scope, "JSON.stringify(__claimLog)"),
            r#"[["foreign","data-a",55296,true],["foreign","data-b",null,true],["main","data-a",55296,true],["main","data-b",null,true]]"#
        );
        assert_eq!(
            eval(
                scope,
                "__claimObserverRows.length+__foreignWindow.__claimObserverRows.length"
            ),
            "0"
        );
        scope.perform_microtask_checkpoint();
        for &(_, own_document) in &documents {
            let realm = own_document.get_creation_context(scope).unwrap();
            let inner = &mut v8::ContextScope::new(scope, realm);
            assert_eq!(
                eval(inner, "JSON.stringify(__claimObserverRows)"),
                r#"[["data-a",55296],["data-b",null]]"#
            );
        }
        assert!(report.nonempty_documents >= 4, "{report:?}");
        assert_eq!(report.claim_scope_switches, 0);
        diag::begin(scope);
        let phase = diag::claim_phase(scope);
        {
            let _foreign = DomScope::enter(documents[0].0);
        }
        drop(phase);
        assert_eq!(diag::finish(scope).claim_scope_switches, 2);

        eprintln!(
            "NONEMPTY_CLAIM_REPORT={}",
            serde_json::to_string(&report).unwrap()
        );
    });
}

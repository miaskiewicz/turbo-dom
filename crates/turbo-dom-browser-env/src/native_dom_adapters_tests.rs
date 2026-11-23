//! Public test-window adapter regressions, including independent Chromium controls.
use super::*;
use sha2::Digest;

pub(super) fn evaluate(source: &str) -> String {
    evaluate_with("", source)
}

fn evaluate_with(prelude: &str, source: &str) -> String {
    tests::init_v8();
    let isolate = &mut tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, prelude).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    install(scope);
    let code = v8::String::new(scope, source).unwrap();
    let result = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "Native adapter failed: {}",
                scope.exception().unwrap().to_rust_string_lossy(scope)
            )
        });
    let result = result.to_rust_string_lossy(scope);
    reset();
    result
}

#[test]
fn dom_parser_parses_native_inert_html() {
    assert_eq!(
        evaluate(
            r#"(() => {
        const d = new DOMParser().parseFromString('<p>hi</p><script>globalThis.ran=true</script>', 'text/html');
        return [d.querySelector('p').textContent, d.contentType, d.defaultView,
            d.location, d !== document, globalThis.ran === undefined,
            d.querySelector('p').ownerDocument === d].join('|');
    })()"#
        ),
        "hi|text/html|||true|true|true"
    );
}

#[test]
fn dom_parser_xml_uses_the_existing_chrome156_control() {
    let probe = include_str!("xml_documents_probe.js");
    let actual: serde_json::Value = serde_json::from_str(&evaluate(&format!(
        "JSON.stringify({})",
        probe.trim().trim_end_matches(';')
    )))
    .unwrap();
    let control: serde_json::Value =
        serde_json::from_str(include_str!("xml_documents_chrome156.json")).unwrap();
    assert_eq!(actual, control["result"]);
}

#[test]
fn dom_parser_converts_arguments_and_checks_receivers() {
    assert_eq!(
        evaluate(
            r#"(() => {
        const parser = new DOMParser(), errors = [], order = [];
        for (const call of [
            () => DOMParser(),
            () => parser.parseFromString(''),
            () => parser.parseFromString('', 'TEXT/HTML'),
            () => parser.parseFromString(Symbol(), 'text/html'),
            () => parser.parseFromString('', Symbol()),
            () => parser.parseFromString.call({}, '', 'text/html')
        ]) { try { call(); } catch (e) { errors.push(e.name); } }
        parser.parseFromString({toString(){order.push('input');return '<p>ok</p>'}},
            {toString(){order.push('type');return 'text/html'}});
        return errors.join() + '|' + order.join() + '|' + Object.prototype.toString.call(parser);
    })()"#
        ),
        "TypeError,TypeError,TypeError,TypeError,TypeError,TypeError|input,type|[object DOMParser]"
    );
}

#[test]
fn dom_parser_preserves_utf16_and_independent_document_ownership() {
    assert_eq!(
        evaluate(
            r#"(() => {
        const parser = new DOMParser();
        const a = parser.parseFromString('<p>\ud800</p>', 'text/html');
        const b = parser.parseFromString('<p>second</p>', 'text/html');
        const p = a.querySelector('p');
        const original = p.textContent.charCodeAt(0);
        b.body.appendChild(p);
        return [original, p.ownerDocument === b, a.querySelector('p') === null,
            document.querySelector('p') === null, p.textContent.charCodeAt(0)].join();
    })()"#
        ),
        "55296,true,true,true,55296"
    );
}

#[test]
fn window_history_updates_location_and_keeps_cloned_state() {
    assert_eq!(
        evaluate(
            r#"(() => {
        const state = {value: 1};
        history.replaceState(state, '', '/x?y=1');
        state.value = 2;
        const replaced = [location.href, location.search, history.state.value, history.length].join();
        history.pushState({value: 3}, '', '#next');
        return replaced + '|' + [location.href, location.hash, history.state.value,
            history.length, window.history === history].join();
    })()"#
        ),
        "http://localhost/x?y=1,?y=1,1,1|http://localhost/x?y=1#next,#next,3,2,true"
    );
}

#[test]
fn window_history_refuses_foreign_urls_and_uncloneable_state_atomically() {
    assert_eq!(evaluate(r#"(() => {
        history.replaceState({value:1}, '', '/safe');
        const errors = [];
        for (const call of [
            () => history.pushState({}, '', 'https://elsewhere.invalid/x'),
            () => history.replaceState({}, '', 'http://user@localhost/x'),
            () => history.pushState({}, '', 'http://['),
            () => history.pushState(() => {}, '', '/bad'),
            () => history.replaceState(Symbol(), '', '/bad'),
            () => history.pushState({}),
            () => history.pushState.call({}, {}, '', '/bad'),
            () => new History()
        ]) { try { call(); } catch (e) { errors.push(e.name); } }
        const cyclic = {map:new Map([['x',1]])}; cyclic.self = cyclic;
        history.pushState(cyclic, '');
        const state = history.state;
        return [errors.join(),location.pathname,history.length,state !== cyclic,
            state.self === state,state.map.get('x'),state === history.state].join('|');
    })()"#), "SecurityError,SecurityError,SecurityError,DataCloneError,DataCloneError,TypeError,TypeError,TypeError|/safe|2|true|true|1|true");
}

#[test]
fn window_history_traversal_is_queued_and_discards_forward_entries() {
    assert_eq!(
        evaluate_with(
            "globalThis.tasks=[];globalThis.setTimeout=function(callback){tasks.push(callback)};",
            r#"(() => {
            const events=[];
            addEventListener('popstate',e=>events.push('pop:'+e.state.n));
            addEventListener('hashchange',e=>events.push('hash:'+e.oldURL.endsWith('#two')+':'+e.newURL.endsWith('#one')));
            history.replaceState({n:0}, '', '/');
            history.pushState({n:1}, '', '#one');
            history.pushState({n:2}, '', '#two');
            history.back();
            const synchronous=history.state.n;
            while(tasks.length)tasks.shift()();
            const back=history.state.n;
            history.pushState({n:3}, '', '#three');
            history.forward();
            while(tasks.length)tasks.shift()();
            history.scrollRestoration='manual';
            history.scrollRestoration='invalid';
            return [synchronous,back,history.state.n,history.length,
                history.scrollRestoration,events.join()].join('|');
        })()"#,
        ),
        "2|1|3|3|manual|pop:1,hash:true:true"
    );
}

#[test]
fn window_history_keeps_embedder_navigation_objects() {
    assert_eq!(evaluate_with(
        "globalThis.location={href:'https://example.invalid/'};globalThis.history={host:true};",
        "[location.href,history.host,typeof globalThis.__historyURL,typeof globalThis.__historyClone].join()",
    ), "https://example.invalid/,true,undefined,undefined");
}

#[test]
fn window_history_reports_missing_navigation_host() {
    assert_eq!(
        evaluate(
            r#"(() => {
        const errors=[];
        for(const call of [()=>history.go(),()=>history.back()]){
            try{call()}catch(e){errors.push(e.name)}
        }
        return errors.join();
    })()"#
        ),
        "NotSupportedError,NotSupportedError"
    );
}

#[test]
fn native_dom_adapters_match_independent_chromium159_control() {
    let probe = include_str!("native_dom_adapters_probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("native_dom_adapters_chrome159.json")).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", sha2::Sha256::digest(probe.as_bytes()))
    );
    let actual: serde_json::Value = serde_json::from_str(&evaluate(&format!(
        "JSON.stringify({})",
        probe.trim().trim_end_matches(';')
    )))
    .unwrap();
    assert_eq!(actual, control["result"]);
}

#[test]
fn dom_parser_keeps_an_existing_embedder_parser() {
    assert_eq!(evaluate_with(
        "globalThis.DOMParser=function HostParser(){};DOMParser.prototype.parseFromString=function(){return 'host'};",
        "new DOMParser().parseFromString('x','text/html')",
    ), "host");
}

#[test]
fn host_can_replace_default_navigation_and_parser_after_install() {
    assert_eq!(
        evaluate(
            r#"(() => {
        const host={navigation:true};
        Object.defineProperty(globalThis,'history',{value:host,configurable:true});
        Object.defineProperty(globalThis,'DOMParser',{value:function HostParser(){},configurable:true});
        return [history===host,DOMParser.name].join();
    })()"#
        ),
        "true,HostParser"
    );
}

#[test]
fn history_traversal_restores_serialized_state_after_guest_mutation() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("native_dom_adapters_chrome159.json")).unwrap();
    assert_eq!(control["traversal"], serde_json::json!([0, 1]));
    assert_eq!(
        evaluate_with(
            "globalThis.tasks=[];globalThis.setTimeout=function(callback){tasks.push(callback)};",
            r#"(() => {
            history.replaceState({n:0}, '', '/');
            history.pushState({n:1}, '', '#one');
            history.state.n=99;
            history.back();
            while(tasks.length)tasks.shift()();
            const back=history.state.n;
            history.forward();
            while(tasks.length)tasks.shift()();
            return [back,history.state.n].join();
        })()"#,
        ),
        "0,1"
    );
}

#[test]
fn dom_parser_inert_location_is_null_and_brand_checked() {
    assert_eq!(
        evaluate(
            r#"(() => {
        const parsed=new DOMParser().parseFromString('<p>hi</p>','text/html');
        const descriptor=Object.getOwnPropertyDescriptor(parsed,'location');
        const errors=[];
        parsed.location='https://elsewhere.invalid/';
        for(const call of [()=>descriptor.get.call({}),()=>descriptor.set.call({},'/bad')]){
            try{call()}catch(e){errors.push(e.name)}
        }
        return [parsed.location===null,descriptor.configurable,descriptor.enumerable,errors.join()].join('|');
    })()"#
        ),
        "true|false|true|TypeError,TypeError"
    );
}

//! turbo-surf#32: HTMLImageElement's image-state members against the Chrome 156 control
//! (`html_image_probe.js` + `html_image_cases.json` -> `html_image_chrome156.json`, recorded by
//! scripts/select-element-chrome-control.mjs), with and without host load state.
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("html_image_probe.js");
const CASES: &[u8] = include_bytes!("html_image_cases.json");
const CONTROL: &str = include_str!("html_image_chrome156.json");

fn cases() -> serde_json::Value {
    serde_json::from_slice(CASES).unwrap()
}

fn control() -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(CONTROL).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156."));
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(CASES))
    );
    control["result"][0]["result"].clone()
}

/// Expose the document record's `imageHost` to `script` (as `imageHostForTest`, removed again
/// before the probe runs), then run `source` to completion and return its JSON result.
fn run(scope: &mut v8::PinScope, source: &str) -> serde_json::Value {
    v8::tc_scope!(let scope, scope);
    let code = format!(
        "Promise.resolve(({})).then(value=>globalThis.imageResult=JSON.stringify(value),error=>globalThis.imageResult='ERROR '+String(error&&error.stack||error))",
        source.trim().trim_end_matches(';')
    );
    let code = v8::String::new(scope, &code).unwrap();
    let Some(_) = v8::Script::compile(scope, code, None).and_then(|script| script.run(scope))
    else {
        panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope));
    };
    scope.perform_microtask_checkpoint();
    let key = v8::String::new(scope, "imageResult").unwrap();
    let global = scope.get_current_context().global(scope);
    let result = global.get(scope, key.into()).unwrap();
    assert!(result.is_string(), "probe did not settle");
    let result = result.to_rust_string_lossy(scope);
    assert!(!result.starts_with("ERROR"), "{result}");
    serde_json::from_str(&result).unwrap()
}

/// Run `setup` with `imageHostForTest` bound (removed again afterwards), then, when `probe` is
/// set, the Chrome probe. Returns the setup result and the probe result.
fn session(setup: &str, probe: bool) -> (serde_json::Value, Option<serde_json::Value>) {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, cases()[0]["html"].as_str().unwrap());
    let document = {
        let id = context.get_slot::<DomContextId>().unwrap().0;
        let _dom = DomScope::enter(id);
        wrap(scope, with_tree(Tree::root).unwrap())
    };
    let record = collection_records::get(scope, document)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let key = v8::String::new(scope, "imageHost").unwrap();
    let host = record.get(scope, key.into()).unwrap();
    assert!(host.is_object(), "the document record carries imageHost");
    let key = v8::String::new(scope, "imageHostForTest").unwrap();
    let global = context.global(scope);
    global.set(scope, key.into(), host);
    let setup = run(scope, setup);
    global.delete(scope, key.into());
    let result = probe.then(|| run(scope, PROBE));
    reset();
    (setup, result)
}

fn probe_with(setup: &str) -> serde_json::Value {
    let (ready, result) = session(setup, true);
    assert_eq!(ready, serde_json::json!(true), "setup");
    result.unwrap()
}

fn mismatches(actual: &serde_json::Value, expected: &serde_json::Value) -> Vec<String> {
    expected
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| &actual[key.as_str()] != *value)
        .map(|(key, value)| format!("{key}\n  binding: {}\n  chrome:  {value}", actual[key]))
        .collect()
}

/// With the host's load state for the decodable image, every section matches Chrome.
#[test]
fn image_state_with_host_load_state_matches_chrome156_control() {
    let expected = control();
    let loaded = &cases()[0]["loaded"];
    let setup = format!(
        "(()=>{{const img=document.getElementById('{}');imageHostForTest.setState(img,{{complete:true,naturalWidth:{},naturalHeight:{},currentSrc:img.getAttribute('src')}});return true;}})()",
        loaded["id"].as_str().unwrap(),
        loaded["naturalWidth"],
        loaded["naturalHeight"]
    );
    let actual = probe_with(&setup);
    let found = mismatches(&actual, &expected);
    assert!(found.is_empty(), "{}", found.join("\n"));
    assert_eq!(actual, expected);
}

/// Without host state (turbo-test: nothing loads) images report Chrome's not-loading answer:
/// everything matches except the decodable image's natural size and its `decode()` (resolved
/// in Chrome, rejected as undecodable here), which only a loader knows.
#[test]
fn image_state_without_host_reports_not_loading_defaults() {
    let mut expected = control();
    let actual = probe_with("true");
    let loaded = &mut expected["parsed"]["loaded"];
    assert_eq!(
        (&loaded["naturalWidth"], &loaded["naturalHeight"]),
        (&serde_json::json!(3), &serde_json::json!(2))
    );
    loaded["naturalWidth"] = serde_json::json!(0);
    loaded["naturalHeight"] = serde_json::json!(0);
    let undecodable = expected["decode"]["parsed"]["broken"].clone();
    let decoded = &mut expected["decode"]["parsed"]["loaded"];
    assert_eq!(decoded, &serde_json::json!("resolved"));
    *decoded = undecodable;
    let found = mismatches(&actual, &expected);
    assert!(found.is_empty(), "{}", found.join("\n"));
    assert_eq!(actual, expected);
}

/// The host hook: validation, copy-on-set coercions, loading state, clearing, per-element state
/// and privacy (the record is unreachable from page JS).
#[test]
fn image_host_set_state_contract() {
    let (result, _) = session(
        r#"(()=>{
          const img=document.createElement('img'),other=document.createElement('img'),errors=[];
          img.setAttribute('src','https://x.test/dir/../pic.png');
          const view=i=>[i.complete,i.naturalWidth,i.naturalHeight,i.currentSrc];
          const before=view(img),src=img.src;
          for(const [target,state] of [[document.createElement('div'),{}],[{},{}],[null,{}],[document.createElementNS('urn:x','img'),{}],[img,5],[img,'x']]){
            try{imageHostForTest.setState(target,state);errors.push('accepted');}catch(e){errors.push(e.name+': '+e.message);}
          }
          const state={complete:false,naturalWidth:-1,naturalHeight:'7.9',currentSrc:{toString(){return 'https://x.test/a.png';}}};
          imageHostForTest.setState(img,state);
          const loading=view(img);
          state.complete=true;state.naturalWidth=9;
          const copied=view(img);
          imageHostForTest.setState(img,{complete:1,naturalWidth:4294967297,naturalHeight:NaN});
          const coerced=view(img);
          const untouched=view(other);
          imageHostForTest.setState(img,undefined);
          const cleared=view(img);
          imageHostForTest.setState(img,{complete:true,naturalWidth:2,naturalHeight:3,currentSrc:'u'});
          imageHostForTest.setState(img,null);
          const nulled=view(img);
          img.setAttribute('src','');
          const empty=view(img);
          img.removeAttribute('src');
          const removed=view(img);
          const hidden=['imageHost' in document,'imageHost' in window,Object.getOwnPropertyNames(img).length,Object.getPrototypeOf(imageHostForTest)];
          return {src,before,errors,loading,copied,coerced,untouched,cleared,nulled,empty,removed,hidden};
        })()"#,
        false,
    );
    // currentSrc without host state is the src getter's resolved URL (resolution needs the
    // embedder's URL global; the bare binding test realm has none and keeps the attribute text).
    let base = result["src"].as_str().unwrap().to_owned();
    assert!(base.contains("pic.png"), "{base}");
    assert_eq!(
        result,
        serde_json::json!({
            "src": base,
            "before": [true, 0, 0, base],
            "errors": [
                "TypeError: Illegal invocation",
                "TypeError: Illegal invocation",
                "TypeError: Illegal invocation",
                "TypeError: Illegal invocation",
                "TypeError: Image state must be an object or null",
                "TypeError: Image state must be an object or null"
            ],
            "loading": [false, 4294967295u32, 7, "https://x.test/a.png"],
            "copied": [false, 4294967295u32, 7, "https://x.test/a.png"],
            "coerced": [true, 1, 0, ""],
            "untouched": [true, 0, 0, ""],
            "cleared": [true, 0, 0, base],
            "nulled": [true, 0, 0, base],
            "empty": [true, 0, 0, ""],
            "removed": [true, 0, 0, ""],
            "hidden": [false, false, 0, null]
        })
    );
}

/// turbo-dom#23: `decode()` waits for a load the host reports in progress and settles with the
/// host's next state — resolved once decoded (complete with a natural size), an EncodingError
/// when it failed or was cleared; every waiting call settles.
#[test]
fn decode_settles_with_host_load_state() {
    let (result, _) = session(
        r#"(async()=>{
          const settle=(p,log,label)=>p.then(()=>log.push(label+':resolved'),e=>log.push(label+':'+e.name));
          const tick=()=>new Promise(r=>Promise.resolve().then(r));
          const log=[];
          const img=new Image();
          imageHostForTest.setState(img,{complete:false});
          settle(img.decode(),log,'a');settle(img.decode(),log,'b');
          await tick();
          const waiting=log.length;
          imageHostForTest.setState(img,{complete:false,naturalWidth:5,naturalHeight:5});
          await tick();
          const stillWaiting=log.length;
          imageHostForTest.setState(img,{complete:true,naturalWidth:5,naturalHeight:4,currentSrc:'u'});
          await tick();await tick();
          const failed=document.createElement('img');
          imageHostForTest.setState(failed,{complete:false});
          settle(failed.decode(),log,'failed');
          imageHostForTest.setState(failed,{complete:true,naturalWidth:0,naturalHeight:0});
          const cleared=document.createElement('img');
          imageHostForTest.setState(cleared,{complete:false});
          settle(cleared.decode(),log,'cleared');
          imageHostForTest.setState(cleared,null);
          settle(img.decode(),log,'after');
          await tick();await tick();
          return {waiting,stillWaiting,log:log.sort()};
        })()"#,
        false,
    );
    assert_eq!(
        result,
        serde_json::json!({
            "waiting": 0,
            "stillWaiting": 0,
            "log": ["a:resolved", "after:resolved", "b:resolved", "cleared:EncodingError", "failed:EncodingError"]
        })
    );
}

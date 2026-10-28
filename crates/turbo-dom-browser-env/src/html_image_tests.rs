//! turbo-surf#32: HTMLImageElement's image-state members against the Chrome 156 control
//! (`html_image_probe.js` + `html_image_cases.json` -> `html_image_chrome156.json`, recorded by
//! scripts/select-element-chrome-control.mjs), with and without host load state.
use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;
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
    let _ = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", thrown(scope.exception(), scope)));
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

/// With the host's load state for the decodable images and its layout of the rendered ones,
/// every section matches Chrome.
#[test]
fn image_state_with_host_load_state_matches_chrome156_control() {
    let expected = control();
    let loaded = &cases()[0]["loaded"];
    // A loader decodes every image of the decodable URL; the host's layout answers the
    // rendered boxes (`layout`: id -> [width, height, x, y]) and null for the others.
    let setup = format!(
        "(()=>{{const src=document.getElementById('{}').getAttribute('src');for(const img of document.querySelectorAll('img'))if(img.getAttribute('src')===src)imageHostForTest.setState(img,{{complete:true,naturalWidth:{},naturalHeight:{},currentSrc:src}});const layout={};imageHostForTest.setLayoutQuery(img=>{{const box=layout[img.id];return box?{{width:box[0],height:box[1],x:box[2],y:box[3]}}:null;}});return true;}})()",
        loaded["id"].as_str().unwrap(),
        loaded["naturalWidth"],
        loaded["naturalHeight"],
        cases()[0]["layout"]
    );
    let actual = probe_with(&setup);
    let found = mismatches(&actual, &expected);
    assert!(found.is_empty(), "{}", found.join("\n"));
    assert_eq!(actual, expected);
}

/// Without host state (turbo-test: nothing loads, nothing is laid out) images report Chrome's
/// not-loading, not-rendered answer: everything matches except the decodable images' natural
/// size, the loaded image's `decode()` (resolved in Chrome, rejected as undecodable here) and the
/// rendered boxes (`width` / `height` / `x` / `y`), which only a loader and a layout know.
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
    // turbo-dom#26: without a layout no image is being rendered and nothing has a natural size,
    // so width / height are the attribute (else 0) and x / y are 0 — Chrome's answer for the
    // hidden images without a natural size ([width, height, x, y, naturalWidth, naturalHeight]).
    let placed = &mut expected["dimensions"]["placed"];
    assert_eq!(placed["sized"], serde_json::json!([40, 30, 20, 10, 3, 2]));
    assert_eq!(placed["styled"], serde_json::json!([10, 5, 23, 40, 3, 2]));
    assert_eq!(placed["intrinsic"], serde_json::json!([3, 2, 20, 45, 3, 2]));
    assert_eq!(
        placed["hiddenIntrinsic"],
        serde_json::json!([3, 2, 0, 0, 3, 2])
    );
    for (id, row) in [
        ("sized", serde_json::json!([40, 30, 0, 0, 0, 0])),
        ("styled", serde_json::json!([40, 30, 0, 0, 0, 0])),
        ("intrinsic", serde_json::json!([0, 0, 0, 0, 0, 0])),
        ("hidden", serde_json::json!([40, 30, 0, 0, 0, 0])),
        ("hiddenIntrinsic", serde_json::json!([0, 0, 0, 0, 0, 0])),
    ] {
        placed[id] = row;
    }
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
    // currentSrc without host state is the selected source (the src attribute) parsed against
    // the document base URL, as in Chrome (turbo-dom#71); the src getter's own resolution needs
    // the embedder's URL global, which the bare binding test realm lacks (it keeps the text).
    let src = result["src"].as_str().unwrap().to_owned();
    assert!(src.contains("pic.png"), "{src}");
    let base = "https://x.test/pic.png";
    assert_eq!(
        result,
        serde_json::json!({
            "src": src,
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

/// turbo-dom#26: the host's layout query — validation, clearing, coercions of its answer
/// (width / height unsigned long, x / y long), "not rendered" for null / undefined, the
/// attribute / natural-size fallbacks, and the query's call shape.
#[test]
fn image_host_layout_query_contract() {
    let (result, _) = session(
        r#"(()=>{
          const img=document.createElement('img'),errors=[],calls=[];
          img.setAttribute('width','12');
          const view=i=>[i.width,i.height,i.x,i.y];
          for(const query of [5,'f',{},true]){
            try{imageHostForTest.setLayoutQuery(query);errors.push('accepted');}catch(e){errors.push(e.name+': '+e.message);}
          }
          const unset=view(img);
          let answer={width:-1,height:'7.9',x:1.9,y:2147483648};
          imageHostForTest.setLayoutQuery(function(image){'use strict';calls.push([image===img,this===undefined,arguments.length]);return answer;});
          const coerced=view(img);
          answer={width:4294967297,height:NaN};
          const partial=view(img);
          answer=null;
          const notRendered=view(img);
          answer=undefined;
          imageHostForTest.setState(img,{complete:true,naturalWidth:5,naturalHeight:6});
          const natural=view(img);
          answer={width:20,height:10,x:3,y:4};
          img.width=2147483648;
          const written=[img.getAttribute('width'),...view(img)];
          imageHostForTest.setLayoutQuery(()=>{throw new RangeError('layout');});
          let thrown;
          try{img.width;}catch(e){thrown=e.name+': '+e.message;}
          imageHostForTest.setLayoutQuery(null);
          const cleared=view(img);
          imageHostForTest.setLayoutQuery(()=>({width:1,height:1,x:1,y:1}));
          imageHostForTest.setLayoutQuery(undefined);
          const undefinedCleared=view(img);
          return {errors,unset,coerced,partial,notRendered,natural,written,thrown,cleared,undefinedCleared,calls:calls.length,shape:calls[0]};
        })()"#,
        false,
    );
    assert_eq!(
        result,
        serde_json::json!({
            "errors": [
                "TypeError: Layout query must be a function or null",
                "TypeError: Layout query must be a function or null",
                "TypeError: Layout query must be a function or null",
                "TypeError: Layout query must be a function or null"
            ],
            "unset": [12, 0, 0, 0],
            "coerced": [4294967295u32, 7, 1, -2147483648],
            "partial": [1, 0, 0, 0],
            "notRendered": [12, 0, 0, 0],
            "natural": [12, 6, 0, 0],
            "written": ["0", 20, 10, 3, 4],
            "thrown": "RangeError: layout",
            "cleared": [0, 6, 0, 0],
            "undefinedCleared": [0, 6, 0, 0],
            "calls": 20,
            "shape": [true, true, 1]
        })
    );
}

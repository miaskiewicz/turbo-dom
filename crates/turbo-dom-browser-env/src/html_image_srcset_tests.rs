//! turbo-dom#71: HTMLImageElement srcset / sizes and HTMLSourceElement srcset / sizes / media /
//! type / width / height, and the srcset / sizes / <picture> source selection, against the
//! Chrome 156 control (`html_image_srcset_probe.js` + `html_image_srcset_cases.json` ->
//! `html_image_srcset_chrome156.json`, recorded by scripts/image-srcset-chrome-control.mjs at
//! 1024x768: `load1` / `load2` load every image at device pixel ratio 1 / 2, `none1` loads
//! nothing). The host side is what turbo-surf's load path does: `imageHost.selectSource`
//! picks the URL to fetch and its density, `imageHost.naturalSize` turns the fetched size into
//! the natural size, `imageHost.setState` publishes it.
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("html_image_srcset_probe.js");
const CASES: &[u8] = include_bytes!("html_image_srcset_cases.json");
const CONTROL: &str = include_str!("html_image_srcset_chrome156.json");
/// The control's page URL, with the port dropped (the probe strips the origin).
const PAGE: &str = "http://127.0.0.1/page/";

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
    control["result"].clone()
}

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .to_rust_string_lossy(scope)
}

/// Install the cases page at [`PAGE`], run `setup` with the document's `imageHost` bound as
/// `imageHostForTest` (removed again), then the probe.
fn probe(setup: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let cases: serde_json::Value = serde_json::from_slice(CASES).unwrap();
    install_independent_html(scope, cases["html"].as_str().unwrap());
    let id = context.get_slot::<DomContextId>().unwrap().0;
    let document = {
        let _dom = DomScope::enter(id);
        event_handlers::install(scope, None, PAGE);
        wrap(scope, with_tree(Tree::root).unwrap())
    };
    let record = collection_records::get(scope, document)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let key = v8::String::new(scope, "imageHost").unwrap();
    let host = record.get(scope, key.into()).unwrap();
    let key = v8::String::new(scope, "imageHostForTest").unwrap();
    let global = context.global(scope);
    global.set(scope, key.into(), host);
    eval(scope, setup);
    global.delete(scope, key.into());
    let result = eval(
        scope,
        &format!("JSON.stringify({})", PROBE.trim().trim_end_matches(';')),
    );
    reset();
    serde_json::from_str(&result).unwrap()
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

/// The reflected attributes' shapes and conversions (the same in every run of the control).
#[test]
#[ignore = "turbo-dom#71: fails until the fix"]
fn srcset_attributes_reflect_like_chrome156() {
    let control = control();
    let actual = probe("true");
    for section in ["shape", "reflect", "width", "height", "receivers"] {
        let found = mismatches(
            &serde_json::json!({ section: &actual[section] }),
            &serde_json::json!({ section: &control["load1"][section] }),
        );
        assert!(found.is_empty(), "{}", found.join("\n"));
    }
}

/// Without host state (turbo-test: nothing loads) every image reports Chrome's answer when
/// nothing loads: the selected URL, complete, 0x0.
#[test]
#[ignore = "turbo-dom#71: fails until the fix"]
fn without_a_host_current_src_is_the_selected_source() {
    let control = control();
    let actual = probe("true");
    let found = mismatches(&actual["images"], &control["none1"]["images"]);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// A host that loads the selected source (turbo-surf) reports Chrome's currentSrc and natural
/// size at device pixel ratio 1 and 2. The control's server answers `/img/<W>x<H>/…` with a
/// W x H image and fails everything else.
#[test]
#[ignore = "turbo-dom#71: fails until the fix"]
fn host_loads_of_the_selected_source_match_chrome156_at_dpr_1_and_2() {
    let control = control();
    for (label, dpr) in [("load1", 1), ("load2", 2)] {
        let setup = format!(
            "(()=>{{for(const img of document.querySelectorAll('img')){{const chosen=imageHostForTest.selectSource(img,{{devicePixelRatio:{dpr}}});if(!chosen)continue;const size=/^http:\\/\\/127\\.0\\.0\\.1\\/img\\/(\\d+)x(\\d+)\\//.exec(chosen.url);const natural=size?imageHostForTest.naturalSize(+size[1],+size[2],chosen.density):{{naturalWidth:0,naturalHeight:0}};imageHostForTest.setState(img,{{complete:true,naturalWidth:natural.naturalWidth,naturalHeight:natural.naturalHeight,currentSrc:chosen.url}});}}return true;}})()"
        );
        let actual = probe(&setup);
        let found = mismatches(&actual["images"], &control[label]["images"]);
        assert!(found.is_empty(), "{label}:\n{}", found.join("\n"));
    }
}

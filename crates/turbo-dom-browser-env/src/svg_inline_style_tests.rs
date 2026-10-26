use super::*;
use sha2::{Digest, Sha256};

/// turbo-dom#87 (from miaskiewicz/turbo-test#79): `style` and `dataset` live on HTMLElement,
/// SVGElement and MathMLElement (Chrome 156 control), so `svg.style.flexGrow = '1'` works (turbo-test#79 item 4).
#[test]
fn svg_and_mathml_elements_have_inline_style_and_dataset_like_chrome() {
    super::tests::init_v8();
    let probe = include_str!("svg_inline_style_probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("svg_inline_style_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.4");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><head></head><body></body>");
    let probe = probe.trim().trim_end_matches(';');
    let code = v8::String::new(scope, &format!("JSON.stringify({probe})")).unwrap();
    v8::tc_scope!(let scope,scope);
    let value = v8::Script::compile(scope, code, None)
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .run(scope)
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&value).unwrap();
    assert_eq!(actual, control["result"]);
    reset();
}

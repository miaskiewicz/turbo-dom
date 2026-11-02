use super::*;

#[test]
fn html_meta_reflection_descriptors_brands_and_private_attribute_reads_match_chrome() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><head></head><body></body>");
    let probe = include_str!("html_meta_probe.js")
        .trim()
        .trim_end_matches(';');
    let code = v8::String::new(scope, &format!("JSON.stringify({probe})")).unwrap();
    v8::tc_scope!(let scope,scope);
    let value = v8::Script::compile(scope, code, None)
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .run(scope)
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&value).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("html_meta_chrome155.json")).unwrap();
    assert_eq!(actual, expected["result"]);
    reset();
}

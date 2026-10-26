use super::*;
fn check(probe: &str, expected: &str) {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    if probe.contains("child.eval(") {
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(context.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<!doctype html><body></body>");
        }
        let global = context.global(scope);
        let child = child.global(scope);
        set_prop(scope, global, "child", child.into());
    }
    let foreign = parse_html_document(scope, "<!doctype html><html><body></body></html>", false);
    let key = v8::String::new(scope, "__adoptionForeignDoc").unwrap();
    context.global(scope).set(scope, key.into(), foreign.into());
    let probe=probe.replace("new DOMParser().parseFromString(\"<!doctype html><html><body></body></html>\", \"text/html\")","__adoptionForeignDoc");
    let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    v8::tc_scope!(let scope,scope);
    let source = v8::String::new(scope, &source).unwrap();
    let actual = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    let actual = actual.unwrap_or_else(|| {
        panic!(
            "adoption control failed: {:?}",
            scope
                .exception()
                .map(|error| error.to_rust_string_lossy(scope))
        )
    });
    let actual: serde_json::Value =
        serde_json::from_str(&actual.to_rust_string_lossy(scope)).unwrap();
    let expected: serde_json::Value = serde_json::from_str(expected).unwrap();
    assert_eq!(actual, expected["result"]);
    reset();
}
#[test]
fn observable_adoption_arrays_match_independent_chrome_mutation_and_conversion_control() {
    check(
        include_str!("css_adoption_probe.js"),
        include_str!("css_adoption_chrome155.json"),
    );
}
#[test]
fn shadow_and_independent_documents_match_chrome_adoption_ownership_and_integrity() {
    check(
        include_str!("css_adoption_ownership_probe.js"),
        include_str!("css_adoption_ownership_chrome155.json"),
    );
}

#[test]
fn shadow_adoption_clears_constructed_sheets_without_public_hooks_matching_chrome() {
    check(
        include_str!("css_adoption_lifecycle_probe.js"),
        include_str!("css_adoption_lifecycle_chrome155.json"),
    );
}

use super::*;

fn evaluate(probe: &str) -> serde_json::Value {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    // turbo-test has no CSS namespace. The probe's CSS.supports column checks the
    // shared selector predicate that hosts install behind `CSS.supports`.
    fn supports(
        scope: &mut v8::PinScope,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) {
        let condition = args.get(0).to_rust_string_lossy(scope);
        let selector = condition
            .trim()
            .strip_prefix("selector(")
            .and_then(|rest| rest.strip_suffix(')'));
        rv.set_bool(selector.is_some_and(css_selector_syntax::supports_selector));
    }
    let global = context.global(scope);
    let css = v8::Object::new(scope);
    let function = v8::Function::new(scope, supports).unwrap();
    set_prop(scope, css, "supports", function.into());
    set_prop(scope, global, "CSS", css.into());
    let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    v8::tc_scope!(let scope,scope);
    let source = v8::String::new(scope, &source).unwrap();
    let actual = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    let actual = actual.unwrap_or_else(|| {
        panic!(
            "CSS control failed: {:?}",
            scope
                .exception()
                .map(|error| error.to_rust_string_lossy(scope))
        )
    });
    let actual = actual.to_rust_string_lossy(scope);
    reset();
    serde_json::from_str(&actual).unwrap()
}

fn fixture(json: &str, probe: &str) -> serde_json::Value {
    use sha2::{Digest, Sha256};
    let expected: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(expected["browser"], "Chrome/156.0.8078.47");
    assert_eq!(
        expected["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    expected["result"].clone()
}

#[test]
fn selector_admission_matches_chrome156_pseudo_inventory() {
    let probe = include_str!("css_selector_admission_probe.js");
    let expected = fixture(include_str!("css_selector_admission_chrome156.json"), probe);
    let actual = evaluate(probe);
    let (expected, actual) = (expected.as_array().unwrap(), actual.as_array().unwrap());
    assert_eq!(expected.len(), actual.len());
    let diverged: Vec<_> = expected
        .iter()
        .zip(actual)
        .filter(|(expected, actual)| expected != actual)
        .map(|(expected, actual)| format!("chrome {expected}\n  ours {actual}"))
        .collect();
    assert!(diverged.is_empty(), "{}", diverged.join("\n"));
}

#[test]
fn implicit_color_inheritance_follows_cssom_mutations_like_chrome156() {
    let probe = include_str!("css_inherited_color_probe.js");
    let expected = fixture(include_str!("css_inherited_color_chrome156.json"), probe);
    assert_eq!(evaluate(probe), expected);
}

#[test]
fn implicit_visibility_inheritance_reads_the_edited_rule() {
    let value = evaluate(
        r#"(()=>{
      const style=document.createElement('style');style.textContent='.p{visibility:hidden}';document.head.append(style);
      document.body.innerHTML="<div class='p'><span><b id='leaf'></b></span></div>";
      const retained=getComputedStyle(document.getElementById('leaf')),rows=[retained.visibility];
      style.sheet.cssRules[0].style.visibility='visible';rows.push(retained.visibility);
      style.sheet.insertRule('span{visibility:inherit}',1);style.sheet.cssRules[0].style.visibility='collapse';rows.push(retained.visibility);
      style.sheet.deleteRule(0);rows.push(retained.visibility);
      return rows;
    })()"#,
    );
    assert_eq!(
        value,
        serde_json::json!(["hidden", "visible", "collapse", ""])
    );
}

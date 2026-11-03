use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;
use sha2::{Digest, Sha256};

// Runs the SAME probe that scripts/select-element-chrome-control.mjs captured in Chrome 156 and
// compares every phase and pseudo-class against the saved control.
#[test]
fn form_pseudo_classes_match_chrome156_control() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let cases_bytes = include_bytes!("form_pseudo_cases.json");
    let probe = include_str!("form_pseudo_probe.js");
    let cases: serde_json::Value = serde_json::from_slice(cases_bytes).unwrap();
    install_independent_html(scope, cases[0]["html"].as_str().unwrap());
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, probe).unwrap();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", thrown(scope.exception(), scope)));
    let actual = v8::json::stringify(scope, result)
        .unwrap()
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&actual).unwrap();
    let control: serde_json::Value =
        serde_json::from_str(include_str!("form_pseudo_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases_bytes))
    );
    let expected = &control["result"][0]["result"];
    let mut mismatches = Vec::new();
    for (phase, chrome) in expected.as_object().unwrap() {
        match chrome.as_object() {
            Some(entries) => {
                for (key, value) in entries {
                    if &actual[phase][key] != value {
                        mismatches.push(format!(
                            "{phase} {key}\n  binding: {}\n  chrome:  {value}",
                            actual[phase][key]
                        ));
                    }
                }
            }
            None if &actual[phase] != chrome => mismatches.push(format!(
                "{phase}\n  binding: {}\n  chrome:  {chrome}",
                actual[phase]
            )),
            None => {}
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert_eq!(&actual, expected);
    reset();
}

// The classifier is private and reads only install-time captures: guest prototype changes
// between queries neither break nor observe it.
#[test]
fn form_pseudo_classifier_ignores_guest_prototypes() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(
        scope,
        "<!doctype html><body><form><input id=a type=checkbox checked><input id=b type=radio name=r><input id=c required></form>",
    );
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(
        scope,
        r#"(() => {
          const seen = [];
          Map.prototype.get = function () { seen.push('map'); return undefined; };
          Array.prototype.indexOf = function () { seen.push('indexOf'); return -1; };
          Object.defineProperty(Array.prototype, '0', { set(value) { seen.push('setter'); }, configurable: true });
          Object.defineProperty(HTMLOptionElement.prototype, 'selected', { get() { seen.push('selected'); return true; }, configurable: true });
          const ids = (selector) => Array.from(document.querySelectorAll(selector), (element) => element.id).join();
          const result = [ids(':checked'), ids(':indeterminate'), ids(':invalid'), ids('input:read-write'),
            typeof globalThis.__captureFormPseudo, typeof globalThis.__captureFormInternals];
          delete Array.prototype[0];
          return JSON.stringify([result, seen]);
        })()"#,
    )
    .unwrap();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", thrown(scope.exception(), scope)));
    assert_eq!(
        result.to_rust_string_lossy(scope),
        r#"[["a","b",",c","c","undefined","undefined"],[]]"#
    );
    reset();
}

const SUBJECT_LOCAL_PAGE: &str = "<!doctype html><body><form id=f><fieldset id=fs disabled><div id=d>\
<input id=a required><span id=s1></span><input id=b type=checkbox checked><span id=s2></span></div></fieldset>\
<input id=r1 type=radio name=g><input id=r2 type=radio name=g><input id=c><textarea id=t readonly></textarea>\
<select id=sel><option id=o1>x</option><option id=o2 selected disabled>y</option></select>\
<fieldset id=fs2><div><input id=in2 required></div></fieldset><x-widget id=w></x-widget></form>\
<div id=far><input id=e disabled><p id=p><span id=deep></span></p></div>\
<form id=g><input id=out form=f required><p><span id=gs></span></p></form>\
<form id=h><p><span id=hs></span></p></form><input id=out2 form=h required></body>";

/// The live answer of matches()/closest() for each selector and element, compared with the
/// answer querySelectorAll gives (every element of the document classified).
const SUBJECT_LOCAL_PROBE: &str = r#"(() => {
  const selectors = [':disabled', ':enabled', ':checked', ':required', ':optional', ':read-only',
    ':read-write', ':default', ':disabled + span', ':disabled ~ span', ':checked ~ span',
    ':disabled span', ':disabled > *', 'fieldset:disabled input', ':not(:disabled)',
    ':is(:required, :checked)', ':indeterminate', 'form:invalid input', 'form:valid input',
    ':invalid', ':valid', 'form:invalid', 'form:valid', 'form:invalid span', 'form:valid span',
    'fieldset:invalid', 'fieldset:valid input', 'fieldset:invalid div', ':user-invalid', ':has(:checked)', 'div:has(> :disabled) span', 'p:not(:read-write) span',
    ':placeholder-shown', 'option:disabled', 'option:checked'];
  const mismatches = [];
  const elements = Array.from(document.querySelectorAll('*'));
  for (const selector of selectors) {
    const all = Array.from(document.querySelectorAll(selector));
    for (const element of elements) {
      const matched = element.matches(selector);
      if (matched !== all.includes(element)) mismatches.push(`${selector} matches #${element.id}: ${matched}`);
      let expected = null;
      for (let node = element; node && node.nodeType === 1; node = node.parentNode) if (all.includes(node)) { expected = node; break; }
      if (element.closest(selector) !== expected) mismatches.push(`${selector} closest #${element.id}`);
    }
  }
  return JSON.stringify(mismatches);
})()"#;

fn run_page_script(scope: &mut v8::PinScope, source: &str) -> String {
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, source).unwrap();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", thrown(scope.exception(), scope)));
    result.to_rust_string_lossy(scope)
}

#[test]
fn subject_local_matches_and_closest_agree_with_query_selector_all() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, SUBJECT_LOCAL_PAGE);
    assert_eq!(run_page_script(scope, SUBJECT_LOCAL_PROBE), "[]");
    reset();
}

// matches()/closest() classify only the elements a match can depend on (the subject's inclusive
// ancestors), not the whole document: user-event asks `element.matches(':disabled')` on every
// pointer step, and classifying a large form's every control each time was quadratic.
#[test]
fn matches_classifies_only_the_subjects_ancestors() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let inputs = "<input>".repeat(300);
    install_independent_html(
        scope,
        &format!(
            "<!doctype html><body><form>{inputs}<div><button id=x>go</button></div></form></body>"
        ),
    );
    let classified = |scope: &mut v8::PinScope, source: &str| {
        form_pseudo::CLASSIFIED.with(|count| count.set(0));
        run_page_script(scope, source);
        form_pseudo::CLASSIFIED.with(|count| count.get())
    };
    let x = "document.getElementById('x')";
    // html, body, form, div, button
    assert_eq!(classified(scope, &format!("{x}.matches(':disabled')")), 5);
    assert_eq!(classified(scope, &format!("{x}.closest(':enabled')")), 5);
    // A sibling combinator adds the preceding siblings of the subject and its ancestors.
    // (html, head, body, form, 300 inputs, div, button: the inputs and head precede ancestors)
    assert_eq!(
        classified(scope, &format!("{x}.matches(':enabled ~ div > button')")),
        306
    );
    // :has(), the form-wide :invalid and the radio groups of :indeterminate classify the tree.
    let all = 306;
    assert_eq!(
        classified(scope, &format!("{x}.matches(':has(:disabled)')")),
        all
    );
    // A form's validity reads its controls (not its other descendants): 5 + the 300 inputs.
    assert_eq!(
        classified(scope, &format!("{x}.matches('form:invalid button')")),
        305
    );
    assert_eq!(
        classified(scope, &format!("{x}.matches(':indeterminate')")),
        all
    );
    assert_eq!(
        classified(scope, "document.querySelectorAll(':disabled').length"),
        all
    );
    reset();
}

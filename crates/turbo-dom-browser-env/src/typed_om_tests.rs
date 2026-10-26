use super::*;
use sha2::{Digest, Sha256};

fn run_probe(html: &str, probe: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    // Pin the UA font to the macOS Chrome 156 control's capture ("Times"/16px), as
    // css_live_tests does, so computed font-family is identical on every host.
    assert!(set_computed_font_defaults(context, "Times", 16.0));
    install_independent_html(scope, html);
    v8::tc_scope!(let scope,scope);
    let source = v8::String::new(scope, probe).unwrap();
    let Some(result) =
        v8::Script::compile(scope, source, None).and_then(|script| script.run(scope))
    else {
        let message = scope.exception().unwrap().to_rust_string_lossy(scope);
        reset();
        panic!("{message}");
    };
    let actual = v8::json::stringify(scope, result)
        .unwrap()
        .to_rust_string_lossy(scope);
    reset();
    serde_json::from_str(&actual).unwrap()
}

fn control(json: &str, probe: &str, cases: &[u8]) -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases))
    );
    control["result"][0]["result"].clone()
}

/// Replace `actual[section][index]` after asserting the exact known divergence.
fn pin(
    actual: &mut serde_json::Value,
    expected: &serde_json::Value,
    section: &str,
    label: &str,
    check: impl Fn(&serde_json::Value, &serde_json::Value),
) {
    let rows = actual[section].as_array_mut().unwrap();
    let row = rows.iter_mut().find(|row| row[0] == label).unwrap();
    let chrome = expected[section]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row[0] == label)
        .unwrap();
    assert_ne!(
        &*row, chrome,
        "{section}/{label} now matches Chrome; drop the pin"
    );
    check(row, chrome);
    *row = chrome.clone();
}

// Runs the SAME probe scripts/typed-om-chrome-control.mjs captured in Chrome 156 and compares
// every section; the three pinned rows are environment or engine deltas asserted exactly.
#[test]
fn typed_om_matches_chrome156_control() {
    let probe = include_str!("typed_om_probe.js");
    let cases_bytes = include_bytes!("typed_om_cases.json");
    let expected = control(include_str!("typed_om_chrome156.json"), probe, cases_bytes);
    let cases: serde_json::Value = serde_json::from_slice(cases_bytes).unwrap();
    let mut actual = run_probe(cases[0]["html"].as_str().unwrap(), probe);
    // KNOWN_GAP: Lightning CSS's <integer> grammar rejects calc(), so the binding's
    // CSSStyleDeclaration drops the `calc(1.5)` Typed OM writes for an out-of-range integer.
    for (label, property) in [
        ("set order fraction", "order"),
        ("set z-index fraction", "z-index"),
    ] {
        pin(&mut actual, &expected, "inline", label, |ours, chrome| {
            let attribute = |row: &serde_json::Value| row[2]["attr"].as_str().unwrap().to_string();
            assert!(attribute(chrome).ends_with(&format!("{property}: calc(1.5);")));
            assert_eq!(attribute(ours), "width: 10px; color: red; --brand: blue");
        });
    }
    // KNOWN_GAP: Chrome does not restyle after DeclaredStylePropertyMap::SetProperty (it only
    // notifies the sheet), so its computed width stays at the old rule value; the binding's
    // computed style reads the mutated rule.
    pin(
        &mut actual,
        &expected,
        "rule",
        "set reaches computed",
        |ours, chrome| {
            assert_eq!(chrome[1]["s"], "10px");
            assert_eq!(ours[1]["s"], "42px");
        },
    );
    // KNOWN_GAP: the standalone harness document has an about:blank base URL (it ignores the
    // <base> element), so computed url() stays relative; Chrome resolves it against <base>.
    pin(
        &mut actual,
        &expected,
        "computedStyled",
        "background-image",
        |ours, chrome| {
            assert_eq!(
                chrome[1][0]["s"],
                "url(\"https://typed-om.test/assets/a.png\")"
            );
            assert_eq!(ours[1][0]["s"], "url(\"a.png\")");
        },
    );
    let mismatches: Vec<String> = expected
        .as_object()
        .unwrap()
        .iter()
        .flat_map(|(key, value)| {
            let ours = &actual[key.as_str()];
            match (value.as_array(), ours.as_array()) {
                (Some(rows), Some(our_rows)) if rows.len() == our_rows.len() => rows
                    .iter()
                    .zip(our_rows)
                    .filter(|(a, b)| a != b)
                    .map(|(a, b)| format!("{key}\n  binding: {b}\n  chrome:  {a}"))
                    .collect::<Vec<_>>(),
                _ if ours != value => vec![format!("{key}\n  binding: {ours}\n  chrome:  {value}")],
                _ => vec![],
            }
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert_eq!(&actual, &expected);
}

// The registry is generated from the Chrome 156 control it records.
#[test]
fn typed_om_registry_tracks_its_chrome156_control() {
    let registry: serde_json::Value =
        serde_json::from_str(include_str!("typed_om_properties.json")).unwrap();
    let control_json = include_str!("typed_om_registry_chrome156.json");
    let probe = include_str!("typed_om_registry_probe.js");
    let measured = control(
        control_json,
        probe,
        include_bytes!("typed_om_registry_cases.json"),
    );
    assert_eq!(
        registry["source"]["controlSha256"],
        format!("{:x}", Sha256::digest(control_json.as_bytes()))
    );
    let computable: Vec<&str> = measured["computed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry[0].as_str().unwrap())
        .collect();
    let listed: Vec<&str> = registry["computable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    assert_eq!(listed, computable);
    assert_eq!(
        registry["properties"].as_object().unwrap().len(),
        measured["enabled"].as_array().unwrap().len()
    );
}

// Typed OM keeps its own captures: page patches of CSSStyleDeclaration, getComputedStyle and
// Array.prototype do not change what the maps read or write.
#[test]
fn typed_om_ignores_page_patches() {
    let result = run_probe(
        "<!doctype html><body><div id=t style='width: 5px'></div></body>",
        r#"(() => {
          const t = document.getElementById('t'), map = t.attributeStyleMap;
          CSSStyleDeclaration.prototype.setProperty = () => { throw Error('patched setProperty'); };
          CSSStyleDeclaration.prototype.getPropertyValue = () => 'patched';
          window.getComputedStyle = () => { throw Error('patched getComputedStyle'); };
          Array.prototype[Symbol.iterator] = function* () { throw Error('patched iterator'); };
          map.set('height', new CSSUnitValue(7, 'px'));
          return [String(map.get('width')), t.getAttribute('style'), String(t.computedStyleMap().get('height')), map.size];
        })()"#,
    );
    assert_eq!(
        result,
        serde_json::json!(["5px", "width: 5px; height: 7px;", "7px", 2])
    );
}

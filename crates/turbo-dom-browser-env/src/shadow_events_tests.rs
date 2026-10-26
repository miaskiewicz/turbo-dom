//! turbo-dom#66: the DOM event path across shadow trees, against the Chrome 156 control
//! `shadow_events_{probe.js,cases.json,chrome156.json}` (recorded by
//! scripts/select-element-chrome-control.mjs): composed events cross shadow roots into their
//! hosts (uncomposed ones stop at the root of the target's tree), each listener sees `target`
//! and `relatedTarget` retargeted against its own tree, hosts are AT_TARGET, `composedPath()`
//! hides closed shadow trees the listener is outside of, slotted nodes pass through their
//! slot, propagation stops where the retargeted `relatedTarget` is the host, and after
//! dispatch `target` / `relatedTarget` are cleared when they stayed inside a shadow tree.
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("shadow_events_probe.js");
const CASES: &str = include_str!("shadow_events_cases.json");
const CONTROL: &str = include_str!("shadow_events_chrome156.json");

fn rows(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                rows(&format!("{prefix}.{key}"), value, out);
            }
        }
        serde_json::Value::Array(items) => {
            out.push((format!("{prefix}.length"), items.len().to_string()));
            for (index, value) in items.iter().enumerate() {
                rows(&format!("{prefix}.{index}"), value, out);
            }
        }
        _ => out.push((prefix.to_owned(), value.to_string())),
    }
}

#[test]
#[ignore = "turbo-dom#66: fails until the fix"]
fn shadow_event_paths_match_chrome156_control() {
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
        format!("{:x}", Sha256::digest(CASES.as_bytes()))
    );
    let cases: serde_json::Value = serde_json::from_str(CASES).unwrap();
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, cases[0]["html"].as_str().unwrap());
    let code = format!("JSON.stringify({})", PROBE.trim().trim_end_matches(';'));
    let code = v8::String::new(scope, &code).unwrap();
    v8::tc_scope!(let scope, scope);
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .to_rust_string_lossy(scope);
    reset();
    let actual: serde_json::Value = serde_json::from_str(&value).unwrap();
    let (mut want, mut got) = (Vec::new(), Vec::new());
    rows("", &control["result"][0]["result"], &mut want);
    rows("", &actual, &mut got);
    let got: std::collections::BTreeMap<_, _> = got.into_iter().collect();
    let found: Vec<String> = want
        .into_iter()
        .filter(|(path, value)| got.get(path) != Some(value))
        .map(|(path, value)| {
            let binding = got
                .get(&path)
                .cloned()
                .unwrap_or_else(|| "<missing>".into());
            format!("{path}: binding {binding}, chrome {value}")
        })
        .collect();
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

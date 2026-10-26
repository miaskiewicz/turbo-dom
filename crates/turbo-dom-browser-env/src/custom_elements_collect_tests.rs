//! turbo-surf#41: parsed `is=` elements stay "undefined" (`:not(:defined)`) across native
//! storage collection, against the Chrome 156 control `custom_elements_defined_chrome156.json`
//! (probe + cases recorded by scripts/select-element-chrome-control.mjs). turbo-surf's page
//! session collects native storage at the first host boundary after a V8 GC
//! (`collect_native_storage`), which re-materializes the parsed nodes; reproduced here with the
//! same sequence: page install, a script, a GC, the collection, then the probe.
use super::*;
use turbo_dom_parser::rtdom::tree::CollectionBudget;

const PROBE: &str = include_str!("custom_elements_defined_probe.js");
const CASES: &[u8] = include_bytes!("custom_elements_defined_cases.json");
const CONTROL: &str = include_str!("custom_elements_defined_chrome156.json");

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .to_rust_string_lossy(scope)
}

#[test]
#[ignore = "turbo-surf#41: fails until the fix"]
fn defined_matches_chrome156_after_native_storage_collection() {
    let control: serde_json::Value = serde_json::from_str(CONTROL).unwrap();
    let cases: serde_json::Value = serde_json::from_slice(CASES).unwrap();
    super::tests::init_v8();
    let mut isolate = v8::Isolate::new(Default::default());
    let context = {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        // The page load: parse + install, then a page script reads the parsed state.
        install_html(scope, cases[0]["html"].as_str().unwrap());
        assert_eq!(
            eval(
                scope,
                "String(document.getElementById('btn').matches(':defined'))"
            ),
            "false"
        );
        v8::Global::new(scope, context)
    };
    // A V8 GC, then the host boundary's native storage collection (turbo-surf storage_gc).
    for _ in 0..3 {
        isolate.low_memory_notification();
    }
    {
        v8::scope!(let scope, &mut isolate);
        let local = v8::Local::new(scope, &context);
        let scope = &mut v8::ContextScope::new(scope, local);
        let budget = CollectionBudget {
            max_nodes: 100_000,
            max_edges: 1_000_000,
            max_payload_bytes: 16 * 1024 * 1024,
        };
        let report = collect_native_storage(scope, budget, 32).unwrap();
        assert!(!report.documents.is_empty(), "the document was collected");
        let actual: serde_json::Value = serde_json::from_str(&eval(
            scope,
            &format!("JSON.stringify({})", PROBE.trim().trim_end_matches(';')),
        ))
        .unwrap();
        let expected = &control["result"][0]["result"];
        let mismatches: Vec<String> = expected
            .as_object()
            .unwrap()
            .iter()
            .filter(|(key, value)| &actual[key.as_str()] != *value)
            .map(|(key, value)| format!("{key}\n  binding: {}\n  chrome:  {value}", actual[key]))
            .collect();
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    }
    reset();
    drop(context);
}

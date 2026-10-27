//! turbo-dom#90: `rtdom::css_shorthand` against the Chrome 156 control
//! `tests/fixtures/css_shorthand_chrome156.json` (probe `css-shorthand-probe.js`, cases
//! `css-shorthand-cases.json` from `scripts/make-css-shorthand-cases.py`, recorded with
//! `crates/turbo-dom-browser-env/scripts/select-element-chrome-control.mjs`): the shorthand
//! and alias tables, and every scenario's `cssText`, longhands, priorities, shorthand values,
//! return values and its re-assembly from longhands, replayed on a `DeclarationBlock`.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::css_shorthand::{
    longhand_alias, longhand_value, longhands, DeclarationBlock, Spelled, LONGHAND_ALIASES,
};

const PROBE: &str = include_str!("fixtures/css-shorthand-probe.js");
const CASES: &str = include_str!("fixtures/css-shorthand-cases.json");
const CONTROL: &str = include_str!("fixtures/css_shorthand_chrome156.json");

/// Scenarios whose Chrome result differs for a documented reason, by id.
const KNOWN_GAPS: &[&str] = &[
    // turbo-dom#117: math functions, hsl() hue, place-content / offset / columns /
    // timeline-trigger spellings (the npm port shares them).
    "margin|calc(1px + abs(1px))",
    "margin|calc(min(1px) * min(2px))",
    "margin|min(1px 2px)",
    "margin|min(1px, calc(1px + 1em))",
    "place-content|baseline center",
    "offset|path('M 0 0') 10px 0",
    "offset|path(M 0 0)",
    "offset|path('M 0 0') 10px auto 0",
    "background|hsl(1px 50% 50%)",
    "background|hsl(10% 50% 50%)",
    "transition|opacity cubic-bezier(calc(1px), 0, 1, 1)",
    "transition|opacity cubic-bezier(min(0.5, 1), 0, 1, 1)",
    "transition|opacity linear(calc(0), 1)",
    "transition|opacity linear(0, 1 calc(50%))",
    "cov-columns-wrap",
    "cov-columns-nowrap",
    "cov-trigger-ranges-mismatch",
];

fn sha(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// Longhands set directly: `longhand_value`'s spelling (turbo-dom#97); the control's
/// longhands are all valid, so an unchecked spelling is taken as admitted.
fn longhand(name: &str, value: &str) -> Option<String> {
    match longhand_value(name, value) {
        Spelled::Valid(text) | Spelled::Unchecked(text) => Some(text),
        Spelled::Invalid => None,
    }
}

fn record(block: &DeclarationBlock, table: &Value) -> Value {
    let mut items = Vec::new();
    let mut shorthands = serde_json::Map::new();
    for index in 0..block.len() {
        let name = block.item(index).unwrap();
        let value = block.property_value(name);
        let priority = block.property_priority(name);
        items.push(if priority.is_empty() {
            json!([name, value])
        } else {
            json!([name, value, priority])
        });
        for (shorthand, list) in table.as_object().unwrap() {
            if list
                .as_array()
                .unwrap()
                .iter()
                .any(|longhand| longhand == name)
                && !shorthands.contains_key(shorthand)
            {
                shorthands.insert(shorthand.clone(), json!(block.property_value(shorthand)));
            }
        }
    }
    json!({"cssText": block.css_text(), "items": items, "shorthands": shorthands})
}

fn run(ops: &[Value]) -> (DeclarationBlock, Vec<Value>) {
    let mut block = DeclarationBlock::new();
    let mut returns = Vec::new();
    for op in ops {
        let op = op.as_array().unwrap();
        let text = |index: usize| op.get(index).and_then(Value::as_str).unwrap_or("");
        match text(0) {
            "css" => block = DeclarationBlock::parse(text(1), &longhand),
            "set" => {
                block.set_property(text(1), text(2), text(3), &longhand);
            }
            "remove" => returns.push(json!(block.remove_property(text(1)))),
            "get" => returns.push(json!(block.property_value(text(1)))),
            "priority" => returns.push(json!(block.property_priority(text(1)))),
            "recss" => returns.push(json!(
                DeclarationBlock::parse(&block.css_text(), &longhand).css_text()
            )),
            other => panic!("unknown op {other}"),
        }
    }
    (block, returns)
}

fn differences(path: &str, actual: &Value, expected: &Value, out: &mut Vec<String>) {
    if actual != expected {
        out.push(format!(
            "{path}\n    ours:   {actual}\n    chrome: {expected}"
        ));
    }
}

#[test]
fn shorthands_expand_and_serialize_like_chrome156() {
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.4");
    assert_eq!(control["errors"], json!([]));
    assert_eq!(control["probeSha256"], sha(PROBE));
    assert_eq!(control["casesSha256"], sha(CASES));
    let result = &control["result"][0]["result"];
    let table = &result["table"];

    // The shorthand table is Chrome's, and so is the longhand alias table.
    let ours: serde_json::Map<String, Value> = turbo_dom::rtdom::css_shorthand::shorthand_names()
        .map(|name| (name.to_string(), json!(longhands(name).unwrap())))
        .collect();
    assert_eq!(&Value::Object(ours), table);
    let aliases: serde_json::Map<String, Value> = LONGHAND_ALIASES
        .iter()
        .map(|(alias, name)| ((*alias).to_string(), json!(name)))
        .collect();
    assert_eq!(&Value::Object(aliases), &result["aliases"]);
    assert!(LONGHAND_ALIASES
        .windows(2)
        .all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(longhand_alias("word-wrap"), Some("overflow-wrap"));
    assert_eq!(longhand_alias("overflow-wrap"), None);

    let cases: Value = serde_json::from_str(CASES).unwrap();
    let html = cases[0]["html"].as_str().unwrap();
    let json_start = html.find("id=\"scenarios\">").unwrap() + "id=\"scenarios\">".len();
    let json_end = html[json_start..].find("</script>").unwrap() + json_start;
    let scenarios: Vec<Value> =
        serde_json::from_str(&html[json_start..json_end].replace("<\\/", "</")).unwrap();
    let rows = result["results"].as_array().unwrap();
    assert_eq!(scenarios.len(), rows.len());

    let mut failures = Vec::new();
    let mut failed_ids = Vec::new();
    for (scenario, row) in scenarios.iter().zip(rows) {
        let id = scenario["id"].as_str().unwrap();
        assert_eq!(row["id"], id);
        let mut out = Vec::new();
        let (block, returns) = run(scenario["ops"].as_array().unwrap());
        assert_eq!(block.is_empty(), block.item(0).is_none(), "{id}");
        let ours = record(&block, table);
        for key in ["cssText", "items", "shorthands"] {
            differences(key, &ours[key], &row[key], &mut out);
        }
        if let Some(expected) = row.get("returns") {
            differences("returns", &json!(returns), expected, &mut out);
        }
        if let Some(assembled) = row.get("assembled") {
            let items: Vec<(String, String)> = (0..block.len())
                .map(|index| {
                    let name = block.item(index).unwrap().to_string();
                    let value = block.property_value(&name);
                    (name, value)
                })
                .collect();
            let build = |list: &[(String, String, &str)]| {
                let mut target = DeclarationBlock::new();
                for (name, value, priority) in list.iter().rev() {
                    target.set_property(name, value, priority, &longhand);
                }
                record(&target, table)
            };
            let all: Vec<_> = items
                .iter()
                .map(|(n, v)| (n.clone(), v.clone(), ""))
                .collect();
            let mut keyword = all.clone();
            keyword[0].1 = "inherit".into();
            let mut important = all.clone();
            important[0].2 = "important";
            for (label, built) in [
                ("all", build(&all)),
                ("partial", build(&all[1..])),
                ("keyword", build(&keyword)),
                ("important", build(&important)),
            ] {
                for key in ["cssText", "items", "shorthands"] {
                    differences(
                        &format!("assembled.{label}.{key}"),
                        &built[key],
                        &assembled[label][key],
                        &mut out,
                    );
                }
            }
        }
        if !out.is_empty() {
            failed_ids.push(id.to_string());
            if !KNOWN_GAPS.contains(&id) {
                failures.push(format!("{id}:\n  {}", out.join("\n  ")));
            }
        }
    }
    for rule in result["rules"].as_array().unwrap() {
        let block = DeclarationBlock::parse(rule[0].as_str().unwrap(), &longhand);
        assert_eq!(
            block.css_text(),
            rule[2].as_str().unwrap(),
            "rule {}",
            rule[0]
        );
        assert_eq!(
            block.len(),
            rule[3].as_u64().unwrap() as usize,
            "rule {}",
            rule[0]
        );
    }
    let stale: Vec<_> = KNOWN_GAPS
        .iter()
        .filter(|id| !failed_ids.iter().any(|failed| failed == *id))
        .collect();
    assert!(stale.is_empty(), "stale KNOWN_GAPS: {stale:?}");
    assert!(
        failures.is_empty(),
        "{} of {} scenarios differ from Chrome:\n{}",
        failures.len(),
        scenarios.len(),
        failures.join("\n")
    );
}

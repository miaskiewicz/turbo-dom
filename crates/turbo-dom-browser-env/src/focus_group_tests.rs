use super::super::reference_target::probe_harness::{control, differences, run};

const PROBE: &str = include_str!("focus_group_probe.js");

/// focusGroup/focusGroupStart reflection and trusted arrow/Home/End navigation among a
/// focusgroup's items (behaviors, axes, wrapping, direction, nesting, opt-out, consumers).
#[test]
fn focus_groups_match_chrome156() {
    let expected = control(include_str!("focus_group_chrome156.json"), PROBE);
    let actual = run(PROBE);
    let mut out = Vec::new();
    let mut expected = expected;
    // KNOWN_GAP: the engine has no MathMLElement interface, so its focusGroup and
    // focusGroupStart accessors are absent (HTMLElement and SVGElement match Chrome).
    assert_eq!(expected["reflection"]["mathml"][0], "function");
    assert_eq!(
        expected["reflection"]["mathml"][1],
        expected["reflection"]["descriptors"][0].as_array().unwrap()[1..]
            .iter()
            .cloned()
            .collect::<serde_json::Value>()
    );
    assert_eq!(
        actual["reflection"]["mathml"],
        serde_json::json!(["undefined", null])
    );
    expected["reflection"]["mathml"] = actual["reflection"]["mathml"].clone();
    differences("result", &actual, &expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

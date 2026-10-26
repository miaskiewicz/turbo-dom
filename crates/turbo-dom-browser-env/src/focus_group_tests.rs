use super::super::reference_target::probe_harness::{control, differences, run};

const PROBE: &str = include_str!("focus_group_probe.js");

/// focusGroup/focusGroupStart reflection and trusted arrow/Home/End navigation among a
/// focusgroup's items (behaviors, axes, wrapping, direction, nesting, opt-out, consumers).
#[test]
fn focus_groups_match_chrome156() {
    let expected = control(include_str!("focus_group_chrome156.json"), PROBE);
    let actual = run(PROBE);
    let mut out = Vec::new();
    differences("result", &actual, &expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

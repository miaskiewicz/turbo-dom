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

/// turbo-dom#194: the ARIA element references, DOMTokenList, focusgroup, referenceTarget and
/// interest-invoker members (and the trusted keydown / hover / Escape paths behind them) keep
/// working when the page replaces every configurable builtin, like Chrome 156.
#[test]
fn members_survive_page_patched_builtins() {
    let probe = include_str!("patched_builtins_members_probe.js");
    let expected = control(include_str!("patched_builtins_members_chrome156.json"), probe);
    let actual = run(probe);
    let mut out = Vec::new();
    differences("result", &actual, &expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

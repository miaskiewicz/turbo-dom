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
    let expected = control(
        include_str!("patched_builtins_members_chrome156.json"),
        probe,
    );
    let actual = run(probe);
    let mut out = Vec::new();
    differences("result", &actual, &expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

/// turbo-dom#199: the captured-intrinsics rule, mechanically. With every configurable member of
/// every global constructor, prototype and namespace replaced, every interface member and
/// constructor reaches only the patched builtins Chrome 156 reaches (spec-observable argument
/// conversions); any other entry is engine JS calling a page-patchable builtin.
#[test]
fn every_member_avoids_page_patched_builtins() {
    let probe = include_str!("patched_builtins_sweep_probe.js");
    let expected = control(
        include_str!("patched_builtins_sweep_chrome156.json"),
        probe,
    );
    let actual = run(probe);
    assert_eq!(actual["patched"], true, "the sweep patched and exercised too little");
    assert_eq!(expected["patched"], true);
    // Chrome's entries are the spec-observable conversions; the binding may skip one of those
    // (it is not this rule's business), but every builtin it reaches must be one Chrome reaches.
    let mut out = Vec::new();
    for (member, reached) in actual["hits"].as_object().unwrap() {
        for builtin in reached.as_object().unwrap().keys() {
            if expected["hits"][member][builtin].is_null() {
                out.push(format!("{member} -> {builtin}"));
            }
        }
    }
    assert!(out.is_empty(), "engine JS reached page-patched builtins:\n{}", out.join("\n"));
}

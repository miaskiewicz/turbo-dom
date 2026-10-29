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
    // A [PutForwards] setter's first step is Get(O, id): Chrome stops at that patched getter, so
    // what the forwarded set reaches after it is unobservable in Chrome.
    let forwards = |member: &str| {
        let name = member.rsplit('.').next().unwrap();
        expected["hits"][member].as_object().is_some_and(|chrome| {
            chrome
                .keys()
                .any(|builtin| builtin.starts_with("get ") && builtin.ends_with(&format!(".prototype.{name}")))
        })
    };
    let mut out = Vec::new();
    for (member, reached) in actual["hits"].as_object().unwrap() {
        if forwards(member) {
            continue;
        }
        for builtin in reached.as_object().unwrap().keys() {
            if expected["hits"][member][builtin].is_null() {
                out.push(format!("{member} -> {builtin}"));
            }
        }
    }
    // The violations still open under #199 are listed; the list only shrinks.
    let known: std::collections::BTreeSet<&str> = include_str!("patched_builtins_sweep_known.txt")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let found: std::collections::BTreeSet<&str> = out.iter().map(String::as_str).collect();
    let new: Vec<_> = found.difference(&known).collect();
    let fixed: Vec<_> = known.difference(&found).collect();
    assert!(new.is_empty(), "engine JS reached page-patched builtins:\n{new:#?}");
    assert!(fixed.is_empty(), "fixed; delete from patched_builtins_sweep_known.txt:\n{fixed:#?}");
}

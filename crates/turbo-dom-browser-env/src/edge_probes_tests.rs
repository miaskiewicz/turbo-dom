//! Edge probes of the DOM / CSSOM binding against their Chrome 156 controls
//! (`*_probe.js` → `*_chrome156.json`, recorded by `scripts/trusted-input-chrome-control.mjs`
//! with the frozen Chrome 156.0.8078.47): manual / named slot assignment, pre-insertion
//! validity of the mutation methods, selector admission and serialization, and the CSSOM
//! at-rule parser. Every leaf must equal Chrome's except the KNOWN_GAP rows, which must still
//! differ exactly so (a fix removes its rows).
use crate::browser_env::reference_target::probe_harness::{control, differences, run};

/// The leaves where `actual` differs from `expected` and is not a listed gap, then the gaps
/// that no longer differ as listed (stale rows).
fn mismatches(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    gaps: &[(&str, &str)],
) -> Vec<String> {
    let mut out = Vec::new();
    differences("result", actual, expected, &mut out);
    let mut seen = Vec::new();
    out.retain(|entry| {
        let (path, rest) = entry.split_once('\n').unwrap_or((entry, ""));
        let binding = rest
            .lines()
            .find_map(|line| line.trim().strip_prefix("binding: "))
            .unwrap_or("");
        let known = gaps.contains(&(path, binding));
        if known {
            seen.push(path.to_owned());
        }
        !known
    });
    for (path, _) in gaps {
        if !seen.iter().any(|seen| seen == path) {
            out.push(format!("stale known gap: {path}"));
        }
    }
    out
}

fn check(probe: &str, json: &str, gaps: &[(&str, &str)]) {
    let expected = control(json, probe);
    assert_eq!(
        mismatches(&run(probe), &expected, gaps),
        Vec::<String>::new()
    );
}

#[test]
fn mismatches_reports_differences_extras_and_stale_gaps() {
    let expected = serde_json::json!({"a": [1, 2], "b": "x", "c": {"d": true}});
    let same = expected.clone();
    assert_eq!(mismatches(&same, &expected, &[]), Vec::<String>::new());
    let actual = serde_json::json!({"a": [1, 3], "b": "y", "c": {"d": true, "e": 0}});
    assert_eq!(
        mismatches(
            &actual,
            &expected,
            &[("result.b", "\"y\""), ("result.c.d", "false")]
        ),
        vec![
            "result.a[1]\n  binding: 3\n  chrome:  2".to_owned(),
            "result.c.e: extra in binding".to_owned(),
            "stale known gap: result.c.d".to_owned(),
        ]
    );
    // A gap row only excuses the binding value it names.
    assert_eq!(
        mismatches(
            &actual,
            &expected,
            &[
                ("result.a[1]", "4"),
                ("result.b", "\"y\""),
                ("result.c.e: extra in binding", "")
            ]
        ),
        vec![
            "result.a[1]\n  binding: 3\n  chrome:  2".to_owned(),
            "stale known gap: result.a[1]".to_owned(),
        ]
    );
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#133): a manually assigned node keeps its removed slot.
const SLOT_GAPS: &[(&str, &str)] = &[
    (
        "result.changes",
        "[\"s1\",\"s1\",\"s2\",\"s1\",\"s1\",\"s1\",\"s1\",\"s1\"]",
    ),
    ("result.slotRemoved[1]", "null"),
    ("result.slotRemoved[2]", "null"),
];

#[test]
fn shadow_slot_edges_match_chrome156() {
    check(
        include_str!("shadow_slots_edges_probe.js"),
        include_str!("shadow_slots_edges_chrome156.json"),
        SLOT_GAPS,
    );
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#132): Document pre-insert validity of replaceChildren.
const MUTATION_GAPS: &[(&str, &str)] = &[
    (
        "result.matrix.replaceChildren.withBoth<doctype[0]",
        "\"ok\"",
    ),
    (
        "result.matrix.replaceChildren.withBoth<doctype[1]",
        "\"html\"",
    ),
    (
        "result.matrix.replaceChildren.withBoth<element[0]",
        "\"ok\"",
    ),
    ("result.matrix.replaceChildren.withBoth<element[1]", "\"B\""),
    (
        "result.matrix.replaceChildren.withDoctype<doctype[0]",
        "\"ok\"",
    ),
    (
        "result.matrix.replaceChildren.withDoctype<doctype[1]",
        "\"html\"",
    ),
    (
        "result.matrix.replaceChildren.withElement<doctype[0]",
        "\"ok\"",
    ),
    (
        "result.matrix.replaceChildren.withElement<doctype[1]",
        "\"html\"",
    ),
    (
        "result.matrix.replaceChildren.withElement<element[0]",
        "\"ok\"",
    ),
    (
        "result.matrix.replaceChildren.withElement<element[1]",
        "\"b\"",
    ),
    (
        "result.placement.replaceChildrenDoctypeElement[0]",
        "\"ok\"",
    ),
    (
        "result.placement.replaceChildrenDoctypeElement[1]",
        "\"y,A\"",
    ),
];

#[test]
fn dom_mutation_validity_matches_chrome156() {
    check(
        include_str!("dom_mutation_validity_probe.js"),
        include_str!("dom_mutation_validity_chrome156.json"),
        MUTATION_GAPS,
    );
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#134): CSSOM at-rule cssText.
const SELECTOR_GAPS: &[(&str, &str)] = &[
    (
        "result.at.@container (width > 1px) { }[0]",
        "\"@container (width > 1px) {  }\"",
    ),
    (
        "result.at.@container (width > 1px) { }[1]",
        "\"@container (width > 1px) {  }\"",
    ),
    (
        "result.at.@counter-style none { system: cyclic }[0]",
        "\"@counter-style none { system: cyclic; }\"",
    ),
    (
        "result.at.@counter-style none { system: cyclic }[1]",
        "\"@counter-style none { system: cyclic; }\"",
    ),
    (
        "result.at.@font-face { font-feature-settings: \"liga\" 1 }[0]",
        "\"@font-face { }\"",
    ),
    (
        "result.at.@font-face { font-feature-settings: \"liga\" 1 }[1]",
        "\"@font-face { }\"",
    ),
    (
        "result.at.@font-face { font-variation-settings: \"wght\" 400 }[0]",
        "\"@font-face { }\"",
    ),
    (
        "result.at.@font-face { font-variation-settings: \"wght\" 400 }[1]",
        "\"@font-face { }\"",
    ),
    (
        "result.at.@font-face { font-variation-settings: normal }[0]",
        "\"@font-face { }\"",
    ),
    (
        "result.at.@font-face { font-variation-settings: normal }[1]",
        "\"@font-face { }\"",
    ),
    (
        "result.at.@keyframes \"k2\" { 0% { color: red } }[0]",
        "\"@keyframes \\\"k2\\\" { 0% { color: red; } }\"",
    ),
    (
        "result.at.@keyframes \"k2\" { 0% { color: red } }[1]",
        "\"@keyframes \\\"k2\\\" { 0% { color: red; } }\"",
    ),
    (
        "result.at.@keyframes k { from { color: red } }[0]",
        "\"@keyframes k { from { color: red; } }\"",
    ),
    (
        "result.at.@keyframes k { from { color: red } }[1]",
        "\"@keyframes k { from { color: red; } }\"",
    ),
    ("result.at.@layer a, b;[0]", "null"),
    ("result.at.@layer a, b;[1]", "\"SyntaxError\""),
    (
        "result.at.@scope (a) to (b) { }[0]",
        "\"@scope (a) to (b) {  }\"",
    ),
    (
        "result.at.@scope (a) to (b) { }[1]",
        "\"@scope (a) to (b) {  }\"",
    ),
    (
        "result.at.@starting-style { a { color: red } }[0]",
        "\"@starting-style { a { color: red; } }\"",
    ),
    (
        "result.at.@starting-style { a { color: red } }[1]",
        "\"@starting-style { a { color: red; } }\"",
    ),
    (
        "result.at.@view-transition { navigation: bogus }[0]",
        "\"@view-transition {  }\"",
    ),
    (
        "result.at.@view-transition { navigation: bogus }[1]",
        "\"@view-transition {  }\"",
    ),
];

#[test]
fn selector_and_at_rule_edges_match_chrome156() {
    check(
        include_str!("selector_edges_probe.js"),
        include_str!("selector_edges_chrome156.json"),
        SELECTOR_GAPS,
    );
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#135): `<?target data?>` parses as a bogus comment.
const SANITIZER_GAPS: &[(&str, &str)] = &[
    (
        "result.parseHTML[2][0]",
        "\"<div id=\\\"h\\\"><span>light</span></div><!--c--><!--?pi x?-->\"",
    ),
    (
        "result.setHTMLComments[0]",
        "\"<div id=\\\"h\\\"><span>light</span></div><!--c--><!--?pi x?-->\"",
    ),
    (
        "result.setHTMLPi[0]",
        "\"<div id=\\\"h\\\"><span>light</span></div>\"",
    ),
    (
        "result.setHTMLUnsafe[0]",
        "\"<div id=\\\"h\\\"><span onclick=\\\"y\\\">light</span></div><!--c--><!--?pi x?-->\"",
    ),
];

#[test]
fn sanitizer_edges_match_chrome156() {
    check(
        include_str!("sanitizer_edges_probe.js"),
        include_str!("sanitizer_edges_chrome156.json"),
        SANITIZER_GAPS,
    );
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#136, #137): querySelector rejects keyword view
/// transition types; adoptNode(shadowRoot) throws the wrong DOMException name.
const DOCUMENT_GAPS: &[(&str, &str)] = &[
    ("result.nodes.adoptShadow", "\"NotSupportedError\""),
    (
        "result.viewTransition.:active-view-transition-type(inherit)[0]",
        "\"SyntaxError\"",
    ),
    (
        "result.viewTransition.:active-view-transition-type(inherit)[1]",
        "\"SyntaxError\"",
    ),
    (
        "result.viewTransition.:active-view-transition-type(inherit)[2]",
        "\"SyntaxError\"",
    ),
    (
        "result.viewTransition.:active-view-transition-type(none)[0]",
        "\"SyntaxError\"",
    ),
    (
        "result.viewTransition.:active-view-transition-type(none)[1]",
        "\"SyntaxError\"",
    ),
    (
        "result.viewTransition.:active-view-transition-type(none)[2]",
        "\"SyntaxError\"",
    ),
];

#[test]
fn document_edges_match_chrome156() {
    check(
        include_str!("document_edges_probe.js"),
        include_str!("document_edges_chrome156.json"),
        DOCUMENT_GAPS,
    );
}

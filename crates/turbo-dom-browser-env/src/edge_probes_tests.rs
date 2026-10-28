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

#[test]
fn dom_mutation_validity_matches_chrome156() {
    check(
        include_str!("dom_mutation_validity_probe.js"),
        include_str!("dom_mutation_validity_chrome156.json"),
        &[],
    );
}

/// Document pre-insert validity counts the parent's current children: a single node argument
/// that is already a child still counts as one, and replaceChildren does not first remove the
/// existing children (miaskiewicz/turbo-dom#132). Expected values: Chrome 156.0.8078.47.
#[test]
fn document_pre_insert_validity_counts_current_children() {
    let probe = r#"(async () => {
      const attempt = run => { try { run(); return 'ok'; } catch (e) { return e.name; } };
      const names = p => Array.from(p.childNodes, n => n.nodeName).join(',');
      const cases = {
        append: d => d.append(d.documentElement),
        appendChild: d => d.appendChild(d.documentElement),
        prepend: d => d.prepend(d.documentElement),
        rcHtml: d => d.replaceChildren(d.documentElement),
        rcDoctype: d => d.replaceChildren(d.doctype),
        rcBoth: d => d.replaceChildren(d.doctype, d.documentElement),
        rcFrag: d => { const f = d.createDocumentFragment(); f.append(d.createElement('q')); d.replaceChildren(f); },
        doctypeReplaceWithHtml: d => d.doctype.replaceWith(d.documentElement),
        replaceChildHtmlForDoctype: d => d.replaceChild(d.documentElement, d.doctype),
        insertBeforeDoctype: d => d.insertBefore(d.doctype, d.documentElement),
        htmlReplaceWithSelf: d => d.documentElement.replaceWith(d.documentElement),
        moveBeforeHtml: d => d.moveBefore(d.documentElement, null),
        moveBeforeSelf: d => d.moveBefore(d.documentElement, d.documentElement),
        moveBeforeComment: d => { d.appendChild(d.createComment('c')); d.moveBefore(d.lastChild, d.documentElement); },
        moveBeforePi: d => { d.appendChild(d.createProcessingInstruction('a', 'b')); d.moveBefore(d.lastChild, d.doctype); },
        moveBeforeLone: d => { d.doctype.remove(); d.moveBefore(d.documentElement, null); },
        ibSelf: d => d.insertBefore(d.documentElement, d.documentElement),
        ibDoctypeSelf: d => d.insertBefore(d.doctype, d.doctype),
        rcSelf: d => d.replaceChild(d.documentElement, d.documentElement),
        rcDoctypeSelf: d => d.replaceChild(d.doctype, d.doctype),
        commentSelf: d => { const c = d.appendChild(d.createComment('c')); d.insertBefore(c, c); },
      };
      const out = {};
      for (const [k, f] of Object.entries(cases)) { const d = document.implementation.createHTMLDocument(''); out[k] = [attempt(() => f(d)), names(d)]; }
      return out;
    })()"#;
    let error = |name: &str| serde_json::json!([name, "html,HTML"]);
    assert_eq!(
        run(probe),
        serde_json::json!({
            "append": error("HierarchyRequestError"),
            "appendChild": error("HierarchyRequestError"),
            "prepend": error("HierarchyRequestError"),
            "rcHtml": error("HierarchyRequestError"),
            "rcDoctype": error("HierarchyRequestError"),
            "rcBoth": error("ok"),
            "rcFrag": error("HierarchyRequestError"),
            "doctypeReplaceWithHtml": error("HierarchyRequestError"),
            "replaceChildHtmlForDoctype": error("HierarchyRequestError"),
            "insertBeforeDoctype": error("HierarchyRequestError"),
            "htmlReplaceWithSelf": error("ok"),
            "moveBeforeHtml": error("HierarchyRequestError"),
            "moveBeforeSelf": error("HierarchyRequestError"),
            "moveBeforeComment": ["ok", "html,#comment,HTML"],
            "moveBeforePi": ["ok", "a,html,HTML"],
            "moveBeforeLone": ["HierarchyRequestError", "HTML"],
            "ibSelf": error("HierarchyRequestError"),
            "ibDoctypeSelf": error("HierarchyRequestError"),
            "rcSelf": error("ok"),
            "rcDoctypeSelf": error("ok"),
            "commentSelf": ["ok", "html,HTML,#comment"],
        })
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

#[test]
fn document_edges_match_chrome156() {
    check(
        include_str!("document_edges_probe.js"),
        include_str!("document_edges_chrome156.json"),
        &[],
    );
}

/// Chrome 156 admits every identifier as a view transition type in querySelector (as in
/// stylesheets); none of them matches outside an active transition.
#[test]
fn active_view_transition_type_admits_keyword_types() {
    let probe = r#"(async () => ['none', 'NONE', 'initial', 'inherit', 'unset', 'default', 'revert', 'revert-layer', 'auto'].map(type => {
      const selector = `:active-view-transition-type(${type})`;
      try { return [document.querySelectorAll(selector).length, document.documentElement.matches(selector)]; } catch (e) { return e.name; }
    }))()"#;
    assert_eq!(
        run(probe),
        serde_json::Value::Array(vec![serde_json::json!([0, false]); 9])
    );
}

/// DOM adopt: a shadow root throws HierarchyRequestError (a document NotSupportedError).
#[test]
fn adopting_a_shadow_root_throws_hierarchy_request_error() {
    let probe = r#"(async () => [document.createElement('div').attachShadow({ mode: 'open' }), document.implementation.createHTMLDocument('')].map(node => {
      try { document.adoptNode(node); return 'ok'; } catch (e) { return [e.name, e instanceof DOMException]; }
    }))()"#;
    assert_eq!(
        run(probe),
        serde_json::json!([["HierarchyRequestError", true], ["NotSupportedError", true]])
    );
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#143): Text.prototype.wholeText is missing.
const RECEIVER_GAPS: &[(&str, &str)] = &[("result.Text.wholeText", "\"missing\"")];

#[test]
fn native_member_receivers_match_chrome156() {
    check(
        include_str!("receivers_edges_probe.js"),
        include_str!("receivers_edges_chrome156.json"),
        RECEIVER_GAPS,
    );
}

#[test]
fn event_member_receivers_and_listener_signals_match_chrome156() {
    check(
        include_str!("event_receivers_edges_probe.js"),
        include_str!("event_receivers_edges_chrome156.json"),
        &[],
    );
}

/// AddEventListenerOptions.signal is a non-nullable AbortSignal.
#[test]
fn add_event_listener_rejects_a_null_signal() {
    let probe = r#"(async () => [new EventTarget(), document.createElement('p')].map(target => {
      try { target.addEventListener('a', () => {}, { signal: null }); return 'ok'; } catch (e) { return e.name; }
    }))()"#;
    assert_eq!(run(probe), serde_json::json!(["TypeError", "TypeError"]));
}

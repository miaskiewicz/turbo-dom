//! Member edges of the text, animation, range, script, XPath, reference-target, invoker,
//! hyperlink, input, select, ARIA-relation, shorthand and focusgroup bindings against their
//! Chrome 156 control (`member_edges_probe.js` → `member_edges_chrome156.json`, recorded by
//! `scripts/trusted-input-chrome-control.mjs`), plus the regression tests of the bugs that
//! probe found.
use crate::browser_env::crate_surface::harness_tests::thrown;
use crate::browser_env::reference_target::probe_harness::{control, differences, run};
use crate::browser_env::*;

const PROBE: &str = include_str!("member_edges_probe.js");

/// KNOWN_GAP rows `(path, binding value)`: leaves where the binding still differs from
/// Chrome 156. Each must still differ exactly so; a fix removes its row.
const KNOWN_GAPS: &[(&str, &str)] = &[];

/// The differing leaves of `actual` that are not a listed gap, then the listed gaps that no
/// longer differ exactly so.
pub(super) fn unexpected(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    gaps: &[(&str, &str)],
) -> Vec<String> {
    let mut found = Vec::new();
    differences("result", actual, expected, &mut found);
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for entry in found {
        let (path, rest) = entry.split_once('\n').unwrap_or((&entry, ""));
        let binding = rest
            .lines()
            .find_map(|line| line.trim().strip_prefix("binding: "))
            .unwrap_or("");
        match gaps.iter().find(|gap| gap.0 == path && gap.1 == binding) {
            Some(gap) => seen.push(gap.0),
            None => out.push(entry),
        }
    }
    out.extend(
        gaps.iter()
            .filter(|gap| !seen.contains(&gap.0))
            .map(|gap| format!("stale known gap: {}", gap.0)),
    );
    out
}

#[test]
fn unexpected_lists_differences_and_stale_gaps() {
    let expected = serde_json::json!({"a": 1, "b": "x", "c": [true]});
    assert_eq!(unexpected(&expected, &expected, &[]), Vec::<String>::new());
    let actual = serde_json::json!({"a": 2, "b": "y", "c": [true]});
    assert_eq!(
        unexpected(
            &actual,
            &expected,
            &[("result.b", "\"y\""), ("result.c[0]", "true")]
        ),
        vec![
            "result.a\n  binding: 2\n  chrome:  1".to_owned(),
            "stale known gap: result.c[0]".to_owned(),
        ]
    );
    // A row excuses only the binding value it names.
    assert_eq!(
        unexpected(&actual, &expected, &[("result.a", "3")]),
        vec![
            "result.a\n  binding: 2\n  chrome:  1".to_owned(),
            "result.b\n  binding: \"y\"\n  chrome:  \"x\"".to_owned(),
            "stale known gap: result.a".to_owned(),
        ]
    );
}

#[test]
fn member_edges_match_chrome156() {
    let expected = control(include_str!("member_edges_chrome156.json"), PROBE);
    assert_eq!(
        unexpected(&run(PROBE), &expected, KNOWN_GAPS),
        Vec::<String>::new()
    );
}

/// Run `source` (an expression) in a fresh page and return its JSON serialization.
fn page_json(html: &str, source: &str) -> String {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, &format!("JSON.stringify({source})")).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .map(|value| value.to_rust_string_lossy(scope));
    let value = value.unwrap_or_else(|| panic!("page threw: {}", thrown(scope.exception(), scope)));
    reset();
    value
}

/// turbo-dom#152: an inline handler's scope chain holds its form owner (Chrome 156).
#[test]
fn inline_handlers_resolve_their_form_owners_members() {
    assert_eq!(
        page_json(
            "<form action=/go><p><button id=b type=button onclick=\"globalThis.seen = [typeof elements, typeof action, id]\">b</button></p></form>",
            "(document.getElementById('b').click(), globalThis.seen)",
        ),
        r#"["object","string","b"]"#
    );
}

/// turbo-dom#153: a handler body that does not compile is reported to the window as its
/// SyntaxError (Chrome 156), and the handler is null.
#[test]
fn inline_handler_syntax_errors_are_reported() {
    assert_eq!(
        page_json(
            "<div id=d onclick='}'></div>",
            "(() => { const errors = []; addEventListener('error', event => errors.push([event.error.name, event.message.startsWith('Uncaught SyntaxError')])); return [document.getElementById('d').onclick, errors]; })()",
        ),
        r#"[null,[["SyntaxError",true]]]"#
    );
}

/// turbo-dom#154: content of a closed `<details>` is rendered with its contents skipped, so
/// its innerText is empty (Chrome 156), a display:none descendant's too.
#[test]
fn closed_details_content_has_empty_inner_text() {
    assert_eq!(
        page_json(
            "<details><summary>s</summary><p id=p>hidden <b id=b>p</b><i id=i style=display:none>n</i></p></details>",
            "['p', 'b', 'i'].map(id => document.getElementById(id).innerText)",
        ),
        r#"["","",""]"#
    );
}

/// miaskiewicz/turbo-dom#155: getKeyframes() member values spell numbers, var() text and
/// bare zeros like Chrome 156 (`keyframe_values_probe.js` → `keyframe_values_chrome156.json`).
#[test]
fn keyframe_values_match_chrome156() {
    let probe = include_str!("keyframe_values_probe.js");
    let expected = control(include_str!("keyframe_values_chrome156.json"), probe);
    assert_eq!(
        unexpected(&run(probe), &expected, &[]),
        Vec::<String>::new()
    );
}

/// turbo-dom#153: a handler that fails to compile reports once, when its event is first
/// dispatched along the path (or the handler is read); a replaced attribute never reports
/// (Chrome 156: `{"dispatch":["SyntaxError"],"afterDispatch":[null,[]],"replaced":
/// ["function",[]],"otherType":[],"ancestor":["SyntaxError"],"again":[]}`).
#[test]
fn inline_handler_syntax_errors_report_lazily() {
    assert_eq!(
        page_json(
            "<div id=a onclick='}'></div><div id=b onclick='}'></div><div id=c onclick='}'></div><p id=p onclick='}'><i id=i></i></p>",
            "(() => { const errors = []; addEventListener('error', e => { errors.push(e.error.name); e.preventDefault(); }); const $ = id => document.getElementById(id); const out = {}; \
             $('a').click(); out.dispatch = errors.splice(0); \
             out.afterDispatch = [$('a').onclick, errors.splice(0)]; \
             $('b').setAttribute('onclick', 'return 1'); out.replaced = [typeof $('b').onclick, errors.splice(0)]; \
             $('c').dispatchEvent(new Event('focus')); out.otherType = errors.splice(0); \
             $('i').click(); out.ancestor = errors.splice(0); \
             $('i').click(); out.again = errors.splice(0); return out; })()",
        ),
        r#"{"dispatch":["SyntaxError"],"afterDispatch":[null,[]],"replaced":["function",[]],"otherType":[],"ancestor":["SyntaxError"],"again":[]}"#
    );
}

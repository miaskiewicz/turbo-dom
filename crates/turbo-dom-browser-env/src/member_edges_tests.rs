//! Member edges of the text, animation, range, script, XPath, reference-target, invoker,
//! hyperlink, input, select, ARIA-relation, shorthand and focusgroup bindings against their
//! Chrome 156 control (`member_edges_probe.js` → `member_edges_chrome156.json`, recorded by
//! `scripts/trusted-input-chrome-control.mjs`), plus the regression tests of the bugs that
//! probe found.
use crate::browser_env::crate_surface::harness_tests::thrown;
use crate::browser_env::*;

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
#[ignore = "turbo-dom#152: fails until the fix"]
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
#[ignore = "turbo-dom#153: fails until the fix"]
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
#[ignore = "turbo-dom#154: fails until the fix"]
fn closed_details_content_has_empty_inner_text() {
    assert_eq!(
        page_json(
            "<details><summary>s</summary><p id=p>hidden <b id=b>p</b><i id=i style=display:none>n</i></p></details>",
            "['p', 'b', 'i'].map(id => document.getElementById(id).innerText)",
        ),
        r#"["","",""]"#
    );
}

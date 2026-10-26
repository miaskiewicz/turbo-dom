//! Element-owned legacy members measured against Chrome: `webkitMatchesSelector`
//! and the Element-level event handler IDL attributes.
use super::*;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};

/// Chrome exposes these handlers as IDL attributes only; their content attributes are inert.
const IDL_ONLY: [&str; 3] = ["onsearch", "onfullscreenchange", "onfullscreenerror"];
/// Element handlers whose content attributes compile, but only on HTML, SVG and MathML elements.
const NAMESPACED: [&str; 5] = [
    "onbeforecopy",
    "onbeforecut",
    "onbeforepaste",
    "onwebkitfullscreenchange",
    "onwebkitfullscreenerror",
];
const COMPILING_NAMESPACES: [&str; 3] = [
    "http://www.w3.org/1999/xhtml",
    "http://www.w3.org/2000/svg",
    "http://www.w3.org/1998/Math/MathML",
];

/// Whether `name` on `node` is an event handler content attribute Chrome compiles.
pub(super) fn compiles_content_attribute(node: Handle, name: &str) -> bool {
    if IDL_ONLY.contains(&name) {
        return false;
    }
    !NAMESPACED.contains(&name)
        || with_tree(|tree| {
            tree.namespace_uri(node)
                .is_some_and(|namespace| COMPILING_NAMESPACES.contains(&namespace))
        })
        .unwrap_or(false)
}

/// SVG compiles authored handlers with the legacy `evt` parameter name.
pub(super) fn handler_parameter(node: Handle) -> &'static str {
    let svg = with_tree(|tree| tree.namespace_uri(node) == Some("http://www.w3.org/2000/svg"))
        .unwrap_or(false);
    if svg {
        "evt"
    } else {
        "event"
    }
}

/// Exactly one style rule: a selector cannot smuggle a block or a second rule.
fn valid_selector(source: &str) -> bool {
    let text = format!("{source} {{ color: red }}");
    StyleSheet::parse(&text, ParserOptions::default())
        .is_ok_and(|sheet| matches!(sheet.rules.0.as_slice(), [CssRule::Style(_)]))
}

/// The legacy alias reports grammar errors under its own operation name, then
/// shares the native matcher with `matches`.
fn webkit_matches<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    if receiver_handle(scope, &args, &[1]).is_none() {
        return;
    }
    let Some(units) = dom_units(scope, args.get(0)) else {
        return;
    };
    let selector = String::from_utf16_lossy(&units);
    let prefix = "Failed to execute 'webkitMatchesSelector' on 'Element': ";
    if selector.is_empty() {
        throw_dom_error(
            scope,
            "SyntaxError",
            &format!("{prefix}The provided selector is empty."),
        );
        return;
    }
    if !valid_selector(&selector) {
        throw_dom_error(
            scope,
            "SyntaxError",
            &format!("{prefix}'{selector}' is not a valid selector."),
        );
        return;
    }
    el_matches(scope, args, rv);
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__elementMembersMatches", webkit_matches);
    // Rebind the inline-handler hooks the bootstrap already captured and deleted.
    event_handlers::register(scope);
    run_js(scope, include_str!("element_members.js"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_probe(scope: &mut v8::PinScope, source: &str) -> serde_json::Value {
        let code = v8::String::new(scope, source).unwrap();
        v8::tc_scope!(let scope, scope);
        let value = v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
            .to_rust_string_lossy(scope);
        serde_json::from_str(&value).unwrap()
    }

    /// Selector-engine gaps shared with `Element.matches` (same native matcher), and
    /// unknown pseudos that Lightning CSS admits (see the ignored CSSOM admission test).
    const SHARED_MATCHER_GAPS: [&str; 6] = [
        ":scope",
        ":nth-child(1 of p)",
        ":defined",
        "*|p",
        ":unknown-pseudo",
        "p::unknown",
    ];

    fn probe_against_control() -> (serde_json::Value, serde_json::Value) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
        );
        let actual = run_probe(scope, include_str!("element_members_probe.js"));
        reset();
        let control: serde_json::Value =
            serde_json::from_str(include_str!("element_members_chrome156.json")).unwrap();
        assert_eq!(control["browser"], "Chrome/156.0.8078.47");
        (actual, control["result"].clone())
    }

    fn without_shared_gaps(mut value: serde_json::Value) -> serde_json::Value {
        value["matchesSelector"]["grammar"]
            .as_array_mut()
            .unwrap()
            .retain(|row| !SHARED_MATCHER_GAPS.contains(&row[0].as_str().unwrap()));
        value
    }

    #[test]
    fn element_members_match_chrome156_control() {
        let (actual, control) = probe_against_control();
        let (actual, control) = (without_shared_gaps(actual), without_shared_gaps(control));
        for key in ["matchesSelector", "handlers", "order"] {
            assert_eq!(actual[key], control[key], "{key}");
        }
    }

    #[test]
    #[ignore = "Shared Element.matches selector-engine gaps: https://github.com/miaskiewicz/turbo-surf/issues/28"]
    fn webkit_matches_selector_full_grammar_matches_chrome156() {
        let (actual, control) = probe_against_control();
        assert_eq!(
            actual["matchesSelector"]["grammar"],
            control["matchesSelector"]["grammar"]
        );
    }

    #[test]
    fn selector_grammar_rejects_blocks_and_accepts_quoted_braces() {
        assert!(valid_selector("p"));
        assert!(valid_selector("[data-k='{']"));
        assert!(!valid_selector("p { color: red } p"));
        assert!(!valid_selector("p,"));
        assert!(!valid_selector(" "));
    }
}

//! ASCII attribute modifiers preserve original UTF-16 and source-node identity.
use turbo_dom::rtdom::Tree;

#[test]
fn ascii_modifiers_cover_all_operators_and_do_not_fold_unicode() {
    let tree = Tree::parse("<body><div id=a data-v='AbC def-GH' data-word='AbC' data-dash='AbC-en' data-unicode='Ä'></div></body>");
    let node = tree.query_selector("#a").unwrap();
    for selector in [
        "[data-word=abc i]",
        "[data-word='ABC' I]",
        "[data-v~=abc i]",
        "[data-dash|=abc i]",
        "[data-v^=ab i]",
        "[data-v$=gh i]",
    ] {
        assert!(tree.matches_checked(node, selector).unwrap(), "{selector}");
        assert!(
            tree.matches_units(node, &selector.encode_utf16().collect::<Vec<_>>()),
            "{selector}"
        );
    }
    assert!(tree.matches(node, "[data-v*='c d' i]"));
    for selector in [
        "[data-word=abc]",
        "[data-word=abc s]",
        "[data-unicode='ä' i]",
        "[data-v^='' i]",
        "[data-v*='' i]",
        "[data-v~='' i]",
    ] {
        assert!(!tree.matches_checked(node, selector).unwrap(), "{selector}");
    }
    // Rust supports the CSS-standard sensitive modifier; Chrome155's JS admission
    // rejects it. This assertion is a Rust contract, not a Chrome parity claim.
    assert!(tree.matches_checked(node, "[data-word=AbC S]").unwrap());
}

#[test]
fn quoted_literals_escapes_and_invalid_modifiers_are_distinct() {
    let tree = Tree::parse("<body><div id=a data-v='word i' data-x='Hi' data-eq='yes' data-comment='/*x*/'></div></body>");
    let node = tree.query_selector("#a").unwrap();
    for selector in [
        "[data-v='word i']",
        "[data-v='WORD I'i]",
        "[data-x=H\\69  i]",
        "[data-x='hi'\\69]",
        "[data-\\65 q]",
        "[data-comment='/*X*/' i]",
        "[data-x='hi'/**/i]",
        "[data-x='hi' i/**/]",
    ] {
        assert!(tree.matches_checked(node, selector).unwrap(), "{selector}");
        assert!(tree.matches_units(node, &selector.encode_utf16().collect::<Vec<_>>()));
    }
    for selector in [
        "[data-x=hi z]",
        "[data-x=hi i i]",
        "[data-x=hi is]",
        "[data-eq i]",
        ":not([data-x=hi z])",
    ] {
        assert!(tree.query_selector_checked(selector).is_err(), "{selector}");
        assert!(tree.matches_checked(node, selector).is_err(), "{selector}");
    }
}

#[test]
fn html_input_type_default_and_explicit_sensitive_policy_are_namespace_bound() {
    let tree = Tree::parse("<body><input id=lower type=hidden name=q><input id=upper type=HIDDEN name=q><textarea id=visible name=q></textarea><svg><input id=foreign type=HIDDEN></input></svg></body>");
    let lower = tree.query_selector("#lower").unwrap();
    let upper = tree.query_selector("#upper").unwrap();
    assert!(tree.matches(upper, "[type=hidden]"));
    assert!(!tree.matches(upper, "[type=hidden s]"));
    assert!(tree.matches(lower, "[type=hidden s]"));
    assert_eq!(
        tree.query_selector("textarea[name=q],input[name=q]:not([type='hidden' i])"),
        tree.query_selector("#visible")
    );
    let foreign = tree.query_selector("#foreign").unwrap();
    assert!(!tree.matches(foreign, "[type=hidden]"));
    assert!(tree.matches(foreign, "[type=hidden i]"));
}

#[test]
fn quoted_utf16_values_keep_lone_surrogates_under_ascii_folding() {
    let mut tree = Tree::parse("<body><div id=a></div></body>");
    let node = tree.query_selector("#a").unwrap();
    tree.set_attribute_ns_utf16(node, None, "data-v", &[65, 0xd800, 66]);
    let mut selector = "[data-v='a".encode_utf16().collect::<Vec<_>>();
    selector.extend([0xd800]);
    selector.extend("b' i]".encode_utf16());
    assert!(tree.matches_units(node, &selector));
    assert_eq!(tree.query_selector_units(&selector), Some(node));
    selector[10] = 0xd801;
    assert!(!tree.matches_units(node, &selector));
}

fn units(value: &serde_json::Value) -> Vec<u16> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u16::try_from(u.as_u64().unwrap()).unwrap())
        .collect()
}
fn ids(
    tree: &Tree,
    nodes: impl IntoIterator<Item = turbo_dom::rtdom::tree::Handle>,
) -> serde_json::Value {
    serde_json::json!(nodes
        .into_iter()
        .map(|h| tree.get_attribute(h, "id").unwrap().to_owned())
        .collect::<Vec<_>>())
}
fn compare_case(tree: &Tree, case: &serde_json::Value) {
    let selector = units(&case["selectorUnits"]);
    let scalar = String::from_utf16(&selector);
    if case["document"]["all"]["error"] == "SyntaxError" {
        let scalar = scalar.unwrap();
        assert!(tree.query_selector_all_checked(&scalar).is_err(), "{case}");
        return;
    }
    let expected = &case["document"]["all"]["value"];
    let found = tree.query_selector_all_units(&selector);
    assert_eq!(ids(tree, found.iter().copied()), *expected, "{case}");
    let first = found.first().map(|&h| tree.get_attribute(h, "id").unwrap());
    assert_eq!(
        serde_json::json!(first),
        case["document"]["first"]["value"],
        "{case}"
    );
    let root = tree.query_selector("#attribute-fixture").unwrap();
    assert_eq!(
        ids(
            tree,
            tree.query_selector_all_from_units(root, &selector)
                .iter()
                .copied()
        ),
        case["element"]["all"]["value"],
        "{case}"
    );
    if let Ok(scalar) = &scalar {
        assert_eq!(
            ids(
                tree,
                tree.query_selector_all_checked(scalar)
                    .unwrap_or_else(|error| panic!("{case}: {error}"))
                    .iter()
                    .copied()
            ),
            *expected,
            "{case}"
        );
    }
    for row in case["matches"].as_array().unwrap() {
        let node = tree
            .query_selector(&format!("#{}", row["id"].as_str().unwrap()))
            .unwrap();
        assert_eq!(
            tree.matches_units(node, &selector),
            row["result"]["value"].as_bool().unwrap(),
            "{case} {row}"
        );
        if let Ok(scalar) = &scalar {
            assert_eq!(
                tree.matches_checked(node, scalar).unwrap(),
                row["result"]["value"].as_bool().unwrap(),
                "{case} {row}"
            );
        }
    }
}
#[test]
fn independent_chrome_attribute_matrix_matches_the_rust_comparable_subset() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/attribute-case/chrome155.json")).unwrap();
    assert_eq!(fixture["errors"], serde_json::json!([]));
    assert_eq!(
        fixture["probeSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/attribute-case/probe.js"))
        )
    );
    assert_eq!(
        fixture["collectorSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/attribute-case/collect.mjs"))
        )
    );
    let result = &fixture["result"];
    assert_eq!(result["cases"].as_array().unwrap().len(), 64);
    assert_eq!(result["xmlCases"].as_array().unwrap().len(), 5);
    let html = String::from_utf16(&units(&result["fixtureHtmlUnits"])).unwrap();
    let mut tree = Tree::parse(&html);
    for row in result["mutations"].as_array().unwrap() {
        let node = tree
            .query_selector(&format!("#{}", row["id"].as_str().unwrap()))
            .unwrap();
        tree.set_attribute_ns_utf16(
            node,
            None,
            row["name"].as_str().unwrap(),
            &units(&row["valueUnits"]),
        );
    }
    let mut comparable = 0;
    let mut standards_sensitive = 0;
    for case in result["cases"].as_array().unwrap() {
        // Chrome155 rejects s; the public Rust API deliberately supports its
        // standards-defined exact comparison. Keep all seven raw controls.
        if matches!(
            case["name"].as_str().unwrap(),
            "unsupported-s"
                | "unsupported-S"
                | "s-not"
                | "s-list"
                | "s-is-forgiving"
                | "s-where-forgiving"
                | "s-escaped"
        ) {
            assert!(tree
                .query_selector_all_checked(
                    &String::from_utf16(&units(&case["selectorUnits"])).unwrap()
                )
                .is_ok());
            standards_sensitive += 1;
        } else {
            compare_case(&tree, case);
            comparable += 1;
        }
    }
    assert_eq!((comparable, standards_sensitive), (57, 7));
}
#[test]
fn null_namespace_xml_controls_do_not_acquire_html_case_rules() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/attribute-case/chrome155.json")).unwrap();
    let mut tree = Tree::parse("<body></body>");
    let parent = tree.query_selector("body").unwrap();
    for (id, value) in [("xml-upper", "HIDDEN"), ("xml-lower", "hidden")] {
        let node = tree.create_element_namespace(None, "input");
        tree.set_attribute(node, "id", id);
        tree.set_attribute(node, "type", value);
        tree.set_attribute(
            node,
            "data-eq",
            if id == "xml-upper" { "AbC" } else { "abc" },
        );
        tree.append_child(parent, node);
    }
    for case in fixture["result"]["xmlCases"].as_array().unwrap() {
        let selector = units(&case["selectorUnits"]);
        assert_eq!(
            ids(
                &tree,
                tree.query_selector_all_units(&selector).iter().copied()
            ),
            case["all"]["value"],
            "{case}"
        );
        for row in case["matches"].as_array().unwrap() {
            let node = tree
                .query_selector(&format!("#{}", row["id"].as_str().unwrap()))
                .unwrap();
            assert_eq!(
                tree.matches_units(node, &selector),
                row["result"]["value"].as_bool().unwrap(),
                "{case} {row}"
            );
        }
    }
}

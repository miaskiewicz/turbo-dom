use turbo_dom_layout::style::{
    cascade_matched_tokens, resolve_matched_style, CascadeOrigin, ComputedStyle, Declaration,
    MatchedDeclaration,
};

fn declaration(value: &str, important: bool, inline: bool, order: usize) -> MatchedDeclaration {
    MatchedDeclaration {
        declaration: Declaration {
            property: "color".into(),
            value: value.into(),
            important,
        },
        origin: CascadeOrigin::Author,
        scope_depth: 0,
        inline,
        specificity: (0, 0, 1),
        order,
    }
}

#[test]
fn raw_winners_reuse_the_native_rank_without_resolving_tokens() {
    let tokens = "var(--paint, /* unchanged */ #ee2244)";
    let declarations = [
        declaration("blue", false, true, 99),
        declaration(tokens, true, false, 0),
    ];
    assert_eq!(cascade_matched_tokens(&declarations)["color"], tokens);
    assert_eq!(
        resolve_matched_style(&ComputedStyle::default(), &declarations).get("color"),
        Some("/* unchanged */ #ee2244")
    );
}

#[test]
fn raw_winners_preserve_css_wide_values_and_source_order() {
    let declarations = [
        declaration("red", false, true, 0),
        declaration("inherit", false, true, 1),
    ];
    assert_eq!(cascade_matched_tokens(&declarations)["color"], "inherit");
    assert_eq!(
        resolve_matched_style(
            &ComputedStyle::from_pairs([("color", "green")]),
            &declarations
        )
        .get("color"),
        Some("green")
    );
}

#[test]
fn ordered_raw_candidates_retain_lower_fallback_before_important_winner() {
    use turbo_dom_layout::style::ordered_matched_declarations;
    let lower = declaration("blue", false, true, 9);
    let higher = declaration("var(--missing)", true, false, 0);
    let ordered = ordered_matched_declarations(&[higher.clone(), lower.clone()]);
    assert_eq!(ordered, [lower.declaration, higher.declaration]);
    assert!(ordered_matched_declarations(&[]).is_empty());
}

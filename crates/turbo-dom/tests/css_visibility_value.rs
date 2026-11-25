use turbo_dom::rtdom::css_shorthand::{longhand_value, Spelled};

#[test]
fn visibility_uses_the_shared_keyword_grammar() {
    for (source, expected) in [
        ("visible", "visible"),
        ("HIDDEN", "hidden"),
        ("collapse", "collapse"),
        (r"h\69 dden", "hidden"),
    ] {
        assert_eq!(
            longhand_value("visibility", source),
            Spelled::Valid(expected.into()),
            "{source}"
        );
    }
    for source in ["invalid", "'hidden'", "hidden visible"] {
        assert_eq!(
            longhand_value("visibility", source),
            Spelled::Invalid,
            "{source}"
        );
    }
    assert_eq!(
        longhand_value("visibility", "var(--visibility)"),
        Spelled::Unchecked("var(--visibility)".into())
    );
}

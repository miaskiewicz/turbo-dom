use turbo_dom_layout::layout::value::{resolve_box_style, resolve_font_size, ResolveCtx};
use turbo_dom_layout::style::ComputedStyle;

#[test]
fn font_only_resolution_matches_complete_box_resolution() {
    for (raw, expected) in [
        ("24px", 24.0),
        ("1.5em", 30.0),
        ("150%", 30.0),
        ("invalid", 20.0),
    ] {
        let style = ComputedStyle::from_pairs([
            ("font-size".to_owned(), raw.to_owned()),
            ("border".to_owned(), "12px solid red".to_owned()),
            ("padding".to_owned(), "25%".to_owned()),
        ]);
        assert_eq!(resolve_font_size(&style, 20.0), expected);
        assert_eq!(
            resolve_font_size(&style, 20.0),
            resolve_box_style(
                &style,
                ResolveCtx {
                    parent_font_size: 20.0,
                    cb_width: 320.0
                }
            )
            .font_size
        );
    }
    assert_eq!(
        resolve_font_size(
            &ComputedStyle::from_pairs(std::iter::empty::<(String, String)>()),
            23.0
        ),
        23.0
    );
}

#[test]
fn font_only_resolution_uses_the_current_document_root_font_scope() {
    let style = ComputedStyle::from_pairs([("font-size".to_owned(), "2rem".to_owned())]);
    let _scope = turbo_dom_layout::root_font_scope(27.0);
    assert_eq!(resolve_font_size(&style, 12.0), 54.0);
    assert_eq!(
        resolve_box_style(
            &style,
            ResolveCtx {
                parent_font_size: 12.0,
                cb_width: 0.0
            }
        )
        .font_size,
        54.0
    );
}

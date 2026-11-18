use super::*;
use crate::layout::value::Rgba;

#[test]
fn inline_text_resolution_does_not_resolve_box_frame_or_paint_properties() {
    let fonts = crate::test_support::fixture_fonts();
    let item = InlineItem::Text {
        node_id: super::super::fragment::NodeId(73),
        style: crate::ComputedStyle::from_pairs([
            ("font-family", "sans-serif"),
            ("font-size", "20px"),
            ("font-weight", "700"),
            ("line-height", "1.5"),
            ("color", "#123456"),
            ("border", "8px groove red"),
            ("background", "linear-gradient(red, blue)"),
            ("transform", "rotate(20deg)"),
            ("white-space", "pre-wrap"),
            ("text-transform", "uppercase"),
            ("-webkit-locale", "tr"),
        ]),
        text: "shaped content".to_owned(),
    };
    crate::perf::reset();
    let run = text_run(&item, 16.0, 320.0, &fonts).expect("text face selected");
    assert_eq!(crate::perf::count("layout.resolve_text_style"), 1);
    assert_eq!(crate::perf::count("layout.resolve_box_style"), 0);
    assert_eq!(
        (run.node_id, run.text.as_str()),
        (super::super::fragment::NodeId(73), "shaped content")
    );
    assert_eq!(
        (run.font_size, run.weight, run.line_height),
        (20.0, 700, Some(30.0))
    );
    assert_eq!(
        (run.nowrap, run.preserve_spaces, run.forced_newlines),
        (false, true, true),
    );
    assert_eq!(
        run.text_transform,
        crate::text::TextTransform {
            case: crate::text::CaseTransform::Uppercase,
            locale: crate::text::CaseLocale::Turkic,
        },
    );
    assert_eq!(
        run.color,
        Rgba {
            r: 0x12,
            g: 0x34,
            b: 0x56,
            a: 255
        }
    );
}

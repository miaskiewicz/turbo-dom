//! Unit tests of [`super`]: the counter styles, HTML integers, the `list-item` counters
//! and the outside-marker placement corners the Chrome control does not reach.
use super::*;
use crate::cssom_view::hit;
use crate::live::testing::{id, layout_with_fonts};
use crate::style::StyledText;
use crate::tag::{Attr, MarkerKind};
use crate::Diagnostics;

fn text(list_style_type: Option<&str>, ordinal: i64) -> Option<String> {
    match content(list_style_type, ordinal)? {
        Content::Text(text) => Some(text),
        Content::Symbol(symbol) => Some(format!("{symbol:?}")),
    }
}

#[test]
fn counter_styles_and_their_fallbacks() {
    let cases: &[(Option<&str>, i64, Option<&str>)] = &[
        (None, 1, Some("Disc")),
        (Some("initial"), 1, Some("Disc")),
        (Some("CIRCLE"), 1, Some("Circle")),
        (Some("square"), 1, Some("Square")),
        (Some("none"), 1, None),
        (Some("decimal"), -3, Some("-3. ")),
        (Some("decimal-leading-zero"), 7, Some("07. ")),
        (Some("decimal-leading-zero"), -7, Some("-07. ")),
        (Some("decimal-leading-zero"), 123, Some("123. ")),
        (Some("lower-alpha"), 28, Some("ab. ")),
        (Some("upper-latin"), 703, Some("AAA. ")),
        (Some("lower-alpha"), -1, Some("-1. ")),
        (Some("lower-greek"), 25, Some("αα. ")),
        (Some("lower-roman"), 3999, Some("mmmcmxcix. ")),
        (Some("upper-roman"), 0, Some("0. ")),
        (Some("'» '"), 9, Some("» ")),
        (Some("\"x\""), 9, Some("x")),
        (Some("'unterminated"), 2, Some("2. ")),
        (Some("no-such-style"), 4, Some("4. ")),
    ];
    for &(style, ordinal, want) in cases {
        assert_eq!(text(style, ordinal).as_deref(), want, "{style:?} {ordinal}");
    }
}

#[test]
fn html_integers() {
    for (input, want) in [
        ("  +5x", Some(5)),
        ("\t-12", Some(-12)),
        ("7", Some(7)),
        ("x7", None),
        ("-", None),
        ("", None),
    ] {
        assert_eq!(parse_integer(input), want, "{input:?}");
    }
}

fn element(tag: Tag, attrs: &[(&str, &str)], children: Vec<StyledNode>) -> StyledElement {
    StyledElement {
        tag,
        attrs: attrs
            .iter()
            .map(|(name, value)| Attr {
                name: (*name).to_string(),
                value: (*value).to_string(),
            })
            .collect(),
        style: ComputedStyle::default(),
        children,
        source: None,
    }
}

fn item(display: &str, children: Vec<StyledNode>) -> StyledNode {
    let mut li = element(Tag::Html("li".into()), &[], children);
    li.style.set("display", display.into());
    StyledNode::Element(li)
}

#[test]
fn list_item_counters() {
    let mut counters = ListCounters::default();
    // A marker element and a non-list element start no list.
    let marker = MarkerKind {
        id: 1,
        inline: false,
    };
    assert!(!counters.enter(&element(Tag::Marker(marker), &[], Vec::new())));
    assert!(!counters.enter(&element(Tag::Html("div".into()), &[], Vec::new())));
    let li = element(Tag::Html("li".into()), &[], Vec::new());
    assert_eq!((counters.ordinal(&li), counters.ordinal(&li)), (1, 2));
    // A reversed list counts its own list items, not a nested list's, a block's or text.
    let nested = StyledNode::Element(element(
        Tag::Html("ul".into()),
        &[],
        vec![item("list-item", Vec::new())],
    ));
    let wrapper = StyledNode::Element(element(
        Tag::Html("div".into()),
        &[],
        vec![item("list-item", Vec::new()), item("block", Vec::new())],
    ));
    let text = StyledNode::Text(StyledText {
        text: "x".into(),
        source: None,
    });
    let ol = element(
        Tag::Html("ol".into()),
        &[("REVERSED", "")],
        vec![item("list-item", vec![nested]), wrapper, text],
    );
    assert!(counters.enter(&ol));
    let valued = element(Tag::Html("li".into()), &[("value", " 10 ")], Vec::new());
    assert_eq!(counters.ordinal(&li), 2);
    assert_eq!((counters.ordinal(&valued), counters.ordinal(&li)), (10, 9));
    counters.leave();
    // `start` numbers only an `ol`; a `menu` is a list too.
    let menu = element(Tag::Html("menu".into()), &[("start", "5")], Vec::new());
    assert!(counters.enter(&menu));
    assert_eq!(counters.ordinal(&li), 1);
    counters.leave();
    assert_eq!(counters.ordinal(&li), 3);
}

/// The marker fragments (boxes holding a shape or text) of `html`'s static layout:
/// `(x, y, width, height)` of each painted disc / text line beside an item.
fn painted(html: &str) -> Vec<(f32, f32, f32, f32)> {
    fn walk(frag: &Fragment, out: &mut Vec<(f32, f32, f32, f32)>) {
        for child in &frag.children {
            let shape = matches!(
                child.content,
                FragmentContent::Box { background: Some(_), border_radius, .. } if border_radius.is_rounded()
            );
            let digit =
                matches!(&child.content, FragmentContent::TextLine { text, .. } if text == "1. ");
            if shape || digit {
                out.push((child.x, child.y, child.width, child.height));
            }
            walk(child, out);
        }
    }
    let fonts = crate::test_support::fixture_fonts();
    let mut diags = Diagnostics::default();
    let galley = crate::layout_html(html, "", 400.0, &fonts, &mut diags).unwrap();
    let mut out = Vec::new();
    walk(&galley, &mut out);
    out
}

const CSS: &str = "body{margin:0;font:16px 'Liberation Serif'}ul,ol{margin:0}";
const STYLE: &str = "<style>body{margin:0;font:16px 'Liberation Serif'}ul,ol{margin:0}</style>";

/// A list item laid out atomically inside an inline (the static path flattens an
/// inline's nested blocks into atoms) keeps its own outside marker: its symbol is not
/// laid out as an inside marker's box, and its text marker is placed with it.
#[test]
fn an_atomic_list_item_places_its_outside_marker() {
    let disc = painted(&format!("{STYLE}<ul><span><li>x</li></span></ul>"));
    let digit = painted(&format!("{STYLE}<ol><span><li>x</li></span></ol>"));
    assert_eq!((disc.len(), digit.len()), (1, 1));
    // Beside the item (border box at x=40), not in its line.
    assert!(disc[0].0 < 40.0 && digit[0].0 < 40.0, "{disc:?} {digit:?}");
}

/// A positioned first child's text is not the item's first line: with no line box the
/// marker sits at the item's content top (its disc `3*(14-9)/2 - 14` = -7 below the
/// strut baseline) and the item is one marker line tall.
#[test]
fn a_positioned_first_line_is_not_the_markers_line() {
    let html =
        format!("{STYLE}<ul><li data-cid=li><b style='position:absolute;top:50px'>x</b></li></ul>");
    let disc = painted(&html);
    assert_eq!(disc.len(), 1);
    assert_eq!((disc[0].1 - disc[0].3, disc[0].2), (2.0, 5.0), "{disc:?}");
    let fonts = crate::test_support::fixture_fonts();
    let boxes =
        crate::layout_boxes(&html, "", 400.0, 300.0, &fonts, &mut Diagnostics::default()).unwrap();
    assert!(boxes[0].height > 17.0, "{boxes:?}");
}

/// The top of the first text line containing `text` under `frag`.
fn line_of(frag: &Fragment, text: &str) -> Option<f32> {
    frag.children.iter().find_map(|c| match &c.content {
        FragmentContent::TextLine { text: t, .. } if t.contains(text) => Some(c.y),
        _ => line_of(c, text),
    })
}

/// An inside marker goes into the item's leading inline run when a block follows it
/// (not onto a line of its own).
#[test]
fn an_inside_marker_joins_the_leading_run_before_a_block() {
    // The marker and the leading `a` share the first line ("1. a"), above `b`'s.
    let fonts = crate::test_support::fixture_fonts();
    let mut diags = Diagnostics::default();
    let galley = crate::layout_html(
        &format!("{STYLE}<ol style='list-style-position:inside'><li>a<div>b</div></li></ol>"),
        "",
        400.0,
        &fonts,
        &mut diags,
    )
    .unwrap();
    let first = line_of(&galley, "1.").expect("the marker's run");
    assert_eq!(line_of(&galley, "a"), Some(first));
    assert!(line_of(&galley, "b").unwrap() > first);
}

/// The `type` attribute maps only its list's values: `<ol type=disc>` / `<ul type=a>`
/// set nothing, `<li>` takes both.
#[test]
fn list_type_attributes() {
    let hint = |tag: &str, value: &str| {
        crate::style::presentational_declarations(
            Some(tag),
            &[Attr {
                name: "TYPE".into(),
                value: value.into(),
            }],
        )
        .into_iter()
        .find(|d| d.property == "list-style-type")
        .map(|d| d.value)
    };
    assert_eq!(hint("ol", "disc"), None);
    assert_eq!(hint("ul", "a"), None);
    assert_eq!(hint("ul", "Square").as_deref(), Some("square"));
    assert_eq!(hint("li", "i").as_deref(), Some("lower-roman"));
    assert_eq!(hint("li", "circle").as_deref(), Some("circle"));
    assert_eq!(hint("div", "1"), None);
}

/// Other sourceless text (a button input's `value` label) is no marker: it hits nothing
/// itself, and the input's own box takes the hit.
#[test]
fn sourceless_text_that_is_no_marker_hits_its_box() {
    let fonts = crate::test_support::fixture_fonts();
    let html = "<input id=b type=button value=Go style='width:80px;height:30px'>";
    let (tree, snapshot, styles) =
        layout_with_fonts(html, CSS, (400.0, 300.0), Some(1.0), |_| {}, &fonts);
    assert!(snapshot
        .text_runs()
        .iter()
        .any(|run| run.source_id.is_none()));
    let got = hit::elements_at_point(
        &snapshot,
        &styles,
        &std::collections::BTreeMap::new(),
        [10.0, 15.0],
    )
    .unwrap();
    assert_eq!(got.first(), Some(&id(&tree, "#b")), "{got:?}");
}

/// A marker hits its list item only while the item takes hits (Chrome 156).
#[test]
fn marker_hits_follow_the_items_pointer_events_and_visibility() {
    let fonts = crate::test_support::fixture_fonts();
    for (style, hits) in [
        ("", true),
        ("pointer-events:none", false),
        ("visibility:hidden", false),
    ] {
        let html = format!(
            "{STYLE}<ol><li id=li style='{style}'><span style='visibility:visible;pointer-events:auto'>item</span></li></ol>"
        );
        let (tree, snapshot, styles) =
            layout_with_fonts(&html, CSS, (400.0, 300.0), Some(1.0), |_| {}, &fonts);
        let li = id(&tree, "#li");
        let got = hit::elements_at_point(
            &snapshot,
            &styles,
            &std::collections::BTreeMap::new(),
            [30.0, 9.0],
        )
        .unwrap();
        assert_eq!(got.contains(&li), hits, "{style}: {got:?}");
    }
}

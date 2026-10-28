//! Box generation (§5.1): styled tree -> box tree, anonymous-block wrapping of
//! inline runs (AC-5.1), `display:none` dropping, and `t:` directive markers.
//!
//! Split at the S1 relocation: the cases that need `TKind` directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/layout_boxgen.rs`.

use turbo_dom_layout::layout::boxgen::*;
use turbo_dom_layout::layout::value::Display;
use turbo_dom_layout::tag::Tag;
use turbo_dom_layout::{ComputedStyle, StyledElement, StyledNode};

fn cs(pairs: &[(&str, &str)]) -> ComputedStyle {
    ComputedStyle::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
}

fn el(tag: &str, pairs: &[(&str, &str)], children: Vec<StyledNode>) -> StyledNode {
    StyledNode::Element(StyledElement {
        tag: Tag::Html(tag.to_string()),
        attrs: vec![],
        style: cs(pairs),
        children,
        source: None,
    })
}

fn txt(s: &str) -> StyledNode {
    StyledNode::Text(s.into())
}

fn as_block(kind: &BoxKind) -> Vec<LayoutBox> {
    match kind {
        BoxKind::Block(b) => b.clone(),
        _ => panic!("expected Block"),
    }
}

fn as_lines(kind: &BoxKind) -> Vec<InlineItem> {
    match kind {
        BoxKind::Lines(items) => items.clone(),
        _ => panic!("expected Lines"),
    }
}

#[test]
fn text_only_flow_is_inline_context() {
    let root = build_box_tree(&[txt("hello world")]);
    assert_eq!(root.node_id.0, 0);
    assert_eq!(root.display, Display::Block);
    assert_eq!(as_lines(&root.kind).len(), 1);
}

#[test]
fn empty_flow_is_empty_lines() {
    let root = build_box_tree(&[]);
    assert!(as_lines(&root.kind).is_empty());
}

#[test]
fn mixed_block_inline_wraps_anonymous_block() {
    // "before" <span>mid</span> <p>para</p>  -> [anon Lines(before,mid), block p]
    let nodes = vec![
        txt("before"),
        el("span", &[("display", "inline")], vec![txt("mid")]),
        el("p", &[], vec![txt("para")]),
    ];
    let root = build_box_tree(&nodes);
    let boxes = as_block(&root.kind);
    assert_eq!(boxes.len(), 2);
    assert_eq!(as_lines(&boxes[0].kind).len(), 2); // before + mid in one anon block
    assert_eq!(boxes[1].display, Display::Block);
    assert_eq!(as_lines(&boxes[1].kind).len(), 1); // the <p>'s text
}

#[test]
fn display_none_is_dropped() {
    let nodes = vec![
        el("p", &[("display", "none")], vec![txt("hidden")]),
        el("p", &[], vec![txt("shown")]),
    ];
    let boxes = as_block(&build_box_tree(&nodes).kind);
    assert_eq!(boxes.len(), 1);
}

#[test]
fn inline_block_becomes_atomic() {
    let nodes = vec![el("span", &[("display", "inline-block")], vec![txt("x")])];
    let items = as_lines(&build_box_tree(&nodes).kind).clone();
    assert!(matches!(items[0], InlineItem::Atomic(_)));
}

#[test]
fn block_nested_in_inline_splits_the_inline() {
    // A block inside an inline splits it (CSS 2.2 §9.2.1.1; miaskiewicz/turbo-dom#101):
    // the flow becomes a block context of the block alone (the empty runs around it
    // generate nothing) ...
    let nodes = vec![el(
        "span",
        &[("display", "inline")],
        vec![el("div", &[], vec![txt("d")])],
    )];
    let boxes = as_block(&build_box_tree(&nodes).kind);
    assert_eq!(boxes.len(), 1);
    assert_eq!(boxes[0].display, Display::Block);
    assert_eq!(as_lines(&boxes[0].kind).len(), 1);
    // ... and the inline content around it goes to anonymous line boxes before and
    // after it, in order; an absolutely positioned box stays in its run.
    let nodes = vec![el(
        "span",
        &[("display", "inline")],
        vec![
            txt("a"),
            el("i", &[("position", "absolute")], vec![]),
            el("div", &[], vec![txt("d")]),
            txt("b"),
        ],
    )];
    let boxes = as_block(&build_box_tree(&nodes).kind);
    assert_eq!(boxes.len(), 3);
    let before = as_lines(&boxes[0].kind);
    assert_eq!(before.len(), 2);
    assert!(matches!(&before[0], InlineItem::Text { text, .. } if text.as_str() == "a"));
    assert!(matches!(before[1], InlineItem::Atomic(_)));
    assert_eq!(boxes[1].display, Display::Block);
    let after = as_lines(&boxes[2].kind);
    assert!(matches!(&after[..], [InlineItem::Text { text, .. }] if text.as_str() == "b"));
}

#[test]
fn inline_element_drops_hidden_child() {
    let nodes = vec![el(
        "span",
        &[("display", "inline")],
        vec![
            el("em", &[("display", "none")], vec![txt("hidden")]),
            txt("vis"),
        ],
    )];
    let items = as_lines(&build_box_tree(&nodes).kind).clone();
    assert_eq!(items.len(), 1); // only "vis"
}

#[test]
fn flex_container_keeps_only_real_items() {
    let nodes = vec![el(
        "div",
        &[("display", "flex")],
        vec![
            el("div", &[], vec![txt("a")]),
            txt("   "),
            el("div", &[("display", "none")], vec![]),
        ],
    )];
    let boxes = as_block(&build_box_tree(&nodes).kind);
    match &boxes[0].kind {
        BoxKind::Flex(items) => assert_eq!(items.len(), 1),
        _ => panic!("expected Flex"),
    }
}

#[test]
fn table_structure_preserved() {
    let nodes = vec![el(
        "table",
        &[("display", "table")],
        vec![el(
            "tr",
            &[("display", "table-row")],
            vec![el("td", &[("display", "table-cell")], vec![txt("c")])],
        )],
    )];
    let boxes = as_block(&build_box_tree(&nodes).kind);
    let BoxKind::Table(rows) = &boxes[0].kind else {
        panic!("expected Table")
    };
    assert_eq!(rows[0].display, Display::TableRow);
    let cells = as_block(&rows[0].kind);
    assert_eq!(cells[0].display, Display::TableCell);
}

#[test]
fn whitespace_between_blocks_makes_no_anonymous_box() {
    let nodes = vec![
        el("p", &[], vec![txt("a")]),
        txt("   "),
        el("p", &[], vec![txt("b")]),
    ];
    let boxes = as_block(&build_box_tree(&nodes).kind);
    assert_eq!(boxes.len(), 2); // blank run dropped
}

#[test]
fn nonblank_inline_run_between_blocks_makes_anonymous_box() {
    let nodes = vec![
        el("p", &[], vec![txt("a")]),
        txt("x"),
        el("p", &[], vec![txt("b")]),
    ];
    let boxes = as_block(&build_box_tree(&nodes).kind);
    assert_eq!(boxes.len(), 3);
    assert_eq!(as_lines(&boxes[1].kind).len(), 1); // anon block holding "x"
}

#[test]
fn node_ids_are_preorder() {
    let root = build_box_tree(&[txt("a")]);
    assert_eq!(root.node_id.0, 0);
    match &as_lines(&root.kind)[0] {
        InlineItem::Text { node_id, .. } => assert_eq!(node_id.0, 1),
        _ => panic!("expected text"),
    }
}

#[test]
fn background_shorthand_url_becomes_image_source() {
    // No `background-image` longhand: the url is scanned out of the `background`
    // shorthand's tokens (a non-replaced image source painted behind the box).
    let root = build_box_tree(&[el(
        "div",
        &[("background", "url(bg.png) no-repeat")],
        vec![],
    )]);
    let div = &as_block(&root.kind)[0];
    let img = div.image.as_ref().expect("background image source");
    assert_eq!(img.name, "bg.png");
    assert!(!img.replaced, "background image is not replaced content");
}

#[test]
fn mask_shorthand_url_is_extracted_via_token_scan() {
    // A `mask` shorthand whose value is not a bare `url(...)` (it has trailing
    // keywords) falls to the token-scan path to pull the url out.
    let root = build_box_tree(&[el(
        "div",
        &[("mask", "url(icon.svg) no-repeat center")],
        vec![],
    )]);
    let div = &as_block(&root.kind)[0];
    assert_eq!(div.mask.as_deref(), Some("icon.svg"));
}

#[test]
fn img_lazy_data_url_and_srcset_become_replaced_source() {
    use turbo_dom_layout::tag::Attr;
    let img = |attrs: &[(&str, &str)]| {
        StyledNode::Element(StyledElement {
            tag: Tag::Html("img".into()),
            attrs: attrs
                .iter()
                .map(|(n, v)| Attr {
                    name: n.to_string(),
                    value: v.to_string(),
                })
                .collect(),
            style: cs(&[("display", "block")]),
            children: vec![],
            source: None,
        })
    };
    let src_of = |node: StyledNode| {
        let root = build_box_tree(&[node]);
        let b = as_block(&root.kind)[0].clone();
        let s = b.image.as_ref().expect("img -> replaced source");
        assert!(s.replaced, "an <img> source is replaced content");
        s.name.clone()
    };
    // No `src`: fall back to the lazy `data-landscape-url` (nike's hero pattern).
    assert_eq!(
        src_of(img(&[("data-landscape-url", "hero.jpg")])),
        "hero.jpg"
    );
    // No `src`/data-*: take the first `srcset` candidate's url.
    assert_eq!(src_of(img(&[("srcset", "a.jpg 1x, b.jpg 2x")])), "a.jpg");
    // An empty `src` is skipped in favor of the lazy attribute.
    assert_eq!(
        src_of(img(&[("src", "  "), ("data-src", "lazy.jpg")])),
        "lazy.jpg"
    );
    // A real `src` still wins over any lazy attribute.
    assert_eq!(
        src_of(img(&[("src", "real.jpg"), ("data-src", "lazy.jpg")])),
        "real.jpg"
    );
}

#[test]
fn input_button_value_renders_as_text() {
    use turbo_dom_layout::tag::Attr;
    let input = |ty: &str| {
        StyledNode::Element(StyledElement {
            tag: Tag::Html("input".into()),
            attrs: vec![
                Attr {
                    name: "type".into(),
                    value: ty.into(),
                },
                Attr {
                    name: "value".into(),
                    value: "Go".into(),
                },
            ],
            style: cs(&[]),
            children: vec![],
            source: None,
        })
    };
    // submit/button/reset → the value becomes the box's text.
    for ty in ["submit", "button", "reset"] {
        let root = build_box_tree(&[input(ty)]);
        let inner = as_block(&root.kind);
        let lines = as_lines(&inner[0].kind);
        let has_go = lines
            .iter()
            .any(|it| matches!(it, InlineItem::Text { text, .. } if text.contains("Go")));
        assert!(has_go, "{ty} input value should render as text");
    }
    // a text input's value is NOT drawn as content (it's an editable field).
    let root = build_box_tree(&[input("text")]);
    let lines = as_lines(&as_block(&root.kind)[0].kind);
    assert!(
        lines.is_empty(),
        "text input value is not flowed as content"
    );
}

#[test]
fn background_image_longhand_wins_over_the_shorthand() {
    let root = build_box_tree(&[
        el(
            "div",
            &[
                ("background-image", "url( 'long.png' )"),
                ("background", "url(short.png) no-repeat"),
            ],
            vec![],
        ),
        el(
            "div",
            &[
                ("background-image", "none"),
                ("background", "url(short.png) no-repeat"),
            ],
            vec![],
        ),
    ]);
    let divs = as_block(&root.kind);
    let img = divs[0].image.as_ref().expect("longhand image source");
    assert_eq!(img.name, "long.png");
    assert!(!img.replaced);
    assert_eq!(
        divs[1].image.as_ref().unwrap().name,
        "short.png",
        "a non-url longhand falls through"
    );
}

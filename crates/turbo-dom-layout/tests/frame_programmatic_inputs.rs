//! Programmatic styles bypass CSS parsing but must retain safe frame semantics.
use turbo_dom_layout::{
    layout, ComputedStyle, Diagnostics, FontRegistry, Fragment, StyledElement, StyledNode, Tag,
};

fn node(pairs: &[(&str, &str)], children: Vec<StyledNode>) -> StyledNode {
    StyledNode::Element(StyledElement {
        tag: Tag::Html("div".into()),
        attrs: Vec::new(),
        style: ComputedStyle::from_pairs(
            std::iter::once(("display", "block")).chain(pairs.iter().copied()),
        ),
        children,
        source: None,
    })
}

fn graph(property: &str, padding: &str) -> Vec<StyledNode> {
    vec![node(
        &[("display", "flex"), ("width", "400px")],
        vec![
            node(
                &[
                    ("width", "80px"),
                    ("height", "40px"),
                    ("flex", "none"),
                    (property, padding),
                ],
                vec![node(&[("width", "100%"), ("height", "10px")], Vec::new())],
            ),
            node(
                &[("width", "40px"), ("height", "20px"), ("flex", "none")],
                Vec::new(),
            ),
        ],
    )]
}

fn geometry(fragment: &Fragment, boxes: &mut Vec<[f32; 4]>) {
    boxes.push([fragment.x, fragment.y, fragment.width, fragment.height]);
    for child in &fragment.children {
        geometry(child, boxes);
    }
}

#[test]
fn programmatic_auto_padding_preserves_zero_frame_geometry() {
    let fonts = FontRegistry::new();
    for property in ["padding", "padding-left"] {
        let capture = |value| {
            let fragment = layout(
                &graph(property, value),
                400.0,
                &fonts,
                &mut Diagnostics::default(),
            );
            let mut boxes = Vec::new();
            geometry(&fragment, &mut boxes);
            boxes
        };
        // A caller can supply an invalid computed value directly. It falls back
        // to zero padding, rather than becoming an automatic or non-finite edge.
        let actual = capture("auto");
        assert_eq!(actual, capture("0px"));
        assert!(actual.contains(&[0.0, 0.0, 80.0, 40.0]));
        assert!(actual.contains(&[80.0, 0.0, 40.0, 20.0]));
        assert!(actual.contains(&[0.0, 0.0, 80.0, 10.0]));
    }
}

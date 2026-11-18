use turbo_dom_layout::style::ComputedStyle;
use turbo_dom_layout::turbo_dom::{Handle, Tree};
use turbo_dom_layout::{
    layout_html_tree_capture_in_viewport_with, layout_html_tree_in_viewport_with,
    layout_html_tree_recorded_in_viewport_with, Diagnostics, FontRegistry, Fragment,
    FragmentContent, HtmlOptions, ImageCtx, LaidOut, NoImages, StyledNode,
};

fn patch_target(nodes: &mut [StyledNode], target: Handle) {
    for node in nodes {
        if let StyledNode::Element(element) = node {
            if element.source == Some(target) {
                element.style =
                    ComputedStyle::from_pairs(element.style.iter().chain([("width", "2rem")]));
            }
            patch_target(&mut element.children, target);
        }
    }
}

fn source_box(laid: &LaidOut, source: Handle) -> &Fragment {
    fn find(fragment: &Fragment, id: turbo_dom_layout::NodeId) -> Option<&Fragment> {
        if fragment.node_id == id {
            return Some(fragment);
        }
        fragment.children.iter().find_map(|child| find(child, id))
    }
    let id = laid
        .sources
        .elements
        .iter()
        .find_map(|(id, handle)| (*handle == source).then_some(*id))
        .expect("recorded element source");
    find(&laid.galley, id).expect("source resolves back to a real fragment")
}

#[test]
fn recording_keeps_canvas_cascade_hook_viewport_and_transformed_source_geometry() {
    for (backgrounds, expected) in [
        (
            "html{background:rgb(1,2,3)}body{background:rgb(4,5,6)}",
            Some((1, 2, 3)),
        ),
        ("body{background:rgb(4,5,6)}", Some((4, 5, 6))),
        ("", None),
    ] {
        let tree = Tree::parse("<!doctype html><html><body><div id=target style='position:fixed;top:50%;left:10px;width:1px;height:25%;transform:translate(3px,4px);transform-origin:0 0'></div></body></html>");
        let target = tree.get_element_by_id("target").unwrap();
        let before =
            turbo_dom_layout::turbo_dom::rtdom::serialize::serialize_outer(&tree, tree.root());
        let css = format!("html{{font-size:20px}}body{{margin:0}}{backgrounds}");
        let fonts = FontRegistry::new();
        let images = ImageCtx {
            resolver: &NoImages,
            body_height: None,
        };
        let options = HtmlOptions::default();
        let ordinary = layout_html_tree_in_viewport_with(
            &tree,
            &css,
            (120.0, 80.0),
            &fonts,
            &images,
            &options,
            |nodes| patch_target(nodes, target),
            &mut Diagnostics::default(),
        );
        let capture = layout_html_tree_capture_in_viewport_with(
            &tree,
            &css,
            (120.0, 80.0),
            &fonts,
            &images,
            &options,
            |nodes| patch_target(nodes, target),
            &mut Diagnostics::default(),
        );
        assert_eq!(format!("{ordinary:?}"), format!("{capture:?}"));
        let recorded = layout_html_tree_recorded_in_viewport_with(
            &tree,
            &css,
            (120.0, 80.0),
            &fonts,
            &images,
            &options,
            |nodes| patch_target(nodes, target),
            &mut Diagnostics::default(),
        );
        // Fragment has no PartialEq; this compares every paint/layout field, including
        // descendants and transforms, rather than a hand-picked geometry projection.
        assert_eq!(format!("{ordinary:?}"), format!("{:?}", recorded.galley));
        let FragmentContent::Box { background, .. } = &recorded.galley.content else {
            panic!("the document galley is a box");
        };
        assert_eq!(
            background.map(|color| (color.r, color.g, color.b)),
            expected
        );
        let fragment = source_box(&recorded, target);
        assert_eq!(
            (fragment.x, fragment.y, fragment.width, fragment.height),
            (10.0, 40.0, 40.0, 20.0)
        );
        let FragmentContent::Box {
            transform: Some(transform),
            ..
        } = &fragment.content
        else {
            panic!("the mapped source must retain its paint transform");
        };
        assert_eq!(transform.matrix, [1.0, 0.0, 0.0, 1.0, 3.0, 4.0]);
        assert_eq!(
            before,
            turbo_dom_layout::turbo_dom::rtdom::serialize::serialize_outer(&tree, tree.root())
        );
        assert!(recorded.sources.next.0 > recorded.sources.root.0);
    }
}

#[test]
fn recorded_handles_belong_to_the_supplied_tree_across_independent_documents() {
    let first = Tree::parse("<body><div id=first style='width:11px;height:7px'></div></body>");
    let second =
        Tree::parse("<body><p></p><div id=second style='width:23px;height:9px'></div></body>");
    let fonts = FontRegistry::new();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let lay = |tree: &Tree| {
        layout_html_tree_recorded_in_viewport_with(
            tree,
            "body{margin:0}",
            (120.0, 80.0),
            &fonts,
            &images,
            &HtmlOptions::default(),
            |_| {},
            &mut Diagnostics::default(),
        )
    };
    let a = lay(&first);
    let b = lay(&second);
    assert_eq!(
        source_box(&a, first.get_element_by_id("first").unwrap()).width,
        11.0
    );
    assert_eq!(
        source_box(&b, second.get_element_by_id("second").unwrap()).width,
        23.0
    );
    assert_eq!(
        source_box(&a, first.get_element_by_id("first").unwrap()).width,
        11.0
    );
}

#[test]
fn capture_keeps_zero_area_clipped_boxes_and_their_positioned_descendants() {
    fn boxes(fragment: &Fragment) -> Vec<&Fragment> {
        let mut out = vec![fragment];
        for child in &fragment.children {
            out.extend(boxes(child));
        }
        out
    }
    let tree = Tree::parse("<!doctype html><html><body style='margin:0'><div id=target style='position:absolute;top:20px;width:1px;height:20px;clip:rect(0,0,0,0)'><div style='position:absolute;top:220px;width:11px;height:9px;background:red'></div></div></body></html>");
    let target = tree.get_element_by_id("target").unwrap();
    let fonts = FontRegistry::new();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let capture = layout_html_tree_capture_in_viewport_with(
        &tree,
        "html{font-size:20px}",
        (100.0, 80.0),
        &fonts,
        &images,
        &HtmlOptions::default(),
        |nodes| patch_target(nodes, target),
        &mut Diagnostics::default(),
    );
    let fragments = boxes(&capture);
    let clipped = fragments.iter().find(|f| f.clip.is_some()).unwrap();
    assert_eq!(
        (clipped.x, clipped.y, clipped.width, clipped.height),
        (0.0, 20.0, 40.0, 20.0)
    );
    assert_eq!(clipped.clip.unwrap().width, 0.0);
    assert_eq!(clipped.clip.unwrap().height, 0.0);
    assert!(fragments
        .iter()
        .any(|f| (f.x, f.y, f.width, f.height) == (0.0, 240.0, 11.0, 9.0)));
    let ordinary = layout_html_tree_in_viewport_with(
        &tree,
        "html{font-size:20px}",
        (100.0, 80.0),
        &fonts,
        &images,
        &HtmlOptions::default(),
        |_| {},
        &mut Diagnostics::default(),
    );
    assert!(boxes(&ordinary).iter().all(|f| f.clip.is_none()));
}

fn inspect_shadow_layers(nodes: &[StyledNode], target: Handle) -> Vec<turbo_dom_layout::BoxShadow> {
    for node in nodes {
        if let StyledNode::Element(element) = node {
            if element.source == Some(target) {
                return turbo_dom_layout::layout::value::box_shadow_layers(
                    &element.style,
                    turbo_dom_layout::layout::value::ResolveCtx {
                        parent_font_size: 16.0,
                        cb_width: 120.0,
                    },
                );
            }
            let found = inspect_shadow_layers(&element.children, target);
            if !found.is_empty() {
                return found;
            }
        }
    }
    vec![]
}

#[test]
fn optional_recording_hook_observes_cascade_and_changes_geometry_before_layout() {
    use turbo_dom_layout::layout_html_tree_in_viewport_with_optional_sources;
    let tree = Tree::parse("<!doctype html><div id=target></div>");
    let target = tree.get_element_by_id("target").unwrap();
    let fonts = FontRegistry::new();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    for shadow in ["none", "1px 2px red,3px 4px blue"] {
        let css = format!("html{{font-size:20px}}body{{margin:0;background:rgb(1,2,3)}}#target{{width:1px;height:10px;box-shadow:{shadow}}}");
        let mut observed = vec![];
        let (galley, sources) = layout_html_tree_in_viewport_with_optional_sources(
            &tree,
            &css,
            (120.0, 80.0),
            &fonts,
            &images,
            &HtmlOptions::default(),
            false,
            |nodes| {
                observed = inspect_shadow_layers(nodes, target);
                patch_target(nodes, target);
                observed.len() > 1
            },
            &mut Diagnostics::default(),
        );
        assert_eq!(sources.is_some(), shadow != "none");
        let ordinary = layout_html_tree_in_viewport_with(
            &tree,
            &css,
            (120.0, 80.0),
            &fonts,
            &images,
            &HtmlOptions::default(),
            |nodes| patch_target(nodes, target),
            &mut Diagnostics::default(),
        );
        assert_eq!(format!("{galley:?}"), format!("{ordinary:?}"));
        if let Some(sources) = sources {
            let target_box = source_box(&LaidOut { galley, sources }, target).clone();
            assert_eq!(target_box.width, 40.0);
            assert_eq!((observed[1].offset_x, observed[1].offset_y), (3, 4));
        }
    }
}

#[test]
fn optional_source_hook_keeps_capture_clips_and_logical_edge_pairs() {
    use turbo_dom_layout::layout_html_tree_in_viewport_with_optional_sources;
    let tree = Tree::parse("<!doctype html><div id=target style='width:10px;height:10px;padding-inline:2px 3px;padding-block:4px 5px;margin-inline:1px 2px 3px;clip:rect(0,0,0,0)'></div>");
    let target = tree.get_element_by_id("target").unwrap();
    let fonts = FontRegistry::new();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let (galley, sources) = layout_html_tree_in_viewport_with_optional_sources(
        &tree,
        "body{margin:0}",
        (120.0, 80.0),
        &fonts,
        &images,
        &HtmlOptions::default(),
        true,
        |_| true,
        &mut Diagnostics::default(),
    );
    let laid = LaidOut {
        galley,
        sources: sources.unwrap(),
    };
    let f = source_box(&laid, target);
    assert_eq!((f.width, f.height), (15.0, 19.0));
}

#[test]
fn logical_edge_pairs_preserve_physical_fallback_for_malformed_pairs() {
    use turbo_dom_layout::layout::value::{resolve_box_style, ResolveCtx};
    let style = ComputedStyle::from_pairs([
        ("margin", "7px"),
        ("margin-block", "2px 3px"),
        ("margin-inline", "1px 2px 3px"),
    ]);
    let used = resolve_box_style(
        &style,
        ResolveCtx {
            parent_font_size: 16.0,
            cb_width: 120.0,
        },
    );
    assert_eq!((used.margin.top, used.margin.bottom), (2.0, 3.0));
    assert_eq!((used.margin.left, used.margin.right), (7.0, 7.0));
}

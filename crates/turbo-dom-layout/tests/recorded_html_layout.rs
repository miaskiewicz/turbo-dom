use turbo_dom_layout::style::ComputedStyle;
use turbo_dom_layout::turbo_dom::{Handle, Tree};
use turbo_dom_layout::{
    layout_html_tree_in_viewport_with, layout_html_tree_recorded_in_viewport_with, Diagnostics,
    FontRegistry, Fragment, FragmentContent, HtmlOptions, ImageCtx, LaidOut, NoImages, StyledNode,
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

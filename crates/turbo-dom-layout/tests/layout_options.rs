//! Runtime layout options and source recording (consolidation S2): markers from a
//! classifier, internal links laid out atomically, per-render node-id ranges and
//! the recorded box -> source map html2pdf derives PDF semantics from.

mod common;

use turbo_dom_layout::turbo_dom::Tree;
use turbo_dom_layout::{
    build_cascade, layout_recorded, layout_with_options, parse_html_tree, style_input, Fragment,
    FragmentContent, ImageCtx, LaidOut, LayoutOptions, NoImages, NodeId, Roots, StyleInput,
    StyleOptions, StyledNode, TreeView,
};

fn styled(tree: &Tree, css: &str) -> Vec<StyledNode> {
    let view = TreeView::new(tree).markers(&common::marker_of);
    let body = view.find_element("body").unwrap();
    let roots = tree.children(body);
    style_input(
        &StyleInput {
            view,
            roots: Roots::Detached(&roots),
        },
        &build_cascade(css, "", turbo_dom_layout::style::TokenSet::default()),
        &StyleOptions::default(),
    )
}

fn lay(tree: &Tree, css: &str, options: LayoutOptions) -> LaidOut {
    layout_recorded(
        &styled(tree, css),
        400.0,
        &common::registry(),
        &ImageCtx {
            resolver: &NoImages,
            body_height: None,
        },
        &options,
        &mut turbo_dom_layout::Diagnostics::default(),
    )
}

fn all(frag: &Fragment, out: &mut Vec<Fragment>) {
    out.push(frag.clone());
    for child in &frag.children {
        all(child, out);
    }
}

fn fragments(galley: &Fragment) -> Vec<Fragment> {
    let mut out = Vec::new();
    all(galley, &mut out);
    out
}

fn source_name(tree: &Tree, laid: &LaidOut, id: NodeId) -> Option<String> {
    let h = *laid.sources.elements.get(&id)?;
    tree.local_name(h).map(str::to_string)
}

#[test]
fn internal_links_get_their_own_box_only_when_asked() {
    let tree =
        parse_html_tree(r##"<body><p>a <a href="#t">link</a> <a href="#">x</a> b</p></body>"##);
    let boxed = |options| {
        let laid = lay(&tree, "", options);
        fragments(&laid.galley)
            .iter()
            .filter(|f| matches!(f.content, FragmentContent::Box { .. }))
            .filter_map(|f| source_name(&tree, &laid, f.node_id))
            .filter(|name| name == "a")
            .count()
    };
    assert_eq!(boxed(LayoutOptions::default()), 0);
    let on = LayoutOptions {
        atomic_internal_links: true,
        ..LayoutOptions::default()
    };
    assert_eq!(boxed(on), 1, "only the non-empty #target link is atomic");
}

#[test]
fn recorded_sources_cover_elements_texts_rows_and_the_id_range() {
    let tree = parse_html_tree(
        "<body><div id=d>t<table><tr id=r><td>c</td></tr></table></div><p>z</p></body>",
    );
    let first = 100;
    let laid = lay(
        &tree,
        "",
        LayoutOptions {
            first_node_id: first,
            ..LayoutOptions::default()
        },
    );
    assert_eq!(laid.sources.root, NodeId(first));
    assert_eq!(laid.galley.node_id, NodeId(first));
    let frags = fragments(&laid.galley);
    assert!(frags
        .iter()
        .all(|f| f.node_id.0 >= first && f.node_id.0 < laid.sources.next.0));
    let row = laid
        .sources
        .elements
        .iter()
        .find(|(_, &h)| tree.get_attribute(h, "id") == Some("r"))
        .map(|(id, _)| *id)
        .expect("row recorded");
    assert!(laid.sources.table_rows.contains(&row));
    assert_eq!(laid.sources.table_rows.len(), 1);
    let texts: Vec<_> = laid
        .sources
        .texts
        .values()
        .map(|&h| tree.character_data(h).unwrap().into_owned())
        .collect();
    assert!(texts.contains(&"t".to_string()) && texts.contains(&"z".to_string()));
    // The same document without recording/shift lays out identically (ids aside).
    let plain = layout_with_options(
        &styled(&tree, ""),
        400.0,
        &common::registry(),
        &ImageCtx {
            resolver: &NoImages,
            body_height: None,
        },
        &LayoutOptions::default(),
        &mut turbo_dom_layout::Diagnostics::default(),
    );
    let shape = |f: &Fragment| (f.x, f.y, f.width, f.height);
    let plain_frags = fragments(&plain);
    assert_eq!(plain_frags.len(), frags.len());
    for (a, b) in plain_frags.iter().zip(&frags) {
        assert_eq!(shape(a), shape(b));
        assert_eq!(a.node_id.0 + first, b.node_id.0);
    }
}

#[test]
fn markers_are_opaque_kept_and_unmatched_by_type_selectors() {
    let tree = parse_html_tree(
        "<body><p>a<x-marker-1-i style='display:none'>n</x-marker-1-i>b</p>\
         <x-marker-2-b style='display:inline;color:red'>block</x-marker-2-b></body>",
    );
    let css = "x-marker-2-b { margin-top: 9px } p { color: blue }";
    let nodes = styled(&tree, css);
    let StyledNode::Element(block) = &nodes[1] else {
        panic!("block marker")
    };
    assert_eq!(
        block.style.get("margin-top"),
        None,
        "type selectors skip markers"
    );
    assert_eq!(block.style.get("color"), Some("red"), "style is kept");
    let laid = lay(&tree, css, LayoutOptions::default());
    let markers: Vec<_> = fragments(&laid.galley)
        .into_iter()
        .filter_map(|f| match f.content {
            FragmentContent::Marker(kind) => Some((kind.id, kind.inline, f.width, f.height)),
            _ => None,
        })
        .collect();
    assert!(
        markers.contains(&(1, true, 0.0, 0.0)),
        "hidden inline marker kept"
    );
    assert!(
        markers.iter().any(|m| m.0 == 2 && !m.1),
        "block marker laid out as a block box"
    );
}

/// miaskiewicz/turbo-dom#51: a captioned table is a table wrapper box. Its fragment holds
/// the caption's and the anonymous grid box's (recorded in `table_grids`, unsourced), the
/// grid's the rows; the table's background paints the grid only.
#[test]
fn recorded_sources_mark_the_grid_box_of_a_captioned_table() {
    let tree = parse_html_tree(
        "<body><table id=t style='background:#f00'><caption id=c>x</caption><tr id=r><td>c</td></tr></table><table><tr><td>d</td></tr></table></body>",
    );
    let laid = lay(&tree, "", LayoutOptions::default());
    let id_of = |name: &str| {
        laid.sources
            .elements
            .iter()
            .find(|(_, &h)| tree.get_attribute(h, "id") == Some(name))
            .map(|(id, _)| *id)
            .unwrap()
    };
    assert_eq!(
        laid.sources.table_grids.len(),
        1,
        "only the captioned table"
    );
    let grid = *laid.sources.table_grids.iter().next().unwrap();
    assert!(!laid.sources.elements.contains_key(&grid));
    let frags = fragments(&laid.galley);
    let table = frags.iter().find(|f| f.node_id == id_of("t")).unwrap();
    let kids: Vec<NodeId> = table.children.iter().map(|f| f.node_id).collect();
    assert_eq!(kids, [id_of("c"), grid]);
    let grid_frag = &table.children[1];
    assert_eq!(grid_frag.children[0].node_id, id_of("r"));
    let background = |f: &Fragment| match &f.content {
        FragmentContent::Box { background, .. } => *background,
        _ => None,
    };
    assert_eq!(background(table), None);
    assert!(background(grid_frag).is_some());
}

/// miaskiewicz/turbo-html2pdf#39: `layout_recorded_in_viewport` gives the recorded layout
/// an initial containing block of the given height (a paged layout's page area): a fixed
/// box resolves `%` insets, a `%` height and `bottom` against it; `layout_recorded` (and a
/// height of 0) leaves it unknown, the box at its static position with no `%` height. The
/// galley and sources are otherwise those of `layout_recorded`.
#[test]
fn recorded_layout_in_a_viewport_resolves_fixed_insets_against_its_height() {
    let html = "<html><body><div style=\"height:7px\"></div><div id=\"a\" style=\"position:fixed;top:50%;left:0;width:10px;height:25%\"></div><div id=\"b\" style=\"position:fixed;bottom:20px;right:0;width:10px;height:30px\"></div></body></html>";
    let tree = Tree::parse(html);
    let laid = |height: Option<f32>| {
        let styled = styled(&tree, "");
        let images = ImageCtx {
            resolver: &NoImages,
            body_height: None,
        };
        let mut diags = turbo_dom_layout::Diagnostics::default();
        let options = LayoutOptions::default();
        match height {
            Some(h) => turbo_dom_layout::layout_recorded_in_viewport(
                &styled,
                (400.0, h),
                &common::registry(),
                &images,
                &options,
                &mut diags,
            ),
            None => layout_recorded(
                &styled,
                400.0,
                &common::registry(),
                &images,
                &options,
                &mut diags,
            ),
        }
    };
    let rects = |laid: &LaidOut| -> Vec<(String, f32, f32, f32, f32)> {
        fragments(&laid.galley)
            .iter()
            .filter_map(|f| {
                let h = *laid.sources.elements.get(&f.node_id)?;
                let id = tree.get_attribute(h, "id")?.to_string();
                Some((id, f.x, f.y, f.width, f.height))
            })
            .collect()
    };
    let paged = laid(Some(320.0));
    assert_eq!(
        rects(&paged),
        vec![
            ("a".to_string(), 0.0, 160.0, 10.0, 80.0),
            ("b".to_string(), 390.0, 270.0, 10.0, 30.0),
        ]
    );
    for unknown in [laid(None), laid(Some(0.0))] {
        assert_eq!(
            rects(&unknown),
            vec![
                ("a".to_string(), 0.0, 0.0, 10.0, 0.0),
                ("b".to_string(), 390.0, 7.0, 10.0, 30.0),
            ]
        );
        assert_eq!(unknown.sources, laid(None).sources);
    }
    assert_eq!(paged.sources, laid(None).sources);
}

/// With the initial containing block's height unknown (`layout_recorded`: html2pdf's
/// region / footnote flows), boxes keep their previous bases: a fixed box inside a
/// positioned ancestor anchors `bottom` to that ancestor (the paged layout re-anchors
/// fixed boxes to the page area itself), and an absolutely positioned box of the initial
/// containing block with `top` + `bottom` keeps its content height (no stretch against
/// an unknown height; miaskiewicz/turbo-dom#81, #89).
#[test]
fn an_unknown_initial_containing_block_keeps_the_previous_bases() {
    let html = "<html><body><div style=\"position:relative;height:100px\"><div id=\"f\" style=\"position:fixed;bottom:0;left:0;width:10px;height:20px\"></div></div><div id=\"s\" style=\"position:absolute;top:10px;bottom:20px;left:0;width:10px\"><div style=\"height:30px\"></div></div></body></html>";
    let tree = Tree::parse(html);
    let laid = lay(&tree, "", LayoutOptions::default());
    let rect = |id: &str| {
        fragments(&laid.galley)
            .into_iter()
            .find(|f| {
                laid.sources
                    .elements
                    .get(&f.node_id)
                    .is_some_and(|&h| tree.get_attribute(h, "id") == Some(id))
            })
            .map(|f| (f.x, f.y, f.width, f.height))
            .unwrap()
    };
    assert_eq!(rect("f"), (0.0, 80.0, 10.0, 20.0));
    assert_eq!(rect("s"), (0.0, 10.0, 10.0, 30.0));
}

fn count(nodes: &[StyledNode], n: &mut usize) {
    for node in nodes {
        if let StyledNode::Element(e) = node {
            *n += usize::from(matches!(e.tag, turbo_dom_layout::Tag::Marker(_)));
            count(&e.children, n);
        }
    }
}

fn marker_options() -> turbo_dom_layout::HtmlOptions<'static> {
    turbo_dom_layout::HtmlOptions {
        markers: Some(&common::marker_of),
        ..turbo_dom_layout::HtmlOptions::default()
    }
}

#[test]
fn html_drive_classifies_markers_and_they_take_no_intrinsic_width() {
    // A block marker is a zero-size box: as a flex item it adds nothing to its
    // shrink-to-fit ancestor's max-content width, and a fullscreen sibling still hoists
    // to the top layer around it (the marker stays in place).
    let html = "<body style='margin:0'>\
        <div data-cid=w style='float:left'><div style='display:flex'>\
        <x-marker-2-b></x-marker-2-b><p data-cid=p style='margin:0;width:30px;height:5px'></p></div></div>\
        <div><x-marker-3-b data-cid=m></x-marker-3-b><div data-cid=f style='height:7px'></div></div></body>";
    let mut tree = parse_html_tree(html);
    let f = tree.query_selector("[data-cid=\"f\"]").unwrap();
    let styled = turbo_dom_layout::style_html_tree(&tree, "", 400.0, &marker_options());
    let mut markers = 0;
    count(&styled, &mut markers);
    assert_eq!(markers, 2, "both x-marker elements classify");
    let plain = turbo_dom_layout::style_html_tree(
        &tree,
        "",
        400.0,
        &turbo_dom_layout::HtmlOptions::default(),
    );
    let mut none = 0;
    count(&plain, &mut none);
    assert_eq!(none, 0, "no classifier, no markers");

    assert!(tree.set_fullscreen_stack(&[f]));
    let boxes = turbo_dom_layout::layout_boxes_tree(
        &tree,
        "",
        400.0,
        300.0,
        &common::registry(),
        &marker_options(),
        &mut turbo_dom_layout::Diagnostics::default(),
    );
    let get = |cid: &str| {
        let b = boxes.iter().find(|b| b.cid == cid).unwrap();
        (b.x, b.y, b.width, b.height)
    };
    assert_eq!(get("w"), (0.0, 0.0, 30.0, 5.0));
    assert_eq!(get("p"), (0.0, 0.0, 30.0, 5.0));
    assert_eq!(
        get("f"),
        (0.0, 0.0, 400.0, 300.0),
        "fullscreen fills the viewport"
    );
}

#[test]
fn snapshot_records_marker_fragments_as_marker_geometry() {
    use turbo_dom_layout::layout_snapshot::{
        snapshot_live_tree, GeometryKind, SnapshotInputs, SourceId, SourceIdentity,
    };
    use turbo_dom_layout::test_support::literal::{tree, Element, Node};
    use turbo_dom_layout::{MarkerKind, Tag};
    let nodes = vec![Node::Element(Element {
        tag: Tag::Html("div".into()),
        attrs: vec![],
        children: vec![Node::Element(Element {
            tag: Tag::Marker(MarkerKind {
                id: 4,
                inline: false,
            }),
            attrs: vec![],
            children: vec![],
        })],
    })];
    let fonts = turbo_dom_layout::FontRegistry::new();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let snapshot = snapshot_live_tree(
        &tree(&nodes).input(),
        &[
            SourceIdentity {
                id: SourceId(1),
                path: vec![0],
            },
            SourceIdentity {
                id: SourceId(2),
                path: vec![0, 0],
            },
        ],
        &build_cascade("", "", turbo_dom_layout::style::TokenSet::default()),
        &SnapshotInputs {
            width: 100.0,
            height: 100.0,
            fonts: &fonts,
            images: &images,
        },
        &mut turbo_dom_layout::Diagnostics::default(),
    )
    .unwrap();
    let kinds: Vec<_> = snapshot
        .source(SourceId(2))
        .unwrap()
        .fragment_indices
        .iter()
        .map(|&i| snapshot.fragments()[i].kind)
        .collect();
    assert_eq!(kinds, [GeometryKind::Marker]);
    let marker = &snapshot.fragments()[snapshot.source(SourceId(2)).unwrap().fragment_indices[0]];
    assert_eq!(
        (marker.rect.width, marker.rect.height),
        (100.0, 0.0),
        "a block marker is a zero-height block"
    );
}

#[test]
fn canvas_background_needs_an_html_root() {
    use turbo_dom_layout::html_layout::propagate_canvas_background;
    use turbo_dom_layout::Rgba;
    let tree = parse_html_tree("<html style='background:rgb(1, 2, 3)'><body></body></html>");
    let mut document = turbo_dom_layout::style_html_tree(
        &tree,
        "",
        100.0,
        &turbo_dom_layout::HtmlOptions::default(),
    );
    // Another root before the document element is skipped.
    let mut fragment = styled(
        &parse_html_tree("<body><p style='background:red'>x</p></body>"),
        "",
    );
    assert_eq!(
        propagate_canvas_background(&mut fragment),
        None,
        "no <html> root"
    );
    fragment.append(&mut document);
    // The root paints none: the body's colour moves to the canvas (past <head>).
    let tree =
        parse_html_tree("<html><head></head><body style='background:rgb(4, 5, 6)'></body></html>");
    let mut body_only = turbo_dom_layout::style_html_tree(
        &tree,
        "",
        100.0,
        &turbo_dom_layout::HtmlOptions::default(),
    );
    assert_eq!(
        propagate_canvas_background(&mut body_only),
        Some(Rgba::new(4, 5, 6, 255))
    );
    assert_eq!(
        propagate_canvas_background(&mut fragment),
        Some(Rgba::new(1, 2, 3, 255))
    );
}

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

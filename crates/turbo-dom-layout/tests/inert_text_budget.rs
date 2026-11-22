use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_text, LayoutSnapshot, SnapshotInputs, SourceId, SourceIdentity,
    TextSnapshotError,
};
use turbo_dom_layout::test_support::literal::{self, Element, Node};
use turbo_dom_layout::{build_cascade, Attr, Diagnostics, ImageCtx, NoImages, Tag};

fn element(tag: &str, style: &str, children: Vec<Node>) -> Node {
    Node::Element(Element {
        tag: Tag::Html(tag.into()),
        attrs: vec![Attr {
            name: "style".into(),
            value: style.into(),
        }],
        children,
    })
}

fn capture(hidden: Node) -> Result<LayoutSnapshot, TextSnapshotError> {
    let nodes = [
        hidden,
        element(
            "div",
            "width:100px;height:20px",
            vec![Node::Text("ok".into())],
        ),
    ];
    let mut hidden_text = &nodes[0];
    let mut hidden_path = vec![0];
    while let Node::Element(element) = hidden_text {
        hidden_text = &element.children[0];
        hidden_path.push(0);
    }
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    snapshot_html_live_tree_with_text(
        &literal::tree(&nodes).input(),
        &[
            SourceIdentity {
                id: SourceId(10),
                path: vec![0],
            },
            SourceIdentity {
                id: SourceId(11),
                path: hidden_path,
            },
            SourceIdentity {
                id: SourceId(20),
                path: vec![1],
            },
            SourceIdentity {
                id: SourceId(21),
                path: vec![1, 0],
            },
        ],
        &build_cascade(
            "script{display:none}",
            "",
            turbo_dom_layout::style::TokenSet::default(),
        ),
        &SnapshotInputs {
            width: 200.0,
            height: 100.0,
            fonts: &fonts,
            images: &images,
        },
        &mut Diagnostics::default(),
    )
}

fn assert_visible_sibling(snapshot: &LayoutSnapshot) {
    assert_eq!(
        snapshot.text_original_units(SourceId(11)).unwrap().len(),
        1_000_100
    );
    assert!(!snapshot
        .text_runs()
        .iter()
        .any(|run| run.source_id == Some(SourceId(11))));
    assert!(snapshot.fragments().iter().any(|fragment| {
        fragment.source_id == Some(SourceId(20)) && fragment.rect.height > 0.0
    }));
    assert!(snapshot
        .text_runs()
        .iter()
        .any(|run| run.source_id == Some(SourceId(21))));
}

#[test]
fn inert_script_source_does_not_spend_rendered_glyph_allowance() {
    let snapshot = capture(element(
        "script",
        "",
        vec![Node::Text("x".repeat(1_000_100))],
    ))
    .unwrap();
    assert_visible_sibling(&snapshot);
}

#[test]
fn a_display_none_ancestor_keeps_descendants_out_of_the_glyph_allowance() {
    let snapshot = capture(element(
        "div",
        "display:none",
        vec![element(
            "span",
            "display:block",
            vec![Node::Text("x".repeat(1_000_100))],
        )],
    ))
    .unwrap();
    assert_visible_sibling(&snapshot);
}

#[test]
fn hidden_source_still_obeys_the_original_raw_byte_budget() {
    assert!(matches!(
        capture(element(
            "div",
            "display:none",
            vec![Node::Text(" ".repeat(16 * 1024 * 1024 + 1),)]
        )),
        Err(TextSnapshotError::Budget)
    ));
}

#[test]
fn an_author_display_override_restores_the_rendered_character_budget() {
    assert!(matches!(
        capture(element(
            "script",
            "display:block",
            vec![Node::Text("x".repeat(1_000_100),)]
        )),
        Err(TextSnapshotError::Budget)
    ));
}

#[test]
fn visibility_hidden_still_spends_rendered_character_allowance() {
    assert!(matches!(
        capture(element(
            "div",
            "visibility:hidden",
            vec![Node::Text("x".repeat(1_000_100),)]
        )),
        Err(TextSnapshotError::Budget)
    ));
}

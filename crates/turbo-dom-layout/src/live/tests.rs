use std::collections::HashMap;

use turbo_dom::{Handle, Tree};

use super::testing::{capture_light, light_children, DOCUMENT};
use super::*;
use crate::layout_snapshot::GeometryKind;
use crate::style::parse_declarations;
use crate::{FontRegistry, ImageCtx, NoImages, StyledElement};

fn live<'a>(tree: &'a Tree, provider: &'a dyn Fn(Handle) -> &'a [Handle]) -> LiveTree<'a> {
    LiveTree {
        tree,
        document: DOCUMENT,
        children: provider,
    }
}

fn captured(tree: &Tree) -> Result<LiveCapture, LiveError> {
    let children = light_children(tree);
    let provider = |h: Handle| children.get(&h).map_or(&[][..], Vec::as_slice);
    capture(&live(tree, &provider))
}

fn path(capture: &LiveCapture, node: Handle) -> Vec<usize> {
    capture
        .identities
        .iter()
        .find(|identity| identity.id == source_id(DOCUMENT, node))
        .unwrap()
        .path
        .clone()
}

fn body(tree: &Tree) -> Handle {
    tree.query_selector("body").unwrap()
}

#[test]
fn source_ids_carry_the_document_and_the_handle() {
    let tree = Tree::parse("<p></p>");
    let p = tree.query_selector("p").unwrap();
    let id = source_id(7, p);
    assert_eq!(source_document(id), 7);
    assert_eq!(source_handle(id), p);
    assert_ne!(source_id(8, p), id);
    let empty = LiveCapture::empty(3, 9);
    assert_eq!((empty.document, empty.tree_version), (3, 9));
    assert!(empty.identities.is_empty() && empty.body.is_none() && !empty.quirks);
}

#[test]
fn paths_count_kept_composed_children_and_skip_comments_doctypes_and_pis() {
    let mut tree = Tree::parse("<!doctype html><body><i id=a></i><!--c--><b id=b>t</b></body>");
    let body = body(&tree);
    let pi = tree.create_processing_instruction("x", "y");
    tree.insert_before(body, pi, tree.children(body).first().copied());
    let (a, b) = (
        tree.get_element_by_id("a").unwrap(),
        tree.get_element_by_id("b").unwrap(),
    );
    let capture = captured(&tree).unwrap();
    assert_eq!(path(&capture, a), [0, 1, 0]);
    assert_eq!(path(&capture, b), [0, 1, 1]);
    let text = tree.children(b)[0];
    assert_eq!(path(&capture, text), [0, 1, 1, 0]);
    // Children are recorded before their parents.
    let order: Vec<_> = capture
        .identities
        .iter()
        .map(|identity| identity.id)
        .collect();
    let at = |node| order.iter().position(|&id| id == source_id(DOCUMENT, node));
    assert!(at(text) < at(b) && at(b) < at(body));
    assert_eq!(
        capture.original_text[&source_id(DOCUMENT, text)],
        "t".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(capture.body, Some(source_id(DOCUMENT, body)));
    assert!(!capture.quirks);
    assert_eq!(capture.tree_version, tree.version);
}

#[test]
fn a_composed_provider_reorders_and_hides_children() {
    let tree = Tree::parse("<body><i id=a></i><b id=b></b><u id=c></u></body>");
    let body = body(&tree);
    let [a, b, c] = ["a", "b", "c"].map(|id| tree.get_element_by_id(id).unwrap());
    let mut children = light_children(&tree);
    children.insert(body, vec![c, a]);
    let provider = |h: Handle| children.get(&h).map_or(&[][..], Vec::as_slice);
    let capture = capture(&live(&tree, &provider)).unwrap();
    assert_eq!(path(&capture, c), [0, 1, 0]);
    assert_eq!(path(&capture, a), [0, 1, 1]);
    assert!(capture
        .identities
        .iter()
        .all(|identity| identity.id != source_id(DOCUMENT, b)));
}

#[test]
fn quirks_documents_and_framesets_report_their_body() {
    let tree = Tree::parse("<p>x</p>");
    let capture = captured(&tree).unwrap();
    assert!(capture.quirks);
    assert_eq!(capture.body, Some(source_id(DOCUMENT, body(&tree))));
    let frames = Tree::parse("<!doctype html><frameset></frameset>");
    assert_eq!(captured(&frames).unwrap().body, None);
}

#[test]
fn only_the_outermost_svg_is_a_viewport_and_other_namespaces_are_rejected() {
    let tree = Tree::parse("<svg id=o><svg id=i></svg></svg><p><svg id=s></svg></p>");
    let capture = captured(&tree).unwrap();
    let ids = ["o", "s"].map(|id| source_id(DOCUMENT, tree.get_element_by_id(id).unwrap()));
    assert_eq!(capture.svg_viewports, ids);
    let mut tree = Tree::parse("");
    let body = body(&tree);
    let other = tree.create_element_namespace(None, "x");
    tree.append_child(body, other);
    assert_eq!(captured(&tree).unwrap_err(), LiveError::Namespace);
}

// miaskiewicz/turbo-dom#376: a composed MathML subtree no longer fails the capture; its
// outermost element is tracked as a MathML root so the style pass can give it placeholder
// box metrics (inline-block, 0×0), and nested `<math>` subtrees never nest their roots.
#[test]
fn only_the_outermost_math_is_a_mathml_root() {
    let tree = Tree::parse(
        "<math id=o><mrow><mi>x</mi><math id=i></math></mrow></math><p><math id=s></math></p>",
    );
    let capture = captured(&tree).unwrap();
    let ids = ["o", "s"].map(|id| source_id(DOCUMENT, tree.get_element_by_id(id).unwrap()));
    assert_eq!(capture.math_roots, ids);
}

// miaskiewicz/turbo-dom#376: the placeholder MathML style. The root is a 0x0
// inline-block and every MathML element under it takes `display: none`, so no MathML
// content (here a `<mrow>` with an `<mi>`'s text) gets a box until MathML layout lands.
#[test]
fn mathml_descendants_take_display_none_under_a_zero_sized_root() {
    let (tree, snapshot, styles) = super::testing::layout(
        "<body style='margin:0'><math id=m><mrow id=r><mi id=x>x</mi></mrow></math></body>",
        "",
        (800.0, 600.0),
        Some(1.0),
    );
    let m = tree.get_element_by_id("m").unwrap();
    assert_eq!(
        styles[&source_id(DOCUMENT, m)].style.get("display"),
        Some("inline-block")
    );
    assert_eq!(border(&snapshot, m), (0.0, 0.0));
    for id in ["r", "x"] {
        let node = tree.get_element_by_id(id).unwrap();
        let source = source_id(DOCUMENT, node);
        assert_eq!(styles[&source].style.get("display"), Some("none"), "{id}");
        assert!(
            snapshot
                .source(source)
                .is_none_or(|placed| placed.fragment_indices.is_empty()),
            "{id} is laid out"
        );
    }
}

#[test]
fn budgets_follow_the_counting_rules() {
    // Nodes: counted before filtering (3 elements + comments).
    let mut tree = Tree::parse("");
    let body = body(&tree);
    for _ in 0..MAX_NODES - 3 {
        let comment = tree.create_comment("");
        tree.append_child(body, comment);
    }
    assert_eq!(captured(&tree).unwrap().identities.len(), 3);
    let comment = tree.create_comment("");
    tree.append_child(body, comment);
    assert_eq!(captured(&tree).unwrap_err(), LiveError::NodeBudget);
    // Depth: elements up to MAX_DEPTH (html 1, body 2).
    let mut tree = Tree::parse("");
    let mut parent = self::body(&tree);
    for _ in 0..MAX_DEPTH - 2 {
        let div = tree.create_element("div");
        tree.append_child(parent, div);
        parent = div;
    }
    let text = tree.create_text_node("x");
    tree.append_child(parent, text);
    captured(&tree).unwrap();
    let deeper = tree.create_element("i");
    tree.append_child(parent, deeper);
    assert_eq!(captured(&tree).unwrap_err(), LiveError::DepthBudget);
    // Text: the lossy UTF-8 length (a lone surrogate is 3 bytes).
    let mut tree = Tree::parse("");
    let body = self::body(&tree);
    let lone = tree.create_text_node_utf16(&vec![0xDC00; MAX_TEXT_BYTES / 3]);
    tree.append_child(body, lone);
    let rest = tree.create_text_node(&"x".repeat(MAX_TEXT_BYTES % 3));
    tree.append_child(body, rest);
    captured(&tree).unwrap();
    tree.append_data(rest, "x");
    assert_eq!(captured(&tree).unwrap_err(), LiveError::TextBudget);
    // Attributes: qualified names plus values.
    let mut tree = Tree::parse("<svg><use></use></svg>");
    let use_element = tree.query_selector("use").unwrap();
    let value = "a".repeat(MAX_ATTRIBUTE_BYTES - "xlink:href".len());
    tree.set_attribute_ns(
        use_element,
        Some("http://www.w3.org/1999/xlink"),
        "xlink:href",
        &value,
    );
    captured(&tree).unwrap();
    tree.set_attribute(use_element, "x", "");
    assert_eq!(captured(&tree).unwrap_err(), LiveError::AttributeBudget);
    let mut left = 0;
    assert_eq!(
        capture::charge(&mut left, usize::MAX, 1),
        Err(LiveError::AttributeOverflow)
    );
}

#[test]
fn the_first_violation_in_composed_order_wins() {
    let mut tree = Tree::parse("");
    let body = body(&tree);
    let foreign = tree.create_element_namespace(None, "x");
    tree.set_attribute(foreign, "title", &"a".repeat(MAX_ATTRIBUTE_BYTES));
    tree.append_child(body, foreign);
    let text = tree.create_text_node(&"x".repeat(MAX_TEXT_BYTES + 1));
    tree.append_child(body, text);
    assert_eq!(captured(&tree).unwrap_err(), LiveError::Namespace);
    tree.remove_child(body, foreign);
    tree.append_child(body, foreign);
    assert_eq!(captured(&tree).unwrap_err(), LiveError::TextBudget);
}

struct Styled {
    tree: Tree,
    children: HashMap<Handle, Vec<Handle>>,
}

impl Styled {
    fn new(html: &str) -> Self {
        let tree = Tree::parse(html);
        let children = light_children(&tree);
        Styled { tree, children }
    }

    fn el(&self, id: &str) -> Handle {
        self.tree.get_element_by_id(id).unwrap()
    }

    fn run<T>(
        &self,
        origins: &[OriginRules],
        overrides: &[(Handle, &str)],
        depth: &dyn Fn(Handle) -> Result<u32, String>,
        hook: impl FnOnce(&mut [StyledNode]) -> Result<T, String>,
    ) -> Result<(LayoutSnapshot, T), LiveError> {
        let provider = |h: Handle| self.children.get(&h).map_or(&[][..], Vec::as_slice);
        let capture = capture_light(&self.tree, &self.children);
        let overrides: HashMap<_, _> = overrides.iter().copied().collect();
        let inline_override = |h: Handle| overrides.get(&h).copied();
        let inline_declarations = |text: &str| {
            if text.contains("FAIL") {
                Err("inline parse failed".to_string())
            } else {
                Ok(parse_declarations(text))
            }
        };
        let style = LiveStyle {
            tree: live(&self.tree, &provider),
            origins,
            scope_depth: depth,
            inline_override: &inline_override,
            inline_declarations: &inline_declarations,
        };
        let fonts = FontRegistry::new();
        let images = ImageCtx {
            resolver: &NoImages,
            body_height: None,
        };
        snapshot(
            &style,
            &capture,
            &LiveOptions { layout_scale: 1.0 },
            hook,
            &SnapshotInputs {
                width: 200.0,
                height: 100.0,
                fonts: &fonts,
                images: &images,
            },
            &mut Diagnostics::default(),
        )
    }
}

fn rule(declarations: &str, targets: &[Handle], specificity: (u32, u32, u32)) -> PreMatched {
    PreMatched {
        depth: 0,
        declarations: parse_declarations(declarations),
        targets: targets
            .iter()
            .map(|&h| MatchedTarget {
                source: source_id(DOCUMENT, h),
                specificity,
            })
            .collect(),
    }
}

fn border(snapshot: &LayoutSnapshot, node: Handle) -> (f32, f32) {
    let source = snapshot.source(source_id(DOCUMENT, node)).unwrap();
    let fragment = source
        .fragment_indices
        .iter()
        .map(|&i| &snapshot.fragments()[i])
        .find(|f| f.kind == GeometryKind::BorderBox)
        .unwrap();
    (fragment.rect.width, fragment.rect.height)
}

fn find(nodes: &[StyledNode], node: Handle) -> Option<&StyledElement> {
    nodes.iter().find_map(|styled| {
        let element = styled.as_element()?;
        if element.source == Some(node) {
            Some(element)
        } else {
            find(&element.children, node)
        }
    })
}

#[test]
#[allow(clippy::similar_names)] // `snapshot` / `styles` name the two results of one call
fn pre_matched_rules_rank_by_origin_specificity_and_order_under_inline_and_hints() {
    let styled = Styled::new(
        "<body style='margin:0'><div id=a width=20 style='height:5px'></div><div id=b bgcolor=red width=30></div></body>",
    );
    let (a, b) = (styled.el("a"), styled.el("b"));
    // Chrome's `html.css` `div { display: block }` (miaskiewicz/turbo-dom#375): this
    // test builds its own origins instead of using `LIVE_UA`, so the block rule joins
    // the UA origin here.
    let ua = [
        rule("width:90px;height:90px", &[a, b], (0, 0, 1)),
        rule("display:block", &[a, b], (0, 0, 1)),
    ];
    let author = [
        rule("width:40px", &[a], (0, 1, 0)),
        rule("width:50px", &[a], (0, 0, 1)),
        rule("height:7px", &[b], (0, 0, 0)),
    ];
    let origins = [
        OriginRules {
            origin: CascadeOrigin::UserAgent,
            rules: ua.iter().map(Some).collect(),
        },
        OriginRules {
            origin: CascadeOrigin::Author,
            rules: author.iter().map(Some).collect(),
        },
    ];
    let (snapshot, styles) = styled
        .run(&origins, &[], &|_| Ok(0), |nodes| {
            Ok([a, b].map(|node| find(nodes, node).unwrap().style.clone()))
        })
        .unwrap();
    // The more specific author rule beats the later one; the inline height wins.
    assert_eq!(border(&snapshot, a), (40.0, 5.0));
    // Presentational hints sit under author rules but over the UA.
    assert_eq!(border(&snapshot, b), (30.0, 7.0));
    assert_eq!(styles[1].get("background-color"), Some("red"));
}

#[test]
fn the_inline_override_replaces_an_existing_style_attribute_only() {
    let mut styled = Styled::new(
        "<body style='margin:0'><div id=a style='width:10px;height:5px' title=t></div><div id=b></div></body>",
    );
    let (a, b) = (styled.el("a"), styled.el("b"));
    styled
        .tree
        .set_attribute_ns(a, Some("http://www.w3.org/1999/xlink"), "xlink:title", "x");
    styled.children = light_children(&styled.tree);
    let overrides = [(a, "width:30px;height:5px"), (b, "width:40px;height:5px")];
    // Chrome's `html.css` `div { display: block }` (miaskiewicz/turbo-dom#375): this
    // test builds its own origins instead of using `LIVE_UA`, so the block rule joins
    // the UA origin here (CSSOM-style inline overrides sit above the UA).
    let ua = [rule("display:block", &[a, b], (0, 0, 1))];
    let origins = [OriginRules {
        origin: CascadeOrigin::UserAgent,
        rules: ua.iter().map(Some).collect(),
    }];
    let (snapshot, attrs) = styled
        .run(&origins, &overrides, &|_| Ok(0), |nodes| {
            Ok(find(nodes, a).unwrap().attrs.clone())
        })
        .unwrap();
    assert_eq!(border(&snapshot, a), (30.0, 5.0));
    assert_eq!(border(&snapshot, b), (200.0, 0.0));
    let attrs: Vec<_> = attrs
        .iter()
        .map(|attr| (attr.name.as_str(), attr.value.as_str()))
        .collect();
    assert_eq!(
        attrs,
        [
            ("id", "a"),
            ("style", "width:30px;height:5px"),
            ("title", "t"),
            ("xlink:title", "x")
        ]
    );
    assert_eq!(
        styled.tree.get_attribute(a, "style"),
        Some("width:10px;height:5px")
    );
}

#[test]
fn text_sources_keep_their_handle_and_lossy_text() {
    let mut styled = Styled::new("<body><p id=p></p></body>");
    let p = styled.el("p");
    let text = styled.tree.create_text_node_utf16(&[0x61, 0xD800]);
    styled.tree.append_child(p, text);
    styled.children = light_children(&styled.tree);
    let (_, nodes) = styled
        .run(&[], &[], &|_| Ok(0), |nodes| {
            Ok(find(nodes, p).unwrap().children.clone())
        })
        .unwrap();
    let StyledNode::Text(styled_text) = &nodes[0] else {
        panic!("text expected")
    };
    assert_eq!(styled_text.text, "a\u{FFFD}");
    assert_eq!(styled_text.source, Some(text));
}

#[test]
fn declaration_budget_missing_scope_and_embedder_errors_are_reported() {
    let styled = Styled::new("<body><div id=a style='x:FAIL'></div><div id=b></div></body>");
    let (a, b) = (styled.el("a"), styled.el("b"));
    let half = MAX_DECLARATIONS / 2;
    let filler = [crate::style::Declaration {
        property: "height".into(),
        value: "1px".into(),
        important: false,
    }];
    let big = PreMatched {
        depth: 0,
        declarations: std::iter::repeat_n(filler[0].clone(), half).collect(),
        targets: rule("", &[a, b], (0, 0, 0)).targets,
    };
    let one = rule("height:2px", &[b], (0, 0, 0));
    #[allow(clippy::items_after_statements)] // local helper declared next to its only use
    fn author(rules: Vec<Option<&PreMatched>>) -> [OriginRules<'_>; 1] {
        [OriginRules {
            origin: CascadeOrigin::Author,
            rules,
        }]
    }
    let ok = |_: &mut [StyledNode]| Ok(());
    let depth = |_: Handle| Ok(0);
    assert_eq!(
        styled
            .run(&author(vec![Some(&big), Some(&one)]), &[], &depth, ok)
            .err()
            .unwrap(),
        LiveError::DeclarationBudget
    );
    assert_eq!(
        styled
            .run(&author(vec![Some(&one), None]), &[], &depth, ok)
            .err()
            .unwrap(),
        LiveError::MissingScope
    );
    // Within budget, the inline parser's own error surfaces verbatim.
    assert_eq!(
        styled
            .run(&author(vec![Some(&big)]), &[], &depth, ok)
            .err()
            .unwrap(),
        LiveError::Embedder("inline parse failed".into())
    );
    assert_eq!(
        styled
            .run(&[], &[(a, "height:1px")], &|_| Err("no scope".into()), ok)
            .err()
            .unwrap()
            .to_string(),
        "no scope"
    );
    let (_, ()) = styled.run(&[], &[(a, "height:1px")], &depth, ok).unwrap();
    assert_eq!(
        styled
            .run(&[], &[(a, "height:1px")], &depth, |_| Err::<(), _>(
                "hook".into()
            ))
            .err()
            .unwrap(),
        LiveError::Embedder("hook".into())
    );
}

#[test]
fn the_hook_sees_the_whole_styled_forest_before_layout() {
    let styled = Styled::new(
        "<body style='margin:0'><div id=a style='width:10px;height:10px'></div></body>",
    );
    let a = styled.el("a");
    let (snapshot, seen) = styled
        .run(&[], &[], &|_| Ok(0), |nodes| {
            let element = find(nodes, a).cloned();
            // A hook may rewrite used styles (an embedder's zoom).
            #[allow(clippy::items_after_statements)] // local helper declared next to its only use
            fn widen(nodes: &mut [StyledNode], a: Handle) {
                for node in nodes {
                    if let StyledNode::Element(element) = node {
                        if element.source == Some(a) {
                            // Chrome's `html.css` `div { display: block }`
                            // (miaskiewicz/turbo-dom#375): this test builds its origins
                            // without `LIVE_UA`, so the rewritten used style sets block.
                            element.style = crate::ComputedStyle::from_pairs([
                                ("display", "block"),
                                ("width", "20px"),
                                ("height", "10px"),
                            ]);
                        }
                        widen(&mut element.children, a);
                    }
                }
            }
            widen(nodes, a);
            Ok(element.is_some())
        })
        .unwrap();
    assert!(seen);
    assert_eq!(border(&snapshot, a), (20.0, 10.0));
}

// miaskiewicz/turbo-dom#395: the surf-level oracle. A CSSOM inline override on a
// styled element must drive layout even when the embedder supplies no UA origin
// — the live style pass has to recognise a known HTML block tag's default
// `display: block` the way `test_support::ensure_default_display` does. Pre-0.8.0
// the implicit default already did this; after #375 flipped the default to
// `inline`, surf's captured `Composition` (which carries no UA rules in the
// budget-test shape) saw every `div` collapse to zero. The border box of `a`
// must be `(30, 5)` (the override replaces the existing style attribute) and
// `b` must be `(200, 0)` (no style attribute, no override applied, but a block
// div fills the viewport).
#[test]
fn inline_override_replaces_only_an_existing_style_attribute_395() {
    let styled = Styled::new(
        "<body style='margin:0'><div id=a style='width:10px;height:5px'></div><div id=b></div></body>",
    );
    let (a, b) = (styled.el("a"), styled.el("b"));
    let overrides = [(a, "width:30px;height:5px"), (b, "width:40px;height:5px")];
    let (snapshot, ()) = styled.run(&[], &overrides, &|_| Ok(0), |_| Ok(())).unwrap();
    assert_eq!(border(&snapshot, a), (30.0, 5.0));
    // `b` has no `style` attribute, so the override must not apply: a block div
    // takes the container width with auto (zero) content height.
    assert_eq!(border(&snapshot, b), (200.0, 0.0));
    // The DOM itself is never rewritten.
    assert_eq!(
        styled.tree.get_attribute(a, "style"),
        Some("width:10px;height:5px")
    );
    assert_eq!(styled.tree.get_attribute(b, "style"), None);
}

/// turbo-dom R1 (expert H2 + meshde): #395's `ensure_default_display` fix covered
/// only the block-tag list. The same regression — a surf-level inline override on
/// a styled element collapsing the border box to `(0, 0)` because the embedder
/// supplied no UA origin — occurs for every HTML tag whose Chrome `html.css`
/// default display is not `inline`: `<li>` (`list-item`), form controls
/// (`inline-block`: `<input>`, `<button>`, `<select>`, `<textarea>`, `<progress>`,
/// `<meter>`), and the table family (`<table>`, `<tr>`, `<td>`, `<th>`, `<thead>`,
/// `<tbody>`, `<tfoot>`, `<caption>`). Each tag with a width/height override must
/// honour the override under the live-input path with no UA origin supplied.
///
/// Review R2 layout M5 tightens this test: assert the resolved `Display`
/// variant too (not just the inline-override width), so the dead-string
/// regression R2 H2 flagged — `<tbody>` / `<col>` / `<colgroup>` falling
/// through to `Display::Block` — fails loudly next time instead of masking
/// the width assertion.
#[test]
fn inline_override_applies_to_every_non_inline_html_tag_default() {
    use crate::layout::value::{display_of, Display};
    // Non-inline HTML tags whose Chrome UA default is not `inline`. Each carries
    // a seed `style` attribute so `inline_style` picks up the override (mirrors
    // the #395 oracle: a surf page's CSSOM override only lands on an element
    // whose `style` attribute already exists). The <ul> wraps <li> so list-item
    // layout runs under a block ancestor. The <tbody> / <col> / <colgroup>
    // rows cover the R2 H2 variants that previously fell through to
    // `Display::Block` end-to-end.
    let styled = Styled::new(
        "<body style='margin:0'>\
         <ul style='margin:0;padding:0'><li id=li style='width:0;height:0'></li></ul>\
         <table id=table style='border-spacing:0;width:0;height:0'>\
           <colgroup id=colgroup style='width:0'><col id=col style='width:0'></colgroup>\
           <thead id=thead style='height:0'><tr id=trhead style='height:0'><th id=th style='padding:0'></th></tr></thead>\
           <tbody id=tbody style='height:0'><tr id=tr style='height:0'><td id=td style='padding:0'></td></tr></tbody>\
           <tfoot id=tfoot style='height:0'><tr id=trfoot style='height:0'><td id=tdfoot style='padding:0'></td></tr></tfoot>\
           <caption id=caption style='padding:0'></caption>\
         </table>\
         <input id=input style='padding:0;border:0'>\
         <button id=button style='padding:0;border:0'></button>\
         <select id=select style='padding:0;border:0'></select>\
         <textarea id=textarea style='padding:0;border:0'></textarea>\
         <progress id=progress style='padding:0'></progress>\
         <meter id=meter style='padding:0'></meter>\
         </body>",
    );
    // (id, expected Display variant). The width/height overrides below still
    // go through border(&snapshot, _) when the fragment has a BorderBox entry;
    // tags that do not produce a BorderBox fragment (col / colgroup / the
    // internal table groups outside a caption-bearing table wrapper) are
    // kept to the `Display` assertion so the variant fix is still pinned
    // without re-baselining.
    let cases: &[(&str, Display)] = &[
        ("li", Display::ListItem),
        ("table", Display::Table),
        ("colgroup", Display::TableColumnGroup),
        ("col", Display::TableColumn),
        ("thead", Display::TableHeaderGroup),
        ("tbody", Display::TableRowGroup),
        ("tfoot", Display::TableFooterGroup),
        ("tr", Display::TableRow),
        ("trhead", Display::TableRow),
        ("trfoot", Display::TableRow),
        ("td", Display::TableCell),
        ("tdfoot", Display::TableCell),
        ("th", Display::TableCell),
        ("caption", Display::TableCaption),
        ("input", Display::InlineBlock),
        ("button", Display::InlineBlock),
        ("select", Display::InlineBlock),
        ("textarea", Display::InlineBlock),
        ("progress", Display::InlineBlock),
        ("meter", Display::InlineBlock),
    ];
    let handles: Vec<_> = cases
        .iter()
        .map(|(id, want)| (*id, styled.el(id), *want))
        .collect();
    // Each element already has a `style` attribute (so `inline_override` picks
    // up the override).
    for (id, h, _) in &handles {
        assert!(
            styled.tree.get_attribute(*h, "style").is_some(),
            "{id}: seed style attribute must exist for the override to land",
        );
    }
    let decl = "width:30px;height:5px";
    let overrides: Vec<(Handle, &str)> = handles.iter().map(|(_, h, _)| (*h, decl)).collect();
    // Pull every target's resolved style back through the StyledNode tree hook.
    let targets: Vec<Handle> = handles.iter().map(|(_, h, _)| *h).collect();
    let (snapshot, resolved) = styled
        .run(&[], &overrides, &|_| Ok(0), |nodes| {
            let mut out = Vec::new();
            for h in &targets {
                let el = find(nodes, *h).ok_or_else(|| "no styled element".to_string())?;
                out.push(el.style.clone());
            }
            Ok(out)
        })
        .unwrap();
    for ((id, h, want), style) in handles.iter().zip(resolved.iter()) {
        // R2 M5: the Display variant must be exactly what the Chrome keyword
        // resolved to — a regression that drops the match arm for a new
        // `table-*` keyword would re-land on Block and fail loudly here.
        assert_eq!(
            display_of(style),
            *want,
            "{id}: Display variant mismatch (specified_display match arm missing?)",
        );
        // If the element produces a border-box fragment, assert the inline
        // override still applies to its width — the R1 H2 width guarantee.
        if snapshot
            .source(super::source_id(DOCUMENT, *h))
            .and_then(|s| {
                s.fragment_indices
                    .iter()
                    .map(|&i| &snapshot.fragments()[i])
                    .find(|f| f.kind == GeometryKind::BorderBox)
            })
            .is_some()
        {
            let (w, _) = border(&snapshot, *h);
            assert_eq!(w, 30.0, "{id}: inline override must apply (width = 30 px)");
        }
    }
}

/// Review R2 layout H1: Chrome's UA has `input[type=hidden] { display: none }`.
/// `ensure_default_display` is tag-only and gave every `<input>` `inline-block`,
/// so a surf-level `<input type=hidden>` under the live-input path (no UA
/// origin) rendered its default text-field box. Honour the attribute.
#[test]
fn input_type_hidden_resolves_to_display_none_in_the_live_input_path() {
    use crate::layout::value::{display_of, Display};
    let styled = Styled::new(
        "<body style='margin:0'>\
         <input id=plain style='padding:0;border:0'>\
         <input id=hidden type=hidden>\
         <input id=hidden_mixed TYPE='Hidden'>\
         <input id=text type=text>\
         </body>",
    );
    let targets: Vec<Handle> = ["plain", "hidden", "hidden_mixed", "text"]
        .iter()
        .map(|id| styled.el(id))
        .collect();
    let (_, resolved) = styled
        .run(&[], &[], &|_| Ok(0), |nodes| {
            let mut out = Vec::new();
            for h in &targets {
                out.push(find(nodes, *h).unwrap().style.clone());
            }
            Ok(out)
        })
        .unwrap();
    assert_eq!(
        display_of(&resolved[0]),
        Display::InlineBlock,
        "input: plain"
    );
    assert_eq!(
        display_of(&resolved[1]),
        Display::None,
        "input[type=hidden]: display must be none"
    );
    assert_eq!(
        display_of(&resolved[2]),
        Display::None,
        "input[TYPE=Hidden]: attribute match is case-insensitive"
    );
    assert_eq!(
        display_of(&resolved[3]),
        Display::InlineBlock,
        "input[type=text]: unchanged"
    );
}

/// Review R3 M5: Chrome's UA `input[type=hidden] { display: none }` has
/// attribute specificity (0, 1, 1), which beats an author `input { display:
/// block }` (0, 0, 1). The prior R2 H1 continuation fallback stamped
/// `display: none` only when the computed `display` was absent or the
/// `inline-block` from `ensure_default_display` — so an author rule that
/// set `display` to `block` / `flex` / `grid` left the hidden input
/// visible, diverging from Chrome. The fix drops the narrow guard so the
/// fallback stamps `display: none` regardless of the current computed
/// value: it represents the UA rule, which outranks author `input { ... }`.
#[test]
fn input_type_hidden_overrides_author_display_block_in_live_path() {
    use crate::layout::value::{display_of, Display};
    // An author inline style sets display: block on an input[type=hidden].
    // Chrome's UA rule (specificity (0,1,1)) beats the author's tag-only
    // inline (which stays in the author origin as a tag rule once parsed —
    // and even as an inline it ties on origin but loses on attribute
    // specificity to a UA [type=hidden] rule). The engine's live fallback
    // must honour that.
    let styled = Styled::new(
        "<body style='margin:0'>\
         <input id=auth_block type=hidden style='display:block'>\
         <input id=auth_flex type=hidden style='display:flex'>\
         </body>",
    );
    let targets: Vec<Handle> = ["auth_block", "auth_flex"]
        .iter()
        .map(|id| styled.el(id))
        .collect();
    let (_, resolved) = styled
        .run(&[], &[], &|_| Ok(0), |nodes| {
            let mut out = Vec::new();
            for h in &targets {
                out.push(find(nodes, *h).unwrap().style.clone());
            }
            Ok(out)
        })
        .unwrap();
    assert_eq!(
        display_of(&resolved[0]),
        Display::None,
        "input[type=hidden] with author display:block must still be none \
         — Chrome's UA rule outranks the author `input` rule"
    );
    assert_eq!(
        display_of(&resolved[1]),
        Display::None,
        "input[type=hidden] with author display:flex must still be none"
    );
}

#[test]
fn snapshot_errors_pass_through() {
    let styled = Styled::new("<body></body>");
    let provider = |h: Handle| styled.children.get(&h).map_or(&[][..], Vec::as_slice);
    let capture = capture_light(&styled.tree, &styled.children);
    let style = LiveStyle {
        tree: live(&styled.tree, &provider),
        origins: &[],
        scope_depth: &|_| Ok(0),
        inline_override: &|_| None,
        inline_declarations: &|text| Ok(parse_declarations(text)),
    };
    let fonts = FontRegistry::new();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let error = snapshot(
        &style,
        &capture,
        &LiveOptions { layout_scale: 1.0 },
        |_| Ok(()),
        &SnapshotInputs {
            width: f32::NAN,
            height: 10.0,
            fonts: &fonts,
            images: &images,
        },
        &mut Diagnostics::default(),
    )
    .err()
    .unwrap();
    assert!(matches!(error, LiveError::Snapshot(_)));
    assert_eq!(
        error.to_string(),
        "native HTML viewport dimensions must be finite and non-negative"
    );
}

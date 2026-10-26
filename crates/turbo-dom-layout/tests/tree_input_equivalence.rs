//! Consolidation S2 gate: the turbo-dom [`Tree`] input styles (and lays out) every
//! document exactly like the legacy owned-`Node` input it replaces.
//!
//! For each input HTML document the legacy reference — html5ever + rcdom ->
//! `markup::parse_with_roots` -> `style_tree_with_roots` over the stripped body for
//! the HTML path, `style_tree` over the body children for detached content — is
//! compared with the Tree input (`parse_html_tree` -> `style_input` with
//! `Roots::Shell` / `Roots::Detached`): the styled trees must be equal (up to the
//! new `source` fields), the collected author CSS equal, the legacy `Node` copy
//! equal, and the HTML-path galley equal.
//!
//! Inputs: the adversarial documents below, plus every `*.html` under the
//! directories listed in `TURBO_DOM_LAYOUT_EQUIV_DIRS` (`:`-separated, searched
//! recursively; used locally for the consolidation corpora). html2pdf's marker
//! classifier, templates and fixtures are covered by its own equivalence test.

mod common;

use std::path::{Path, PathBuf};

use turbo_dom_layout::html_layout::strip_non_visual;
use turbo_dom_layout::markup;
use turbo_dom_layout::style::{style_tree_with_roots, TokenSet};
use turbo_dom_layout::{
    build_cascade_with_width, collect_style_css, collect_style_css_tree, layout, layout_html,
    parse_html_tree, style_input, style_tree, Diagnostics, FontRegistry, Roots, StyleInput,
    StyleOptions, StyledNode, TreeView,
};

/// The adversarial documents from the consolidation plan (§1.2) plus selector
/// shapes that depend on the structure the two inputs must agree on.
const ADVERSARIAL: &[(&str, &str)] = &[
    (
        "comment-between-siblings",
        "<body><style>p+p{color:red} p:nth-child(2){font-weight:bold} p:last-child{margin:1px}</style>\
         <p>a</p><!-- c --><p>b</p>x<!--y-->z</body>",
    ),
    (
        "style-first-child",
        "<body><style>p:first-child{color:red} p:first-of-type{margin:3px}</style><p>a</p><p>b</p></body>",
    ),
    (
        "script-sole-child",
        "<body><style>div:empty{color:red} div:only-child{margin:2px}</style>\
         <div><script>var x = '<p>';</script></div><section><script>1</script><b>k</b></section></body>",
    ),
    (
        "style-sole-child",
        "<body><div><style>div:empty{color:blue}</style></div></body>",
    ),
    (
        "template",
        "<body><style>template + p{color:red} p:first-child{margin:1px} :empty{padding:1px}</style>\
         <template><p>inside</p></template><p>after</p><div><template></template></div></body>",
    ),
    (
        "noscript-head-and-body",
        "<html><head><noscript><style>p{color:red}</style></noscript></head><body>\
         <noscript><p>no js</p></noscript><p>a</p><style>noscript+p{color:green}</style></body></html>",
    ),
    (
        "svg-xlink-href",
        r##"<body><style>[href]{color:red} a[href^="#"]{margin:1px}</style><svg xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 10 10"><a xlink:href="#t"><text>x</text></a><use xlink:href="#u"/><linearGradient id="g"/></svg></body>"##,
    ),
    (
        "content-in-head",
        "<html><head><title>t</title><p class=early>early</p></head><body><p>b</p></body></html>",
    ),
    (
        "template-with-unknown-t",
        "<body><template><t:bogus/></template><p>x</p></body>",
    ),
    ("svg-with-t-element", "<body><svg><t:x/></svg><p>y</p></body>"),
    (
        "t-elements-are-plain",
        "<body><t:footnote>n</t:footnote><t:page/><p>after</p></body>",
    ),
    (
        "html-body-classes",
        r#"<html class="client-js theme" lang="en"><body class="skin" style="color:navy"><style>
         .client-js .x{color:red} body.skin > p{margin:4px} html:root .x{padding:1px}
         :root{--v: 3px} .x{border-width: var(--v)}</style><p class="x">t</p></body></html>"#,
    ),
    (
        "siblings-and-types",
        "<body><style>li:nth-of-type(2n){color:red} li:last-of-type{margin:1px} span ~ li{padding:1px}\
         li:only-of-type{border:1px solid}</style><ul><li>1</li><span>s</span><li>2</li><li>3</li></ul>\
         <ol><li>only</li></ol></body>",
    ),
    (
        "presentational-hints",
        "<body><table cellpadding=3 bgcolor=#eee width=50%><tr><td height=20>a</td><td><font color=red>f</font></td></tr></table>\
         <input type=hidden><input disabled><input type=checkbox checked><a href=x>l</a></body>",
    ),
    (
        "frames-and-pis",
        "<!doctype html><?pi x?><body><p>a<?x y?>b</p><![CDATA[c]]><p>d</p></body>",
    ),
    ("empty-document", ""),
    ("text-only", "plain text, no tags"),
];

fn input_dirs() -> Vec<PathBuf> {
    std::env::var("TURBO_DOM_LAYOUT_EQUIV_DIRS")
        .map(|dirs| {
            dirs.split(':')
                .filter(|d| !d.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

fn html_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        if path.is_dir() {
            html_files(&path, out);
        } else if name.ends_with(".html") {
            out.push(path);
        }
    }
}

fn stripped(nodes: &[StyledNode]) -> Vec<StyledNode> {
    nodes.iter().map(StyledNode::without_sources).collect()
}

/// Compare both inputs on one document; returns the number of styled roots.
fn check(name: &str, html: &str, fonts: &FontRegistry) -> usize {
    // Legacy reference.
    let (nodes, roots) = markup::parse_with_roots(html).expect("legacy parse");
    let css = collect_style_css(&nodes);
    let cascade = build_cascade_with_width(&css, "", TokenSet::default(), 1280.0);
    let legacy_shell = style_tree_with_roots(&strip_non_visual(nodes.clone()), &cascade, &roots);
    let legacy_detached = style_tree(&nodes, &cascade);

    // Tree input.
    let tree = parse_html_tree(html);
    let view = TreeView::new(&tree);
    let body = view.find_element("body").expect("body");
    let body_children = tree.children(body);
    assert_eq!(view.to_nodes(&body_children), nodes, "{name}: Node copy");
    assert_eq!(
        collect_style_css_tree(&view, &view.children(body)),
        css,
        "{name}: author CSS"
    );
    let shell = style_input(
        &StyleInput {
            view: view.strip_non_visual(true),
            roots: Roots::Shell,
        },
        &cascade,
        &StyleOptions::default(),
    );
    assert_eq!(stripped(&shell), legacy_shell, "{name}: shell styling");
    let detached = style_input(
        &StyleInput {
            view,
            roots: Roots::Detached(&body_children),
        },
        &cascade,
        &StyleOptions::default(),
    );
    assert_eq!(
        stripped(&detached),
        legacy_detached,
        "{name}: detached styling"
    );

    // The HTML drive end to end: legacy styled tree laid out == layout_html.
    let legacy_galley = layout(&legacy_shell, 1280.0, fonts, &mut Diagnostics::default());
    let galley =
        layout_html(html, "", 1280.0, fonts, &mut Diagnostics::default()).expect("layout_html");
    assert_eq!(
        format!("{galley:?}"),
        format!("{legacy_galley:?}"),
        "{name}: HTML-path galley"
    );
    shell.len()
}

#[test]
fn tree_input_matches_the_legacy_node_input() {
    let fonts = FontRegistry::new();
    for (name, html) in ADVERSARIAL {
        check(name, html, &fonts);
    }
    let mut files = Vec::new();
    for dir in input_dirs() {
        html_files(&dir, &mut files);
    }
    for path in &files {
        let html = std::fs::read_to_string(path).expect("read input");
        check(&path.display().to_string(), &html, &fonts);
    }
    eprintln!(
        "tree_input_equivalence: {} adversarial + {} corpus documents equal",
        ADVERSARIAL.len(),
        files.len()
    );
}

#[test]
fn shell_styling_records_sources_and_skips_non_content() {
    let tree = parse_html_tree(
        "<body><!--c--><p id=a>x<b>y</b></p><style>p{}</style><template><i>t</i></template></body>",
    );
    let view = TreeView::new(&tree);
    let styled = style_input(
        &StyleInput {
            view: view.strip_non_visual(true),
            roots: Roots::Shell,
        },
        &turbo_dom_layout::build_cascade("", "", TokenSet::default()),
        &StyleOptions::default(),
    );
    assert_eq!(
        styled.len(),
        1,
        "comment, style and template are not content"
    );
    let StyledNode::Element(p) = &styled[0] else {
        panic!("p element")
    };
    assert_eq!(view.attr(p.source.unwrap(), "id"), Some("a"));
    let StyledNode::Text(text) = &p.children[0] else {
        panic!("text")
    };
    assert_eq!(&**text, "x");
    assert_eq!(view.text(text.source.unwrap()), "x");
    // Without stripping the style + template elements are content (template empty).
    let body = view.find_element("body").unwrap();
    assert_eq!(view.children(body).len(), 3);
    assert_eq!(view.find_element("missing"), None);
}

#[test]
fn markers_classify_by_local_name_and_never_strip() {
    let tree = parse_html_tree("<body><x-marker-3-i>n</x-marker-3-i><script>s</script></body>");
    let view = TreeView::new(&tree)
        .markers(&common::marker_of)
        .strip_non_visual(true);
    let body = view.find_element("body").unwrap();
    let kids = view.children(body);
    assert_eq!(kids.len(), 1, "script stripped, marker kept");
    let kind = view.marker(kids[0]).expect("marker");
    assert_eq!((kind.id, kind.inline), (3, true));
    assert_eq!(view.tag(kids[0]), turbo_dom_layout::Tag::Marker(kind));
    assert_eq!(view.tag(body), turbo_dom_layout::Tag::Html("body".into()));
}

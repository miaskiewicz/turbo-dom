//! Shared helpers for the layout integration tests: font fixtures loaded from
//! the in-repo `assets/fonts/` (never a system path) and a `ComputedStyle`
//! builder. `dead_code` is allowed because each test binary uses a subset.
#![allow(dead_code)]

use turbo_dom_layout::text::FontRegistry;
use turbo_dom_layout::{ComputedStyle, FontFace};

pub fn font_bytes(name: &str) -> Vec<u8> {
    let path = format!("{}/assets/fonts/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|_| panic!("fixture {path}"))
}

pub fn evolventa() -> FontFace {
    FontFace::from_bytes(font_bytes("Evolventa-zLXL.ttf"), "Evolventa", 400, false).unwrap()
}

pub fn evolventa_bold() -> FontFace {
    FontFace::from_bytes(
        font_bytes("EvolventaBold-55Xv.ttf"),
        "Evolventa",
        700,
        false,
    )
    .unwrap()
}

pub fn go() -> FontFace {
    FontFace::from_bytes(font_bytes("Go-Regular.ttf"), "Go", 400, false).unwrap()
}

/// A registry with Evolventa (regular + bold) and Go registered.
pub fn registry() -> FontRegistry {
    let mut reg = FontRegistry::new();
    reg.add(evolventa());
    reg.add(evolventa_bold());
    reg.add(go());
    reg
}

/// Build a `ComputedStyle` from `(property, value)` pairs.
pub fn style(pairs: &[(&str, &str)]) -> ComputedStyle {
    ComputedStyle::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
}

/// The element name a marker gets in a [`tree_from_nodes`] copy, and its inverse.
pub fn marker_name(kind: turbo_dom_layout::MarkerKind) -> String {
    format!(
        "x-marker-{}-{}",
        kind.id,
        if kind.inline { "i" } else { "b" }
    )
}

pub fn marker_of(local: &str) -> Option<turbo_dom_layout::MarkerKind> {
    let rest = local.strip_prefix("x-marker-")?;
    let (id, display) = rest.split_once('-')?;
    Some(turbo_dom_layout::MarkerKind {
        id: id.parse().ok()?,
        inline: display == "i",
    })
}

/// Style a legacy [`Node`] forest through BOTH inputs — the legacy `style_tree` and
/// the turbo-dom Tree input (`tree_from_nodes` + `style_input` over detached roots)
/// — assert they agree (up to the `source` fields) and return the Tree-input
/// result, sources included.
pub fn style_nodes(
    nodes: &[turbo_dom_layout::Node],
    cascade: &turbo_dom_layout::Cascade,
) -> Vec<turbo_dom_layout::StyledNode> {
    use turbo_dom_layout::{style_input, style_tree, Roots, StyleInput, StyleOptions, TreeView};
    let legacy = style_tree(nodes, cascade);
    let copy = turbo_dom_layout::tree_from_nodes(nodes, &marker_name);
    let input = StyleInput {
        view: TreeView::new(&copy.tree).markers(&marker_of),
        roots: Roots::Detached(&copy.roots),
    };
    let styled = style_input(&input, cascade, &StyleOptions::default());
    let stripped: Vec<_> = styled.iter().map(|n| n.without_sources()).collect();
    assert_eq!(
        stripped, legacy,
        "legacy Node input == turbo-dom Tree input"
    );
    styled
}

//! Test support for the live-DOM input and the CSSOM-View modules: a light-DOM
//! document styled with this crate's own cascade, its rules pre-matched through
//! turbo-dom's selector engine on the live tree (as an embedder would match them).
use std::collections::HashMap;

use turbo_dom::rtdom::query::{match_for_cascade, LiveNavigator};
use turbo_dom::rtdom::tree::NodeType;
use turbo_dom::{Handle, Tree};

use super::{
    capture, snapshot, source_id, LiveCapture, LiveOptions, LiveStyle, LiveTree, MatchedTarget,
    OriginRules, PreMatched,
};
use crate::cssom_view::scroll::{SourceStyle, SourceStyles};
use crate::layout_snapshot::{LayoutSnapshot, SnapshotInputs};
use crate::style::{parse_declarations, CascadeOrigin};
use crate::{Diagnostics, FontRegistry, ImageCtx, NoImages, StyledNode, Tag};

/// The document number test captures use.
pub(crate) const DOCUMENT: u32 = 1;

/// turbo-surf's live user-agent supplement (its `LIVE_UA`), so layouts match the
/// embedder's defaults.
const LIVE_UA: &str = concat!(
    "body{margin:8px;font-family:serif}",
    "button{display:inline-block;font-family:Arial;font-size:13.3333px;font-style:normal;font-weight:400;font-stretch:normal;font-variant-caps:normal;line-height:normal;margin:0;padding:1px 6px;border-width:2px;vertical-align:baseline}",
    "iframe{display:inline;border:2px inset}",
    "head,style,script,title,meta,link,base,noscript{display:none}",
);

/// Every node's tree children (a document without shadow roots composes as itself).
pub(crate) fn light_children(tree: &Tree) -> HashMap<Handle, Vec<Handle>> {
    let mut children = HashMap::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        let kids = tree.children(node);
        pending.extend(kids.iter().copied());
        children.insert(node, kids);
    }
    children
}

/// The UA (+ live supplement) and author rules of `css`, matched against every element.
pub(crate) fn prematch(
    tree: &Tree,
    css: &str,
    width: f32,
) -> [(CascadeOrigin, Vec<PreMatched>); 2] {
    let mut cascade =
        crate::build_cascade_with_width(css, "", crate::style::TokenSet::default(), width);
    let ua = crate::style::parse_stylesheet(LIVE_UA);
    let start = cascade.rules.len();
    cascade
        .rules
        .extend(
            ua.rules
                .into_iter()
                .enumerate()
                .map(|(i, rule)| crate::style::LeveledRule {
                    level: 0,
                    order: start + i,
                    rule,
                }),
        );
    cascade.rules.sort_by_key(|rule| (rule.level, rule.order));
    let elements: Vec<Handle> = tree
        .handles()
        .filter(|&h| tree.node_type(h) == NodeType::Element)
        .collect();
    let nav = LiveNavigator(tree);
    let mut origins = [
        (CascadeOrigin::UserAgent, Vec::new()),
        (CascadeOrigin::Author, Vec::new()),
    ];
    for rule in &cascade.rules {
        let targets = elements
            .iter()
            .filter_map(|&h| {
                let specificity = rule
                    .rule
                    .selectors
                    .selectors()
                    .iter()
                    .filter(|selector| match_for_cascade(&nav, h, selector))
                    .map(turbo_dom::rtdom::query::CascadeSelector::specificity)
                    .max()?;
                Some(MatchedTarget {
                    source: source_id(DOCUMENT, h),
                    specificity,
                })
            })
            .collect();
        origins[usize::from(rule.level > 0)].1.push(PreMatched {
            depth: 0,
            declarations: rule.rule.declarations.clone(),
            targets,
        });
    }
    origins
}

/// The capture of a light-DOM `tree`.
pub(crate) fn capture_light(tree: &Tree, children: &HashMap<Handle, Vec<Handle>>) -> LiveCapture {
    let provider = |h: Handle| children.get(&h).map_or(&[][..], Vec::as_slice);
    capture(&LiveTree {
        tree,
        document: DOCUMENT,
        children: &provider,
    })
    .unwrap()
}

/// Style and lay out `html` under `css` at `width` x `height`, collecting the flat
/// source styles (effective zoom 1) in the post-cascade hook. `layout_scale` `Some`
/// takes the live HTML snapshot (native text, SVG viewports, the embedder's path);
/// `None` the generic box snapshot.
pub(crate) fn layout(
    html: &str,
    css: &str,
    viewport: (f32, f32),
    layout_scale: Option<f32>,
) -> (Tree, LayoutSnapshot, SourceStyles) {
    layout_with(html, css, viewport, layout_scale, |_| {})
}

/// [`layout`] of the parsed `html` after `prepare` sets embedder state on the tree (the
/// fullscreen stack the binding reports, miaskiewicz/turbo-dom#82).
pub(crate) fn layout_with(
    html: &str,
    css: &str,
    (width, height): (f32, f32),
    layout_scale: Option<f32>,
    prepare: impl FnOnce(&mut Tree),
) -> (Tree, LayoutSnapshot, SourceStyles) {
    let mut tree = Tree::parse(html);
    prepare(&mut tree);
    let children = light_children(&tree);
    let captured = capture_light(&tree, &children);
    let origins = prematch(&tree, css, width);
    let origins: Vec<_> = origins
        .iter()
        .map(|(origin, rules)| OriginRules {
            origin: *origin,
            rules: rules.iter().map(Some).collect(),
        })
        .collect();
    let provider = |h: Handle| children.get(&h).map_or(&[][..], Vec::as_slice);
    let style = LiveStyle {
        tree: LiveTree {
            tree: &tree,
            document: DOCUMENT,
            children: &provider,
        },
        origins: &origins,
        scope_depth: &|_| Ok(0),
        inline_override: &|_| None,
        inline_declarations: &|text| Ok(parse_declarations(text)),
    };
    let fonts = FontRegistry::new();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let inputs = SnapshotInputs {
        width,
        height,
        fonts: &fonts,
        images: &images,
    };
    let mut diagnostics = Diagnostics::default();
    let (snapshot, styles) = if let Some(layout_scale) = layout_scale {
        snapshot(
            &style,
            &captured,
            &LiveOptions { layout_scale },
            |styled| Ok(collect(styled)),
            &inputs,
            &mut diagnostics,
        )
        .unwrap()
    } else {
        let styled = super::style::style(&style, &captured).unwrap();
        let snapshot = crate::layout_snapshot::snapshot_styled_tree(
            &styled,
            &captured.identities,
            &inputs,
            &mut diagnostics,
        )
        .unwrap();
        (snapshot, collect(&styled))
    };
    (tree, snapshot, styles)
}

/// Each element's style, flat parent and tag (zoom 1).
pub(crate) fn collect(styled: &[StyledNode]) -> SourceStyles {
    fn walk(nodes: &[StyledNode], parent: Option<super::SourceId>, out: &mut SourceStyles) {
        for node in nodes {
            let StyledNode::Element(element) = node else {
                continue;
            };
            let id = source_id(DOCUMENT, element.source.unwrap());
            let tag = match &element.tag {
                Tag::Html(tag) => tag.clone(),
                Tag::Marker(_) => String::new(),
            };
            out.insert(
                id,
                SourceStyle {
                    style: element.style.clone(),
                    parent,
                    tag,
                    zoom: 1.0,
                },
            );
            walk(&element.children, Some(id), out);
        }
    }
    let mut out = SourceStyles::new();
    walk(styled, None, &mut out);
    out
}

/// The source id of the first element matching `selector`.
pub(crate) fn id(tree: &Tree, selector: &str) -> super::SourceId {
    source_id(DOCUMENT, tree.query_selector(selector).unwrap())
}

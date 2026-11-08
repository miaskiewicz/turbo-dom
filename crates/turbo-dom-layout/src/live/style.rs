//! The style pass: resolve every kept composed source from its pre-matched
//! declarations, presentational hints and inline style, parents before children.
use std::collections::{HashMap, HashSet};

use turbo_dom::rtdom::tree::NodeType;
use turbo_dom::Handle;

use super::capture::MATHML;
use super::{
    source_id, LiveCapture, LiveError, LiveStyle, MatchedTarget, OriginRules, PreMatched,
    MAX_DECLARATIONS,
};
use crate::layout_snapshot::SourceId;
use crate::style::{
    ensure_default_display, presentational_declarations_of, resolve_matched_style, CascadeOrigin,
    ComputedStyle, MatchedDeclaration,
};
use crate::tag::{Attr, Tag};
use crate::{StyledElement, StyledNode, StyledText};

type Matched = HashMap<SourceId, Vec<MatchedDeclaration>>;

pub(super) fn style(
    style: &LiveStyle,
    capture: &LiveCapture,
) -> Result<Vec<StyledNode>, LiveError> {
    let matched = matched(style.origins)?;
    let math_roots: HashSet<SourceId> = capture.math_roots.iter().copied().collect();
    let styler = Styler {
        style,
        capture,
        matched: &matched,
        math_roots: &math_roots,
    };
    styler.children(style.tree.tree.root(), &ComputedStyle::default())
}

/// Every pre-matched declaration by target, in origin then rule order.
fn matched(origins: &[OriginRules]) -> Result<Matched, LiveError> {
    let mut matched = Matched::new();
    let mut left = MAX_DECLARATIONS;
    for origin in origins {
        for (order, rule) in origin.rules.iter().enumerate() {
            let rule = rule.ok_or(LiveError::MissingScope)?;
            push_targets(&mut matched, &mut left, (origin.origin, order), rule)?;
        }
    }
    Ok(matched)
}

fn push_targets(
    matched: &mut Matched,
    left: &mut usize,
    rank: (CascadeOrigin, usize),
    rule: &PreMatched,
) -> Result<(), LiveError> {
    for target in &rule.targets {
        let entry = matched.entry(target.source).or_default();
        push_rule(entry, left, rank, rule, target)?;
    }
    Ok(())
}

/// One rule's declarations for one target, charged against the budget.
fn push_rule(
    entry: &mut Vec<MatchedDeclaration>,
    left: &mut usize,
    (origin, order): (CascadeOrigin, usize),
    rule: &PreMatched,
    target: &MatchedTarget,
) -> Result<(), LiveError> {
    for declaration in &rule.declarations {
        *left = left.checked_sub(1).ok_or(LiveError::DeclarationBudget)?;
        entry.push(MatchedDeclaration {
            declaration: declaration.clone(),
            origin,
            scope_depth: rule.depth,
            inline: false,
            specificity: target.specificity,
            order,
        });
    }
    Ok(())
}

struct Styler<'s, 'a> {
    style: &'s LiveStyle<'a>,
    capture: &'s LiveCapture,
    matched: &'s Matched,
    /// The composed `MathML` roots captured for this document; see [`LiveCapture::math_roots`].
    math_roots: &'s HashSet<SourceId>,
}

impl Styler<'_, '_> {
    fn children(
        &self,
        parent: Handle,
        style: &ComputedStyle,
    ) -> Result<Vec<StyledNode>, LiveError> {
        let tree = self.style.tree.tree;
        let children = (self.style.tree.children)(parent);
        let mut out = Vec::with_capacity(children.len());
        for &child in children {
            match tree.node_type(child) {
                NodeType::Element => out.push(StyledNode::Element(self.element(child, style)?)),
                NodeType::Text => out.push(StyledNode::Text(self.text(child))),
                _ => {}
            }
        }
        Ok(out)
    }

    fn text(&self, node: Handle) -> StyledText {
        let units = self.capture.original_text.get(&self.id(node));
        StyledText {
            text: units.map_or_else(String::new, |units| String::from_utf16_lossy(units)),
            source: Some(node),
        }
    }

    fn id(&self, node: Handle) -> SourceId {
        source_id(self.style.tree.document, node)
    }

    fn element(&self, node: Handle, parent: &ComputedStyle) -> Result<StyledElement, LiveError> {
        let tree = self.style.tree.tree;
        let depth = (self.style.scope_depth)(node).map_err(LiveError::Embedder)?;
        let local = tree.local_name(node).unwrap_or("");
        let mut declarations: Vec<_> = presentational_declarations_of(tree, node, local)
            .into_iter()
            .map(|declaration| author(declaration, depth, false, 0))
            .collect();
        // Chrome's quirks.css (miaskiewicz/turbo-dom#282): a BackCompat document's forms
        // take `margin-block-end: 1em` at the UA level, beneath every embedder rule.
        if self.capture.quirks && local == "form" {
            declarations.extend(crate::style::quirks_form_declarations());
        }
        // Placeholder `MathML` metrics (miaskiewicz/turbo-dom#376): the composed MathML
        // root is an inline-block of 0×0 (matches Chrome 156's empty `<math>` placeholder
        // box); its descendants take `display: none`, so no `MathML` content renders until
        // the native `MathML` layout is implemented.
        if tree.namespace_uri(node) == Some(MATHML) {
            if self.math_roots.contains(&self.id(node)) {
                declarations.extend(crate::style::mathml_root_declarations());
            } else {
                declarations.extend(crate::style::mathml_descendant_declarations());
            }
        }
        declarations.extend(
            self.matched
                .get(&self.id(node))
                .into_iter()
                .flatten()
                .cloned()
                .map(|mut matched| {
                    matched.order += 1;
                    matched
                }),
        );
        let inline = self.inline_style(node);
        if let Some(inline) = inline {
            let parsed = (self.style.inline_declarations)(inline).map_err(LiveError::Embedder)?;
            declarations.extend(
                parsed
                    .into_iter()
                    .map(|declaration| author(declaration, depth, true, usize::MAX)),
            );
        }
        let mut style = resolve_matched_style(parent, &declarations);
        // Chrome's `html.css` `display: block` for known block tags
        // (miaskiewicz/turbo-dom#395): the CSS initial `display` is `inline`
        // (#375), so with no UA origin to carry the UA rule an embedder's
        // inline-override `width` on a `<div>` would collapse to a zero-sized
        // inline. Fall back to the HTML default when no declaration set it; a
        // UA rule in the pre-matched origins still wins, and unknown tags stay
        // inline.
        ensure_default_display(&mut style, local);
        if self.capture.quirks {
            style.set(crate::style::QUIRKS_LINE_HEIGHT, "1".to_string());
            if matches!(local, "body" | "td" | "th") {
                style.set(crate::style::QUIRKY_CONTAINER, "1".to_string());
            }
        }
        let children = self.children(node, &style)?;
        Ok(StyledElement {
            tag: Tag::Html(local.to_string()),
            attrs: self.attrs(node, inline),
            style,
            children,
            source: Some(node),
        })
    }

    /// The inline style layout sees: the CSSOM override when the element has a `style`
    /// attribute, else that attribute's value.
    fn inline_style(&self, node: Handle) -> Option<&str> {
        let original = self.style.tree.tree.get_attribute(node, "style")?;
        Some((self.style.inline_override)(node).unwrap_or(original))
    }

    /// The attributes by qualified name, the first `style` carrying `inline`.
    fn attrs(&self, node: Handle, inline: Option<&str>) -> Vec<Attr> {
        let mut attrs = Vec::new();
        let mut inline = inline;
        self.style.tree.tree.for_each_attr(node, |name, value| {
            let value = match inline.filter(|_| name == "style") {
                Some(replaced) => {
                    inline = None;
                    replaced
                }
                None => value,
            };
            attrs.push(Attr {
                name: name.to_string(),
                value: value.to_string(),
            });
        });
        attrs
    }
}

fn author(
    declaration: crate::style::Declaration,
    scope_depth: u32,
    inline: bool,
    order: usize,
) -> MatchedDeclaration {
    MatchedDeclaration {
        declaration,
        origin: CascadeOrigin::Author,
        scope_depth,
        inline,
        specificity: (0, 0, 0),
        order,
    }
}

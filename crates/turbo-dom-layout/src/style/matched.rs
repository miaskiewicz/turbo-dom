//! Apply declarations selected by an embedder against its actual tree scopes.
use super::{
    collect_presentational, expand_quad_candidates, inherit, is_border_property,
    mark_author_border, pick_winners, Attrs, Cand, Subject,
};
use super::{ComputedStyle, Declaration};
use crate::tag::Attr;

/// CSS origin, before encapsulation, specificity and source-order resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CascadeOrigin {
    UserAgent,
    User,
    Author,
}
impl CascadeOrigin {
    fn rank(self, important: bool) -> u8 {
        let normal = match self {
            Self::UserAgent => 0,
            Self::User => 1,
            Self::Author => 2,
        };
        if important {
            2 - normal
        } else {
            normal
        }
    }
}

/// A declaration whose selector already matched in the original DOM tree.
/// Scope depth is zero for the document, increasing through shadow roots.
/// Inline declarations use their element's own scope, rather than a selector's.
#[derive(Debug, Clone)]
pub struct MatchedDeclaration {
    pub declaration: Declaration,
    pub origin: CascadeOrigin,
    pub scope_depth: u32,
    pub inline: bool,
    pub specificity: (u32, u32, u32),
    pub order: usize,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    important: bool,
    origin: u8,
    scope: u32,
    inline: bool,
    specificity: (u32, u32, u32),
    order: usize,
}
impl MatchedDeclaration {
    fn rank(&self) -> Rank {
        let important = self.declaration.important;
        Rank {
            important,
            origin: self.origin.rank(important),
            scope: if important {
                self.scope_depth
            } else {
                u32::MAX - self.scope_depth
            },
            inline: self.inline,
            specificity: self.specificity,
            order: self.order,
        }
    }
}

/// Resolve pre-matched declarations with CSS origin and shadow encapsulation
/// ordering. The caller supplies UA rules, hints and inline declarations too.
/// This reuses the engine's shorthand, variable, inheritance and font resolution;
/// selectors and tree scope membership remain the caller's responsibility.
#[must_use]
pub fn resolve_matched_style(
    parent: &ComputedStyle,
    declarations: &[MatchedDeclaration],
) -> ComputedStyle {
    let author_border = declarations.iter().any(|matched| {
        matched.origin != CascadeOrigin::UserAgent
            && is_border_property(&matched.declaration.property)
    });
    let mut ordered: Vec<_> = declarations.iter().collect();
    ordered.sort_by_key(|declaration| declaration.rank());
    let candidates = ordered
        .into_iter()
        .enumerate()
        .map(|(order, matched)| Cand {
            key: (matched.declaration.important, 0, (0, 0, 0), order, 0),
            property: matched.declaration.property.clone(),
            value: matched.declaration.value.clone(),
        })
        .collect();
    mark_author_border(
        inherit(
            pick_winners(expand_quad_candidates(candidates, parent)),
            parent,
        ),
        author_border,
    )
}

/// The same legacy attribute hints (`bgcolor`, `width`, `height`, `color`, `align`, a
/// list's `type`) the
/// ordinary tree cascade applies, for an element named `local` (`None` = a marker)
/// with `attrs`. Embedders can assign these the owning element's scope and an early
/// author source order.
#[must_use]
pub fn presentational_declarations(local: Option<&str>, attrs: &[Attr]) -> Vec<Declaration> {
    declarations_of(Subject {
        tag: local,
        attrs: Attrs::List(attrs),
    })
}

/// The attributes [`presentational_declarations`] reads.
const HINT_ATTRIBUTES: [&str; 6] = ["bgcolor", "width", "height", "color", "align", "type"];

/// [`presentational_declarations`] of a live element `h` named `local`, reading its
/// attributes by qualified name (the live-DOM input, [`crate::live`]).
pub(crate) fn presentational_declarations_of(
    tree: &turbo_dom::Tree,
    h: turbo_dom::Handle,
    local: &str,
) -> Vec<Declaration> {
    let attrs: Vec<Attr> = HINT_ATTRIBUTES
        .iter()
        .filter_map(|&name| {
            tree.get_attribute(h, name).map(|value| Attr {
                name: name.to_string(),
                value: value.to_string(),
            })
        })
        .collect();
    let mut declarations = declarations_of(Subject {
        tag: Some(local),
        attrs: Attrs::List(&attrs),
    });
    declarations.extend(dir_auto_declaration(tree, h));
    declarations
}

/// The `direction` an element with `dir=auto` takes: right-to-left when the first
/// strong character of its text is, skipping `<bdi>`, `<script>`, `<style>`,
/// `<textarea>` and descendants with their own valid `dir` (HTML's auto
/// directionality; Chrome 156, miaskiewicz/turbo-dom#238). `None` without `dir=auto`.
pub(crate) fn dir_auto_declaration(
    tree: &turbo_dom::Tree,
    h: turbo_dom::Handle,
) -> Option<Declaration> {
    let auto = tree
        .get_attribute(h, "dir")
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("auto"));
    if !auto {
        return None;
    }
    let rtl = first_strong_in(tree, h).unwrap_or(false);
    Some(Declaration {
        property: "direction".to_string(),
        value: if rtl { "rtl" } else { "ltr" }.to_string(),
        important: false,
    })
}

fn first_strong_in(tree: &turbo_dom::Tree, h: turbo_dom::Handle) -> Option<bool> {
    let mut child = tree.first_child(h);
    while let Some(node) = child {
        let found = match tree.node_type(node) {
            turbo_dom::rtdom::tree::NodeType::Text => tree
                .node_value(node)
                .and_then(|text| crate::layout::inline::bidi::first_strong_rtl(&text)),
            turbo_dom::rtdom::tree::NodeType::Element if !skips_auto_direction(tree, node) => {
                first_strong_in(tree, node)
            }
            _ => None,
        };
        if found.is_some() {
            return found;
        }
        child = tree.next_sibling(node);
    }
    None
}

fn skips_auto_direction(tree: &turbo_dom::Tree, h: turbo_dom::Handle) -> bool {
    let own_dir = tree.get_attribute(h, "dir").is_some_and(|value| {
        ["ltr", "rtl", "auto"]
            .iter()
            .any(|keyword| value.trim().eq_ignore_ascii_case(keyword))
    });
    own_dir
        || tree
            .local_name(h)
            .is_some_and(|name| matches!(name, "bdi" | "script" | "style" | "textarea"))
}

fn declarations_of(subject: Subject) -> Vec<Declaration> {
    let mut candidates = Vec::new();
    collect_presentational(subject, &mut candidates);
    candidates
        .into_iter()
        .map(|candidate| Declaration {
            property: candidate.property,
            value: candidate.value,
            important: false,
        })
        .collect()
}

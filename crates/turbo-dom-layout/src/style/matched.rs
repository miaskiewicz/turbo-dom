//! Apply declarations selected by an embedder against its actual tree scopes.
use super::{
    collect_presentational, expand_quad_candidates, inherit, pick_winners, Attrs, Cand, Subject,
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
    inherit(
        pick_winners(expand_quad_candidates(candidates, parent)),
        parent,
    )
}

/// The same legacy attribute hints (`bgcolor`, `width`, `height`, `color`) the
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
const HINT_ATTRIBUTES: [&str; 4] = ["bgcolor", "width", "height", "color"];

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
    declarations_of(Subject {
        tag: Some(local),
        attrs: Attrs::List(&attrs),
    })
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

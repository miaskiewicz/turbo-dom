//! Apply declarations selected by an embedder against its actual tree scopes.
use super::{
    collect_presentational, expand_quad_candidates, inherit, is_border_property,
    mark_author_border, origin_rank, pick_winners, resolve_paragraph_style, Attrs, Cand, Subject,
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
    resolve_matched_style_impl(parent, declarations, false)
}

/// The paragraph-only resolver keeps the winning UA margin provenance for quirks.
pub(crate) fn resolve_matched_paragraph_style(
    parent: &ComputedStyle,
    declarations: &[MatchedDeclaration],
) -> ComputedStyle {
    resolve_matched_style_impl(parent, declarations, true)
}

fn resolve_matched_style_impl(
    parent: &ComputedStyle,
    declarations: &[MatchedDeclaration],
    paragraph: bool,
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
            key: (
                matched.declaration.important,
                candidate_origin(matched),
                (0, 0, 0),
                order,
                0,
            ),
            property: matched.declaration.property.clone(),
            value: matched.declaration.value.clone(),
        })
        .collect();
    let candidates = expand_quad_candidates(candidates, parent);
    let style = if paragraph {
        resolve_paragraph_style(candidates, parent)
    } else {
        inherit(pick_winners(candidates), parent)
    };
    mark_author_border(style, author_border)
}

/// This monotonic remapping retains origin identity without changing the sorted
/// cascade order: UA normal is 0 and UA important uses the ordinary cascade's
/// sentinel; the other origins occupy the intervening ranks.
fn candidate_origin(matched: &MatchedDeclaration) -> u8 {
    let important = matched.declaration.important;
    if matched.origin == CascadeOrigin::UserAgent {
        origin_rank(important, 0)
    } else {
        matched.origin.rank(important) + 1
    }
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

#[cfg(test)]
mod paragraph_origin_tests {
    use super::*;
    use crate::style::{parse_declarations, UA_QUIRKY_MARGIN_START};

    fn declarations(origin: CascadeOrigin, css: &str) -> Vec<MatchedDeclaration> {
        parse_declarations(css)
            .into_iter()
            .enumerate()
            .map(|(order, declaration)| MatchedDeclaration {
                declaration,
                origin,
                scope_depth: 0,
                inline: origin == CascadeOrigin::Author,
                specificity: (0, 0, 1),
                order,
            })
            .collect()
    }

    fn margin_mask(style: &ComputedStyle) -> u8 {
        style
            .get(UA_QUIRKY_MARGIN_START)
            .map_or(0, |value| value.parse().unwrap())
    }

    fn assert_provenance(groups: &[(CascadeOrigin, &str)], ua: bool) {
        let declarations: Vec<_> = groups
            .iter()
            .flat_map(|(origin, css)| declarations(*origin, css))
            .collect();
        let parent = ComputedStyle::default();
        let mut paragraph = resolve_matched_paragraph_style(&parent, &declarations);
        assert_eq!(margin_mask(&paragraph) & 1 != 0, ua);
        paragraph.remove(UA_QUIRKY_MARGIN_START);
        assert_eq!(paragraph, resolve_matched_style(&parent, &declarations));
    }

    #[test]
    fn ua_paragraph_margin_provenance_tracks_origin_not_equal_computed_values() {
        use CascadeOrigin::{Author, User, UserAgent};
        assert_provenance(&[(UserAgent, "margin:1em 0")], true);
        for css in ["margin:1em 0", "margin-top:1em", "margin-block-start:1em"] {
            assert_provenance(&[(UserAgent, "margin:1em 0"), (Author, css)], false);
        }
        assert_provenance(
            &[(UserAgent, "margin:1em 0"), (Author, "margin-bottom:2em")],
            true,
        );
        assert_provenance(
            &[
                (UserAgent, "margin:1em 0"),
                (User, "margin-top:1em !important"),
            ],
            false,
        );
        assert_provenance(
            &[
                (UserAgent, "margin-top:1em !important"),
                (Author, "margin-top:2em !important"),
            ],
            true,
        );
    }

    #[test]
    fn physical_side_origins_survive_vertical_writing_and_logical_overrides() {
        use CascadeOrigin::{Author, UserAgent};
        for (mode, css, expected) in [
            ("vertical-rl", "margin-right:10px", 13),
            ("vertical-lr", "margin-left:10px", 7),
            ("vertical-rl", "margin-block-start:1em", 13),
            ("vertical-lr", "margin-block-start:1em", 7),
            (
                "vertical-rl",
                "writing-mode:horizontal-tb;margin-block-start:1em",
                14,
            ),
            ("vertical-lr", "writing-mode:inherit;margin-left:10px", 7),
            (
                "vertical-rl",
                "writing-mode:initial;margin-block-start:1em",
                14,
            ),
            (
                "vertical-rl",
                "writing-mode:unset;margin-block-start:1em",
                13,
            ),
            (
                "vertical-rl",
                "--flow:horizontal-tb;writing-mode:var(--flow);margin-block-start:1em",
                14,
            ),
        ] {
            let parent = ComputedStyle::from_pairs([("writing-mode", mode)]);
            let mut matched = declarations(UserAgent, "margin:1em 0");
            matched.extend(declarations(Author, css));
            let style = resolve_matched_paragraph_style(&parent, &matched);
            assert_eq!(margin_mask(&style), expected, "{mode}: {css}");
        }
        for direction in ["rtl", " RTL "] {
            let parent = ComputedStyle::from_pairs([
                ("writing-mode", "vertical-rl"),
                ("direction", direction),
            ]);
            let mut matched = declarations(UserAgent, "margin:1em 0");
            matched.extend(declarations(Author, "margin-inline-start:10px"));
            assert_eq!(
                margin_mask(&resolve_matched_paragraph_style(&parent, &matched)),
                11
            );
        }
    }

    #[test]
    fn invalid_computed_author_margin_does_not_restore_ua_provenance() {
        use CascadeOrigin::{Author, UserAgent};
        assert_provenance(
            &[
                (UserAgent, "margin:1em 0"),
                (Author, "margin:var(--missing)"),
            ],
            false,
        );
        assert_provenance(
            &[(UserAgent, "margin:1em 0"), (Author, "margin:inherit")],
            false,
        );
    }
}

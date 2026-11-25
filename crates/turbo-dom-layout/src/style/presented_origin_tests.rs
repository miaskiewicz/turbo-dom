use super::*;
use crate::style::parse_declarations;

fn group(origin: CascadeOrigin, css: &str) -> Vec<MatchedDeclaration> {
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

fn resolve(groups: &[(CascadeOrigin, &str)]) -> ComputedStyle {
    let declarations: Vec<_> = groups
        .iter()
        .flat_map(|(origin, css)| group(*origin, css))
        .collect();
    resolve_matched_style(&ComputedStyle::default(), &declarations)
}

#[test]
fn presented_origins_follow_all_css_cascade_strata_in_either_source_order() {
    use CascadeOrigin::{Animation, Author, Transition, User, UserAgent};
    let strata = [
        (UserAgent, "width:10px"),
        (User, "width:20px"),
        (Author, "width:30px"),
        (Animation, "width:40px"),
        (Author, "width:50px!important"),
        (User, "width:60px!important"),
        (UserAgent, "width:70px!important"),
        (Transition, "width:80px"),
    ];
    for lower in 0..strata.len() {
        for higher in lower + 1..strata.len() {
            for groups in [
                [strata[lower], strata[higher]],
                [strata[higher], strata[lower]],
            ] {
                let expected = format!("{}px", (higher + 1) * 10);
                assert_eq!(
                    resolve(&groups).get("width"),
                    Some(expected.as_str()),
                    "{groups:?}"
                );
            }
        }
    }
}

#[test]
fn sampled_animation_cannot_acquire_importance_from_its_transport_declaration() {
    use CascadeOrigin::{Animation, Author};
    let style = resolve(&[
        (Author, "width:100px!important"),
        (Animation, "width:150px!important"),
    ]);
    assert_eq!(style.get("width"), Some("100px"));
}

#[test]
fn presented_parent_properties_resolve_before_child_inheritance_and_variables() {
    use CascadeOrigin::{Animation, Author};
    let parent = resolve(&[
        (Author, "font-size:20px;--animated-size:100px"),
        (Animation, "font-size:30px;--animated-size:200px"),
    ]);
    let declarations = group(Author, "width:var(--animated-size);height:1em");
    let child = resolve_matched_style(&parent, &declarations);
    assert_eq!(child.get("font-size"), Some("30px"));
    assert_eq!(child.get("width"), Some("200px"));
}

#[test]
fn transition_margin_is_never_misclassified_as_user_agent_provenance() {
    use crate::style::UA_QUIRKY_MARGIN_START;
    use CascadeOrigin::{Transition, UserAgent};
    let mut declarations = group(UserAgent, "margin-top:1em!important");
    let baseline = resolve_matched_paragraph_style(&ComputedStyle::default(), &declarations);
    assert_eq!(baseline.get(UA_QUIRKY_MARGIN_START), Some("1"));
    declarations.extend(group(Transition, "margin-top:2em"));
    let style = resolve_matched_paragraph_style(&ComputedStyle::default(), &declarations);
    assert_eq!(style.get("margin-top"), Some("2em"));
    let mask: u8 = style
        .get(UA_QUIRKY_MARGIN_START)
        .map_or(0, |value| value.parse().unwrap());
    assert_eq!(mask & 1, 0);
}

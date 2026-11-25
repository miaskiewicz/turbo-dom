//! Optional canonical host-family grammar admission. Raw selector units stay owned
//! by the caller; `LightningCSS` serialization never crosses the matching boundary.
use super::variables::{ComputationLimits, ComputedCssError};
use super::{CssSource, CssText};
use cssparser::{Parser, ParserInput, Token};
use lightningcss::selector::{Component, PseudoClass, PseudoElement, Selector, SelectorList};
use lightningcss::stylesheet::ParserOptions;
use lightningcss::traits::ParseWithOptions;

fn parse_failure(
    error: &cssparser::ParseError<'_, lightningcss::error::ParserError<'_>>,
) -> Result<SelectorPolicyInspection, ComputedCssError> {
    // Keep backend resource errors distinct even if a future pinned grammar
    // introduces an edge not represented by the conservative preflight.
    if super::grammar::parser_nesting(error) {
        Err(ComputedCssError::NestingLimit)
    } else {
        Ok(SelectorPolicyInspection::Invalid)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedPseudo {
    Before,
    After,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectorTarget {
    Element,
    Generated { pseudo: GeneratedPseudo },
    Slotted { pseudo: Option<GeneratedPseudo> },
    UnsupportedComposed,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectorPolicy {
    pub index: usize,
    pub host_reference: bool,
    pub target: SelectorTarget,
    pub specificity: Option<(u32, u32, u32)>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectorPolicyInspection {
    Unchanged,
    Invalid,
    Items(Vec<SelectorPolicy>),
}

pub fn inspect_selector_policy(
    raw: &CssText,
    limits: ComputationLimits,
) -> Result<SelectorPolicyInspection, ComputedCssError> {
    super::grammar::check_grammar_limits(raw, limits).map_err(|error| match error {
        super::grammar::GrammarLimitError::Computation(error) => error,
        super::grammar::GrammarLimitError::NestingLimit => ComputedCssError::NestingLimit,
    })?;
    let source = CssSource::new(raw.as_units());
    let mut input = ParserInput::new(source.scalar());
    if !has_family(&mut Parser::new(&mut input)) {
        return Ok(SelectorPolicyInspection::Unchanged);
    }
    let mut input = ParserInput::new(source.scalar());
    let mut parser = Parser::new(&mut input);
    let list = match SelectorList::parse_with_options(&mut parser, &ParserOptions::default()) {
        Ok(list) => list,
        Err(error) => return parse_failure(&error),
    };
    // The pinned SelectorList parser exhausts each comma-delimited slice and
    // returns success only after its final input.next() reports end-of-input.
    Ok(SelectorPolicyInspection::Items(
        list.0.iter().enumerate().map(policy).collect(),
    ))
}

fn family_name(name: &str) -> bool {
    ["host", "host-context", "slotted", "part"]
        .iter()
        .any(|v| name.eq_ignore_ascii_case(v))
}
fn has_family(parser: &mut Parser<'_, '_>) -> bool {
    let mut colon = false;
    let mut found = false;
    while let Ok(token) = parser.next().cloned() {
        match token {
            Token::Colon => {
                colon = true;
                continue;
            }
            Token::Ident(name) if colon && family_name(&name) => found = true,
            Token::Function(name) => {
                found |= colon && family_name(&name);
                let nested = parser
                    .parse_nested_block(|nested| {
                        Ok::<_, cssparser::ParseError<'_, ()>>(has_family(nested))
                    })
                    // The callback consumes every token, including BADSTRING,
                    // BADURL and unmatched closing tokens, and cannot return a
                    // custom error. cssparser's only remaining error would mean
                    // a successful callback left input unconsumed.
                    .expect("family token callback exhausts its block");
                found |= nested;
            }
            _ => {}
        }
        colon = false;
    }
    found
}
fn has_host(selector: &Selector<'_>) -> bool {
    selector
        .iter_raw_match_order()
        .any(|component| match component {
            Component::Host(_) => true,
            Component::Is(list)
            | Component::Where(list)
            | Component::Negation(list)
            | Component::Has(list)
            | Component::Any(_, list) => list.iter().any(has_host),
            Component::NthOf(nth) => nth.selectors().iter().any(has_host),
            _ => false,
        })
}
fn nested_unsupported(component: &Component<'_>) -> bool {
    let unsupported =
        |selector: &Selector<'_>| target(selector) == SelectorTarget::UnsupportedComposed;
    match component {
        Component::Is(list)
        | Component::Where(list)
        | Component::Negation(list)
        | Component::Has(list)
        | Component::Any(_, list) => list.iter().any(unsupported),
        Component::NthOf(nth) => nth.selectors().iter().any(unsupported),
        Component::Host(Some(selector)) | Component::Slotted(selector) => unsupported(selector),
        _ => false,
    }
}
fn target(selector: &Selector<'_>) -> SelectorTarget {
    let mut slotted = false;
    let mut generated = None;
    for component in selector.iter_raw_match_order() {
        if nested_unsupported(component) {
            return SelectorTarget::UnsupportedComposed;
        }
        match component {
            Component::Slotted(_) => slotted = true,
            Component::NonTSPseudoClass(PseudoClass::CustomFunction { name, .. })
                if name.eq_ignore_ascii_case("host-context") =>
            {
                return SelectorTarget::UnsupportedComposed
            }
            Component::PseudoElement(PseudoElement::Before) => {
                generated = Some(GeneratedPseudo::Before);
            }
            Component::PseudoElement(PseudoElement::After) => {
                generated = Some(GeneratedPseudo::After);
            }
            Component::Part(_) | Component::PseudoElement(_) => {
                return SelectorTarget::UnsupportedComposed
            }
            _ => {}
        }
    }
    if slotted {
        return SelectorTarget::Slotted { pseudo: generated };
    }
    generated.map_or(SelectorTarget::Element, |pseudo| {
        SelectorTarget::Generated { pseudo }
    })
}
fn policy((index, selector): (usize, &Selector<'_>)) -> SelectorPolicy {
    let host_reference = has_host(selector);
    let packed = selector.specificity();
    SelectorPolicy {
        index,
        host_reference,
        target: target(selector),
        specificity: host_reference.then_some((packed >> 20, (packed >> 10) & 1023, packed & 1023)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inspect(value: &str) -> SelectorPolicyInspection {
        inspect_selector_policy(
            &CssText::from_units(value.encode_utf16().collect()),
            ComputationLimits {
                max_nesting: 32,
                max_output_units: 4096,
                max_total_units: 4096,
            },
        )
        .unwrap()
    }
    #[test]
    fn ordinary_contract_and_raw_quoted_names_are_unchanged() {
        for value in [".ordinary", "[data-x=':host']", ".host", r".\:host"] {
            assert_eq!(inspect(value), SelectorPolicyInspection::Unchanged);
        }
    }
    #[test]
    fn canonical_host_compound_admission_and_specificity() {
        let SelectorPolicyInspection::Items(items) = inspect(":host(#a.foo), :is(:host(.foo))")
        else {
            panic!()
        };
        assert_eq!(items[0].specificity, Some((1, 2, 0)));
        assert_eq!(items[1].specificity, Some((0, 2, 0)));
        assert!(items.iter().all(|item| item.host_reference));
        for value in [":host(.a,.b)", ":host(.a > .b)", ":host(:not(.a .b))"] {
            assert_eq!(inspect(value), SelectorPolicyInspection::Invalid, "{value}");
        }
    }
    #[test]
    fn slotted_and_generated_targets_are_separate() {
        let SelectorPolicyInspection::Items(items) =
            inspect("slot::slotted(.x), ::slotted(.x)::before, :host::after, ::part(x)")
        else {
            panic!()
        };
        assert_eq!(items[0].target, SelectorTarget::Slotted { pseudo: None });
        assert_eq!(
            items[1].target,
            SelectorTarget::Slotted {
                pseudo: Some(GeneratedPseudo::Before)
            }
        );
        assert_eq!(
            items[2].target,
            SelectorTarget::Generated {
                pseudo: GeneratedPseudo::After
            }
        );
        assert_eq!(items[3].target, SelectorTarget::UnsupportedComposed);
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;
    fn raw(value: &str) -> CssText {
        CssText::from_units(value.encode_utf16().collect())
    }
    fn limits(depth: usize, units: usize) -> ComputationLimits {
        ComputationLimits {
            max_nesting: depth,
            max_output_units: units,
            max_total_units: units,
        }
    }
    #[test]
    fn malformed_family_tokens_do_not_hide_or_invent_references() {
        for value in [
            ":host)",
            ":host]",
            ":host[data=\"bad\n]",
            ":host[url= url(\"bad\n)]",
        ] {
            assert_eq!(
                inspect_selector_policy(&raw(value), limits(32, 4096)),
                Ok(SelectorPolicyInspection::Invalid),
                "{value}"
            );
        }
        // These are outside this host-family admission policy. The ordinary
        // selector compiler, rather than this token probe, decides validity.
        for value in [
            "[data=':host']",
            "[data=\":host\n]",
            "[data=url(:host)]",
            "[data=')']",
            r".\:host",
            "[data=':host'] ]",
        ] {
            assert_eq!(
                inspect_selector_policy(&raw(value), limits(32, 4096)),
                Ok(SelectorPolicyInspection::Unchanged),
                "{value}"
            );
        }
        for value in [":is(:host, .x)", ":is(.x, :host)", ":not(:host(.x))"] {
            let source = CssSource::new(raw(value).as_units());
            let mut input = ParserInput::new(source.scalar());
            assert!(has_family(&mut Parser::new(&mut input)), "{value}");
        }
    }
    #[test]
    fn exact_size_and_depth_limits_preserve_typed_errors() {
        assert_eq!(
            inspect_selector_policy(&raw(":host"), limits(0, 5)),
            Err(ComputedCssError::NestingLimit)
        );
        assert_eq!(
            inspect_selector_policy(&raw(":host"), limits(1, 4)),
            Err(ComputedCssError::ResourceLimit)
        );
        assert!(matches!(
            inspect_selector_policy(&raw(":host"), limits(1, 5)),
            Ok(SelectorPolicyInspection::Items(_))
        ));
        assert_eq!(
            inspect_selector_policy(&raw(":host(.x)"), limits(1, 9)),
            Err(ComputedCssError::NestingLimit)
        );
        assert!(matches!(
            inspect_selector_policy(&raw(":host(.x)"), limits(2, 9)),
            Ok(SelectorPolicyInspection::Items(_))
        ));
    }
}

#[cfg(test)]
mod backend_conversion_contract {
    use super::*;
    #[test]
    fn structured_backend_error_is_not_downgraded_to_invalid_selector() {
        // This tests the error-conversion contract, not a public-input parser RED.
        let mut source = ParserInput::new("");
        let parser = Parser::new(&mut source);
        let error = parser.new_custom_error(lightningcss::error::ParserError::MaximumNestingDepth);
        assert_eq!(parse_failure(&error), Err(ComputedCssError::NestingLimit));
        let syntax = parser.new_error(cssparser::BasicParseErrorKind::EndOfInput);
        assert_eq!(
            parse_failure(&syntax),
            Ok(SelectorPolicyInspection::Invalid)
        );
    }
}

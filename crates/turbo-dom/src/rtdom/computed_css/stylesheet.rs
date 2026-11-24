//! Shared CSS Syntax rule tree. Group interpretation belongs to the caller's context.
use super::{CssSource, CssText};
use cssparser::{
    AtRuleParser, CowRcStr, ParseError, ParseErrorKind, Parser, ParserInput, ParserState,
    QualifiedRuleParser, StyleSheetParser,
};

/// Recognized conditional or ordering group; its semantics are not evaluated here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupKind {
    Media,
    Supports,
    Layer,
    Container,
    Scope,
    StartingStyle,
}

/// Whether recognized group bodies become nested rules or remain raw payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupMode {
    Recursive,
    Opaque,
}

/// Caller-supplied bounds, counted across one complete source and all parsed groups.
#[derive(Clone, Copy, Debug)]
pub struct SheetLimits {
    /// Maximum original UTF-16 units.
    pub source_units: usize,
    /// Maximum encountered rules, including ignored or malformed rule preludes.
    pub rules: usize,
    /// Maximum nested recognized groups in recursive mode; outermost is depth one.
    pub group_depth: usize,
}

/// A resource boundary, distinct from CSS syntax recovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SheetError {
    SourceUnits,
    RuleCount,
    GroupDepth,
}

/// Owned source records. Selector/declaration grammar and group evaluation are separate.
/// Raw rule bodies retain comments and original UTF-16 units.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SheetRule {
    Style {
        selector: CssText,
        body: CssText,
    },
    Group {
        kind: GroupKind,
        prelude: CssText,
        rules: Vec<SheetRule>,
    },
    Statement {
        name: String,
        prelude: CssText,
    },
    Opaque {
        name: String,
        prelude: CssText,
        body: CssText,
    },
    Ignored,
}

#[derive(Clone, Copy)]
enum AtKind {
    Group(GroupKind),
    Statement,
    Descriptor,
    Ignored,
}

struct AtPrelude {
    name: String,
    text: CssText,
    kind: AtKind,
}

struct RuleParser<'source, 'count> {
    source: &'source CssSource,
    count: &'count mut usize,
    limits: SheetLimits,
    depth: usize,
    mode: GroupMode,
}

fn at_kind(name: &str) -> AtKind {
    const GROUPS: &[(&str, GroupKind)] = &[
        ("media", GroupKind::Media),
        ("supports", GroupKind::Supports),
        ("layer", GroupKind::Layer),
        ("container", GroupKind::Container),
        ("scope", GroupKind::Scope),
        ("starting-style", GroupKind::StartingStyle),
    ];
    if let Some((_, kind)) = GROUPS.iter().find(|(candidate, _)| *candidate == name) {
        return AtKind::Group(*kind);
    }
    if ["import", "namespace", "charset"].contains(&name) {
        return AtKind::Statement;
    }
    if [
        "font-face",
        "property",
        "page",
        "keyframes",
        "-webkit-keyframes",
    ]
    .contains(&name)
    {
        return AtKind::Descriptor;
    }
    AtKind::Ignored
}

impl RuleParser<'_, '_> {
    fn charge(&mut self) -> Result<(), SheetError> {
        if *self.count >= self.limits.rules {
            return Err(SheetError::RuleCount);
        }
        *self.count += 1;
        Ok(())
    }

    fn take(&self, input: &mut Parser<'_, '_>) -> CssText {
        let start = input.position().byte_index();
        while input.next_including_whitespace_and_comments().is_ok() {}
        self.source
            .raw_range(start..input.position().byte_index())
            .expect("CSS parser spans refer to this source")
    }

    fn children(&mut self, input: &mut Parser<'_, '_>) -> Result<Vec<SheetRule>, SheetError> {
        if self.depth >= self.limits.group_depth {
            return Err(SheetError::GroupDepth);
        }
        collect_rules(
            input,
            &mut RuleParser {
                source: self.source,
                count: self.count,
                limits: self.limits,
                depth: self.depth + 1,
                mode: self.mode,
            },
        )
    }
}

impl<'i> AtRuleParser<'i> for RuleParser<'_, '_> {
    type Prelude = AtPrelude;
    type AtRule = SheetRule;
    type Error = SheetError;

    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<AtPrelude, ParseError<'i, SheetError>> {
        self.charge()
            .map_err(|error| input.new_custom_error(error))?;
        let name = name.to_ascii_lowercase();
        Ok(AtPrelude {
            kind: at_kind(&name),
            name,
            text: self.take(input).trim_css(),
        })
    }

    fn rule_without_block(&mut self, prelude: AtPrelude, _: &ParserState) -> Result<SheetRule, ()> {
        match prelude.kind {
            AtKind::Statement | AtKind::Group(GroupKind::Layer) => Ok(SheetRule::Statement {
                name: prelude.name,
                prelude: prelude.text,
            }),
            AtKind::Ignored => Ok(SheetRule::Ignored),
            _ => Err(()),
        }
    }

    fn parse_block<'t>(
        &mut self,
        prelude: AtPrelude,
        _: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<SheetRule, ParseError<'i, SheetError>> {
        match prelude.kind {
            AtKind::Group(kind) if self.mode == GroupMode::Recursive => {
                let rules = self
                    .children(input)
                    .map_err(|error| input.new_custom_error(error))?;
                Ok(SheetRule::Group {
                    kind,
                    prelude: prelude.text,
                    rules,
                })
            }
            AtKind::Group(_) | AtKind::Descriptor => Ok(SheetRule::Opaque {
                name: prelude.name,
                prelude: prelude.text,
                body: self.take(input),
            }),
            _ => {
                self.take(input);
                Ok(SheetRule::Ignored)
            }
        }
    }
}

impl<'i> QualifiedRuleParser<'i> for RuleParser<'_, '_> {
    type Prelude = CssText;
    type QualifiedRule = SheetRule;
    type Error = SheetError;

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<CssText, ParseError<'i, SheetError>> {
        self.charge()
            .map_err(|error| input.new_custom_error(error))?;
        Ok(self.take(input).trim_css())
    }

    fn parse_block<'t>(
        &mut self,
        selector: CssText,
        _: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<SheetRule, ParseError<'i, SheetError>> {
        Ok(SheetRule::Style {
            selector,
            body: self.take(input),
        })
    }
}

fn collect_rules(
    input: &mut Parser<'_, '_>,
    parser: &mut RuleParser<'_, '_>,
) -> Result<Vec<SheetRule>, SheetError> {
    let mut rules = Vec::new();
    for item in StyleSheetParser::new(input, parser) {
        match item {
            Ok(SheetRule::Ignored) => {}
            Ok(rule) => rules.push(rule),
            Err((error, _)) => {
                if let ParseErrorKind::Custom(error) = error.kind {
                    return Err(error);
                }
            }
        }
    }
    Ok(rules)
}

/// Parse CSS rule structure while retaining original selector, prelude and body units.
/// Unknown at-rules and malformed rules are discarded by CSS Syntax recovery. Known
/// descriptors stay opaque; style bodies are not recursively interpreted as nesting.
/// No selector admission, condition evaluation or declaration computation occurs.
///
/// # Errors
/// Returns [`SheetError`] when a caller limit is exceeded; limits never become syntax
/// recovery or an empty successful result.
pub fn parse_sheet(
    units: &[u16],
    limits: SheetLimits,
    mode: GroupMode,
) -> Result<Vec<SheetRule>, SheetError> {
    if units.len() > limits.source_units {
        return Err(SheetError::SourceUnits);
    }
    let source = CssSource::new(units);
    let mut parser_input = ParserInput::new(source.scalar());
    let mut input = Parser::new(&mut parser_input);
    let mut count = 0;
    collect_rules(
        &mut input,
        &mut RuleParser {
            source: &source,
            count: &mut count,
            limits,
            depth: 0,
            mode,
        },
    )
}

pub(crate) fn legacy_sheet(source: &str) -> Vec<SheetRule> {
    let units: Vec<u16> = source.encode_utf16().collect();
    // Each rule consumes at least one source unit. Opaque groups do not recurse,
    // so these input-derived bounds add no new rejection to the old raw API.
    let limits = SheetLimits {
        source_units: units.len(),
        rules: units.len().max(1),
        group_depth: 0,
    };
    parse_sheet(&units, limits, GroupMode::Opaque)
        .expect("input-derived legacy bounds admit every source rule")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }
    fn limits() -> SheetLimits {
        SheetLimits {
            source_units: 4096,
            rules: 32,
            group_depth: 8,
        }
    }
    fn parse(text: &str) -> Vec<SheetRule> {
        parse_sheet(&units(text), limits(), GroupMode::Recursive).unwrap()
    }

    #[test]
    fn conditional_rule_tree_retains_conditions_and_source_order() {
        let rules = parse(".t{color:red}@media (width > 10px){@supports (color:blue){.t{color:blue}}}.t{color:green}");
        assert_eq!(rules.len(), 3);
        let SheetRule::Group {
            kind,
            prelude,
            rules: media,
        } = &rules[1]
        else {
            panic!("media group")
        };
        assert_eq!(*kind, GroupKind::Media);
        assert_eq!(prelude.as_units(), units("(width > 10px)"));
        let SheetRule::Group {
            kind,
            prelude,
            rules: support,
        } = &media[0]
        else {
            panic!("supports group")
        };
        assert_eq!(*kind, GroupKind::Supports);
        assert_eq!(prelude.as_units(), units("(color:blue)"));
        let SheetRule::Style { selector, body } = &support[0] else {
            panic!("style rule")
        };
        assert_eq!(selector.as_units(), units(".t"));
        assert_eq!(
            super::super::declarations(body.as_units())[0]
                .value
                .as_units(),
            units("blue")
        );
    }

    #[test]
    fn statements_and_opaque_rules_do_not_leak_descendant_styles() {
        let rules = parse("@layer reset,app;@layer app{.t{color:red}}@font-face{font-family:x;src:url(x)}@unknown{.t{color:blue}}@unknown;@media print;.t{color:green}dangling");
        assert_eq!(rules.len(), 4);
        let SheetRule::Statement { name, prelude } = &rules[0] else {
            panic!("layer ordering")
        };
        assert_eq!(name, "layer");
        assert_eq!(prelude.as_units(), units("reset,app"));
        assert!(matches!(
            rules[1],
            SheetRule::Group {
                kind: GroupKind::Layer,
                ..
            }
        ));
        let SheetRule::Opaque { name, body, .. } = &rules[2] else {
            panic!("font face")
        };
        assert_eq!(name, "font-face");
        assert_eq!(body.as_units(), units("font-family:x;src:url(x)"));
        assert!(matches!(rules[3], SheetRule::Style { .. }));
        let statements = parse("@import url(x);@namespace svg url(x);@namespace x{} .t{}");
        assert_eq!(statements.len(), 3);
    }

    #[test]
    fn source_units_survive_tokenizer_scalar_view() {
        let mut text = units("[title=\"");
        text.push(0xd800);
        text.extend(units("\"]{--Token:\""));
        text.push(0xdc00);
        text.extend(units("\"}"));
        let rules = parse_sheet(&text, limits(), GroupMode::Recursive).unwrap();
        let SheetRule::Style { selector, body } = &rules[0] else {
            panic!("style rule")
        };
        assert!(selector.as_units().contains(&0xd800));
        assert_eq!(
            super::super::declarations(body.as_units())[0]
                .value
                .as_units(),
            [34, 0xdc00, 34]
        );
    }

    #[test]
    fn syntax_errors_are_recovered_but_limits_are_not_discarded() {
        let style = units(".t{}");
        assert_eq!(
            parse_sheet(
                &style,
                SheetLimits {
                    source_units: 0,
                    ..limits()
                },
                GroupMode::Recursive
            ),
            Err(SheetError::SourceUnits)
        );
        for text in [".t{}", "@media screen{}"] {
            assert_eq!(
                parse_sheet(
                    &units(text),
                    SheetLimits {
                        rules: 0,
                        ..limits()
                    },
                    GroupMode::Recursive
                ),
                Err(SheetError::RuleCount)
            );
        }
        assert_eq!(
            parse_sheet(
                &units("@media screen{}"),
                SheetLimits {
                    group_depth: 0,
                    ..limits()
                },
                GroupMode::Recursive
            ),
            Err(SheetError::GroupDepth)
        );
        assert!(parse("dangling").is_empty());
        assert_eq!(legacy_sheet("@media screen{.t{color:red}}").len(), 1);
    }
}

//! Scope prelude structure retains original selector units; matching is separate.
use super::{check_component_limits, ComputationLimits, ComputedCssError, CssSource, CssText};
use cssparser::{ParseError, Parser, ParserInput, Token};

pub struct ScopePrelude {
    pub start: Option<CssText>,
    pub end: Option<CssText>,
}
struct ScopeParser<'a> {
    source: &'a CssSource,
}
impl ScopeParser<'_> {
    fn selector<'i>(&self, input: &mut Parser<'i, '_>) -> Result<CssText, ParseError<'i, ()>> {
        input.parse_nested_block(|input| {
            let start = input.position().byte_index();
            while input.next_including_whitespace_and_comments().is_ok() {}
            let text = self
                .source
                .raw_range(start..input.position().byte_index())
                .expect("parser range from original scalar view")
                .trim_css();
            if text.as_units().is_empty() {
                return Err(input.new_custom_error(()));
            }
            Ok(text)
        })
    }
    fn end<'i>(&self, input: &mut Parser<'i, '_>) -> Result<CssText, ParseError<'i, ()>> {
        match input.next()?.clone() {
            Token::Function(name) if name.eq_ignore_ascii_case("to") => self.selector(input),
            Token::Ident(name) if name.eq_ignore_ascii_case("to") => {
                input.expect_parenthesis_block()?;
                self.selector(input)
            }
            token => Err(input.new_unexpected_token_error(token)),
        }
    }
    fn prelude<'i>(&self, input: &mut Parser<'i, '_>) -> Result<ScopePrelude, ParseError<'i, ()>> {
        if input.is_exhausted() {
            return Ok(ScopePrelude {
                start: None,
                end: None,
            });
        }
        let start = if input.try_parse(Parser::expect_parenthesis_block).is_ok() {
            Some(self.selector(input)?)
        } else {
            None
        };
        let end = if input.is_exhausted() {
            None
        } else {
            Some(self.end(input)?)
        };
        input.expect_exhausted()?;
        Ok(ScopePrelude { start, end })
    }
}
/// CSS-invalid prelude syntax is absent; resource errors never become absent scopes.
pub fn parse_scope_prelude(
    raw: &CssText,
    limits: ComputationLimits,
) -> Result<Option<ScopePrelude>, ComputedCssError> {
    check_component_limits(raw, limits)?;
    let source = CssSource::new(raw.as_units());
    let mut input = ParserInput::new(source.scalar());
    Ok(ScopeParser { source: &source }
        .prelude(&mut Parser::new(&mut input))
        .ok())
}

/// A mechanical selector answer for one candidate root in the target's ancestor path.
/// Index zero denotes the target. `limits` includes every matching limit in that path;
/// deciding whether a match cuts off a candidate scope belongs to this module.
pub struct ScopeWitness {
    pub enclosing_root: usize,
    pub root: usize,
    pub start_matches: bool,
    pub limits: Vec<usize>,
}
impl ScopeWitness {
    fn valid_path(&self, length: usize) -> bool {
        self.root < length
            && self.enclosing_root < length
            && self.limits.iter().all(|index| *index < length)
    }
    fn applies(&self, enclosing_roots: &[usize]) -> bool {
        self.start_matches
            && self.root <= self.enclosing_root
            && enclosing_roots.contains(&self.enclosing_root)
            && !self.limits.iter().any(|index| *index <= self.root)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeFactError {
    InvalidPath,
    ResourceLimit,
}
/// Produce all applicable root contexts, nearest first. The matching adapter must
/// enumerate candidate starts and limits; it never chooses the closest root itself.
pub fn applicable_roots(
    path_length: usize,
    enclosing_roots: &[usize],
    witnesses: &[ScopeWitness],
    maximum_facts: usize,
) -> Result<Vec<(usize, usize)>, ScopeFactError> {
    if path_length == 0 || enclosing_roots.iter().any(|root| *root >= path_length) {
        return Err(ScopeFactError::InvalidPath);
    }
    let mut count = witnesses.len();
    let mut output = Vec::new();
    for witness in witnesses {
        count = count
            .checked_add(witness.limits.len())
            .ok_or(ScopeFactError::ResourceLimit)?;
        if count > maximum_facts {
            return Err(ScopeFactError::ResourceLimit);
        }
        if !witness.valid_path(path_length) {
            return Err(ScopeFactError::InvalidPath);
        }
        if witness.applies(enclosing_roots) {
            output.push((witness.enclosing_root, witness.root));
        }
    }
    output.sort_unstable_by_key(|(parent, root)| (*root, *parent));
    output.dedup();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn text(value: &str) -> CssText {
        CssText::from_units(value.encode_utf16().collect())
    }
    fn limits() -> ComputationLimits {
        ComputationLimits {
            max_nesting: 16,
            max_output_units: 4096,
            max_total_units: 8192,
        }
    }
    #[test]
    fn scope_boundaries_are_selectors_not_string_split_tokens() {
        for source in ["(.root) to(.limit)", "(.root) to (.limit)"] {
            let parsed = parse_scope_prelude(&text(source), limits())
                .unwrap()
                .unwrap();
            assert_eq!(parsed.start.unwrap().as_units(), text(".root").as_units());
            assert_eq!(parsed.end.unwrap().as_units(), text(".limit").as_units());
        }
        let parsed =
            parse_scope_prelude(&text("([title=\"to(.other)\"]):invalid"), limits()).unwrap();
        assert!(parsed.is_none());
        let parsed = parse_scope_prelude(&text("([title=\"to(.other)\"])"), limits())
            .unwrap()
            .unwrap();
        assert_eq!(
            parsed.start.unwrap().as_units(),
            text("[title=\"to(.other)\"]").as_units()
        );
    }
    #[test]
    fn implicit_starts_and_raw_quoted_units_survive_context_compilation() {
        let empty = parse_scope_prelude(&text(""), limits()).unwrap().unwrap();
        assert!(empty.start.is_none() && empty.end.is_none());
        assert!(parse_scope_prelude(&text("to(.limit)"), limits())
            .unwrap()
            .unwrap()
            .start
            .is_none());
        let mut source = "([title=\"".encode_utf16().collect::<Vec<_>>();
        source.push(0xd800);
        source.extend("\"])".encode_utf16());
        assert!(parse_scope_prelude(&CssText::from_units(source), limits())
            .unwrap()
            .unwrap()
            .start
            .unwrap()
            .as_units()
            .contains(&0xd800));
        for source in [
            "to",
            "()",
            "(.x) to()",
            "(.x) trailing",
            "(.x) to .limit",
            "to(.x) extra",
        ] {
            assert!(
                parse_scope_prelude(&text(source), limits())
                    .unwrap()
                    .is_none(),
                "{source}"
            );
        }
    }
}

#[cfg(test)]
mod fact_tests {
    use super::*;
    fn witness(root: usize, limits: Vec<usize>) -> ScopeWitness {
        ScopeWitness {
            enclosing_root: 4,
            root,
            start_matches: true,
            limits,
        }
    }
    #[test]
    fn nearest_root_and_inclusive_limit_match_chrome159_controls() {
        // The oracle's root itself matches the limit: neither it nor its child is scoped.
        assert!(
            applicable_roots(5, &[4], &[witness(0, vec![0]), witness(1, vec![1])], 4)
                .unwrap()
                .is_empty()
        );
        let actual = applicable_roots(
            5,
            &[4],
            &[
                witness(3, vec![]),
                witness(1, vec![]),
                witness(2, vec![3]),
                witness(2, vec![0]),
            ],
            6,
        )
        .unwrap();
        assert_eq!(actual, vec![(4, 1), (4, 2), (4, 3)]);
    }
    #[test]
    fn facts_are_bounded_and_unknown_paths_do_not_become_empty_matches() {
        assert_eq!(
            applicable_roots(0, &[], &[], 0),
            Err(ScopeFactError::InvalidPath)
        );
        assert_eq!(
            applicable_roots(5, &[5], &[], 0),
            Err(ScopeFactError::InvalidPath)
        );
        assert_eq!(
            applicable_roots(5, &[4], &[witness(1, vec![5])], 2),
            Err(ScopeFactError::InvalidPath)
        );
        assert_eq!(
            applicable_roots(5, &[4], &[witness(1, vec![])], 0),
            Err(ScopeFactError::ResourceLimit)
        );
        let wrong_parent = ScopeWitness {
            enclosing_root: 3,
            ..witness(1, vec![])
        };
        assert!(applicable_roots(5, &[4], &[wrong_parent], 1)
            .unwrap()
            .is_empty());
    }
}

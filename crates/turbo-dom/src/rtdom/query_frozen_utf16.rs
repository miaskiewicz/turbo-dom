//! FROZEN UTF-16 selector parser for the `selector-dual` gate: the S3a tip (4d5a63d) `query_utf16.rs`
//! after the scripted rewrite in `scripts/verify-query-frozen.rules`. Never edit by hand.
//! Parse original DOM string units into native selector records. Identifiers use
//! CSS replacement rules; raw quoted attribute strings preserve their units.
use super::{
    nth_of_position, parse_nth, parse_pseudo, Attr, AttrMatch, Combinator, Complex, Compound,
    Handle, NodeType, Pseudo, RelativeSelector, Tree, TypeNamespace,
};
#[derive(Debug, Clone)]
pub(super) struct UnitAttrMatch {
    op: u16,
    value: Vec<u16>,
    insensitive: bool,
    sensitive: bool,
}
impl UnitAttrMatch {
    pub(super) fn scalar(source: &AttrMatch) -> Self {
        let (op, value) = match source {
            AttrMatch::Present => (0, ""),
            AttrMatch::Equals(value) => (61, value.as_str()),
            AttrMatch::Includes(value) => (126, value.as_str()),
            AttrMatch::Dash(value) => (124, value.as_str()),
            AttrMatch::Prefix(value) => (94, value.as_str()),
            AttrMatch::Suffix(value) => (36, value.as_str()),
            AttrMatch::Substr(value) => (42, value.as_str()),
            AttrMatch::Units(value) => return value.clone(),
        };
        Self {
            op,
            value: value.encode_utf16().collect(),
            insensitive: false,
            sensitive: false,
        }
    }

    pub(super) fn matches(&self, got: &[u16]) -> bool {
        self.matches_case(got, false)
    }
    pub(super) fn matches_case(&self, got: &[u16], implicit: bool) -> bool {
        let value = &self.value;
        let insensitive = !self.sensitive && (self.insensitive || implicit);
        let equal = |a: &[u16], b: &[u16]| equal_units(a, b, insensitive);
        match self.op {
            0 => true,
            61 => equal(got, value),
            94 => !value.is_empty() && got.get(..value.len()).is_some_and(|a| equal(a, value)),
            36 => {
                !value.is_empty()
                    && got
                        .get(got.len().saturating_sub(value.len())..)
                        .is_some_and(|a| equal(a, value))
            }
            42 => !value.is_empty() && got.windows(value.len()).any(|a| equal(a, value)),
            126 => !value.is_empty() && got.split(|&u| whitespace(u)).any(|a| equal(a, value)),
            124 => {
                equal(got, value)
                    || (got.get(value.len()) == Some(&45)
                        && got.get(..value.len()).is_some_and(|a| equal(a, value)))
            }
            _ => false,
        }
    }
}
fn equal_units(a: &[u16], b: &[u16], insensitive: bool) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(&a, &b)| {
            let fold = |u| {
                if insensitive && (65..=90).contains(&u) {
                    u + 32
                } else {
                    u
                }
            };
            fold(a) == fold(b)
        })
}
fn whitespace(unit: u16) -> bool {
    matches!(unit, 9 | 10 | 12 | 13 | 32)
}
pub(super) fn has_class_units(got: &[u16], value: &[u16]) -> bool {
    !value.is_empty()
        && got
            .split(|&unit| whitespace(unit))
            .any(|word| word == value)
}
fn trim(mut units: &[u16]) -> &[u16] {
    while units.first().is_some_and(|&u| whitespace(u)) {
        units = &units[1..];
    }
    while units.last().is_some_and(|&u| whitespace(u)) {
        units = &units[..units.len() - 1];
    }
    units
}
// Hex escapes denote Unicode scalar values. Raw quoted units take a different
// path and must not be repaired just because a DOM string is not valid UTF-8.
fn decoded(units: &[u16], quoted: bool) -> Vec<u16> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        i += 1;
        if unit != 92 {
            out.push(if unit == 0 { 0xfffd } else { unit });
            continue;
        }
        if i == units.len() {
            continue;
        }
        if matches!(units[i], 10 | 12 | 13) {
            if units[i] == 13 && units.get(i + 1) == Some(&10) {
                i += 1;
            }
            i += 1;
            continue;
        }
        let mut hex = 0u32;
        let mut count = 0;
        while count < 6 && i < units.len() {
            let Some(digit) = char::from_u32(u32::from(units[i])).and_then(|c| c.to_digit(16))
            else {
                break;
            };
            hex = hex * 16 + digit;
            i += 1;
            count += 1;
        }
        if count != 0 {
            if i < units.len() && whitespace(units[i]) {
                if units[i] == 13 && units.get(i + 1) == Some(&10) {
                    i += 1;
                }
                i += 1;
            }
            let scalar = char::from_u32(hex)
                .filter(|&c| c != '\0')
                .unwrap_or('\u{fffd}');
            let mut buffer = [0; 2];
            out.extend_from_slice(scalar.encode_utf16(&mut buffer));
        } else {
            out.push(units[i]);
            i += 1;
        }
    }
    if quoted {
        out
    } else {
        String::from_utf16_lossy(&out).encode_utf16().collect()
    }
}
fn identifier(units: &[u16]) -> String {
    String::from_utf16_lossy(&decoded(units, false))
}
pub(super) fn attr(inner: &[u16]) -> Attr {
    let cleaned = attribute_comments(inner);
    let inner = cleaned.as_deref().unwrap_or(inner);
    let Some(eq) = inner.iter().position(|&u| u == 61) else {
        return Attr::new(
            identifier(trim(inner)),
            AttrMatch::Units(UnitAttrMatch {
                op: if unquoted_end(trim(inner)) == trim(inner).len() {
                    0
                } else {
                    u16::MAX
                },
                value: Vec::new(),
                insensitive: false,
                sensitive: false,
            }),
        );
    };
    let raw = trim(&inner[..eq]);
    let operator = raw
        .last()
        .copied()
        .filter(|u| matches!(u, 126 | 124 | 94 | 36 | 42));
    let name = operator.map_or(raw, |_| &raw[..raw.len() - 1]);
    let source = trim(&inner[eq + 1..]);
    let (value, quoted, modifier) = attribute_value(source);
    let valid = modifier
        .as_deref()
        .is_some_and(|m| m.is_empty() || m == [105] || m == [73] || m == [115] || m == [83]);
    Attr::new(
        identifier(trim(name)),
        AttrMatch::Units(UnitAttrMatch {
            op: if valid {
                operator.unwrap_or(61)
            } else {
                u16::MAX
            },
            value: decoded(value, quoted),
            insensitive: modifier.as_deref().is_some_and(|m| m == [105] || m == [73]),
            sensitive: modifier.as_deref().is_some_and(|m| m == [115] || m == [83]),
        }),
    )
}
fn attribute_comments(source: &[u16]) -> Option<Vec<u16>> {
    if !source.windows(2).any(|p| p == [47, 42]) {
        return None;
    }
    let mut out = Vec::new();
    let (mut index, mut quote) = (0, 0);
    while index < source.len() {
        let unit = source[index];
        if unit == 92 {
            let end = escaped_end(source, index);
            out.extend_from_slice(&source[index..end]);
            index = end;
            continue;
        }
        if quote == 0 && source[index..].starts_with(&[47, 42]) {
            let end = source[index + 2..].windows(2).position(|p| p == [42, 47]);
            let Some(end) = end else {
                return Some(vec![0]);
            };
            index += end + 4;
            out.push(32);
            continue;
        }
        if matches!(unit, 34 | 39) {
            quote = if quote == unit {
                0
            } else if quote == 0 {
                unit
            } else {
                quote
            };
        }
        out.push(unit);
        index += 1;
    }
    Some(out)
}
fn attribute_value(source: &[u16]) -> (&[u16], bool, Option<Vec<u16>>) {
    if source.first().is_some_and(|u| matches!(u, 34 | 39)) {
        let quote = source[0];
        let mut end = 1;
        while end < source.len() {
            if source[end] == quote {
                return (
                    &source[1..end],
                    true,
                    Some(decoded(trim(&source[end + 1..]), false)),
                );
            }
            end = escaped_end(source, end);
        }
        return (source, true, None);
    }
    let end = unquoted_end(source);
    (
        &source[..end],
        false,
        Some(decoded(trim(&source[end..]), false)),
    )
}
fn unquoted_end(source: &[u16]) -> usize {
    let mut end = 0;
    while end < source.len() && !whitespace(source[end]) {
        end = escaped_end(source, end);
    }
    end
}
fn escaped_end(source: &[u16], position: usize) -> usize {
    if source[position] != 92 || position + 1 == source.len() {
        return position + 1;
    }
    let mut end = position + 1;
    while end < source.len()
        && end < position + 7
        && char::from_u32(u32::from(source[end])).is_some_and(|c| c.is_ascii_hexdigit())
    {
        end += 1;
    }
    if end == position + 1 {
        return (position + 2).min(source.len());
    }
    if source.get(end).is_some_and(|&u| whitespace(u)) {
        if source.get(end) == Some(&13) && source.get(end + 1) == Some(&10) {
            end += 1;
        }
        end += 1;
    }
    end
}
pub(super) fn valid_attribute(inner: &str) -> bool {
    let parsed = attr(&inner.encode_utf16().collect::<Vec<_>>());
    !matches!(
        parsed.m,
        AttrMatch::Units(UnitAttrMatch { op: u16::MAX, .. })
    )
}
pub(super) fn specialized_attribute(inner: &str) -> Option<Attr> {
    let parsed = attr(&inner.encode_utf16().collect::<Vec<_>>());
    match &parsed.m {
        AttrMatch::Units(m)
            if m.insensitive
                || m.sensitive
                || m.op == u16::MAX
                || parsed.name.eq_ignore_ascii_case("type")
                || inner.contains("/*")
                || inner.contains('\\') =>
        {
            Some(parsed)
        }
        _ => None,
    }
}
fn split(units: &[u16], separator: u16) -> Vec<&[u16]> {
    let mut parts = Vec::new();
    let (mut depth, mut quote, mut escaped, mut start) = (0, 0, false, 0);
    for (i, &unit) in units.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if unit == 92 {
            escaped = true;
            continue;
        }
        if quote != 0 {
            if unit == quote {
                quote = 0;
            }
            continue;
        }
        match unit {
            34 | 39 => quote = unit,
            40 | 91 => depth += 1,
            41 | 93 => depth -= 1,
            _ => {}
        }
        if unit == separator && depth == 0 && quote == 0 {
            parts.push(&units[start..i]);
            start = i + 1;
        }
    }
    parts.push(&units[start..]);
    parts
}
fn balanced<'a>(source: &'a [u16], index: &mut usize, open: u16, close: u16) -> &'a [u16] {
    let start = *index;
    let (mut depth, mut quote, mut escaped) = (1, 0, false);
    while *index < source.len() {
        let unit = source[*index];
        if escaped {
            escaped = false;
        } else if unit == 92 {
            escaped = true;
        } else if quote != 0 {
            if unit == quote {
                quote = 0;
            }
        } else if matches!(unit, 34 | 39) {
            quote = unit;
        } else if unit == open {
            depth += 1;
        } else if unit == close {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        *index += 1;
    }
    let end = *index;
    if *index < source.len() {
        *index += 1;
    }
    &source[start..end]
}
fn ident(source: &[u16], index: &mut usize) -> String {
    let start = *index;
    while *index < source.len() {
        let unit = source[*index];
        if unit == 92 {
            *index += 1;
            let mut hex = 0;
            while *index < source.len()
                && hex < 6
                && char::from_u32(u32::from(source[*index])).is_some_and(|c| c.is_ascii_hexdigit())
            {
                *index += 1;
                hex += 1;
            }
            if hex != 0 {
                if *index < source.len() && whitespace(source[*index]) {
                    if source[*index] == 13 && source.get(*index + 1) == Some(&10) {
                        *index += 1;
                    }
                    *index += 1;
                }
            } else if *index < source.len() {
                *index += 1;
            }
            continue;
        }
        if whitespace(unit) || matches!(unit, 35 | 46 | 58 | 91 | 93 | 40 | 41 | 62 | 43 | 126 | 44)
        {
            break;
        }
        *index += 1;
    }
    identifier(&source[start..*index])
}
fn pseudo(name: &str, arg: Option<&[u16]>) -> Pseudo {
    let Some(arg) = arg else {
        return parse_pseudo(name, None);
    };
    if name.eq_ignore_ascii_case("state") {
        return state_argument(arg).map_or(Pseudo::Unknown, Pseudo::State);
    }
    match name {
        "is" => Pseudo::Is(list(arg)),
        "where" => Pseudo::Where(list(arg)),
        "not" => Pseudo::Not(list(arg)),
        // `An+B of S`: S keeps its original units (escapes decoded by `list`).
        "nth-child" | "nth-last-child" if nth_of_position(arg).is_some() => {
            let at = nth_of_position(arg).unwrap_or_default();
            let (a, b) = parse_nth(&String::from_utf16_lossy(&arg[..at]));
            let of = list(&arg[at + 2..]);
            if name == "nth-child" {
                Pseudo::NthChildOf(a, b, of)
            } else {
                Pseudo::NthLastChildOf(a, b, of)
            }
        }
        "has" => Pseudo::Has(
            split(arg, 44)
                .into_iter()
                .filter_map(|part| {
                    let part = trim(part);
                    let (leading, rest) = match part.first() {
                        Some(62) => (Combinator::Child, &part[1..]),
                        Some(43) => (Combinator::Adjacent, &part[1..]),
                        Some(126) => (Combinator::GeneralSibling, &part[1..]),
                        _ => (Combinator::Descendant, part),
                    };
                    complex(rest).map(|complex| RelativeSelector {
                        combinator: leading,
                        complex,
                    })
                })
                .collect(),
        ),
        _ => parse_pseudo(name, Some(&String::from_utf16_lossy(arg))),
    }
}
fn complex(source: &[u16]) -> Option<Complex> {
    let mut index = 0;
    let mut first = None;
    let mut rest = Vec::new();
    let mut combinator = Combinator::Descendant;
    while index < source.len() {
        if whitespace(source[index]) {
            index += 1;
            continue;
        }
        match source[index] {
            62 => {
                combinator = Combinator::Child;
                index += 1;
                continue;
            }
            43 => {
                combinator = Combinator::Adjacent;
                index += 1;
                continue;
            }
            126 => {
                combinator = Combinator::GeneralSibling;
                index += 1;
                continue;
            }
            _ => {}
        }
        let begin = index;
        let mut compound = Compound::default();
        if !matches!(source[index], 35 | 46 | 58 | 91) {
            let start = index;
            let tag = ident(source, &mut index);
            if let Some((namespace, name)) = namespaced_type(&source[start..index]) {
                compound.namespace = namespace;
                compound.tag = name;
            } else if tag != "*" && !tag.is_empty() {
                compound.tag = Some(tag);
            }
        }
        while index < source.len() {
            match source[index] {
                35 => {
                    index += 1;
                    compound.id = Some(ident(source, &mut index));
                }
                46 => {
                    index += 1;
                    compound.classes.push(ident(source, &mut index));
                }
                91 => {
                    index += 1;
                    compound
                        .attrs
                        .push(attr(balanced(source, &mut index, 91, 93)));
                }
                58 => {
                    index += 1;
                    let name = ident(source, &mut index);
                    let arg = if source.get(index) == Some(&40) {
                        index += 1;
                        Some(balanced(source, &mut index, 40, 41))
                    } else {
                        None
                    };
                    compound.pseudos.push(pseudo(&name, arg));
                }
                _ => break,
            }
        }
        if index == begin {
            index += 1;
            continue;
        }
        if first.is_none() {
            first = Some(compound);
        } else {
            rest.push((combinator, compound));
        }
        combinator = Combinator::Descendant;
    }
    first.map(|first| Complex { first, rest })
}
/// A type selector's raw units split at an unescaped namespace bar: `*|p`, `|p`, `*|*`.
/// `None` when there is no bar (an escaped `\|` is part of the name). Only `*` (any) and
/// the empty prefix (no namespace) are declared for DOM selector APIs.
fn namespaced_type(raw: &[u16]) -> Option<(TypeNamespace, Option<String>)> {
    let mut i = 0;
    let bar = loop {
        match raw.get(i) {
            None => return None,
            Some(92) => i = escaped_end(raw, i),
            Some(124) => break i,
            Some(_) => i += 1,
        }
    };
    let (prefix, name) = (&raw[..bar], &raw[bar + 1..]);
    let namespace = match prefix {
        _ if name.is_empty() => TypeNamespace::Invalid,
        [42] => TypeNamespace::Any,
        [] => TypeNamespace::Null,
        _ => TypeNamespace::Invalid,
    };
    let name = (name != [42]).then(|| identifier(name));
    Some((namespace, name))
}
pub(super) fn list(source: &[u16]) -> Vec<Complex> {
    split(source, 44).into_iter().filter_map(complex).collect()
}
impl Tree {
    /// Original-unit selector path; its parsed records are not cached under a
    /// lossy scalar key. Ordinary valid UTF-8 calls retain the existing fast API.
    pub fn frozen_query_selector_all_units(&self, source: &[u16]) -> std::rc::Rc<[Handle]> {
        self.frozen_query_selector_all_from_units(self.root(), source)
    }
    pub fn frozen_query_selector_all_from_units(
        &self,
        root: Handle,
        source: &[u16],
    ) -> std::rc::Rc<[Handle]> {
        let selectors = list(source);
        let mut result = Vec::new();
        let mut stack = self.children(root);
        stack.reverse();
        while let Some(node) = stack.pop() {
            if self.node_type(node) == NodeType::Element
                && selectors
                    .iter()
                    .any(|selector| self.frozen_matches_complex(node, selector))
            {
                result.push(node);
            }
            let begin = stack.len();
            self.for_each_child(node, |child| stack.push(child));
            stack[begin..].reverse();
        }
        result.into()
    }
    pub fn frozen_query_selector_units(&self, source: &[u16]) -> Option<Handle> {
        self.frozen_query_selector_all_units(source).first().copied()
    }
    pub fn frozen_matches_units(&self, node: Handle, source: &[u16]) -> bool {
        list(source)
            .iter()
            .any(|selector| self.frozen_matches_complex(node, selector))
    }
    pub fn frozen_get_element_by_id_units(&self, id: &[u16]) -> Option<Handle> {
        let mut stack = vec![self.root()];
        while let Some(node) = stack.pop() {
            if self.node_type(node) == NodeType::Element
                && self.get_attribute_ns_utf16(node, None, "id").as_deref() == Some(id)
            {
                return Some(node);
            }
            let begin = stack.len();
            self.for_each_child(node, |child| stack.push(child));
            stack[begin..].reverse();
        }
        None
    }
}

fn skip_state_trivia(source: &[u16], mut index: usize) -> Option<usize> {
    loop {
        while source.get(index).is_some_and(|&unit| whitespace(unit)) {
            index += 1;
        }
        if source.get(index..index + 2) != Some(&[47, 42]) {
            return Some(index);
        }
        index += 2;
        while source.get(index..index + 2) != Some(&[42, 47]) {
            if index >= source.len() {
                return None;
            }
            index += 1;
        }
        index += 2;
    }
}
fn state_name_start(unit: u16) -> bool {
    unit >= 128 || unit == 95 || (65..=90).contains(&unit) || (97..=122).contains(&unit)
}
fn state_escape_end(source: &[u16], mut index: usize) -> Option<usize> {
    index += 1;
    let next = *source.get(index)?;
    if matches!(next, 10 | 12 | 13) {
        return None;
    }
    let start = index;
    while index < start + 6
        && source.get(index).is_some_and(|&unit| {
            char::from_u32(u32::from(unit)).is_some_and(|c| c.is_ascii_hexdigit())
        })
    {
        index += 1;
    }
    if index == start {
        return Some(index + 1);
    }
    if source.get(index).is_some_and(|&unit| whitespace(unit)) {
        if source.get(index..index + 2) == Some(&[13, 10]) {
            index += 1;
        }
        index += 1;
    }
    Some(index)
}
/// `:state()` accepts one CSS ident, including CSS-wide words. CSS preprocessing
/// replaces NUL/unpaired surrogates independently of exact DOM state membership.
pub(super) fn state_argument(units: &[u16]) -> Option<Vec<u16>> {
    let normalized = String::from_utf16_lossy(units).replace('\0', "\u{fffd}");
    let source: Vec<u16> = normalized.encode_utf16().collect();
    let start = skip_state_trivia(&source, 0)?;
    let first = *source.get(start)?;
    let valid_start = state_name_start(first)
        || first == 92 && state_escape_end(&source, start).is_some()
        || first == 45
            && source.get(start + 1).is_some_and(|&next| {
                next == 45
                    || state_name_start(next)
                    || next == 92 && state_escape_end(&source, start + 1).is_some()
            });
    if !valid_start {
        return None;
    }
    let mut index = start;
    while let Some(&unit) = source.get(index) {
        if state_name_start(unit) || unit == 45 || (48..=57).contains(&unit) {
            index += 1;
        } else if unit == 92 {
            index = state_escape_end(&source, index)?;
        } else {
            break;
        }
    }
    if skip_state_trivia(&source, index)? != source.len() {
        return None;
    }
    Some(decoded(&source[start..index], false))
}

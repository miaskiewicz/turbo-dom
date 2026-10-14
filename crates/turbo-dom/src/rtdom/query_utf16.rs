//! Parse original DOM string units into native selector records. Identifiers use
//! CSS replacement rules; raw quoted attribute strings preserve their units.
use super::{
    parse_pseudo, Attr, AttrMatch, Combinator, Complex, Compound, Handle, NodeType, Pseudo,
    RelativeSelector, Tree,
};
#[derive(Debug, Clone)]
pub(super) struct UnitAttrMatch {
    op: u16,
    value: Vec<u16>,
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
        }
    }

    pub(super) fn matches(&self, got: &[u16]) -> bool {
        let value = &self.value;
        match self.op {
            0 => true,
            61 => got == value,
            94 => !value.is_empty() && got.starts_with(value),
            36 => !value.is_empty() && got.ends_with(value),
            42 => !value.is_empty() && got.windows(value.len()).any(|part| part == value),
            126 => has_class_units(got, value),
            124 => {
                got == value
                    || got
                        .strip_prefix(value.as_slice())
                        .is_some_and(|tail| tail.first() == Some(&45))
            }
            _ => false,
        }
    }
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
fn attr(inner: &[u16]) -> Attr {
    let Some(eq) = inner.iter().position(|&u| u == 61) else {
        return Attr {
            name: identifier(trim(inner)),
            m: AttrMatch::Units(UnitAttrMatch {
                op: 0,
                value: Vec::new(),
            }),
        };
    };
    let raw = trim(&inner[..eq]);
    let operator = raw
        .last()
        .copied()
        .filter(|u| matches!(u, 126 | 124 | 94 | 36 | 42));
    let name = operator.map_or(raw, |_| &raw[..raw.len() - 1]);
    let value = trim(&inner[eq + 1..]);
    let quoted = value.len() >= 2 && matches!(value[0], 34 | 39) && value.first() == value.last();
    let value = if quoted {
        &value[1..value.len() - 1]
    } else {
        value
    };
    Attr {
        name: identifier(trim(name)),
        m: AttrMatch::Units(UnitAttrMatch {
            op: operator.unwrap_or(61),
            value: decoded(value, quoted),
        }),
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
    match name {
        "is" => Pseudo::Is(list(arg)),
        "where" => Pseudo::Where(list(arg)),
        "not" => Pseudo::Not(list(arg)),
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
            let tag = ident(source, &mut index);
            if tag != "*" && !tag.is_empty() {
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
fn list(source: &[u16]) -> Vec<Complex> {
    split(source, 44).into_iter().filter_map(complex).collect()
}
impl Tree {
    /// Original-unit selector path; its parsed records are not cached under a
    /// lossy scalar key. Ordinary valid UTF-8 calls retain the existing fast API.
    pub fn query_selector_all_units(&self, source: &[u16]) -> std::rc::Rc<[Handle]> {
        self.query_selector_all_from_units(self.root(), source)
    }
    pub fn query_selector_all_from_units(
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
                    .any(|selector| self.matches_complex(node, selector))
            {
                result.push(node);
            }
            let begin = stack.len();
            self.for_each_child(node, |child| stack.push(child));
            stack[begin..].reverse();
        }
        result.into()
    }
    pub fn query_selector_units(&self, source: &[u16]) -> Option<Handle> {
        self.query_selector_all_units(source).first().copied()
    }
    pub fn matches_units(&self, node: Handle, source: &[u16]) -> bool {
        list(source)
            .iter()
            .any(|selector| self.matches_complex(node, selector))
    }
    pub fn get_element_by_id_units(&self, id: &[u16]) -> Option<Handle> {
        let mut stack = vec![self.root()];
        while let Some(node) = stack.pop() {
            if self.node_type(node) == NodeType::Element
                && self.get_attribute_utf16(node, "id").as_deref() == Some(id)
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

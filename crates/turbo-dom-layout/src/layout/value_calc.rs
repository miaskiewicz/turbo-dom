//! Typed affine calc arithmetic. Numbers scale lengths; incompatible dimensions
//! and non-finite results invalidate the entire declaration.
use super::{length_pct_from_parts, parse_raw, raw_to_px, LengthPct, RawLength};

#[derive(Clone, Copy)]
struct Value {
    pct: f32,
    px: f32,
    number: bool,
}

impl Value {
    fn scale(self, factor: f32) -> Self {
        Self {
            pct: self.pct * factor,
            px: self.px * factor,
            ..self
        }
    }

    fn combine(self, rhs: Self, op: u8) -> Option<Self> {
        match op {
            b'+' | b'-' if self.number == rhs.number => {
                let sign = if op == b'+' { 1.0 } else { -1.0 };
                Some(Self {
                    pct: self.pct + sign * rhs.pct,
                    px: self.px + sign * rhs.px,
                    ..self
                })
            }
            b'*' if rhs.number => Some(self.scale(rhs.px)),
            b'*' if self.number => Some(rhs.scale(self.px)),
            b'/' if rhs.number && rhs.px != 0.0 => Some(self.scale(1.0 / rhs.px)),
            _ => None,
        }
    }
}

struct Parser<'a> {
    source: &'a str,
    offset: usize,
    font_size: f32,
}

impl Parser<'_> {
    fn whitespace(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset += 1;
        }
    }

    fn expression(&mut self, min_precedence: u8, depth: u8) -> Option<Value> {
        let mut value = self.atom(depth)?;
        loop {
            let before_space = self.offset;
            self.whitespace();
            let Some(&op) = self.source.as_bytes().get(self.offset) else {
                break;
            };
            let precedence = match op {
                b'+' | b'-' => 1,
                b'*' | b'/' => 2,
                _ => 0,
            };
            if precedence < min_precedence {
                self.offset = before_space;
                break;
            }
            if precedence == 1 {
                self.sum_spacing(before_space)?;
            }
            self.offset += 1;
            let rhs = self.expression(precedence + 1, depth)?;
            value = value.combine(rhs, op)?;
        }
        Some(value)
    }

    fn sum_spacing(&self, before_space: usize) -> Option<()> {
        (self.offset > before_space
            && self
                .source
                .as_bytes()
                .get(self.offset + 1)
                .is_some_and(u8::is_ascii_whitespace))
        .then_some(())
    }

    fn atom(&mut self, depth: u8) -> Option<Value> {
        if depth >= 64 {
            return None;
        }
        self.whitespace();
        let tail = self.source.get(self.offset..)?;
        if tail
            .get(..5)
            .is_some_and(|s| s.eq_ignore_ascii_case("calc("))
        {
            self.offset += 4;
        }
        if self.source.as_bytes().get(self.offset) == Some(&b'(') {
            self.offset += 1;
            let value = self.expression(1, depth + 1)?;
            self.whitespace();
            if self.source.as_bytes().get(self.offset) != Some(&b')') {
                return None;
            }
            self.offset += 1;
            return Some(value);
        }
        self.numeric()
    }

    fn numeric(&mut self) -> Option<Value> {
        let start = self.offset;
        let bytes = self.source.as_bytes();
        if matches!(bytes.get(self.offset), Some(b'+' | b'-')) {
            self.offset += 1;
        }
        while bytes
            .get(self.offset)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
        {
            self.offset += 1;
        }
        self.exponent();
        let number_end = self.offset;
        while bytes
            .get(self.offset)
            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'%')
        {
            self.offset += 1;
        }
        let token = &self.source[start..self.offset];
        if self.offset == number_end {
            return Some(Value {
                pct: 0.0,
                px: token.parse().ok()?,
                number: true,
            });
        }
        let (pct, px) = match parse_raw(token)? {
            RawLength::Pct(pct) => (pct, 0.0),
            raw => (0.0, raw_to_px(raw, self.font_size, 0.0)),
        };
        Some(Value {
            pct,
            px,
            number: false,
        })
    }

    fn exponent(&mut self) {
        let bytes = self.source.as_bytes();
        if !matches!(bytes.get(self.offset), Some(b'e' | b'E')) {
            return;
        }
        let mut end = self.offset + 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end > start {
            self.offset = end;
        }
    }
}

pub(super) fn parse(source: &str, font_size: f32) -> Option<LengthPct> {
    let source = source.trim();
    if !source.get(..5)?.eq_ignore_ascii_case("calc(") {
        return None;
    }
    let mut parser = Parser {
        source,
        offset: 0,
        font_size,
    };
    let value = parser.expression(1, 0)?;
    parser.whitespace();
    if parser.offset != source.len() || value.number {
        return None;
    }
    if !value.pct.is_finite() || !value.px.is_finite() {
        return None;
    }
    Some(length_pct_from_parts(value.pct, value.px))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mui_arithmetic_and_precedence() {
        let value = parse("calc(100% * 4 / 12 - (12 - 4) * (32px / 12))", 16.0).unwrap();
        assert!((value.resolve(1152.0).unwrap() - 362.6667).abs() < 0.001);
        assert_eq!(
            parse("calc(2 * (3px + calc(4px / 2)))", 16.0),
            Some(LengthPct::Px(10.0))
        );
        assert_eq!(parse("calc(1e+2px / 2)", 16.0), Some(LengthPct::Px(50.0)));
        assert_eq!(parse("CaLc(2em * -2)", 10.0), Some(LengthPct::Px(-40.0)));
    }

    #[test]
    fn invalid_arithmetic_is_not_partially_accepted() {
        for source in [
            "calc(10px +)",
            "calc(10px 2px)",
            "calc(10px+ 2px)",
            "calc(10px +2px)",
            "calc(10px * 2px)",
            "calc(10px / 0)",
            "calc(1 / 2px)",
            "calc(1px + 2)",
            "calc(1e39px)",
            "calc(2 * 3)",
            "calc((1px)",
            "calc()",
            "calc(1px) extra",
        ] {
            assert!(parse(source, 16.0).is_none(), "{source}");
        }
        assert!(parse(
            &format!("calc({}1px{})", "(".repeat(65), ")".repeat(65)),
            16.0
        )
        .is_none());
    }
}

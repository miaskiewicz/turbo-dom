//! Component values of a shorthand's declaration value: the top-level tokens (a function or
//! a block counts as one component), each with Chrome 156's specified-value spelling where it
//! is context free (numbers, dimensions, strings, `url()`, function arguments), and the value
//! types the shorthand grammars test them against.
use super::super::color::canonicalize_color;
use super::super::css_values::css_number;
use cssparser::{Parser, ParserInput, Token};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Kind {
    /// An identifier, as authored.
    Ident(String),
    Number {
        value: f32,
        integer: bool,
    },
    /// The percentage (50 for `50%`).
    Percentage(f32),
    Dimension {
        value: f32,
        unit: String,
    },
    Hash(String),
    Str(String),
    Url(String),
    /// A function (lowercase name) or a `(` / `[` block (name `(` / `[`).
    Function(String),
    Comma,
    Slash,
    Delim(char),
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Comp {
    pub(super) kind: Kind,
    /// The context-free canonical spelling.
    pub(super) text: String,
}

const LENGTH_UNITS: &[&str] = &[
    "px", "em", "rem", "ex", "rex", "ch", "rch", "ic", "ric", "cap", "rcap", "lh", "rlh", "vw",
    "vh", "vi", "vb", "vmin", "vmax", "svw", "svh", "svi", "svb", "svmin", "svmax", "lvw", "lvh",
    "lvi", "lvb", "lvmin", "lvmax", "dvw", "dvh", "dvi", "dvb", "dvmin", "dvmax", "cqw", "cqh",
    "cqi", "cqb", "cqmin", "cqmax", "cm", "mm", "q", "in", "pt", "pc",
];
const MATH_FUNCTIONS: &[&str] = &[
    "calc",
    "min",
    "max",
    "clamp",
    "round",
    "mod",
    "rem",
    "abs",
    "sign",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "pow",
    "sqrt",
    "hypot",
    "log",
    "exp",
    "calc-size",
];
const IMAGE_FUNCTIONS: &[&str] = &[
    "linear-gradient",
    "radial-gradient",
    "conic-gradient",
    "repeating-linear-gradient",
    "repeating-radial-gradient",
    "repeating-conic-gradient",
    "-webkit-linear-gradient",
    "-webkit-radial-gradient",
    "-webkit-repeating-linear-gradient",
    "-webkit-repeating-radial-gradient",
    "-webkit-gradient",
    "image-set",
    "-webkit-image-set",
    "cross-fade",
    "-webkit-cross-fade",
    "paint",
    "image",
];
const COLOR_FUNCTIONS: &[&str] = &[
    "rgb",
    "rgba",
    "hsl",
    "hsla",
    "hwb",
    "lab",
    "lch",
    "oklab",
    "oklch",
    "color",
    "color-mix",
    "light-dark",
    "contrast-color",
    "-webkit-device-cmyk",
];
const SYSTEM_COLORS: &[&str] = &[
    "currentcolor",
    "transparent",
    "accentcolor",
    "accentcolortext",
    "activetext",
    "buttonborder",
    "buttonface",
    "buttontext",
    "canvas",
    "canvastext",
    "field",
    "fieldtext",
    "graytext",
    "highlight",
    "highlighttext",
    "linktext",
    "mark",
    "marktext",
    "selecteditem",
    "selecteditemtext",
    "visitedtext",
    "activeborder",
    "activecaption",
    "appworkspace",
    "background",
    "buttonhighlight",
    "buttonshadow",
    "captiontext",
    "inactiveborder",
    "inactivecaption",
    "inactivecaptiontext",
    "infobackground",
    "infotext",
    "menu",
    "menutext",
    "scrollbar",
    "threeddarkshadow",
    "threedface",
    "threedhighlight",
    "threedlightshadow",
    "threedshadow",
    "window",
    "windowframe",
    "windowtext",
    "-webkit-link",
    "-webkit-focus-ring-color",
    "-internal-quirk-inherit",
];

/// CSS-wide keywords, lowercase.
pub(super) const CSS_WIDE: &[&str] = &["initial", "inherit", "unset", "revert", "revert-layer"];

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\a "),
            _ => out.push(character),
        }
    }
    out.push('"');
    out
}

fn dimension_text(value: f32, unit: &str) -> String {
    format!("{}{}", css_number(value), unit.to_ascii_lowercase())
}

/// Blink keeps the authored percentage digits; the tokenizer pre-divides by 100.
fn percent_of(token: &str, unit_value: f32) -> f32 {
    token
        .trim()
        .trim_end_matches('%')
        .parse()
        .unwrap_or_else(|_| (f64::from(unit_value) * 100.0) as f32)
}

fn read(input: &mut Parser<'_, '_>, out: &mut Vec<Comp>) -> Option<()> {
    loop {
        let start = input.position();
        let Ok(token) = input.next() else {
            return Some(());
        };
        let comp = match token.clone() {
            Token::Ident(name) => Comp {
                text: name.to_string(),
                kind: Kind::Ident(name.to_string()),
            },
            Token::Number {
                value, int_value, ..
            } => Comp {
                text: css_number(value),
                kind: Kind::Number {
                    value,
                    integer: int_value.is_some(),
                },
            },
            Token::Percentage { unit_value, .. } => {
                let percent = percent_of(input.slice_from(start), unit_value);
                Comp {
                    text: format!("{}%", css_number(percent)),
                    kind: Kind::Percentage(percent),
                }
            }
            Token::Dimension { value, unit, .. } => Comp {
                text: dimension_text(value, &unit),
                kind: Kind::Dimension {
                    value,
                    unit: unit.to_ascii_lowercase(),
                },
            },
            Token::Hash(value) | Token::IDHash(value) => Comp {
                text: format!("#{value}"),
                kind: Kind::Hash(value.to_string()),
            },
            Token::QuotedString(value) => Comp {
                text: quoted(&value),
                kind: Kind::Str(value.to_string()),
            },
            Token::UnquotedUrl(value) => Comp {
                text: format!("url({})", quoted(&value)),
                kind: Kind::Url(value.to_string()),
            },
            Token::Comma => Comp {
                text: ",".into(),
                kind: Kind::Comma,
            },
            Token::Delim('/') => Comp {
                text: "/".into(),
                kind: Kind::Slash,
            },
            Token::Delim(character) if character != '!' => Comp {
                text: character.to_string(),
                kind: Kind::Delim(character),
            },
            Token::Function(name) => {
                let name = name.to_ascii_lowercase();
                let inner = input
                    .parse_nested_block(|input| {
                        let mut inner = Vec::new();
                        read(input, &mut inner)
                            .ok_or_else(|| input.new_custom_error::<(), ()>(()))?;
                        Ok(inner)
                    })
                    .ok()?;
                if name == "url" {
                    let [Comp {
                        kind: Kind::Str(url),
                        ..
                    }] = inner.as_slice()
                    else {
                        return None;
                    };
                    Comp {
                        text: format!("url({})", quoted(url)),
                        kind: Kind::Url(url.clone()),
                    }
                } else {
                    let text = (name == "calc").then(|| calc_sum(&inner)).flatten();
                    Comp {
                        text: text.unwrap_or_else(|| format!("{name}({})", join(&inner))),
                        kind: Kind::Function(name),
                    }
                }
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock => {
                let (open, close) = if matches!(token, Token::ParenthesisBlock) {
                    ("(", ")")
                } else {
                    ("[", "]")
                };
                let inner = input
                    .parse_nested_block(|input| {
                        let mut inner = Vec::new();
                        read(input, &mut inner)
                            .ok_or_else(|| input.new_custom_error::<(), ()>(()))?;
                        Ok(inner)
                    })
                    .ok()?;
                Comp {
                    text: format!("{open}{}{close}", join(&inner)),
                    kind: Kind::Function(open.into()),
                }
            }
            Token::BadUrl(_)
            | Token::BadString(_)
            | Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket
            | Token::Semicolon => return None,
            _ => Comp {
                text: input.slice_from(start).trim().to_string(),
                kind: Kind::Other,
            },
        };
        out.push(comp);
    }
}

/// A flat `calc()` sum (`a + b - c` of numbers, percentages and dimensions) the way Blink
/// simplifies it: like units combined, numbers first, then percentages, then units in ASCII
/// order. `None` for anything else.
fn calc_sum(inner: &[Comp]) -> Option<String> {
    let mut terms: Vec<(String, f64)> = Vec::new();
    let mut sign = 1.0;
    for (index, comp) in inner.iter().enumerate() {
        if index % 2 == 1 {
            sign = match comp.kind {
                Kind::Delim('+') => 1.0,
                Kind::Delim('-') => -1.0,
                _ => return None,
            };
            continue;
        }
        let (unit, value) = match &comp.kind {
            Kind::Number { value, .. } => (String::new(), f64::from(*value)),
            Kind::Percentage(value) => ("%".to_string(), f64::from(*value)),
            Kind::Dimension { value, unit } => (unit.clone(), f64::from(*value)),
            _ => return None,
        };
        match terms.iter_mut().find(|(existing, _)| *existing == unit) {
            Some(term) => term.1 += sign * value,
            None => terms.push((unit, sign * value)),
        }
    }
    if inner.len().is_multiple_of(2) {
        return None;
    }
    let rank = |unit: &str| match unit {
        "" => 0,
        "%" => 1,
        _ => 2,
    };
    terms.sort_by(|a, b| rank(&a.0).cmp(&rank(&b.0)).then(a.0.cmp(&b.0)));
    let mut out = String::from("calc(");
    for (index, (unit, value)) in terms.iter().enumerate() {
        let number = |value: f64| css_number(value as f32);
        if index == 0 {
            out.push_str(&number(*value));
        } else if *value < 0.0 {
            out.push_str(" - ");
            out.push_str(&number(-value));
        } else {
            out.push_str(" + ");
            out.push_str(&number(*value));
        }
        out.push_str(unit);
    }
    out.push(')');
    Some(out)
}

/// Join components with single spaces, commas as `, `.
pub(super) fn join(comps: &[Comp]) -> String {
    let mut out = String::new();
    for (index, comp) in comps.iter().enumerate() {
        if comp.kind == Kind::Comma {
            out.push(',');
            continue;
        }
        if index > 0 {
            out.push(' ');
        }
        out.push_str(&comp.text);
    }
    out
}

/// Tokenize a declaration value into components; `None` for unbalanced or bad input.
pub(super) fn components(value: &str) -> Option<Vec<Comp>> {
    let mut source = ParserInput::new(value);
    let mut input = Parser::new(&mut source);
    let mut out = Vec::new();
    read(&mut input, &mut out)?;
    Some(out)
}

impl Comp {
    pub(super) fn ident(&self) -> Option<String> {
        match &self.kind {
            Kind::Ident(name) => Some(name.to_ascii_lowercase()),
            _ => None,
        }
    }
    pub(super) fn is_ident(&self, names: &[&str]) -> bool {
        self.ident()
            .is_some_and(|name| names.contains(&name.as_str()))
    }
    pub(super) fn keyword(&self, names: &[&str]) -> Option<String> {
        self.ident().filter(|name| names.contains(&name.as_str()))
    }
    pub(super) fn is_comma(&self) -> bool {
        self.kind == Kind::Comma
    }
    pub(super) fn is_slash(&self) -> bool {
        self.kind == Kind::Slash
    }
    fn function(&self) -> Option<&str> {
        match &self.kind {
            Kind::Function(name) => Some(name),
            _ => None,
        }
    }
    fn is_math(&self) -> bool {
        self.function()
            .is_some_and(|name| MATH_FUNCTIONS.contains(&name))
    }
    /// A math function whose arguments carry a unit `pick` accepts (`None`: unitless).
    fn math_of(&self, accepts: fn(&str) -> bool, percent: bool, number: bool) -> bool {
        if !self.is_math() {
            return false;
        }
        let Some(inner) = self
            .text
            .find('(')
            .and_then(|at| components(&self.text[at + 1..self.text.len() - 1]))
        else {
            return false;
        };
        let mut found = false;
        let mut bad = false;
        visit(&inner, &mut |comp| match &comp.kind {
            Kind::Dimension { unit, .. } => {
                if accepts(unit) {
                    found = true;
                } else {
                    bad = true;
                }
            }
            Kind::Percentage(_) => {
                if percent {
                    found = true;
                } else {
                    bad = true;
                }
            }
            _ => {}
        });
        !bad && (found || number)
    }
    /// `<length>` (a unitless zero is `0px`).
    pub(super) fn length(&self, negative: bool) -> Option<String> {
        match &self.kind {
            Kind::Dimension { value, unit } if LENGTH_UNITS.contains(&unit.as_str()) => {
                (negative || *value >= 0.0).then(|| self.text.clone())
            }
            Kind::Number { value, .. } if *value == 0.0 => Some("0px".into()),
            _ if self.math_of(|unit| LENGTH_UNITS.contains(&unit), false, false) => {
                Some(self.text.clone())
            }
            _ => None,
        }
    }
    pub(super) fn percentage(&self, negative: bool) -> Option<String> {
        match &self.kind {
            Kind::Percentage(value) => (negative || *value >= 0.0).then(|| self.text.clone()),
            _ => None,
        }
    }
    /// `<length-percentage>`.
    pub(super) fn length_percentage(&self, negative: bool) -> Option<String> {
        if self.math_of(|unit| LENGTH_UNITS.contains(&unit), true, false) {
            return Some(self.text.clone());
        }
        self.length(negative).or_else(|| self.percentage(negative))
    }
    pub(super) fn number(&self, negative: bool) -> Option<String> {
        match &self.kind {
            Kind::Number { value, .. } => (negative || *value >= 0.0).then(|| self.text.clone()),
            _ if self.math_of(|_| false, false, true) => Some(self.text.clone()),
            _ => None,
        }
    }
    pub(super) fn integer(&self, minimum: i64) -> Option<String> {
        match &self.kind {
            Kind::Number {
                value,
                integer: true,
            } if f64::from(*value) >= minimum as f64 => Some(self.text.clone()),
            _ => None,
        }
    }
    pub(super) fn time(&self, negative: bool) -> Option<String> {
        match &self.kind {
            Kind::Dimension { value, unit } if unit == "s" || unit == "ms" => {
                (negative || *value >= 0.0).then(|| self.text.clone())
            }
            _ if self.math_of(|unit| unit == "s" || unit == "ms", false, false) => {
                Some(self.text.clone())
            }
            _ => None,
        }
    }
    pub(super) fn angle(&self) -> Option<String> {
        match &self.kind {
            Kind::Dimension { unit, .. }
                if matches!(unit.as_str(), "deg" | "grad" | "rad" | "turn") =>
            {
                Some(self.text.clone())
            }
            Kind::Number { value, .. } if *value == 0.0 => Some("0deg".into()),
            _ if self.math_of(
                |unit| matches!(unit, "deg" | "grad" | "rad" | "turn"),
                false,
                false,
            ) =>
            {
                Some(self.text.clone())
            }
            _ => None,
        }
    }
    /// `<color>`: hex and `rgb()` / `hsl()` serialize as `rgb()` / `rgba()`, keywords lowercase.
    pub(super) fn color(&self) -> Option<String> {
        match &self.kind {
            Kind::Hash(value) => (matches!(value.len(), 3 | 4 | 6 | 8)
                && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| canonicalize_color(&self.text, false))
            .flatten(),
            Kind::Ident(_) => {
                let name = self.ident()?;
                (SYSTEM_COLORS.contains(&name.as_str())
                    || canonicalize_color(&name, true).is_some())
                .then_some(name)
            }
            Kind::Function(name) if COLOR_FUNCTIONS.contains(&name.as_str()) => {
                Some(canonicalize_color(&self.text, false).unwrap_or_else(|| self.text.clone()))
            }
            _ => None,
        }
    }
    pub(super) fn url(&self) -> Option<String> {
        matches!(self.kind, Kind::Url(_)).then(|| self.text.clone())
    }
    /// `<image>` (not `none`).
    pub(super) fn image(&self) -> Option<String> {
        self.url().or_else(|| {
            self.function()
                .filter(|name| IMAGE_FUNCTIONS.contains(name))
                .map(|_| self.text.clone())
        })
    }
    pub(super) fn string(&self) -> Option<String> {
        matches!(self.kind, Kind::Str(_)).then(|| self.text.clone())
    }
    /// `<custom-ident>` excluding the CSS-wide keywords, `default` and `excluded`.
    pub(super) fn custom_ident(&self, excluded: &[&str]) -> Option<String> {
        let Kind::Ident(name) = &self.kind else {
            return None;
        };
        let lower = name.to_ascii_lowercase();
        (!CSS_WIDE.contains(&lower.as_str())
            && lower != "default"
            && !excluded.contains(&lower.as_str()))
        .then(|| name.clone())
    }
    pub(super) fn dashed_ident(&self) -> Option<String> {
        match &self.kind {
            Kind::Ident(name) if name.starts_with("--") && name.len() > 2 => Some(name.clone()),
            _ => None,
        }
    }
    pub(super) fn function_named(&self, names: &[&str]) -> Option<String> {
        self.function()
            .filter(|name| names.contains(name))
            .map(|_| self.text.clone())
    }
}

fn visit(comps: &[Comp], callback: &mut dyn FnMut(&Comp)) {
    for comp in comps {
        callback(comp);
        if let Kind::Function(_) = comp.kind {
            if let Some(inner) = comp
                .text
                .find(['(', '['])
                .and_then(|at| components(&comp.text[at + 1..comp.text.len() - 1]))
            {
                visit(&inner, callback);
            }
        }
    }
}

/// Whether the raw value holds a `var()` / `env()` / `attr()` reference anywhere.
pub(super) fn has_substitution(value: &str) -> bool {
    fn scan(input: &mut Parser<'_, '_>) -> bool {
        while let Ok(token) = input.next_including_whitespace_and_comments() {
            match token.clone() {
                Token::Function(name) => {
                    let lower = name.to_ascii_lowercase();
                    if matches!(lower.as_str(), "var" | "env" | "attr" | "if" | "inherit") {
                        return true;
                    }
                    if input
                        .parse_nested_block(|input| {
                            Ok::<_, cssparser::ParseError<'_, ()>>(scan(input))
                        })
                        .unwrap_or(false)
                    {
                        return true;
                    }
                }
                Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock
                    if input
                        .parse_nested_block(|input| {
                            Ok::<_, cssparser::ParseError<'_, ()>>(scan(input))
                        })
                        .unwrap_or(false) =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    }
    let mut source = ParserInput::new(value);
    let mut input = Parser::new(&mut source);
    scan(&mut input)
}

/// The value normalized the way Chrome keeps a pending-substitution shorthand: tokens joined
/// as authored, runs of whitespace collapsed.
pub(super) fn substitution_text(value: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for character in value.trim().chars() {
        if character.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(character);
    }
    out
}

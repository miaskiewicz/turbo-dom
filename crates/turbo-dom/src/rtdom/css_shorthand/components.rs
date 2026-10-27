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

/// Functions whose identifier arguments are names (kept as written).
const NAMING_FUNCTIONS: &[&str] = &[
    "stylistic",
    "styleset",
    "character-variant",
    "swash",
    "ornaments",
    "annotation",
    "local",
    "format",
    "attr",
    "counter",
    "counters",
    "symbols",
    "src",
    "var",
    "env",
];

/// A `calc()` component that computes to a plain number.
fn calc_number(comp: &Comp) -> Option<f64> {
    match &comp.kind {
        Kind::Function(name) if name == "calc" && comp.text.starts_with("calc(") => {
            match sum_terms(&components(&comp.text[5..comp.text.len() - 1])?)?.as_slice() {
                [(unit, value)] if unit.is_empty() => Some(*value),
                _ => None,
            }
        }
        _ => None,
    }
}

/// CSS-wide keywords, lowercase.
pub(super) const CSS_WIDE: &[&str] = &["initial", "inherit", "unset", "revert", "revert-layer"];

pub(super) fn quoted(text: &str) -> String {
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
                    let mut inner = inner;
                    if !NAMING_FUNCTIONS.contains(&name.as_str()) {
                        // Keyword arguments are case-insensitive: Chrome writes them lowercase.
                        for comp in &mut inner {
                            if let Kind::Ident(ident) = &comp.kind {
                                if !ident.starts_with("--") {
                                    comp.text = ident.to_ascii_lowercase();
                                }
                            }
                        }
                    }
                    if name == "cubic-bezier" {
                        // A `calc()` argument that computes to a number is that number.
                        for comp in &mut inner {
                            if let Some(value) = calc_number(comp) {
                                comp.text = css_number(value as f32);
                                comp.kind = Kind::Number {
                                    value: value as f32,
                                    integer: false,
                                };
                            }
                        }
                    }
                    if !function_valid(&name, &inner) {
                        return None;
                    }
                    let text = match name.as_str() {
                        "calc" => calc_sum(&inner),
                        "min" | "max" | "clamp" => min_max(&name, &inner),
                        "rgb" | "rgba" | "hsl" | "hsla" => color_function(&name, &inner),
                        "lab" | "lch" | "oklab" | "oklch" => Some(lab_function(&name, &inner)),
                        "steps" => steps_text(&inner),
                        "linear" => linear_text(&inner),
                        _ => None,
                    };
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
            | Token::CurlyBracketBlock
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
    sum_terms(inner).map(calc_text)
}

type Terms = Vec<(String, f64)>;

/// The like-unit terms of a sum of products (operands: literals, the constants, nested
/// parentheses / `calc()`, one-unit `min()` / `max()` / `clamp()`), absolute lengths combined
/// in px; `None` when Chrome would keep the expression unsimplified.
fn sum_terms(inner: &[Comp]) -> Option<Vec<(String, f64)>> {
    // Operands alternate with operators (`function_valid` / `calc_type` checked every sum
    // before it gets here); products bind first.
    if inner.len().is_multiple_of(2) {
        unreachable!("a checked calc() sum has an odd number of components");
    }
    let nested = |comp: &Comp, open: usize| components(&comp.text[open..comp.text.len() - 1]);
    let operand = |comp: &Comp| -> Option<Terms> {
        Some(match &comp.kind {
            Kind::Number { value, .. } => vec![(String::new(), f64::from(*value))],
            Kind::Percentage(value) => vec![("%".to_string(), f64::from(*value))],
            Kind::Dimension { value, unit } => vec![(unit.clone(), f64::from(*value))],
            // `calc_type` admits only the constants as identifiers: the last one is `nan`.
            Kind::Ident(name) => {
                let value = match name.to_ascii_lowercase().as_str() {
                    "e" => std::f64::consts::E,
                    "pi" => std::f64::consts::PI,
                    "infinity" => f64::INFINITY,
                    "-infinity" => f64::NEG_INFINITY,
                    _ => f64::NAN,
                };
                vec![(String::new(), value)]
            }
            Kind::Function(name) if name == "(" => sum_terms(&nested(comp, 1)?)?,
            // A nested `calc()` / `min()` / `max()` / `clamp()` Chrome simplified already
            // spells `calc(...)`.
            Kind::Function(name)
                if matches!(name.as_str(), "calc" | "min" | "max" | "clamp")
                    && comp.text.starts_with("calc(") =>
            {
                sum_terms(&nested(comp, 5)?)?
            }
            _ => return None,
        })
    };
    let scalar = |terms: &Terms| match terms.as_slice() {
        [(unit, value)] if unit.is_empty() => Some(*value),
        _ => None,
    };
    let scale = |terms: Terms, factor: f64| -> Terms {
        terms
            .into_iter()
            .map(|(unit, value)| (unit, value * factor))
            .collect()
    };
    let mut terms: Terms = Vec::new();
    let push = |terms: &mut Terms, product: Terms, sign: f64| {
        for (unit, value) in product {
            match terms.iter_mut().find(|(existing, _)| *existing == unit) {
                Some(term) => term.1 += sign * value,
                None => terms.push((unit, sign * value)),
            }
        }
    };
    let mut sign = 1.0;
    let mut current = operand(&inner[0])?;
    let mut index = 1;
    while index < inner.len() {
        let next = operand(&inner[index + 1])?;
        match inner[index].kind {
            Kind::Delim('*') => {
                current = match (scalar(&current), scalar(&next)) {
                    (_, Some(factor)) => scale(current, factor),
                    (Some(factor), None) => scale(next, factor),
                    _ => return None,
                };
            }
            Kind::Slash | Kind::Delim('/') => current = scale(current, 1.0 / scalar(&next)?),
            Kind::Delim(operator @ ('+' | '-')) => {
                push(&mut terms, current, sign);
                sign = if operator == '+' { 1.0 } else { -1.0 };
                current = next;
            }
            _ => unreachable!("a checked calc() sum has only * / + - operators"),
        }
        index += 2;
    }
    push(&mut terms, current, sign);
    // Absolute lengths combine in px.
    let absolute = |unit: &str| match unit {
        "in" => Some(96.0),
        "cm" => Some(96.0 / 2.54),
        "mm" => Some(96.0 / 25.4),
        "q" => Some(96.0 / 101.6),
        "pt" => Some(96.0 / 72.0),
        "pc" => Some(16.0),
        _ => None,
    };
    if terms.iter().any(|(unit, _)| absolute(unit).is_some()) {
        let mut px = 0.0;
        let mut rest = Vec::new();
        for (unit, value) in terms {
            match absolute(&unit) {
                Some(factor) => px += value * factor,
                None if unit == "px" => px += value,
                None => rest.push((unit, value)),
            }
        }
        rest.push(("px".to_string(), px));
        terms = rest;
    }
    // Times, angles and frequencies in two units combine in their canonical unit.
    for (canonical, units) in [
        ("s", &[("s", 1.0), ("ms", 0.001)][..]),
        (
            "deg",
            &[
                ("deg", 1.0),
                ("grad", 0.9),
                ("rad", 180.0 / std::f64::consts::PI),
                ("turn", 360.0),
            ][..],
        ),
        ("hz", &[("hz", 1.0), ("khz", 1000.0)][..]),
    ] {
        let factor = |unit: &str| {
            units
                .iter()
                .find(|(name, _)| *name == unit)
                .map(|(_, f)| *f)
        };
        if terms
            .iter()
            .filter(|(unit, _)| factor(unit).is_some())
            .count()
            > 1
        {
            let mut total = 0.0;
            terms.retain(|(unit, value)| match factor(unit) {
                Some(f) => {
                    total += value * f;
                    false
                }
                None => true,
            });
            terms.push((canonical.to_string(), total));
        }
    }
    Some(terms)
}

/// A `calc()` operand or sum's type: a number, a dimension of a category (percentages fit
/// any dimension), or unknown (a nested math function).
#[derive(Clone, Copy, PartialEq)]
enum CalcType {
    Number,
    Percent,
    Unit(u8),
    Unknown,
}

fn unit_category(unit: &str) -> Option<u8> {
    if LENGTH_UNITS.contains(&unit) {
        return Some(0);
    }
    match unit {
        "s" | "ms" => Some(1),
        "deg" | "grad" | "rad" | "turn" => Some(2),
        "hz" | "khz" => Some(3),
        "dpi" | "dpcm" | "dppx" | "x" => Some(4),
        _ => None,
    }
}

/// The type of a `calc()` sum, `None` when Chrome rejects it: a sum mixes numbers and
/// dimensions or two dimension categories, a product multiplies two dimensions, or a divisor
/// has a unit.
fn calc_type(inner: &[Comp]) -> Option<CalcType> {
    let operand = |comp: &Comp| match &comp.kind {
        Kind::Number { .. } => Some(CalcType::Number),
        Kind::Percentage(_) => Some(CalcType::Percent),
        Kind::Dimension { unit, .. } => unit_category(unit).map(CalcType::Unit),
        Kind::Function(name) if name == "(" => {
            calc_type(&components(&comp.text[1..comp.text.len() - 1])?)
        }
        Kind::Function(name) if MATH_FUNCTIONS.contains(&name.as_str()) => Some(CalcType::Unknown),
        Kind::Ident(name)
            if matches!(
                name.to_ascii_lowercase().as_str(),
                "e" | "pi" | "infinity" | "-infinity" | "nan"
            ) =>
        {
            Some(CalcType::Number)
        }
        _ => None,
    };
    let unit = |ty: CalcType| matches!(ty, CalcType::Percent | CalcType::Unit(_));
    let sum = |a: CalcType, b: CalcType| match (a, b) {
        (CalcType::Unknown, other) | (other, CalcType::Unknown) => Some(other),
        (CalcType::Number, CalcType::Number) => Some(CalcType::Number),
        (CalcType::Percent, other) | (other, CalcType::Percent) if unit(other) => Some(other),
        (CalcType::Unit(x), CalcType::Unit(y)) if x == y => Some(a),
        _ => None,
    };
    let (first, rest) = inner.split_first()?;
    let mut total: Option<CalcType> = None;
    let mut product = operand(first)?;
    for pair in rest.chunks(2) {
        let [operator, next] = pair else {
            return None;
        };
        let next = operand(next)?;
        match operator.kind {
            Kind::Delim('*') => {
                product = match (product, next) {
                    (a, b) if unit(a) && unit(b) => return None,
                    // At most one side carries a unit: the product has its type.
                    (CalcType::Number | CalcType::Unknown, other) | (other, _) => other,
                };
            }
            Kind::Slash | Kind::Delim('/') => {
                if unit(next) {
                    return None;
                }
            }
            Kind::Delim('+' | '-') => {
                total = Some(match total {
                    None => product,
                    Some(total) => sum(total, product)?,
                });
                product = next;
            }
            _ => return None,
        }
    }
    match total {
        None => Some(product),
        Some(total) => sum(total, product),
    }
}

/// The comma-separated arguments of a function.
fn arguments(inner: &[Comp]) -> Vec<&[Comp]> {
    inner.split(|comp| comp.kind == Kind::Comma).collect()
}

/// Structural checks Chrome makes before keeping a function: a `calc()` sum alternates
/// operands and operators (a literal product has at most one unit), `rgb()` / `hsl()` use
/// commas everywhere or nowhere, and `linear()` has at least two stops.
fn function_valid(name: &str, inner: &[Comp]) -> bool {
    match name {
        "calc" => {
            if inner.is_empty() || inner.len().is_multiple_of(2) {
                return false;
            }
            let unit =
                |comp: &Comp| matches!(comp.kind, Kind::Dimension { .. } | Kind::Percentage(_));
            inner.iter().enumerate().all(|(index, comp)| {
                if index % 2 == 1 {
                    let product = matches!(comp.kind, Kind::Delim('*'));
                    let operator =
                        product || matches!(comp.kind, Kind::Delim('+' | '-' | '/') | Kind::Slash);
                    operator && !(product && unit(&inner[index - 1]) && unit(&inner[index + 1]))
                } else {
                    !matches!(comp.kind, Kind::Comma | Kind::Slash | Kind::Delim(_))
                }
            }) && calc_type(inner).is_some()
        }
        "min" | "max" => arguments(inner).iter().all(|argument| !argument.is_empty()),
        "clamp" => {
            let arguments = arguments(inner);
            arguments.len() == 3 && arguments.iter().all(|argument| !argument.is_empty())
        }
        "rgb" | "rgba" | "hsl" | "hsla" => {
            let commas = inner.iter().filter(|comp| comp.kind == Kind::Comma).count();
            if inner.first().is_some_and(|comp| comp.is_ident(&["from"])) {
                // Relative color syntax.
                true
            } else if commas > 0 {
                // The legacy syntax: three or four comma-separated values.
                (commas == 2 || commas == 3)
                    && inner.len() == 2 * commas + 1
                    && inner
                        .iter()
                        .enumerate()
                        .all(|(index, comp)| (comp.kind == Kind::Comma) == (index % 2 == 1))
            } else {
                // The modern syntax: three values, then `/ alpha`.
                match inner.iter().position(|comp| comp.kind == Kind::Slash) {
                    Some(slash) => slash == 3 && inner.len() == 5,
                    None => inner.len() == 3,
                }
            }
        }
        // Each stop is an output and at most two input positions.
        "linear" => {
            let output = |comp: &Comp| matches!(comp.kind, Kind::Number { .. }) || comp.is_math();
            let position = |comp: &Comp| matches!(comp.kind, Kind::Percentage(_)) || comp.is_math();
            inner.iter().any(|comp| comp.kind == Kind::Comma)
                && arguments(inner).iter().all(|stop| {
                    (1..=3).contains(&stop.len())
                        && output(&stop[0])
                        && stop[1..].iter().all(position)
                })
        }
        "steps" => {
            // The step count: an integer literal, or a math function Chrome checks later.
            let count = |comp: &Comp, minimum: f32| match comp.kind {
                Kind::Number {
                    value,
                    integer: true,
                } => value >= minimum,
                _ => comp.is_math(),
            };
            match inner {
                [steps] => count(steps, 1.0),
                [steps, comma, position] if comma.kind == Kind::Comma => {
                    let Some(position) = position.keyword(&[
                        "start",
                        "end",
                        "jump-start",
                        "jump-end",
                        "jump-none",
                        "jump-both",
                    ]) else {
                        return false;
                    };
                    let minimum = if position == "jump-none" { 2.0 } else { 1.0 };
                    count(steps, minimum)
                }
                _ => false,
            }
        }
        // The x coordinates are in [0, 1].
        "cubic-bezier" => {
            let arguments = arguments(inner);
            arguments.len() == 4
                && [0, 2].iter().all(|&index| match arguments[index] {
                    [Comp {
                        kind: Kind::Number { value, .. },
                        ..
                    }] => (0.0..=1.0).contains(value),
                    [_] => true,
                    _ => false,
                })
        }
        _ => true,
    }
}

/// `calc()` of simplified terms: numbers, then percentages, then units in ASCII order.
fn calc_text(mut terms: Vec<(String, f64)>) -> String {
    let rank = |unit: &str| match unit {
        "" => 0,
        "%" => 1,
        _ => 2,
    };
    terms.sort_by(|a, b| rank(&a.0).cmp(&rank(&b.0)).then(a.0.cmp(&b.0)));
    if let [(unit, value)] = terms.as_slice() {
        if !value.is_finite() {
            // `calc(1px / 0)` is `calc(infinity * 1px)`.
            let infinity = if value.is_nan() {
                "NaN"
            } else if *value < 0.0 {
                "-infinity"
            } else {
                "infinity"
            };
            let unit = if unit.is_empty() {
                String::new()
            } else {
                format!(" * 1{unit}")
            };
            return format!("calc({infinity}{unit})");
        }
    }
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
    out
}

/// `min()` / `max()` / `clamp()` of literals in one unit: their `calc()` value.
fn min_max(name: &str, inner: &[Comp]) -> Option<String> {
    let mut unit = None;
    let mut values = Vec::new();
    for (index, comp) in inner.iter().enumerate() {
        if index % 2 == 1 {
            if comp.kind != Kind::Comma {
                return None;
            }
            continue;
        }
        let (this, value) = match &comp.kind {
            Kind::Number { value, .. } => (String::new(), f64::from(*value)),
            Kind::Percentage(value) => ("%".to_string(), f64::from(*value)),
            Kind::Dimension { value, unit } => (unit.clone(), f64::from(*value)),
            _ => return None,
        };
        if unit.get_or_insert_with(|| this.clone()) != &this {
            return None;
        }
        values.push(value);
    }
    let value = match (name, values.as_slice()) {
        ("min", [_, ..]) => values.iter().copied().fold(f64::INFINITY, f64::min),
        ("max", [_, ..]) => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ("clamp", [low, value, high]) => value.min(*high).max(*low),
        _ => unreachable!("function_valid admits min() / max() with arguments, clamp() with three"),
    };
    Some(calc_text(vec![(unit?, value)]))
}

/// Blink's alpha spelling: the shortest of two or three decimals naming the same 8-bit alpha.
fn alpha_text(alpha: f64) -> String {
    let alpha = alpha.clamp(0.0, 1.0);
    let byte = (alpha * 255.0).round();
    let two = (alpha * 100.0).round() / 100.0;
    let value = if (two * 255.0).round() == byte {
        two
    } else {
        (alpha * 1000.0).round() / 1000.0
    };
    css_number(value as f32)
}

fn rgb_text(channels: [f64; 3], alpha: f64) -> String {
    // Channels computed from hsl() land a hair off a half: round at 1e-9 first.
    let [r, g, b] =
        channels.map(|channel| ((channel * 1e9).round() / 1e9).round().clamp(0.0, 255.0) as i64);
    if alpha >= 1.0 {
        format!("rgb({r}, {g}, {b})")
    } else {
        format!("rgba({r}, {g}, {b}, {})", alpha_text(alpha))
    }
}

/// `#rgb[a]` / `#rrggbb[aa]` as Chrome serializes them.
fn hex_color(hex: &str) -> Option<String> {
    let digits: Vec<f64> = hex
        .chars()
        .map(|c| c.to_digit(16).map(f64::from))
        .collect::<Option<_>>()?;
    let pairs: Vec<f64> = match digits.len() {
        3 | 4 => digits.iter().map(|d| d * 17.0).collect(),
        6 | 8 => digits
            .chunks(2)
            .map(|pair| pair[0] * 16.0 + pair[1])
            .collect(),
        _ => return None,
    };
    let alpha = pairs.get(3).map_or(1.0, |a| a / 255.0);
    Some(rgb_text([pairs[0], pairs[1], pairs[2]], alpha))
}

/// `rgb()` / `hsl()` in the legacy (commas) or modern (spaces, `/ alpha`) syntax.
fn color_function(name: &str, inner: &[Comp]) -> Option<String> {
    let mut parts: Vec<&Comp> = Vec::new();
    let mut alpha_part = None;
    let commas = inner.iter().any(|comp| comp.kind == Kind::Comma);
    let mut slash = false;
    for (index, comp) in inner.iter().enumerate() {
        match comp.kind {
            Kind::Comma if commas && index % 2 == 1 => {}
            Kind::Slash if !commas && !slash => slash = true,
            _ if commas && index % 2 == 1 => return None,
            _ if slash => {
                if alpha_part.replace(comp).is_some() {
                    return None;
                }
            }
            _ => parts.push(comp),
        }
    }
    if commas && parts.len() == 4 {
        alpha_part = parts.pop();
    }
    if parts.len() != 3 || (slash && alpha_part.is_none()) {
        return None;
    }
    // A channel: a literal, `none` (zero) or a `calc()` that computes to one.
    let computed = |comp: &Comp, unit: &str| match &comp.kind {
        Kind::Function(name) if name == "calc" => {
            match sum_terms(&components(&comp.text[5..comp.text.len() - 1])?)?.as_slice() {
                [(found, value)] if found == unit => Some(*value),
                _ => None,
            }
        }
        _ if comp.is_ident(&["none"]) => Some(0.0),
        _ => None,
    };
    let number = |comp: &Comp| match comp.kind {
        Kind::Number { value, .. } => Some(f64::from(value)),
        _ => computed(comp, ""),
    };
    let percent = |comp: &Comp| match comp.kind {
        Kind::Percentage(value) => Some(f64::from(value)),
        _ => computed(comp, "%"),
    };
    let alpha = match alpha_part {
        None => 1.0,
        Some(comp) => number(comp).or_else(|| percent(comp).map(|p| p / 100.0))?,
    };
    let channels = if name.starts_with("rgb") {
        let channel = |comp: &Comp| number(comp).or_else(|| percent(comp).map(|p| p * 2.55));
        [channel(parts[0])?, channel(parts[1])?, channel(parts[2])?]
    } else {
        let hue = match &parts[0].kind {
            Kind::Number { value, .. } => f64::from(*value),
            _ if parts[0].is_ident(&["none"]) => 0.0,
            Kind::Dimension { value, unit } => {
                f64::from(*value)
                    * match unit.as_str() {
                        "deg" => 1.0,
                        "grad" => 0.9,
                        "rad" => 180.0 / std::f64::consts::PI,
                        "turn" => 360.0,
                        _ => return None,
                    }
            }
            _ => return None,
        };
        let (saturation, lightness) = (percent(parts[1])? / 100.0, percent(parts[2])? / 100.0);
        let hue = hue.rem_euclid(360.0) / 360.0;
        let q = if lightness < 0.5 {
            lightness * (1.0 + saturation)
        } else {
            lightness + saturation - lightness * saturation
        };
        let p = 2.0 * lightness - q;
        let channel = |t: f64| {
            let t = t.rem_euclid(1.0);
            255.0
                * if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 0.5 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                }
        };
        [
            channel(hue + 1.0 / 3.0),
            channel(hue),
            channel(hue - 1.0 / 3.0),
        ]
    };
    Some(rgb_text(channels, alpha))
}

/// `lab()` / `lch()` / `oklab()` / `oklch()`: a percentage lightness spelled as a number.
fn lab_function(name: &str, inner: &[Comp]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (index, comp) in inner.iter().enumerate() {
        let text = match (index, &comp.kind) {
            (0, Kind::Percentage(value)) => {
                let scale = if name.starts_with("ok") { 100.0 } else { 1.0 };
                css_number((f64::from(*value) / scale) as f32)
            }
            _ => comp.text.clone(),
        };
        parts.push(text);
    }
    format!("{name}({})", parts.join(" "))
}

/// `steps(n, end | jump-end)` is `steps(n)`; the position keyword lowercase.
fn steps_text(inner: &[Comp]) -> Option<String> {
    match inner {
        [count, comma, position] if comma.kind == Kind::Comma => {
            let position = position.ident()?;
            Some(if matches!(position.as_str(), "end" | "jump-end") {
                format!("steps({})", count.text)
            } else {
                format!("steps({}, {position})", count.text)
            })
        }
        _ => None,
    }
}

/// `linear()` with every stop's input position filled in (first 0%, last 100%, the rest
/// spread evenly between their known neighbours).
fn linear_text(inner: &[Comp]) -> Option<String> {
    let mut stops: Vec<(String, Vec<Option<f64>>)> = Vec::new();
    for item in inner.split(|comp| comp.kind == Kind::Comma) {
        let (output, positions) = item.split_first()?;
        if !matches!(output.kind, Kind::Number { .. }) || positions.len() > 2 {
            return None;
        }
        let positions = positions
            .iter()
            .map(|comp| match comp.kind {
                Kind::Percentage(value) => Some(Some(f64::from(value))),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        if positions.is_empty() {
            stops.push((output.text.clone(), vec![None]));
        } else {
            // A stop with two input positions is two stops.
            for position in positions {
                stops.push((output.text.clone(), vec![position]));
            }
        }
    }
    if stops.len() < 2 {
        unreachable!("function_valid admits linear() with a comma: two stops or more");
    }
    let mut flat: Vec<Option<f64>> = stops
        .iter()
        .flat_map(|(_, positions)| positions.iter().copied())
        .collect();
    let last = flat.len() - 1;
    flat[0].get_or_insert(0.0);
    flat[last].get_or_insert(100.0);
    // A position below the largest before it is that largest.
    let mut largest = f64::NEG_INFINITY;
    for position in flat.iter_mut().flatten() {
        largest = largest.max(*position);
        *position = largest;
    }
    let mut at = 0;
    while at < flat.len() {
        if flat[at].is_none() {
            let end = (at..flat.len()).find(|&index| flat[index].is_some())?;
            let (low, high) = (flat[at - 1]?, flat[end]?);
            let gaps = (end - at + 1) as f64;
            for (offset, index) in (at..end).enumerate() {
                flat[index] = Some(low + (high - low) * (offset as f64 + 1.0) / gaps);
            }
            at = end;
        }
        at += 1;
    }
    let mut out = Vec::new();
    let mut index = 0;
    for (output, positions) in &stops {
        let texts: Vec<String> = positions
            .iter()
            .map(|_| {
                let value = flat[index].unwrap_or(0.0);
                index += 1;
                format!("{}%", css_number(value as f32))
            })
            .collect();
        out.push(format!("{output} {}", texts.join(" ")));
    }
    Some(format!("linear({})", out.join(", ")))
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
            unreachable!("a math function's text is its tokenized arguments re-joined");
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
            _ if self.math_of(|_| false, false, true) => Some(self.text.clone()),
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
            Kind::Hash(value) => hex_color(value),
            Kind::Ident(_) => {
                let name = self.ident()?;
                (SYSTEM_COLORS.contains(&name.as_str())
                    || canonicalize_color(&name, true).is_some())
                .then_some(name)
            }
            Kind::Function(name) if COLOR_FUNCTIONS.contains(&name.as_str()) => {
                Some(self.text.clone())
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
    // Reads the whole (nested) input: `parse_nested_block` only keeps a block's result when
    // the closure exhausted it (turbo-dom#115).
    fn scan(input: &mut Parser<'_, '_>) -> bool {
        let mut found = false;
        while let Ok(token) = input.next_including_whitespace_and_comments() {
            let nested = match token.clone() {
                Token::Function(name) => {
                    let lower = name.to_ascii_lowercase();
                    found |= matches!(lower.as_str(), "var" | "env" | "attr" | "if" | "inherit");
                    true
                }
                Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                    true
                }
                _ => false,
            };
            if nested {
                found |= input
                    .parse_nested_block(|input| Ok::<_, cssparser::ParseError<'_, ()>>(scan(input)))
                    .unwrap_or(false);
            }
        }
        found
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

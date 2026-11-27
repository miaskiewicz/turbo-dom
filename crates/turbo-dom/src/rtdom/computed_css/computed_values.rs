//! Computed ordinary values, kept separate from layout-dependent CSSOM used values.
use super::{
    check_component_limits,
    properties::{self, InitialContext, WideKeyword},
    ComputationLimits, ComputedCssError, CssSource, CssText,
};
use crate::rtdom::{
    color,
    css_shorthand::{
        computed_length, computed_length_expression, longhand_value_css, LengthContext,
        LengthError, Spelled,
    },
    css_values::css_number,
};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
pub struct ComputedValues {
    pub values: BTreeMap<String, CssText>,
    pub font_size_px: f64,
    pub root_font_size_px: f64,
}

/// Actual initial preferences and measured unit bases, not authored box widths.
/// Font-size metric units refer to parent/initial fonts; other metric units refer
/// to the element's own computed font. em/rem/% are derived below in both cases.
pub struct ValueContext<'a> {
    pub initial: InitialContext<'a>,
    pub unit_bases: &'a [(&'a str, f64)],
    pub font_size_unit_bases: &'a [(&'a str, f64)],
    pub is_root: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueLimits {
    pub computation: ComputationLimits,
    /// Maximum summed UTF-16 value payloads retained in the returned style.
    /// This is separate from custom-property memoization's total-unit policy.
    pub retained_units: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueError {
    Computation(ComputedCssError),
    Length(LengthError),
    Font(super::font_values::FontValueError),
    RequiresInitial(String),
    InvalidFontContext(String),
    UnsupportedValue(String),
}
impl From<ComputedCssError> for ValueError {
    fn from(error: ComputedCssError) -> Self {
        Self::Computation(error)
    }
}
impl From<LengthError> for ValueError {
    fn from(error: LengthError) -> Self {
        match error {
            LengthError::Computation(error) => Self::Computation(error),
            error => Self::Length(error),
        }
    }
}
impl From<super::font_values::FontValueError> for ValueError {
    fn from(error: super::font_values::FontValueError) -> Self {
        use super::font_values::FontValueError;
        match error {
            FontValueError::Computation(error) => Self::Computation(error),
            FontValueError::InvalidLineHeight => Self::UnsupportedValue("line-height".into()),
            FontValueError::InvalidFontWeight => Self::UnsupportedValue("font-weight".into()),
            error => Self::Font(error),
        }
    }
}
type Result<T> = std::result::Result<T, ValueError>;

enum Value<'a> {
    Borrowed(&'a CssText),
    Owned(CssText),
}
impl Value<'_> {
    fn text(&self) -> &CssText {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value,
        }
    }
    fn into_owned(self) -> CssText {
        match self {
            Self::Borrowed(value) => value.clone(),
            Self::Owned(value) => value,
        }
    }
}
enum Origin<'a> {
    Inherited(&'a CssText),
    Declared(&'a CssText),
    Initial(CssText),
}

struct Output {
    values: BTreeMap<String, CssText>,
    retained: usize,
    limits: ValueLimits,
}
impl Output {
    fn insert(&mut self, name: &str, value: Value<'_>) -> Result<()> {
        let units = value.text().as_units().len();
        if units > self.limits.computation.max_output_units {
            return Err(ComputedCssError::ResourceLimit.into());
        }
        let retained = self
            .retained
            .checked_add(units)
            .filter(|units| *units <= self.limits.retained_units)
            .ok_or(ComputedCssError::ResourceLimit)?;
        // Quotas precede even the token guard's temporary scalar view or a parent
        // payload clone. An old larger parent style cannot evade the new budget.
        check_component_limits(value.text(), self.limits.computation)?;
        self.values.insert(name.to_string(), value.into_owned());
        self.retained = retained;
        Ok(())
    }
}

fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
// All callers first validate through font_number, including finite f32 range.
fn pixels(value: f64) -> CssText {
    text(&format!("{}px", css_number(value as f32)))
}
fn font_number(name: &str, value: f64) -> Result<f64> {
    if !value.is_finite() || !(value as f32).is_finite() || value < 0.0 {
        return Err(ValueError::InvalidFontContext(name.to_string()));
    }
    Ok(value)
}

fn font_size_number(name: &str, value: f64) -> Result<f64> {
    let value = font_number(name, value)?;
    Ok(super::font_values::finalize_font_size_px(value)
        .expect("font_number checked finite nonnegative resolved size"))
}

fn keyword(value: &CssText) -> Option<String> {
    let source = CssSource::new(value.as_units());
    let mut input = cssparser::ParserInput::new(source.scalar());
    let mut input = cssparser::Parser::new(&mut input);
    let ident = input.expect_ident_cloned().ok()?;
    input.expect_exhausted().ok()?;
    Some(ident.to_ascii_lowercase())
}

fn initial(name: &str, context: &ValueContext<'_>) -> Result<CssText> {
    properties::initial(name, &context.initial)
        .ok_or_else(|| ValueError::RequiresInitial(name.to_string()))
}

fn inherited_or_initial<'a>(
    name: &str,
    parent: Option<&'a ComputedValues>,
    context: &ValueContext<'_>,
) -> Result<Origin<'a>> {
    parent
        .and_then(|parent| parent.values.get(name))
        .map_or_else(
            || initial(name, context).map(Origin::Initial),
            |value| Ok(Origin::Inherited(value)),
        )
}

fn wide<'a>(
    name: &str,
    kind: WideKeyword,
    parent: Option<&'a ComputedValues>,
    context: &ValueContext<'_>,
) -> Result<Origin<'a>> {
    match kind {
        WideKeyword::Inherit => inherited_or_initial(name, parent, context),
        WideKeyword::Unset if properties::inherited(name) => {
            inherited_or_initial(name, parent, context)
        }
        WideKeyword::Initial | WideKeyword::Unset => initial(name, context).map(Origin::Initial),
        WideKeyword::Revert | WideKeyword::RevertLayer => {
            Err(ComputedCssError::RequiresCascade.into())
        }
    }
}

fn choose<'a>(
    name: &str,
    selected: Option<&'a Option<CssText>>,
    parent: Option<&'a ComputedValues>,
    context: &ValueContext<'_>,
    limits: ValueLimits,
) -> Result<Option<Origin<'a>>> {
    let Some(selected) = selected else {
        return Ok(parent
            .filter(|_| properties::inherited(name))
            .and_then(|parent| parent.values.get(name))
            .map(Origin::Inherited));
    };
    let Some(value) = selected else {
        return wide(name, WideKeyword::Unset, parent, context).map(Some);
    };
    check_component_limits(value, limits.computation)?;
    match properties::wide_keyword(value) {
        Some(kind) => wide(name, kind, parent, context).map(Some),
        None => Ok(Some(Origin::Declared(value))),
    }
}

fn bases<'a>(
    source: &'a [(&'a str, f64)],
    em: f64,
    rem: f64,
    percent: Option<f64>,
) -> Vec<(&'a str, f64)> {
    let mut bases: Vec<_> = source
        .iter()
        .copied()
        .filter(|(unit, _)| {
            !["em", "rem", "%"]
                .iter()
                .any(|reserved| unit.eq_ignore_ascii_case(reserved))
        })
        .collect();
    bases.extend([("em", em), ("rem", rem)]);
    if let Some(percent) = percent {
        bases.push(("%", percent));
    }
    bases
}

fn initial_font(context: &ValueContext<'_>, limits: ValueLimits) -> Result<f64> {
    let context_units = LengthContext {
        unit_bases: context.font_size_unit_bases,
    };
    let value = computed_length(
        context.initial.font_size,
        &context_units,
        limits.computation,
    )?
    .ok_or_else(|| ValueError::RequiresInitial("font-size".into()))?;
    font_size_number("initial font-size", value)
}

fn root_base(
    parent: Option<&ComputedValues>,
    context: &ValueContext<'_>,
    initial: f64,
    limits: ValueLimits,
) -> Result<f64> {
    if context.is_root {
        return Ok(initial);
    }
    if let Some(parent) = parent {
        return font_size_number("root font-size", parent.root_font_size_px);
    }
    let context_units = LengthContext {
        unit_bases: context.unit_bases,
    };
    let value = computed_length("1rem", &context_units, limits.computation)?
        .ok_or_else(|| ValueError::RequiresInitial("root font-size".into()))?;
    font_size_number("root font-size", value)
}

fn font_inherits(selected: Option<&Option<CssText>>, limits: ValueLimits) -> Result<bool> {
    let Some(Some(value)) = selected else {
        return Ok(properties::inherited("font-size"));
    };
    check_component_limits(value, limits.computation)?;
    Ok(match properties::wide_keyword(value) {
        Some(WideKeyword::Inherit) => true,
        Some(WideKeyword::Unset) => properties::inherited("font-size"),
        _ => false,
    })
}

struct FontState<'a> {
    size: f64,
    value: Option<Value<'a>>,
}
fn inherited_font(parent: &ComputedValues, explicit: bool) -> Result<FontState<'_>> {
    let size = font_size_number("parent font-size", parent.font_size_px)?;
    let value = match parent.values.get("font-size") {
        Some(value) => Some(Value::Borrowed(value)),
        None if explicit => Some(Value::Owned(pixels(size))),
        None => None,
    };
    Ok(FontState { size, value })
}

fn declared_font(
    value: &CssText,
    context: &ValueContext<'_>,
    initial: f64,
    parent_size: f64,
    rem: f64,
    limits: ValueLimits,
) -> Result<f64> {
    if keyword(value).as_deref() == Some("medium") {
        return Ok(initial);
    }
    let scalar = String::from_utf16(value.as_units())
        .map_err(|_| ValueError::UnsupportedValue("font-size".into()))?;
    let derived = bases(
        context.font_size_unit_bases,
        parent_size,
        rem,
        Some(parent_size / 100.0),
    );
    let units = LengthContext {
        unit_bases: &derived,
    };
    let size = computed_length(&scalar, &units, limits.computation)?
        .ok_or_else(|| ValueError::UnsupportedValue("font-size".into()))?;
    font_size_number("font-size", size)
}

fn font_from_origin(
    origin: &Origin<'_>,
    context: &ValueContext<'_>,
    initial: f64,
    parent_size: f64,
    rem: f64,
    limits: ValueLimits,
) -> Result<FontState<'static>> {
    let size = match origin {
        Origin::Declared(value) => {
            declared_font(value, context, initial, parent_size, rem, limits)?
        }
        Origin::Initial(_) => initial,
        // compute_font handles every inherited parent path before choosing origin.
        Origin::Inherited(_) => {
            unreachable!("existing inherited font was handled before selection")
        }
    };
    Ok(FontState {
        size,
        value: Some(Value::Owned(pixels(size))),
    })
}

fn compute_font<'a>(
    parent: Option<&'a ComputedValues>,
    selected: &'a BTreeMap<String, Option<CssText>>,
    context: &ValueContext<'_>,
    initial: f64,
    rem: f64,
    limits: ValueLimits,
) -> Result<FontState<'a>> {
    let own = selected.get("font-size");
    if font_inherits(own, limits)? {
        if let Some(parent) = parent {
            return inherited_font(parent, own.is_some());
        }
    }
    let Some(origin) = choose("font-size", own, parent, context, limits)? else {
        return Ok(FontState {
            size: initial,
            value: None,
        });
    };
    let parent_size = parent.map_or(Ok(initial), |parent| {
        font_size_number("parent font-size", parent.font_size_px)
    })?;
    font_from_origin(&origin, context, initial, parent_size, rem, limits)
}

fn parent_color<'a>(
    parent: Option<&'a ComputedValues>,
    context: &ValueContext<'_>,
) -> Result<Value<'a>> {
    if let Some(color) = parent.and_then(|parent| parent.values.get("color")) {
        return Ok(Value::Borrowed(color));
    }
    let color = initial("color", context)?;
    if keyword(&color).as_deref() == Some("currentcolor") {
        return Err(ValueError::RequiresInitial("color".into()));
    }
    // properties::initial("color") exclusively encodes this same &str. Parent
    // and guest token payloads return above and do not enter the initial branch.
    let scalar = context.initial.color.to_owned();
    Ok(Value::Owned(text(
        &color::canonicalize_color(&scalar, true).unwrap_or(scalar),
    )))
}

fn number_line_height(source: &str) -> Result<Option<String>> {
    super::font_values::unitless_line_height(source)
        .map(|value| value.map(css_number))
        .map_err(ValueError::from)
}
fn context_free<'a>(name: &str, value: &'a CssText) -> Result<Value<'a>> {
    let Ok(scalar) = String::from_utf16(value.as_units()) else {
        return Ok(Value::Borrowed(value));
    };
    match longhand_value_css(name, &scalar) {
        Spelled::Valid(value) => Ok(Value::Owned(text(&value))),
        // The canonical token speller already provides CSSOM string quoting.
        // Use it only for this measured computed-property seam; unrelated legacy
        // unchecked properties keep their original partial/raw contract.
        Spelled::Unchecked(serialized) if name == "content" => Ok(Value::Owned(text(&serialized))),
        Spelled::Unchecked(_) => Ok(Value::Borrowed(value)),
        Spelled::Invalid => Err(ValueError::UnsupportedValue(name.to_string())),
    }
}

fn line_height<'a>(
    value: &'a CssText,
    context: &LengthContext<'_>,
    limits: ValueLimits,
) -> Result<Value<'a>> {
    if keyword(value).is_some() {
        return context_free("line-height", value);
    }
    let scalar = String::from_utf16(value.as_units())
        .map_err(|_| ValueError::UnsupportedValue("line-height".into()))?;
    if let Some(number) = number_line_height(&scalar)? {
        return Ok(Value::Owned(text(&number)));
    }
    let value = computed_length(&scalar, context, limits.computation)?
        .ok_or_else(|| ValueError::UnsupportedValue("line-height".into()))?;
    font_number("line-height", value)?;
    Ok(Value::Owned(pixels(value)))
}

fn length_value<'a>(
    name: &str,
    value: &'a CssText,
    context: &LengthContext<'_>,
    limits: ValueLimits,
) -> Result<Value<'a>> {
    if keyword(value).is_some() {
        return context_free(name, value);
    }
    let scalar = String::from_utf16(value.as_units())
        .map_err(|_| ValueError::UnsupportedValue(name.to_string()))?;
    let value = computed_length_expression(&scalar, context, limits.computation)?
        .ok_or_else(|| ValueError::UnsupportedValue(name.to_string()))?;
    Ok(Value::Owned(text(&value)))
}

fn color_value<'a>(
    name: &str,
    value: &'a CssText,
    parent: Option<&'a ComputedValues>,
    context: &ValueContext<'_>,
) -> Result<Value<'a>> {
    if keyword(value).as_deref() == Some("currentcolor") {
        return if name == "color" {
            parent_color(parent, context)
        } else {
            Ok(Value::Owned(text("currentcolor")))
        };
    }
    if let Ok(scalar) = String::from_utf16(value.as_units()) {
        if let Some(color) = color::canonicalize_color(&scalar, true) {
            return Ok(Value::Owned(text(&color)));
        }
    }
    context_free(name, value)
}

fn font_weight_value(
    value: &CssText,
    parent: Option<&ComputedValues>,
    context: &ValueContext<'_>,
    limits: ValueLimits,
) -> Result<Value<'static>> {
    use super::font_values::{compute_font_weight, FontValueError};
    let inherited = parent.and_then(|parent| parent.values.get("font-weight"));
    let computed = match compute_font_weight(value, inherited, limits.computation) {
        Err(FontValueError::RequiresParentFontWeight) if inherited.is_none() => {
            // A missing sparse computed key has the canonical CSS initial value;
            // explicit/inherited noninitial weights are retained in the map.
            let initial = initial("font-weight", context)?;
            compute_font_weight(value, Some(&initial), limits.computation)?
        }
        value => value?,
    };
    Ok(Value::Owned(computed))
}
fn declared<'a>(
    name: &str,
    value: &'a CssText,
    parent: Option<&'a ComputedValues>,
    context: &ValueContext<'_>,
    own_font: f64,
    root_font: f64,
    limits: ValueLimits,
) -> Result<Value<'a>> {
    if name == "font-weight" {
        return font_weight_value(value, parent, context, limits);
    }
    if name == "line-height" {
        let derived = bases(
            context.unit_bases,
            own_font,
            root_font,
            Some(own_font / 100.0),
        );
        return line_height(
            value,
            &LengthContext {
                unit_bases: &derived,
            },
            limits,
        );
    }
    if properties::length_property(name) {
        let derived = bases(context.unit_bases, own_font, root_font, None);
        return length_value(
            name,
            value,
            &LengthContext {
                unit_bases: &derived,
            },
            limits,
        );
    }
    if color::is_color_prop(name) {
        return color_value(name, value, parent, context);
    }
    context_free(name, value)
}

fn compute_selected<'a>(
    name: &str,
    origin: Origin<'a>,
    parent: Option<&'a ComputedValues>,
    context: &ValueContext<'_>,
    font: f64,
    root_font: f64,
    limits: ValueLimits,
) -> Result<Value<'a>> {
    match origin {
        Origin::Inherited(value) => Ok(Value::Borrowed(value)),
        Origin::Declared(value) => declared(name, value, parent, context, font, root_font, limits),
        Origin::Initial(value) => {
            let computed = declared(name, &value, parent, context, font, root_font, limits)?;
            Ok(Value::Owned(computed.into_owned()))
        }
    }
}

fn insert_selected(
    output: &mut Output,
    parent: Option<&ComputedValues>,
    selected: &BTreeMap<String, Option<CssText>>,
    context: &ValueContext<'_>,
    font: f64,
    root_font: f64,
    limits: ValueLimits,
) -> Result<()> {
    for (name, selection) in selected {
        if name == "font-size" {
            continue;
        }
        if let Some(origin) = choose(name, Some(selection), parent, context, limits)? {
            output.insert(
                name,
                compute_selected(name, origin, parent, context, font, root_font, limits)?,
            )?;
        }
    }
    Ok(())
}

fn insert_inherited(
    output: &mut Output,
    parent: Option<&ComputedValues>,
    selected: &BTreeMap<String, Option<CssText>>,
) -> Result<()> {
    let Some(parent) = parent else { return Ok(()) };
    for (name, value) in &parent.values {
        if name == "font-size" || selected.contains_key(name) || !properties::inherited(name) {
            continue;
        }
        output.insert(name, Value::Borrowed(value))?;
    }
    Ok(())
}

fn output_with_font(value: Option<Value<'_>>, limits: ValueLimits) -> Result<Output> {
    let mut output = Output {
        values: BTreeMap::new(),
        retained: 0,
        limits,
    };
    if let Some(value) = value {
        output.insert("font-size", value)?;
    }
    Ok(output)
}

/// Compute a selected, already-substituted ordinary style. Missing values remain
/// missing except existing known inherited parent keys; no UA default map is built.
///
/// Inputs must already be admitted and substituted by `select_style` (or an
/// equivalent canonical admission stage). This is value computation, not a raw
/// declaration parser; unchecked legacy properties retain their raw contract.
///
/// # Errors
/// Missing actual bases/defaults, unsupported computations, cascade rollback and
/// caller source/output/retained limits stay typed; no budget/default is selected here.
pub fn compute_values(
    parent: Option<&ComputedValues>,
    selected: &BTreeMap<String, Option<CssText>>,
    context: &ValueContext<'_>,
    limits: ValueLimits,
) -> Result<ComputedValues> {
    let initial = initial_font(context, limits)?;
    let rem = root_base(parent, context, initial, limits)?;
    let font = compute_font(parent, selected, context, initial, rem, limits)?;
    let root_font = if context.is_root { font.size } else { rem };
    let mut output = output_with_font(font.value, limits)?;
    insert_selected(
        &mut output,
        parent,
        selected,
        context,
        font.size,
        root_font,
        limits,
    )?;
    insert_inherited(&mut output, parent, selected)?;
    Ok(ComputedValues {
        values: output.values,
        font_size_px: font.size,
        root_font_size_px: root_font,
    })
}

//! Stylesheet (cascade-mode) selectors on the one selector engine (consolidation S3b).
//!
//! [`parse_for_cascade`] parses a rule's selector list with the same grammar as the DOM
//! APIs (`query.rs`, and `query_utf16.rs` for escapes) and adds what a stylesheet needs on
//! top, as Chrome 156 does:
//!
//! - **validity**: a stylesheet selector list is all-or-nothing. An empty item, a dangling
//!   combinator, an unknown pseudo-class or pseudo-element, a pseudo-element anywhere but
//!   at the end of the subject compound, or an invalid argument of the unforgiving `:not()`,
//!   `:has()` and `of S` makes the whole list (so the whole rule) invalid. `:is()` and
//!   `:where()` are forgiving: invalid items are dropped.
//! - **pseudo-elements** (`::before`, `::marker`, legacy `:before`, `::-webkit-*`, …) parse
//!   and never match an element; `::slotted(S)` matches only through a [`Navigator`] that
//!   reports slot assignment (a scaffold: turbo-surf's composed cascade still matches
//!   `:host()` / `::slotted()` with its own scope walk, a follow-up of plan §1.4).
//! - [`parse_for_supports`] is the `selector()` support condition: one complex selector,
//!   parsed unforgivingly (`@supports selector(…)` / `CSS.supports("selector(…)")`).
//! - pseudo-classes Chrome knows but the DOM engine does not evaluate parse as valid and
//!   never match (the static resting state: `:hover`, `:focus`, `:visited`, …). Names are
//!   ASCII case-insensitive. `:scope` is `:root` in a stylesheet (the DOM APIs match the
//!   scoping element: `Navigator::is_scope`); `:-webkit-any()`, `:defined` and `:lang()`
//!   are shared with the DOM APIs (turbo-dom#91, #96).
//! - **specificity** per Selectors 4: `:where()` is 0; `:is()`, `:not()` and `:has()` take
//!   their most specific argument; `:nth-child(An+B of S)` is a pseudo-class plus S;
//!   pseudo-elements count as types.
//!
//! [`match_for_cascade`] matches one selector against an element through any
//! [`Navigator`]; [`PositionCache`] memoizes sibling positions for navigators that style a
//! whole tree (each sibling list is scanned once instead of once per structural pseudo).
use super::{
    matches_complex, parse_complex, split_top_level_commas, units, validate_selector_list, Complex,
    Compound, Handle, Navigator, Pseudo, SelectorError, Tree, UnknownPseudo,
};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// Selector specificity `(ids, classes + attributes + pseudo-classes, types +
/// pseudo-elements)`, compared lexicographically.
pub type Specificity = (u32, u32, u32);

/// A valid stylesheet selector list ([`parse_for_cascade`]).
#[derive(Debug, Clone)]
pub struct SelectorList {
    selectors: Vec<CascadeSelector>,
}

impl SelectorList {
    /// The complex selectors, in source order.
    #[must_use]
    pub fn selectors(&self) -> &[CascadeSelector] {
        &self.selectors
    }
}

/// One complex selector of a stylesheet selector list.
#[derive(Debug, Clone)]
pub struct CascadeSelector {
    text: String,
    complex: Complex,
    pseudo_element: Option<PseudoElement>,
    specificity: Specificity,
}

#[derive(Debug, Clone)]
enum PseudoElement {
    /// Generated content and the other pseudo-elements: never an element.
    Inert(String),
    /// `::slotted(S)`: an element assigned to a slot the selector's compound matches.
    Slotted(Vec<Complex>),
}

impl CascadeSelector {
    /// The selector's source text (trimmed).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Its specificity (Selectors 4).
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        self.specificity
    }

    /// The pseudo-element it targets (lowercase name, e.g. `before`), if any.
    #[must_use]
    pub fn pseudo_element(&self) -> Option<&str> {
        match &self.pseudo_element {
            Some(PseudoElement::Inert(name)) => Some(name),
            Some(PseudoElement::Slotted(_)) => Some("slotted"),
            None => None,
        }
    }
}

/// Pseudo-classes Chrome 156 accepts that the engine does not evaluate: valid in a
/// stylesheet, never matching (a static document's resting state). Functional ones take
/// an argument.
const NEVER_PSEUDO_CLASSES: &[&str] = &[
    "active",
    "active-view-transition",
    "autofill",
    "-webkit-autofill",
    "buffering",
    "current",
    "default",
    "focus",
    "focus-visible",
    "focus-within",
    "future",
    "host",
    "hover",
    "in-range",
    "indeterminate",
    "invalid",
    "modal",
    "muted",
    "open",
    "out-of-range",
    "past",
    "paused",
    "picture-in-picture",
    "placeholder-shown",
    "playing",
    "popover-open",
    "seeking",
    "stalled",
    "target",
    "user-invalid",
    "user-valid",
    "valid",
    "visited",
    "volume-locked",
    "xr-overlay",
];
const NEVER_FUNCTIONAL_PSEUDO_CLASSES: &[&str] = &["host", "host-context"];

/// Pseudo-elements Chrome 156 accepts without an argument (any `::-webkit-*` name is
/// accepted too).
const PSEUDO_ELEMENTS: &[&str] = &[
    "after",
    "backdrop",
    "before",
    "checkmark",
    "column",
    "cue",
    "details-content",
    "file-selector-button",
    "first-letter",
    "first-line",
    "grammar-error",
    "marker",
    "picker-icon",
    "placeholder",
    "scroll-marker",
    "scroll-marker-group",
    "search-text",
    "selection",
    "spelling-error",
    "target-text",
    "view-transition",
];
const FUNCTIONAL_PSEUDO_ELEMENTS: &[&str] = &[
    "cue",
    "highlight",
    "part",
    "picker",
    "scroll-button",
    "slotted",
    "view-transition-group",
    "view-transition-image-pair",
    "view-transition-new",
    "view-transition-old",
];
/// Pseudo-classes Chrome allows after a `::-webkit-*` (scrollbar) pseudo-element: user
/// action and the scrollbar states. Such a selector never matches an element anyway.
const AFTER_PSEUDO_ELEMENT: &[&str] = &[
    "active",
    "corner-present",
    "decrement",
    "disabled",
    "double-button",
    "enabled",
    "end",
    "focus",
    "focus-visible",
    "focus-within",
    "horizontal",
    "hover",
    "increment",
    "no-button",
    "single-button",
    "start",
    "vertical",
    "window-inactive",
];

/// User-action pseudo-classes: what Chrome 156 allows after an element-backed
/// pseudo-element (`::file-selector-button:hover`, `::details-content:focus`).
const USER_ACTION: &[&str] = &["active", "focus", "focus-visible", "focus-within", "hover"];
/// Pseudo-elements backed by a real element (user-action pseudo-classes may follow).
const ELEMENT_BACKED: &[&str] = &[
    "details-content",
    "file-selector-button",
    "picker",
    "scroll-button",
    "scroll-marker",
    "scroll-marker-group",
];

/// May pseudo-class `p` follow pseudo-element `element` (lowercase name)? Chrome 156: the
/// scrollbar pseudo-elements take their states, `::selection` only `:window-inactive`,
/// `::part()` any non-structural pseudo-class, element-backed ones user action, and the
/// rest (`::before`, `::marker`, `::placeholder`, `::slotted()`, …) nothing.
fn allowed_after_pseudo_element(element: &str, p: &Pseudo) -> bool {
    let plain = |list: &[&str]| matches!(p, Pseudo::Unknown(u) if u.arg.is_none() && list.contains(&u.name.to_ascii_lowercase().as_str()));
    if element.starts_with("-webkit-") {
        return plain(AFTER_PSEUDO_ELEMENT) || matches!(p, Pseudo::Disabled | Pseudo::Enabled);
    }
    match element {
        "selection" => plain(&["window-inactive"]),
        "part" => {
            plain(NEVER_PSEUDO_CLASSES)
                || matches!(
                    p,
                    Pseudo::Checked
                        | Pseudo::Disabled
                        | Pseudo::Enabled
                        | Pseudo::Required
                        | Pseudo::Optional
                        | Pseudo::ReadOnly
                        | Pseudo::ReadWrite
                        | Pseudo::Link
                        | Pseudo::State(_)
                        | Pseudo::Fullscreen
                )
        }
        _ if ELEMENT_BACKED.contains(&element) => plain(USER_ACTION),
        // the view-transition tree's named pseudo-elements (not the `::view-transition`
        // root) take `:only-child`
        "view-transition-group"
        | "view-transition-image-pair"
        | "view-transition-old"
        | "view-transition-new" => matches!(p, Pseudo::OnlyChild),
        _ => false,
    }
}

/// The CSS2 pseudo-elements that also parse with a single colon.
const LEGACY_PSEUDO_ELEMENTS: &[&str] = &["after", "before", "first-letter", "first-line"];

fn invalid(reason: &'static str) -> SelectorError {
    SelectorError::Invalid(reason)
}

/// Parse a style rule's selector list (the prelude) for the cascade: the DOM engine's
/// grammar plus stylesheet validity, pseudo-elements and specificity (see the module
/// docs).
///
/// # Errors
/// Returns [`SelectorError`] when Chrome would drop the rule: a malformed selector, an
/// empty list item, a dangling combinator, or an unknown / misplaced pseudo.
pub fn parse_for_cascade(src: &str) -> Result<SelectorList, SelectorError> {
    let src = super::source::normalize_str(src, super::source::Mode::Sheet);
    let selectors = split_top_level_commas(&src)
        .into_iter()
        .map(|item| parse_item(item.trim()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SelectorList { selectors })
}

fn parse_item(item: &str) -> Result<CascadeSelector, SelectorError> {
    let complex = parse_complex_strict(item)?;
    let (complex, pseudo_element) = finish_subject(complex)?;
    let specificity = complex_specificity(&complex)
        + pseudo_element
            .as_ref()
            .map_or(Spec::default(), pseudo_element_specificity);
    Ok(CascadeSelector {
        text: item.to_string(),
        complex,
        pseudo_element,
        specificity: specificity.into(),
    })
}

/// Parse the argument of a CSS `selector()` support condition (`@supports selector(…)`,
/// `CSS.supports("selector(…)")`): exactly one complex selector, valid like a stylesheet
/// item, and parsed **unforgivingly** — an invalid item of `:is()` / `:where()` makes it
/// unsupported instead of being dropped (CSS Conditional 4 `<supports-selector-fn>`, as
/// Chrome 156 evaluates it: turbo-surf's `css-supports` Chrome control).
///
/// # Errors
/// Returns [`SelectorError`] when Chrome reports the selector unsupported: a list, a
/// malformed or unknown selector, or an invalid item in a forgiving list.
pub fn parse_for_supports(src: &str) -> Result<CascadeSelector, SelectorError> {
    let src = super::source::normalize_str(src, super::source::Mode::Sheet);
    let item = src.trim();
    if split_top_level_commas(item).len() != 1 {
        return Err(invalid("selector() takes one complex selector"));
    }
    unforgiving_items(&parse_raw(item)?)?;
    parse_item(item)
}

/// Every `:is()` / `:where()` item at any depth is valid on its own.
fn unforgiving_items(complex: &Complex) -> Result<(), SelectorError> {
    std::iter::once(&complex.first)
        .chain(complex.rest.iter().map(|(_, compound)| compound))
        .flat_map(|compound| &compound.pseudos)
        .try_for_each(unforgiving_pseudo)
}

fn unforgiving_pseudo(pseudo: &Pseudo) -> Result<(), SelectorError> {
    match pseudo {
        Pseudo::Is(list) | Pseudo::Where(list) => {
            if list.is_empty() {
                return Err(invalid("empty selector list argument"));
            }
            list.iter().try_for_each(|complex| {
                check_nested(&mut complex.clone())?;
                unforgiving_items(complex)
            })
        }
        Pseudo::Not(list) | Pseudo::NthChildOf(_, _, list) | Pseudo::NthLastChildOf(_, _, list) => {
            list.iter().try_for_each(unforgiving_items)
        }
        Pseudo::Has(list) => list
            .iter()
            .try_for_each(|relative| unforgiving_items(&relative.complex)),
        _ => Ok(()),
    }
}

/// One complex selector (no top-level comma), checked like a stylesheet item and with every
/// nested pseudo checked: `Err` when Chrome rejects it.
fn parse_complex_strict(item: &str) -> Result<Complex, SelectorError> {
    let mut complex = parse_raw(item)?;
    check_compound(&mut complex.first, false)?;
    for (_, compound) in &mut complex.rest {
        check_compound(compound, false)?;
    }
    Ok(complex)
}

/// One complex selector as the shared parser reads it, before the stylesheet checks.
fn parse_raw(item: &str) -> Result<Complex, SelectorError> {
    if item.is_empty() {
        return Err(SelectorError::EmptyCompound);
    }
    validate_selector_list(item)?;
    let mut tail = item.chars().rev();
    if matches!(tail.next(), Some('>' | '+' | '~')) && tail.next() != Some('\\') {
        return Err(SelectorError::EmptyCompound);
    }
    if item.contains('\\') {
        let units: Vec<u16> = item.encode_utf16().collect();
        // one validated item is one complex selector
        units::list(&units)
            .into_iter()
            .next()
            .ok_or(SelectorError::EmptyCompound)
    } else {
        parse_complex(item).ok_or(SelectorError::EmptyCompound)
    }
}

/// Pull the pseudo-element off the subject compound (the last one). Pseudo-elements were
/// left in place as `Unknown` markers by the shared parser; any left elsewhere is an error.
fn finish_subject(mut complex: Complex) -> Result<(Complex, Option<PseudoElement>), SelectorError> {
    let subject = match complex.rest.last_mut() {
        Some((_, compound)) => compound,
        None => &mut complex.first,
    };
    let pseudo_element = take_pseudo_element(subject)?;
    let others = std::iter::once(&complex.first)
        .chain(complex.rest.iter().map(|(_, c)| c))
        .take(complex.rest.len());
    for compound in others {
        if compound.pseudos.iter().any(is_pseudo_element_marker) {
            return Err(invalid("pseudo-element before a combinator"));
        }
    }
    Ok((complex, pseudo_element))
}

/// `Unknown` pseudos that stand for (part of) a pseudo-element: the empty name the shared
/// tokenizer gives the first colon of `::`, and the legacy single-colon names.
fn is_pseudo_element_marker(p: &Pseudo) -> bool {
    matches!(p, Pseudo::Unknown(u) if u.name.is_empty() || LEGACY_PSEUDO_ELEMENTS.contains(&u.name.to_ascii_lowercase().as_str()))
}

fn take_pseudo_element(compound: &mut Compound) -> Result<Option<PseudoElement>, SelectorError> {
    let Some(at) = compound.pseudos.iter().position(is_pseudo_element_marker) else {
        return Ok(None);
    };
    let marker = compound.pseudos.remove(at);
    let Pseudo::Unknown(UnknownPseudo { name, arg }) = marker else {
        unreachable!("is_pseudo_element_marker only accepts Unknown");
    };
    let (element, element_name) = if name.is_empty() {
        // `::name`: the name is the next pseudo the tokenizer produced
        match (at < compound.pseudos.len()).then(|| compound.pseudos.remove(at)) {
            Some(Pseudo::Unknown(UnknownPseudo { name, arg })) => {
                (pseudo_element(&name, arg)?, name.to_ascii_lowercase())
            }
            _ => return Err(invalid("unknown pseudo-element")),
        }
    } else if arg.is_some() {
        return Err(invalid("legacy pseudo-element with an argument"));
    } else {
        let lower = name.to_ascii_lowercase();
        (PseudoElement::Inert(lower.clone()), lower)
    };
    if !compound.pseudos[at..]
        .iter()
        .all(|p| allowed_after_pseudo_element(&element_name, p))
    {
        return Err(invalid("pseudo-class after a pseudo-element"));
    }
    compound.pseudos.truncate(at);
    Ok(Some(element))
}

fn pseudo_element(name: &str, arg: Option<String>) -> Result<PseudoElement, SelectorError> {
    let lower = name.to_ascii_lowercase();
    let known = match &arg {
        None => PSEUDO_ELEMENTS.contains(&lower.as_str()),
        Some(_) => FUNCTIONAL_PSEUDO_ELEMENTS.contains(&lower.as_str()),
    };
    if !(known || lower.starts_with("-webkit-")) {
        return Err(invalid("unknown pseudo-element"));
    }
    match (lower.as_str(), arg) {
        ("slotted", Some(arg)) => {
            let list = unforgiving_list(&arg)?;
            if list.len() != 1 {
                return Err(invalid("::slotted() takes one compound selector"));
            }
            Ok(PseudoElement::Slotted(list))
        }
        _ => Ok(PseudoElement::Inert(lower)),
    }
}

/// Check (and, for cascade-only pseudos, rewrite) every pseudo of one compound. `nested`:
/// the compound sits in a pseudo argument, where pseudo-elements are not allowed.
fn check_compound(compound: &mut Compound, nested: bool) -> Result<(), SelectorError> {
    for pseudo in &mut compound.pseudos {
        if is_pseudo_element_marker(pseudo) {
            // in the subject compound, the pseudo-element and what follows it are checked
            // by `take_pseudo_element`; anywhere else `finish_subject` rejects it
            return if nested {
                Err(invalid("pseudo-element inside a pseudo-class argument"))
            } else {
                Ok(())
            };
        }
        check_pseudo(pseudo)?;
    }
    Ok(())
}

fn check_pseudo(pseudo: &mut Pseudo) -> Result<(), SelectorError> {
    match pseudo {
        Pseudo::Unknown(unknown) => *pseudo = cascade_pseudo(unknown)?,
        // a DOM-API extension (the JS runtime's `:selected`); not a CSS pseudo-class
        Pseudo::Selected => return Err(invalid("unknown pseudo-class")),
        Pseudo::Is(list) | Pseudo::Where(list) => forgive(list),
        Pseudo::Not(list) | Pseudo::NthChildOf(_, _, list) | Pseudo::NthLastChildOf(_, _, list) => {
            // never empty: the shared parser makes a pseudo with an empty, unplaceable or
            // combinator-edged item `Unknown` (an error below), and every other strictly
            // lexed item parses to a complex selector (fuzzed: 1.46M arguments)
            debug_assert!(!list.is_empty(), "empty selector list argument");
            for complex in list {
                check_nested(complex)?;
            }
        }
        Pseudo::Has(list) => {
            // never empty, as above (a relative item may start with a combinator)
            debug_assert!(!list.is_empty(), "empty :has()");
            for relative in list {
                check_nested(&mut relative.complex)?;
                reject_nested_has(&mut relative.complex)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Inside `:has()`, a nested `:has()` is invalid (Chrome 156): an error directly or through
/// the unforgiving `:not()` / `of S`, while the forgiving `:is()` / `:where()` /
/// `:-webkit-any()` drop the items that contain one (`:has(:is(p, :has(q)))` is `:has(p)`).
fn reject_nested_has(complex: &mut Complex) -> Result<(), SelectorError> {
    let compounds =
        std::iter::once(&mut complex.first).chain(complex.rest.iter_mut().map(|(_, c)| c));
    for compound in compounds {
        for pseudo in &mut compound.pseudos {
            match pseudo {
                Pseudo::Has(_) => return Err(invalid(":has() inside :has()")),
                Pseudo::Is(list) | Pseudo::Where(list) | Pseudo::WebkitAny(list) => {
                    list.retain(|c| !contains_has(c));
                }
                Pseudo::Not(list) if list.iter().any(contains_has) => {
                    return Err(invalid(":has() inside :not() inside :has()"));
                }
                Pseudo::NthChildOf(_, _, list) if list.iter().any(contains_has) => {
                    return Err(invalid(":has() inside :nth-child() inside :has()"));
                }
                Pseudo::NthLastChildOf(_, _, list) if list.iter().any(contains_has) => {
                    return Err(invalid(":has() inside :nth-last-child() inside :has()"));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Does `complex` use `:has()` anywhere (through any selector-list argument)?
pub(super) fn contains_has(complex: &Complex) -> bool {
    std::iter::once(&complex.first)
        .chain(complex.rest.iter().map(|(_, c)| c))
        .flat_map(|c| &c.pseudos)
        .any(|p| matches!(p, Pseudo::Has(_)) || selector_list(p).iter().any(contains_has))
}

/// The selector-list argument of `:is()`, `:where()`, `:not()`, `:-webkit-any()` and
/// `of S` (empty for every other pseudo). One arm per variant: LLVM attributes a
/// multi-line or-pattern's continuation lines to never-counted gap regions.
#[allow(clippy::match_same_arms)]
fn selector_list(p: &Pseudo) -> &[Complex] {
    match p {
        Pseudo::Is(list) => list,
        Pseudo::Where(list) => list,
        Pseudo::Not(list) => list,
        Pseudo::WebkitAny(list) => list,
        Pseudo::NthChildOf(_, _, list) => list,
        Pseudo::NthLastChildOf(_, _, list) => list,
        _ => &[],
    }
}

fn check_nested(complex: &mut Complex) -> Result<(), SelectorError> {
    check_compound(&mut complex.first, true)?;
    for (_, compound) in &mut complex.rest {
        check_compound(compound, true)?;
    }
    Ok(())
}

/// Forgiving list (`:is()`, `:where()`): drop the items Chrome would reject.
fn forgive(list: &mut Vec<Complex>) {
    list.retain_mut(|complex| check_nested(complex).is_ok());
}

/// `:-webkit-any()` as Chrome 156 parses it (both the DOM APIs and stylesheets): a
/// non-empty, unforgiving list of COMPOUND selectors without pseudo-elements or `:has()`,
/// where a nested `:not()` also takes compounds only and a nested `:is()` / `:where()`
/// drops its complex items (`:-webkit-any(:is(p, div p))` is `:-webkit-any(:is(p))`).
/// `None`: a `SyntaxError` (the shared parser leaves the pseudo `Unknown`).
pub(super) fn webkit_any(arg: &str) -> Option<Vec<Complex>> {
    split_top_level_commas(arg)
        .into_iter()
        .map(|item| {
            let mut complex = parse_raw(item.trim()).ok()?;
            // no `:has()`: directly or through `:not()` / `of S` an error, while a nested
            // `:is()` / `:where()` drops the items with one (as inside `:has()`)
            reject_nested_has(&mut complex).ok()?;
            check_nested(&mut complex).ok()?;
            compound_only(&mut complex, false).then_some(complex)
        })
        .collect()
}

/// Restrict a `:-webkit-any()` item: no combinators unless `combinators` (an `of S`
/// item), compounds only in nested `:not()`, complex items dropped from nested `:is()` /
/// `:where()`. `false` when the item is invalid.
fn compound_only(complex: &mut Complex, combinators: bool) -> bool {
    if !combinators && !complex.rest.is_empty() {
        return false;
    }
    let compounds =
        std::iter::once(&mut complex.first).chain(complex.rest.iter_mut().map(|(_, c)| c));
    for compound in compounds {
        for pseudo in &mut compound.pseudos {
            match pseudo {
                Pseudo::Is(list) | Pseudo::Where(list) => {
                    list.retain_mut(|item| compound_only(item, false));
                }
                Pseudo::Not(list) => {
                    if !list.iter_mut().all(|item| compound_only(item, false)) {
                        return false;
                    }
                }
                Pseudo::NthChildOf(_, _, list) | Pseudo::NthLastChildOf(_, _, list) => {
                    if !list.iter_mut().all(|item| compound_only(item, true)) {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }
    true
}

/// An unforgiving selector-list argument parsed from its text.
fn unforgiving_list(arg: &str) -> Result<Vec<Complex>, SelectorError> {
    split_top_level_commas(arg)
        .into_iter()
        .map(|item| {
            let mut complex = parse_complex_strict(item.trim())?;
            check_nested(&mut complex)?;
            Ok(complex)
        })
        .collect()
}

/// A pseudo the shared parser left `Unknown`: an implemented one written in another case,
/// a cascade-only one, a valid never-matching one, or an error.
fn cascade_pseudo(unknown: &UnknownPseudo) -> Result<Pseudo, SelectorError> {
    // (`parse_pseudo` already tried the ASCII-lowercased name: an implemented pseudo in
    // another case never reaches here)
    let lower = unknown.name.to_ascii_lowercase();
    let arg = unknown.arg.as_deref();
    match (lower.as_str(), arg) {
        (name, None) if NEVER_PSEUDO_CLASSES.contains(&name) => Ok(Pseudo::unknown(name, None)),
        (name, Some(_)) if NEVER_FUNCTIONAL_PSEUDO_CLASSES.contains(&name) => {
            Ok(Pseudo::unknown(name, arg))
        }
        // `:active-view-transition-type(<custom-ident>#)`: no view transition runs in a
        // static document (turbo-surf#117, Google's consent page)
        ("active-view-transition-type", Some(list)) if is_ident_list(list) => {
            Ok(Pseudo::unknown(lower.as_str(), arg))
        }
        _ => Err(invalid("unknown pseudo-class")),
    }
}

/// A comma-separated list of CSS identifiers (Chrome 156: no empty item, no string, no
/// `*`).
fn is_ident_list(list: &str) -> bool {
    list.split(',').all(|item| {
        let units: Vec<u16> = item.encode_utf16().collect();
        units::lang_range(&units).is_some()
    })
}

/// Does `h` match `sel` through `nav` (a pseudo-element selector never matches an element,
/// except `::slotted()` in a slot-aware navigator)?
pub fn match_for_cascade<N: Navigator + ?Sized>(nav: &N, h: Handle, sel: &CascadeSelector) -> bool {
    match &sel.pseudo_element {
        None => matches_complex(nav, h, &sel.complex),
        Some(PseudoElement::Inert(_)) => false,
        Some(PseudoElement::Slotted(args)) => nav.assigned_slot(h).is_some_and(|slot| {
            args.iter().any(|arg| matches_complex(nav, h, arg))
                && matches_complex(nav, slot, &sel.complex)
        }),
    }
}

// ------------------------------------------------------------------ cascade-only pseudos

/// `:defined`: the element's custom element state is "uncustomized" or "custom"
/// ([`Tree::custom_element_state`]): without an embedder's report an HTML autonomous custom
/// element or an element parsed with an `is` value (`<button is="x-b">`, even `is=""`) is
/// "undefined", as Chrome 156 reports for a document without definitions; the V8 binding
/// reports upgrades (`customElements.define()`, miaskiewicz/turbo-dom#24).
pub(super) fn is_defined(tree: &Tree, h: Handle) -> bool {
    tree.custom_element_state(h).is_defined()
}

/// `:lang(range)`: the element's language — the nearest ancestor-or-self `xml:lang` (XML
/// namespace) or `lang` attribute (any element: SVG's `lang` counts; an HTML element's
/// no-namespace `xml:lang` does not) — equals `range` or starts with it plus `-`, ASCII
/// case-insensitively (Chrome 156: no wildcard ranges, `:lang(zh-TW)` does not match
/// `zh-Hant-TW`). An empty `lang=""` is an unknown language and matches nothing.
pub(super) fn lang_matches(tree: &Tree, h: Handle, range: &str) -> bool {
    let mut node = Some(h);
    while let Some(n) = node {
        let lang = (tree.node_type(n) == super::NodeType::Element)
            .then(|| {
                tree.get_attribute_ns_borrowed(
                    n,
                    Some("http://www.w3.org/XML/1998/namespace"),
                    "lang",
                )
                .or_else(|| tree.get_attribute_ns_borrowed(n, None, "lang"))
            })
            .flatten();
        if let Some(lang) = lang {
            let lang = lang.to_ascii_lowercase();
            return lang == range
                || (lang.len() > range.len()
                    && lang.starts_with(range)
                    && lang.as_bytes()[range.len()] == b'-');
        }
        node = tree.parent(n);
    }
    false
}

/// `:dir()`: is `h`'s HTML directionality `rtl`? Per HTML (as Chrome 156 evaluates it): an
/// HTML element's valid `dir` (`ltr` / `rtl` / `auto`, ASCII case-insensitive) decides —
/// `auto` by its auto directionality (`ltr` without a strong character); with no valid
/// `dir`, a `bdi` is `auto`, an `input type=tel` is `ltr`, and anything else (every
/// non-HTML element too) takes its parent's directionality (a shadow root's host; `ltr` at
/// the top of a tree).
pub(super) fn is_rtl(tree: &Tree, h: Handle) -> bool {
    let mut node = h;
    loop {
        if tree.is_html(node) {
            match dir_state(tree, node) {
                Some(Dir::Ltr) => return false,
                Some(Dir::Rtl) => return true,
                Some(Dir::Auto) => return auto_rtl(tree, node).unwrap_or(false),
                None if tree.local_name(node) == Some("bdi") => {
                    return auto_rtl(tree, node).unwrap_or(false)
                }
                None if tree.local_name(node) == Some("input")
                    && tree
                        .get_attribute(node, "type")
                        .is_some_and(|t| t.eq_ignore_ascii_case("tel")) =>
                {
                    return false
                }
                None => {}
            }
        }
        let parent = match tree.parent(node) {
            Some(p) if tree.is_shadow_root(p) => tree.shadow_host(p),
            Some(p) if tree.node_type(p) == super::NodeType::Element => Some(p),
            _ => None,
        };
        match parent {
            Some(p) => node = p,
            None => return false,
        }
    }
}

#[derive(Clone, Copy)]
enum Dir {
    Ltr,
    Rtl,
    Auto,
}

/// An HTML element's `dir` attribute state (`None`: missing or invalid — "undefined").
fn dir_state(tree: &Tree, h: Handle) -> Option<Dir> {
    let value = tree.get_attribute_ns_borrowed(h, None, "dir")?;
    [("ltr", Dir::Ltr), ("rtl", Dir::Rtl), ("auto", Dir::Auto)]
        .into_iter()
        .find_map(|(name, dir)| value.eq_ignore_ascii_case(name).then_some(dir))
}

/// `<input type>` states whose value decides auto directionality (HTML "auto-directionality
/// form-associated elements"; a missing or unknown type is Text).
const AUTO_DIR_INPUT_TYPES: &[&str] = &[
    "hidden", "text", "search", "tel", "url", "email", "password", "submit", "reset", "button",
];

/// HTML "auto directionality": `Some(rtl)` from the first strong character, `None` when
/// there is none. A text `input` / `textarea` reads its value (a textarea's is its child
/// text content; any non-empty value without
/// a leading strong RTL character is `ltr`); other elements read their descendant text,
/// skipping `bdi`, `script`, `style`, `textarea`, template content and any HTML element
/// with a valid `dir`.
fn auto_rtl(tree: &Tree, h: Handle) -> Option<bool> {
    let value = match tree.local_name(h) {
        // its value: the child text content (direct Text children only; turbo-dom#106)
        Some("textarea") => {
            let mut value = String::new();
            tree.for_each_child(h, |c| {
                if tree.node_type(c) == super::NodeType::Text {
                    value.push_str(&tree.node_value(c).unwrap_or_default());
                }
            });
            Some(value)
        }
        Some("input")
            if tree.get_attribute(h, "type").is_none_or(|t| {
                AUTO_DIR_INPUT_TYPES
                    .iter()
                    .any(|k| t.eq_ignore_ascii_case(k))
                    || !super::KNOWN_INPUT_TYPES
                        .iter()
                        .any(|k| t.eq_ignore_ascii_case(k))
            }) =>
        {
            Some(
                tree.get_attribute(h, "value")
                    .unwrap_or_default()
                    .to_string(),
            )
        }
        _ => None,
    };
    if let Some(value) = value {
        return strong_rtl(&value).or((!value.is_empty()).then_some(false));
    }
    let mut stack = Vec::new();
    tree.for_each_child(h, |c| stack.push(c));
    stack.reverse();
    while let Some(n) = stack.pop() {
        match tree.node_type(n) {
            super::NodeType::Text => {
                if let Some(rtl) = tree.node_value(n).and_then(|t| strong_rtl(&t)) {
                    return Some(rtl);
                }
            }
            super::NodeType::Element => {
                let skipped = tree.is_html(n)
                    && (matches!(
                        tree.local_name(n),
                        Some("bdi" | "script" | "style" | "textarea")
                    ) || dir_state(tree, n).is_some());
                if !skipped {
                    let base = stack.len();
                    tree.for_each_child(n, |c| stack.push(c));
                    stack[base..].reverse();
                }
            }
            _ => {}
        }
    }
    None
}

/// The direction of the first strong character of `text` (bidi class L, R or AL).
fn strong_rtl(text: &str) -> Option<bool> {
    use unicode_bidi::BidiClass;
    text.chars()
        .find_map(|c| match unicode_bidi::bidi_class(c) {
            BidiClass::L => Some(false),
            BidiClass::R | BidiClass::AL => Some(true),
            _ => None,
        })
}

// ------------------------------------------------------------------------- specificity

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Spec(u32, u32, u32);

impl std::ops::Add for Spec {
    type Output = Spec;
    fn add(self, o: Spec) -> Spec {
        Spec(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
}

impl From<Spec> for Specificity {
    fn from(s: Spec) -> Specificity {
        (s.0, s.1, s.2)
    }
}

const CLASS: Spec = Spec(0, 1, 0);
const TYPE: Spec = Spec(0, 0, 1);

fn complex_specificity(complex: &Complex) -> Spec {
    std::iter::once(&complex.first)
        .chain(complex.rest.iter().map(|(_, c)| c))
        .map(compound_specificity)
        .fold(Spec::default(), |a, b| a + b)
}

fn max_specificity<'a>(list: impl IntoIterator<Item = &'a Complex>) -> Spec {
    list.into_iter()
        .map(complex_specificity)
        .max()
        .unwrap_or_default()
}

fn compound_specificity(c: &Compound) -> Spec {
    let ids = Spec(u32::from(c.id.is_some()), 0, 0);
    let classes = Spec(0, (c.classes.len() + c.attrs.len()) as u32, 0);
    let tag = if c.tag.is_some() {
        TYPE
    } else {
        Spec::default()
    };
    c.pseudos
        .iter()
        .map(pseudo_specificity)
        .fold(ids + classes + tag, |a, b| a + b)
}

fn pseudo_specificity(p: &Pseudo) -> Spec {
    match p {
        Pseudo::Where(_) => Spec::default(),
        Pseudo::Is(list) | Pseudo::Not(list) => max_specificity(list),
        Pseudo::Has(list) => max_specificity(list.iter().map(|r| &r.complex)),
        Pseudo::NthChildOf(_, _, list) | Pseudo::NthLastChildOf(_, _, list) => {
            CLASS + max_specificity(list)
        }
        _ => CLASS,
    }
}

fn pseudo_element_specificity(p: &PseudoElement) -> Spec {
    match p {
        PseudoElement::Inert(_) => TYPE,
        PseudoElement::Slotted(list) => TYPE + max_specificity(list),
    }
}

// ----------------------------------------------------------------------- position cache

/// Sibling positions of one sibling list (see [`PositionCache`]).
#[derive(Debug, Default)]
pub struct SiblingGroup {
    elements: Vec<Handle>,
    positions: FxHashMap<Handle, Position>,
}

#[derive(Debug, Clone, Copy)]
struct Position {
    index: usize,
    of_type_index: usize,
    of_type_count: usize,
}

impl SiblingGroup {
    /// The element siblings, in order.
    #[must_use]
    pub fn elements(&self) -> &[Handle] {
        &self.elements
    }

    /// `(index, count)` of `h` among the element siblings (`(0, count)` if absent).
    #[must_use]
    pub fn child_position(&self, h: Handle) -> (usize, usize) {
        (
            self.positions.get(&h).map_or(0, |p| p.index),
            self.elements.len(),
        )
    }

    /// `(index, count)` of `h` among the siblings of its type (`(0, 0)` if absent).
    #[must_use]
    pub fn type_position(&self, h: Handle) -> (usize, usize) {
        self.positions
            .get(&h)
            .map_or((0, 0), |p| (p.of_type_index, p.of_type_count))
    }
}

/// The cascade-mode position cache: a [`Navigator`] that styles a whole tree asks for the
/// position of every element under every structural pseudo of every rule; this builds each
/// sibling list's positions once and shares them (keyed by every member).
#[derive(Debug, Default)]
pub struct PositionCache {
    groups: RefCell<FxHashMap<Handle, Rc<SiblingGroup>>>,
}

impl PositionCache {
    /// The sibling group containing `h`, built on first use from `siblings()`: the element
    /// siblings in order, each with its `*-of-type` key.
    pub fn group<K: std::hash::Hash + Eq>(
        &self,
        h: Handle,
        siblings: impl FnOnce() -> Vec<(Handle, K)>,
    ) -> Rc<SiblingGroup> {
        if let Some(group) = self.groups.borrow().get(&h) {
            return group.clone();
        }
        let siblings = siblings();
        let mut totals: FxHashMap<&K, usize> = FxHashMap::default();
        for (_, key) in &siblings {
            *totals.entry(key).or_default() += 1;
        }
        let mut seen: FxHashMap<&K, usize> = FxHashMap::default();
        let mut group = SiblingGroup::default();
        for (i, (element, key)) in siblings.iter().enumerate() {
            let of_type_index = {
                let n = seen.entry(key).or_default();
                *n += 1;
                *n
            };
            group.elements.push(*element);
            group.positions.insert(
                *element,
                Position {
                    index: i + 1,
                    of_type_index,
                    of_type_count: totals[key],
                },
            );
        }
        let group = Rc::new(group);
        let mut groups = self.groups.borrow_mut();
        for &element in &group.elements {
            groups.insert(element, group.clone());
        }
        groups.entry(h).or_insert_with(|| group.clone());
        group
    }
}

#[cfg(test)]
mod tests {
    use super::super::{LiveNavigator, Navigator};
    use super::*;

    fn spec(sel: &str) -> Specificity {
        let list = parse_for_cascade(sel).unwrap_or_else(|e| panic!("{sel}: {e}"));
        assert_eq!(list.selectors().len(), 1, "{sel}");
        list.selectors()[0].specificity()
    }

    /// View-transition selectors from Google's consent page (turbo-surf#117), with the
    /// verdicts of frozen Chrome 156.0.8078.47 (a `<style>` rule kept or dropped; the DOM
    /// APIs throw exactly when the rule is dropped).
    #[test]
    fn view_transition_selectors_follow_chrome() {
        for good in [
            "html:active-view-transition-type(aimc)",
            "html:active-view-transition-type(a, b)",
            ":ACTIVE-VIEW-TRANSITION-TYPE(A)",
            "html:active-view-transition",
            "html:active-view-transition-type(aimc)::view-transition-new(sb)",
            "html:active-view-transition-type(aimc)::view-transition-old(sb):only-child",
            "html:active-view-transition-type(aimc)::view-transition-group(*)",
            "::view-transition-group(sb):only-child",
            "::view-transition-image-pair(sb):only-child",
            "::view-transition-new(sb)",
        ] {
            assert!(parse_for_cascade(good).is_ok(), "{good} should be valid");
        }
        for bad in [
            "html:active-view-transition-type()",
            "html:active-view-transition-type(1)",
            ":active-view-transition-type(a,)",
            ":active-view-transition-type(a b)",
            ":active-view-transition-type(*)",
            ":active-view-transition-type(\"a\")",
            ":active-view-transition(x)",
            "::view-transition:only-child",
            "::view-transition-new(sb):first-child",
            "::view-transition-new(sb):hover",
        ] {
            assert!(parse_for_cascade(bad).is_err(), "{bad} should be invalid");
        }
        // no view transition runs in a static document: they never match
        let tree = Tree::parse("<p>");
        let html = tree.query_selector("html").unwrap();
        for sel in [
            "html:active-view-transition-type(aimc)",
            "html:active-view-transition",
            "html:not(:active-view-transition)",
        ] {
            let expect = sel.contains(":not(");
            assert_eq!(tree.matches_checked(html, sel), Ok(expect), "{sel}");
        }
        assert!(tree.matches_checked(html, "html:bogus").is_err());
    }

    /// `CSS.supports("selector(…)")` in Chrome 156 (turbo-surf
    /// `harness/detection/fixtures/css-supports-chrome156.json`, the `selector(…)` cases).
    #[test]
    fn supports_selectors_follow_chrome() {
        let cases = [
            ("div > .x", true),
            (":has(> span)", true),
            (":is(div,span)", true),
            (":bogus", false),
            ("div,span", false),
            ("div >", false),
            ("*|p", true),
            ("*|*", true),
            ("|p", true),
            ("ns|p", false),
            ("[*|title]", true),
            (r".\31 23", true),
            (r"#\66oo", true),
            (r".a\.b", true),
            (r"\64iv > .x", true),
            (".é", true),
            (":hover", true),
            ("::before", true),
            ("div::after", true),
            (":is(:bogus)", false),
            (":where(:bogus, p)", false),
            (":not(:bogus)", false),
            (":host", true),
            ("::slotted(p)", true),
            (":nth-child(2n of .x)", true),
            (":empty", true),
            (":link", true),
            (":read-write", true),
        ];
        for (selector, chrome) in cases {
            assert_eq!(parse_for_supports(selector).is_ok(), chrome, "{selector}");
        }
        // Unforgiving at every depth, and an empty forgiving list is unsupported.
        for selector in [
            ":not(:is(:bogus))",
            ":has(:where(p, :bogus))",
            ":is()",
            "a, b",
        ] {
            assert!(parse_for_supports(selector).is_err(), "{selector}");
        }
        let parsed = parse_for_supports(" :is(p, .x) ").unwrap();
        assert_eq!(parsed.specificity(), (0, 1, 0));
    }

    #[test]
    fn validity_follows_chrome_stylesheet_rules() {
        for ok in [
            "p",
            "a, b",
            "a > b + c ~ d e",
            ".md\\:flex",
            "*|p",
            "p::before",
            "p:before",
            "P:AFTER",
            "::-webkit-scrollbar",
            "::-webkit-scrollbar-thumb:hover",
            "::-webkit-scrollbar:horizontal:disabled",
            "a:hover",
            "a:VISITED",
            ":dir(ltr)",
            ":host-context(.x)",
            "::part(label)",
            "::highlight(x)",
            "p::slotted(span)",
            ":is(p, :bogus, ::before)",
            ":where(:bogus)",
            ":FIRST-CHILD",
            ":Not(.a)",
            ":Is(.a, :bogus)",
            ":scope",
            ":defined",
            ":lang(en)",
            ":lang(\\*-CH)",
            ":-webkit-any(p, .x)",
            "li:nth-child(2 of .x)",
            ":not(div p)",
            ":has(> a b)",
            "a\\>",
        ] {
            assert!(parse_for_cascade(ok).is_ok(), "{ok} should be valid");
        }
        for bad in [
            "",
            "a,",
            ",a",
            "a,,b",
            "a >",
            "a +",
            "> a",
            "a > > b",
            "a:bogus",
            "a:bogus, b",
            "::bogus",
            "::",
            "a::",
            "p::before span",
            "p::before:first-child",
            "::first-child",
            ":before(x)",
            "::slotted(a, b)",
            "::slotted(:bogus)",
            "::-moz-selection",
            ":-moz-any(p)",
            ":matches(p)",
            ":not()",
            ":not(:bogus)",
            ":not(::before)",
            ":has()",
            ":has(:bogus)",
            ":nth-child(1 of :bogus)",
            ":nth-child(1 of ::before)",
            ":lang()",
            ":lang(en,)",
            ":lang(en, de)",
            ":lang(\"de\")",
            ":lang(1)",
            ":state(--x y)",
            ":Nth-Child(1 of :bogus)",
            "option:selected",
            "div[a",
            "a!b",
        ] {
            assert!(parse_for_cascade(bad).is_err(), "{bad} should be invalid");
        }
        assert_eq!(
            parse_for_cascade("a:bogus").unwrap_err().to_string(),
            "invalid selector: unknown pseudo-class"
        );
        let list = parse_for_cascade(" a , b.c ").unwrap();
        let texts: Vec<&str> = list.selectors().iter().map(CascadeSelector::text).collect();
        assert_eq!(texts, ["a", "b.c"]);
    }

    #[test]
    fn specificity_follows_selectors_4() {
        assert_eq!(spec("*"), (0, 0, 0));
        assert_eq!(spec("*|*"), (0, 0, 0));
        assert_eq!(spec("|p"), (0, 0, 1));
        assert_eq!(spec("div p.a[x]:hover"), (0, 3, 2));
        assert_eq!(spec("#a .b"), (1, 1, 0));
        assert_eq!(spec(":where(#a, .b) p"), (0, 0, 1));
        assert_eq!(spec(":is(#a, .b)"), (1, 0, 0));
        assert_eq!(spec(":is(.b, :bogus)"), (0, 1, 0));
        assert_eq!(spec(":not(div, .a.b)"), (0, 2, 0));
        assert_eq!(spec(":not(div)"), (0, 0, 1));
        assert_eq!(spec("p:has(> #x, .y)"), (1, 0, 1));
        assert_eq!(spec(":nth-child(2 of #a, .b)"), (1, 1, 0));
        assert_eq!(spec(":nth-last-child(odd of p)"), (0, 1, 1));
        assert_eq!(spec(":nth-child(2)"), (0, 1, 0));
        assert_eq!(spec("p::before"), (0, 0, 2));
        assert_eq!(spec("p:before"), (0, 0, 2));
        assert_eq!(spec("slot::slotted(.a)"), (0, 1, 2));
        assert_eq!(spec(":-webkit-any(#a, p)"), (0, 1, 0));
        assert_eq!(spec(":scope"), (0, 1, 0));
        assert_eq!(spec(":FIRST-CHILD"), (0, 1, 0));
        assert_eq!(spec(".\\31 0"), (0, 1, 0));
    }

    #[test]
    fn pseudo_elements_never_match_and_name_their_target() {
        let tree = Tree::parse("<p>x</p><slot><span class=a>y</span></slot>");
        let p = tree.query_selector("p").unwrap();
        let nav = LiveNavigator(&tree);
        let first = |sel: &str| parse_for_cascade(sel).unwrap().selectors()[0].clone();
        for sel in [
            "p::before",
            "p:after",
            "::marker",
            "p::-webkit-scrollbar:hover",
        ] {
            assert!(!match_for_cascade(&nav, p, &first(sel)), "{sel}");
        }
        assert_eq!(first("p::Before").pseudo_element(), Some("before"));
        assert_eq!(first("p:FIRST-LINE").pseudo_element(), Some("first-line"));
        assert_eq!(
            first("slot::slotted(span)").pseudo_element(),
            Some("slotted")
        );
        assert_eq!(first("p").pseudo_element(), None);
        assert!(match_for_cascade(&nav, p, &first("p")));
    }

    /// A navigator that assigns every `<span>` to the nearest `<slot>` ancestor.
    struct Slotting<'t>(LiveNavigator<'t>);

    impl Navigator for Slotting<'_> {
        fn tree(&self) -> &Tree {
            self.0.tree()
        }
        fn parent(&self, h: Handle) -> Option<Handle> {
            self.0.parent(h)
        }
        fn previous_element_sibling(&self, h: Handle) -> Option<Handle> {
            self.0.previous_element_sibling(h)
        }
        fn next_element_sibling(&self, h: Handle) -> Option<Handle> {
            self.0.next_element_sibling(h)
        }
        fn element_siblings(&self, h: Handle) -> Vec<Handle> {
            self.0.element_siblings(h)
        }
        fn same_type(&self, a: Handle, b: Handle) -> bool {
            self.0.same_type(a, b)
        }
        fn type_name(&self, h: Handle) -> Option<&str> {
            self.0.type_name(h)
        }
        fn is_root(&self, h: Handle) -> bool {
            self.0.is_root(h)
        }
        fn is_empty(&self, h: Handle) -> bool {
            self.0.is_empty(h)
        }
        fn some_descendant(&self, root: Handle, pred: &mut dyn FnMut(Handle) -> bool) -> bool {
            self.0.some_descendant(root, pred)
        }
        fn assigned_slot(&self, h: Handle) -> Option<Handle> {
            let tree = self.tree();
            if tree.local_name(h) != Some("span") {
                return None;
            }
            let mut node = tree.parent(h);
            while let Some(n) = node {
                if tree.local_name(n) == Some("slot") {
                    return Some(n);
                }
                node = tree.parent(n);
            }
            None
        }
    }

    #[test]
    fn slotted_matches_only_through_a_slot_aware_navigator() {
        let tree = Tree::parse("<slot name=s><span class=a>y</span><b>z</b></slot><span>w</span>");
        let span = tree.query_selector("span.a").unwrap();
        let loose = tree.query_selector_all("span")[1];
        let b = tree.query_selector("b").unwrap();
        let sel = |s: &str| parse_for_cascade(s).unwrap().selectors()[0].clone();
        let slotting = Slotting(LiveNavigator(&tree));
        assert!(match_for_cascade(
            &slotting,
            span,
            &sel("slot::slotted(.a)")
        ));
        assert!(match_for_cascade(
            &slotting,
            span,
            &sel("[name=s]::slotted(span)")
        ));
        assert!(!match_for_cascade(
            &slotting,
            span,
            &sel("div::slotted(.a)")
        ));
        assert!(!match_for_cascade(
            &slotting,
            span,
            &sel("slot::slotted(.z)")
        ));
        assert!(!match_for_cascade(&slotting, b, &sel("slot::slotted(b)")));
        assert!(!match_for_cascade(
            &slotting,
            loose,
            &sel("::slotted(span)")
        ));
        // the live tree has no slot assignment: never
        assert!(!match_for_cascade(
            &LiveNavigator(&tree),
            span,
            &sel("slot::slotted(.a)")
        ));
        // `:has()` with sibling heads through the trait's default self-or-descendant walk
        let doc = Tree::parse("<div id=a></div><span id=s><i class=x></i></span><p id=z></p>");
        let nav = Slotting(LiveNavigator(&doc));
        let el = |id: &str| doc.get_element_by_id(id).unwrap();
        assert!(match_for_cascade(&nav, el("a"), &sel("div:has(+ span)")));
        assert!(match_for_cascade(&nav, el("a"), &sel("div:has(+ span .x)")));
        assert!(!match_for_cascade(&nav, el("a"), &sel("div:has(~ .x)")));
        assert!(match_for_cascade(&nav, el("a"), &sel("div:has(~ p)")));
        assert!(!match_for_cascade(&nav, el("z"), &sel("p:has(+ *)")));
        // the delegating methods behave like the live tree
        assert_eq!(slotting.type_position(b), (1, 1));
        assert!(slotting.next_element_sibling(span).is_some());
        assert!(!slotting.is_empty(span));
        assert!(!slotting.is_root(span));
    }

    #[test]
    fn cascade_only_pseudos_match_like_chrome() {
        let tree = Tree::parse(
            "<div lang=en-US><p id=a>x</p><x-foo id=c></x-foo><font-face id=r></font-face></div>\
             <p lang=de id=b xml:lang=fr>y</p><p lang=EN id=e>z</p><p id=n>n</p>\
             <svg><g id=s xml:lang=fr-CA></g><x-bar id=sx></x-bar></svg>",
        );
        let nav = LiveNavigator(&tree);
        let by_id = |id: &str| tree.get_element_by_id(id).unwrap();
        let m = |s: &str, id: &str| {
            match_for_cascade(
                &nav,
                by_id(id),
                &parse_for_cascade(s).unwrap().selectors()[0],
            )
        };
        assert!(m(":lang(en)", "a"));
        assert!(m(":lang(EN-us)", "a"));
        assert!(!m(":lang(en-GB)", "a"));
        assert!(!m(":lang(e)", "a"));
        assert!(m(":lang(de)", "b"), "an HTML element's lang attribute");
        assert!(m(":lang(en)", "e"));
        assert!(!m(":lang(en)", "n"));
        assert!(m(":lang(fr)", "s"), "xml:lang");
        assert!(m(":defined", "a"));
        assert!(
            !m(":defined", "c"),
            "an undefined autonomous custom element"
        );
        assert!(m(":defined", "r"), "reserved names are not custom elements");
        assert!(m(":defined", "sx"), "only HTML elements can be custom");
        assert!(m(":-webkit-any(p, b)", "a"));
        assert!(!m(":-webkit-any(b)", "a"));
        assert_eq!(m(":SCOPE", "a"), m(":root", "a"));
        assert!(m("P:FIRST-CHILD", "a"));
        assert!(!m("p:hover", "a"));
        assert!(m(":dir(ltr)", "a"), "P5b: :dir() is evaluated");
        assert!(!m(":dir(rtl)", "a"));
        assert!(crate::rtdom::tree::is_custom_element_name("x-foo"));
        for not in ["xfoo", "X-foo", "x-Foo", "-x", "annotation-xml"] {
            assert!(!crate::rtdom::tree::is_custom_element_name(not), "{not}");
        }
    }

    /// turbo-dom#18 (P5b): `:dir()` edges the Chrome control does not reach — shadow
    /// trees inherit from the host, template content and detached nodes are `ltr`, a
    /// non-text `input` reads its descendants (none), and live trees.
    #[test]
    fn dir_follows_html_directionality() {
        let mut tree = Tree::parse(
            "<div dir=rtl id=host></div><template id=t><p>x</p></template>\
             <input type=checkbox dir=auto id=cb value=\u{5e9}><input type=bogus dir=auto id=bg value=\u{5e9}>\
             <div dir=auto id=em><style>\u{5e9}</style><!--\u{5e9}-->1</div>",
        );
        let el = |tree: &Tree, id: &str| tree.get_element_by_id(id).unwrap();
        let host = el(&tree, "host");
        let shadow = tree.attach_shadow(host);
        tree.set_inner_html(shadow, "<span>s</span>");
        let in_shadow = tree.children(shadow)[0];
        let detached = tree.create_element("p");
        let rtl = |tree: &Tree, h: Handle| tree.matches(h, ":dir(rtl)");
        assert!(
            rtl(&tree, in_shadow),
            "shadow content inherits the host's direction"
        );
        assert!(!rtl(&tree, detached) && tree.matches(detached, ":dir(ltr)"));
        let template = el(&tree, "t");
        let content = tree
            .children(template)
            .into_iter()
            .find(|&c| tree.node_type(c) == super::super::NodeType::Fragment)
            .unwrap();
        let p = tree.children(content)[0];
        assert!(tree.matches(p, ":dir(ltr)"));
        assert!(!rtl(&tree, el(&tree, "cb")), "a checkbox value is not text");
        assert!(rtl(&tree, el(&tree, "bg")), "an unknown type is Text");
        assert!(
            !rtl(&tree, el(&tree, "em")),
            "style and comments are skipped"
        );
        assert!(!tree.matches(host, ":dir(auto)") && !tree.matches(host, ":dir(foo)"));
        assert_eq!(strong_rtl("123 \u{627}"), Some(true));
        assert_eq!(strong_rtl("1 2"), None);
    }

    #[test]
    fn position_cache_builds_each_sibling_list_once() {
        let tree = Tree::parse("<ul><li>a</li><p>b</p><li>c</li></ul>");
        let items: Vec<Handle> = tree.query_selector_all("ul > *").to_vec();
        let cache = PositionCache::default();
        let mut builds = 0;
        let mut build = || {
            builds += 1;
            items
                .iter()
                .map(|&h| (h, tree.local_name(h).unwrap().to_string()))
                .collect()
        };
        let group = cache.group(items[2], &mut build);
        assert_eq!(group.elements(), &items[..]);
        assert_eq!(group.child_position(items[2]), (3, 3));
        assert_eq!(group.type_position(items[2]), (2, 2));
        assert_eq!(group.type_position(items[1]), (1, 1));
        let again = cache.group(items[0], &mut build);
        assert!(Rc::ptr_eq(&group, &again));
        assert_eq!(builds, 1);
        let ul = tree.query_selector("ul").unwrap();
        assert_eq!(group.child_position(ul), (0, 3));
        assert_eq!(group.type_position(ul), (0, 0));
        // a key absent from the list it builds still maps to that list
        let lonely = cache.group(ul, Vec::<(Handle, u8)>::new);
        assert_eq!(lonely.elements().len(), 0);
        assert!(Rc::ptr_eq(
            &lonely,
            &cache.group(ul, Vec::<(Handle, u8)>::new)
        ));
    }
}

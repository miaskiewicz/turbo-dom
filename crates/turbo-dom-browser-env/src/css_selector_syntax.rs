//! Author selector admission and serialization shared by stylesheet parsing,
//! CSSOM insertion/selectorText and the selector-taking DOM methods.
//!
//! Lightning CSS keeps unknown pseudo-classes as custom components and enforces
//! scrollbar-state placement more strictly than Blink, so it cannot decide
//! admission. Every name and placement rule below comes from
//! `css_selector_admission_chrome156.json`, which enumerates the candidates
//! through stylesheet parsing, insertRule, selectorText, CSS.supports and
//! querySelector in Chrome 156.
use cssparser::{BasicParseError, Delimiter, ParseError, Parser, ParserInput, Token};
use std::cell::RefCell;
use std::collections::HashMap;

type Fallible<'i, T = ()> = Result<T, ParseError<'i, ()>>;

// Plain pseudo-classes Chrome admits in author selectors.
const CLASSES: &[&str] = &[
    "active",
    "active-view-transition",
    "any-link",
    "autofill",
    "checked",
    "corner-present",
    "current",
    "decrement",
    "default",
    "defined",
    "disabled",
    "double-button",
    "empty",
    "enabled",
    "end",
    "first-child",
    "first-of-type",
    "focus",
    "focus-visible",
    "focus-within",
    "fullscreen",
    "future",
    "horizontal",
    "host",
    "hover",
    "in-range",
    "increment",
    "indeterminate",
    "interest-source",
    "interest-target",
    "invalid",
    "last-child",
    "last-of-type",
    "link",
    "modal",
    "no-button",
    "only-child",
    "only-of-type",
    "open",
    "optional",
    "out-of-range",
    "past",
    "picture-in-picture",
    "placeholder-shown",
    "popover-open",
    "read-only",
    "read-write",
    "required",
    "root",
    "scope",
    "single-button",
    "start",
    "target",
    "target-current",
    "user-invalid",
    "user-valid",
    "valid",
    "vertical",
    "visited",
    "window-inactive",
    "xr-overlay",
    "-internal-autofill-selected",
    "-webkit-any-link",
    "-webkit-autofill",
    "-webkit-drag",
    "-webkit-full-page-media",
    "-webkit-full-screen",
    "-webkit-full-screen-ancestor",
];
const FUNCTIONAL_CLASSES: &[&str] = &[
    "active-view-transition-type",
    "dir",
    "has",
    "host",
    "host-context",
    "is",
    "lang",
    "not",
    "nth-child",
    "nth-last-child",
    "nth-last-of-type",
    "nth-of-type",
    "state",
    "where",
    "-webkit-any",
];
// CSS2 pseudo-elements keep their single-colon spelling and serialize as `::`.
const LEGACY_ELEMENTS: &[&str] = &["after", "before", "first-letter", "first-line"];
const ELEMENTS: &[&str] = &[
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
    "-internal-media-controls-overlay-cast-button",
];
const FUNCTIONAL_ELEMENTS: &[&str] = &[
    "cue",
    "highlight",
    "part",
    "picker",
    "scroll-button",
    "slotted",
    "view-transition-group",
    "view-transition-group-children",
    "view-transition-image-pair",
    "view-transition-new",
    "view-transition-old",
];
// Blink-known UA shadow pseudo-elements. Any other non-functional `::-webkit-*`
// name that is not a pseudo-class is admitted as the compatibility custom
// pseudo-element, but CSS.supports() reports it unsupported.
const WEBKIT_ELEMENTS: &[&str] = &[
    "calendar-picker-indicator",
    "color-swatch",
    "color-swatch-wrapper",
    "datetime-edit",
    "datetime-edit-ampm-field",
    "datetime-edit-day-field",
    "datetime-edit-fields-wrapper",
    "datetime-edit-hour-field",
    "datetime-edit-minute-field",
    "datetime-edit-month-field",
    "datetime-edit-second-field",
    "datetime-edit-text",
    "datetime-edit-year-field",
    "file-upload-button",
    "inner-spin-button",
    "input-placeholder",
    "media-controls",
    "media-controls-enclosure",
    "media-controls-panel",
    "media-controls-play-button",
    "media-controls-timeline",
    "media-text-track-container",
    "meter-bar",
    "meter-even-less-good-value",
    "meter-optimum-value",
    "meter-suboptimum-value",
    "progress-bar",
    "progress-inner-element",
    "progress-value",
    "resizer",
    "scrollbar",
    "scrollbar-button",
    "scrollbar-corner",
    "scrollbar-thumb",
    "scrollbar-track",
    "scrollbar-track-piece",
    "search-cancel-button",
    "slider-runnable-track",
    "slider-thumb",
    "textfield-decoration-container",
];
const SCROLLBAR_STATES: &[&str] = &[
    "active",
    "corner-present",
    "decrement",
    "disabled",
    "double-button",
    "enabled",
    "end",
    "horizontal",
    "hover",
    "increment",
    "no-button",
    "single-button",
    "start",
    "vertical",
    "window-inactive",
];
const USER_ACTIONS: &[&str] = &["active", "focus", "focus-visible", "focus-within", "hover"];
const STRUCTURAL: &[&str] = &[
    "empty",
    "first-child",
    "first-of-type",
    "has",
    "last-child",
    "last-of-type",
    "nth-child",
    "nth-last-child",
    "nth-last-of-type",
    "nth-of-type",
    "only-child",
    "only-of-type",
    "root",
];
const SCROLL_BUTTONS: &[&str] = &[
    "block-end",
    "block-start",
    "down",
    "inline-end",
    "inline-start",
    "left",
    "right",
    "up",
];

/// The pseudo-element that ends a compound decides which pseudos may follow it.
#[derive(Clone, Copy, PartialEq)]
enum Element {
    Generated,
    Marker,
    Shadow,
    Scrollbar,
    Selection,
    WebKit,
    ViewTransition,
    Other,
}

#[derive(Clone, Copy, Default)]
struct Context {
    relative: bool,
    in_has: bool,
    no_elements: bool,
    compound: bool,
    // Pseudo-classes inside :is()/:where() after a pseudo-element obey its rules.
    after: Option<Element>,
}

#[derive(Default)]
struct State {
    supported: bool,
}

/// A selector list admitted by the author grammar, serialized as Chrome reports it.
#[derive(Clone)]
pub(super) struct Admitted {
    pub text: String,
    supported: bool,
    complex: usize,
}

fn fail<'i, T>(input: &Parser<'i, '_>) -> Fallible<'i, T> {
    Err(input.new_custom_error(()))
}

fn lower(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn basic<'i>(error: BasicParseError<'i>) -> ParseError<'i, ()> {
    error.into()
}

fn list<'i>(
    input: &mut Parser<'i, '_>,
    context: Context,
    forgiving: bool,
    state: &mut State,
) -> Fallible<'i, Vec<String>> {
    let mut items = Vec::new();
    loop {
        let mut item = String::new();
        let mut nested = State { supported: true };
        let parsed = input.parse_until_before(Delimiter::Comma, |input| {
            complex(input, context, &mut item, &mut nested)
        });
        match parsed {
            Ok(()) => {
                state.supported &= nested.supported;
                items.push(item);
            }
            // Forgiving lists drop an invalid branch and keep its valid neighbors.
            Err(_) if forgiving => state.supported = false,
            Err(error) => return Err(error),
        }
        match input.next() {
            Ok(Token::Comma) => {}
            Ok(_) => return fail(input),
            Err(_) => break,
        }
    }
    if forgiving && items.is_empty() {
        state.supported = false;
    }
    if !forgiving && items.is_empty() {
        return fail(input);
    }
    Ok(items)
}

fn skip_whitespace(input: &mut Parser<'_, '_>) -> bool {
    let mut seen = false;
    loop {
        let start = input.state();
        match input.next_including_whitespace() {
            Ok(Token::WhiteSpace(_)) => seen = true,
            _ => {
                input.reset(&start);
                return seen;
            }
        }
    }
}

fn combinator(input: &mut Parser<'_, '_>) -> Option<char> {
    let start = input.state();
    match input.next_including_whitespace() {
        Ok(Token::Delim(value @ ('>' | '+' | '~'))) => {
            let value = *value;
            // `a||b` is the unsupported column combinator, not a namespace.
            Some(value)
        }
        _ => {
            input.reset(&start);
            None
        }
    }
}

fn complex<'i>(
    input: &mut Parser<'i, '_>,
    context: Context,
    out: &mut String,
    state: &mut State,
) -> Fallible<'i> {
    skip_whitespace(input);
    if context.relative {
        if let Some(value) = combinator(input) {
            out.push(value);
            out.push(' ');
            skip_whitespace(input);
        }
    }
    loop {
        let element = compound(input, context, out, state)?;
        let spaced = skip_whitespace(input);
        if input.is_exhausted() {
            return Ok(());
        }
        let joined = combinator(input);
        if joined.is_none() && !spaced {
            return fail(input);
        }
        // Pseudo-elements end a complex selector; compound-only arguments
        // (:host(), ::slotted(), :-webkit-any()) never take combinators.
        if element.is_some() || context.compound {
            return fail(input);
        }
        match joined {
            Some(value) => {
                out.push(' ');
                out.push(value);
                out.push(' ');
            }
            None => out.push(' '),
        }
        skip_whitespace(input);
        if input.is_exhausted() {
            return fail(input);
        }
    }
}

fn identifier<'i>(input: &mut Parser<'i, '_>) -> Fallible<'i, String> {
    match input.next_including_whitespace().map_err(basic)? {
        Token::Ident(name) => Ok(name.to_string()),
        _ => fail(input),
    }
}

fn delim(input: &mut Parser<'_, '_>, expected: char) -> bool {
    let start = input.state();
    if matches!(input.next_including_whitespace(), Ok(Token::Delim(value)) if *value == expected) {
        return true;
    }
    input.reset(&start);
    false
}

// Only `*|` and the empty namespace are meaningful without @namespace rules.
fn type_selector<'i>(input: &mut Parser<'i, '_>, out: &mut String) -> Fallible<'i, bool> {
    let start = input.state();
    let first = match input.next_including_whitespace() {
        Ok(Token::Ident(name)) => Some(lower(name)),
        Ok(Token::Delim('*')) => None,
        Ok(Token::Delim('|')) => {
            let local = local_name(input)?;
            out.push('|');
            out.push_str(&local);
            return Ok(true);
        }
        _ => {
            input.reset(&start);
            return Ok(false);
        }
    };
    let bar = input.state();
    let namespaced = delim(input, '|');
    if namespaced && delim(input, '|') {
        return fail(input);
    }
    if !namespaced {
        input.reset(&bar);
        out.push_str(first.as_deref().unwrap_or("*"));
        return Ok(true);
    }
    if first.is_some() {
        return fail(input);
    }
    out.push_str(&local_name(input)?);
    Ok(true)
}

fn local_name<'i>(input: &mut Parser<'i, '_>) -> Fallible<'i, String> {
    match input.next_including_whitespace().map_err(basic)? {
        Token::Ident(name) => Ok(lower(name)),
        Token::Delim('*') => Ok("*".to_owned()),
        _ => fail(input),
    }
}

fn attribute<'i>(input: &mut Parser<'i, '_>, out: &mut String) -> Fallible<'i> {
    skip_whitespace(input);
    let mut name = String::new();
    let start = input.state();
    match input.next_including_whitespace().map_err(basic)?.clone() {
        Token::Delim('*') => {
            if !delim(input, '|') {
                return fail(input);
            }
            name.push_str("*|");
            name.push_str(&identifier(input)?);
        }
        Token::Delim('|') => name.push_str(&identifier(input)?),
        Token::Ident(local) => {
            let bar = input.state();
            if delim(input, '|') {
                // A named prefix requires an @namespace declaration.
                if matches!(input.next_including_whitespace(), Ok(Token::Ident(_))) {
                    return fail(input);
                }
                input.reset(&bar);
            } else {
                input.reset(&bar);
            }
            name.push_str(&local);
        }
        _ => {
            input.reset(&start);
            return fail(input);
        }
    }
    out.push('[');
    out.push_str(&name);
    skip_whitespace(input);
    if input.is_exhausted() {
        out.push(']');
        return Ok(());
    }
    let operator = match input.next_including_whitespace().map_err(basic)?.clone() {
        Token::Delim('=') => "=",
        Token::IncludeMatch => "~=",
        Token::DashMatch => "|=",
        Token::PrefixMatch => "^=",
        Token::SuffixMatch => "$=",
        Token::SubstringMatch => "*=",
        _ => return fail(input),
    };
    skip_whitespace(input);
    let value = match input.next_including_whitespace().map_err(basic)?.clone() {
        Token::Ident(value) | Token::QuotedString(value) => value.to_string(),
        _ => return fail(input),
    };
    out.push_str(operator);
    cssparser::serialize_string(&value, out).map_err(|_| input.new_custom_error(()))?;
    skip_whitespace(input);
    if !input.is_exhausted() {
        let flag = identifier(input)?;
        if !flag.eq_ignore_ascii_case("i") {
            return fail(input);
        }
        out.push_str(" i");
        skip_whitespace(input);
    }
    input.expect_exhausted().map_err(basic)?;
    out.push(']');
    Ok(())
}

fn compound<'i>(
    input: &mut Parser<'i, '_>,
    context: Context,
    out: &mut String,
    state: &mut State,
) -> Fallible<'i, Option<Element>> {
    let mut element = context.after;
    let mut any = type_selector(input, out)?;
    if any && element.is_some() {
        return fail(input);
    }
    loop {
        let start = input.state();
        let position = input.position();
        let Ok(token) = input.next_including_whitespace() else {
            break;
        };
        match token.clone() {
            Token::IDHash(_) | Token::Delim('&') if element.is_none() => {
                out.push_str(input.slice_from(position));
            }
            Token::Delim('.') if element.is_none() => {
                if !matches!(input.next_including_whitespace(), Ok(Token::Ident(_))) {
                    return fail(input);
                }
                out.push_str(input.slice_from(position));
            }
            Token::SquareBracketBlock if element.is_none() => {
                input.parse_nested_block(|input| attribute(input, out))?;
            }
            Token::Colon => pseudo(input, context, out, &mut element, state)?,
            Token::WhiteSpace(_) | Token::Delim('>' | '+' | '~') => {
                input.reset(&start);
                break;
            }
            _ => return fail(input),
        }
        any = true;
    }
    if !any {
        return fail(input);
    }
    Ok(element)
}

fn class_follows(element: Element, name: &str) -> bool {
    match element {
        Element::Shadow => !STRUCTURAL.contains(&name),
        Element::Scrollbar => SCROLLBAR_STATES.contains(&name),
        Element::Selection => name == "window-inactive",
        Element::WebKit => USER_ACTIONS.contains(&name),
        Element::ViewTransition => name == "only-child",
        _ => false,
    }
}

fn element_follows(previous: Element, next: &str) -> bool {
    match previous {
        Element::Generated => next == "marker",
        Element::Shadow => !matches!(next, "part" | "slotted"),
        _ => false,
    }
}

fn element_kind(name: &str) -> Element {
    match name {
        "before" | "after" => Element::Generated,
        "marker" => Element::Marker,
        "part" | "slotted" => Element::Shadow,
        "selection" => Element::Selection,
        "-webkit-resizer" => Element::Scrollbar,
        _ if name.starts_with("-webkit-scrollbar") => Element::Scrollbar,
        _ if name.starts_with("-webkit-") => Element::WebKit,
        _ if name.starts_with("view-transition") => Element::ViewTransition,
        _ => Element::Other,
    }
}

fn pseudo<'i>(
    input: &mut Parser<'i, '_>,
    context: Context,
    out: &mut String,
    element: &mut Option<Element>,
    state: &mut State,
) -> Fallible<'i> {
    let mut token = input.next_including_whitespace().map_err(basic)?.clone();
    let double = token == Token::Colon;
    if double {
        token = input.next_including_whitespace().map_err(basic)?.clone();
    }
    let (name, functional) = match &token {
        Token::Ident(name) => (lower(name), false),
        Token::Function(name) => (lower(name), true),
        _ => return fail(input),
    };
    if double || (!functional && LEGACY_ELEMENTS.contains(&name.as_str())) {
        if context.no_elements {
            return fail(input);
        }
        if let Some(previous) = *element {
            if !element_follows(previous, &name) {
                return fail(input);
            }
        }
        out.push_str("::");
        out.push_str(&name);
        if functional {
            if !FUNCTIONAL_ELEMENTS.contains(&name.as_str()) {
                return fail(input);
            }
            out.push('(');
            input.parse_nested_block(|input| element_arguments(input, &name, out, state))?;
            out.push(')');
        } else if let Some(webkit) = name.strip_prefix("-webkit-") {
            if CLASSES.contains(&name.as_str()) {
                return fail(input);
            }
            state.supported &= WEBKIT_ELEMENTS.contains(&webkit);
        } else if !ELEMENTS.contains(&name.as_str()) {
            return fail(input);
        }
        *element = Some(element_kind(&name));
        return Ok(());
    }
    if let Some(previous) = *element {
        // :is()/:where() survive after any pseudo-element, filtering their
        // branches by what that pseudo-element accepts.
        let wrapper = functional && matches!(name.as_str(), "is" | "where");
        let negation = functional && name == "not" && previous == Element::Shadow;
        if !wrapper && !negation && !class_follows(previous, &name) {
            return fail(input);
        }
    }
    out.push(':');
    out.push_str(&name);
    if !functional {
        return if CLASSES.contains(&name.as_str()) {
            Ok(())
        } else {
            fail(input)
        };
    }
    if !FUNCTIONAL_CLASSES.contains(&name.as_str()) {
        return fail(input);
    }
    out.push('(');
    let after = *element;
    input.parse_nested_block(|input| class_arguments(input, &name, context, after, out, state))?;
    out.push(')');
    Ok(())
}

fn joined(out: &mut String, items: Vec<String>) {
    out.push_str(&items.join(", "));
}

fn class_arguments<'i>(
    input: &mut Parser<'i, '_>,
    name: &str,
    context: Context,
    after: Option<Element>,
    out: &mut String,
    state: &mut State,
) -> Fallible<'i> {
    let nested = Context {
        in_has: context.in_has,
        no_elements: true,
        after,
        ..Default::default()
    };
    match name {
        "is" | "where" => joined(out, list(input, nested, true, state)?),
        "not" => joined(out, list(input, nested, false, state)?),
        "has" if !context.in_has => {
            let nested = Context {
                relative: true,
                in_has: true,
                ..nested
            };
            joined(out, list(input, nested, false, state)?);
        }
        "host" | "host-context" => {
            let nested = Context {
                compound: true,
                ..nested
            };
            complex(input, nested, out, state)?;
        }
        "-webkit-any" => {
            let nested = Context {
                compound: true,
                ..nested
            };
            joined(out, list(input, nested, false, state)?);
        }
        "nth-child" | "nth-last-child" | "nth-of-type" | "nth-last-of-type" => {
            skip_whitespace(input);
            let (a, b) = cssparser::parse_nth(input).map_err(basic)?;
            out.push_str(&nth(a, b));
            skip_whitespace(input);
            if !input.is_exhausted() && name.starts_with("nth-") && !name.ends_with("of-type") {
                if !identifier(input)?.eq_ignore_ascii_case("of") {
                    return fail(input);
                }
                // Chrome's `of` branch keeps pseudo-elements admissible.
                let nested = Context {
                    no_elements: false,
                    ..nested
                };
                out.push_str(" of ");
                joined(out, list(input, nested, false, state)?);
            }
        }
        "dir" | "lang" | "state" => {
            skip_whitespace(input);
            let start = input.position();
            identifier(input)?;
            out.push_str(input.slice_from(start));
            skip_whitespace(input);
        }
        "active-view-transition-type" => {
            let mut names = Vec::new();
            loop {
                skip_whitespace(input);
                let start = input.position();
                identifier(input)?;
                names.push(input.slice_from(start).to_owned());
                skip_whitespace(input);
                match input.next() {
                    Ok(Token::Comma) => {}
                    Ok(_) => return fail(input),
                    Err(_) => break,
                }
            }
            joined(out, names);
        }
        _ => return fail(input),
    }
    input.expect_exhausted().map_err(basic)
}

// CSSOM An+B serialization.
fn nth(a: i32, b: i32) -> String {
    let mut text = match a {
        0 => return b.to_string(),
        1 => "n".to_owned(),
        -1 => "-n".to_owned(),
        _ => format!("{a}n"),
    };
    if b > 0 {
        text.push_str(&format!("+{b}"));
    } else if b < 0 {
        text.push_str(&b.to_string());
    }
    text
}

fn element_arguments<'i>(
    input: &mut Parser<'i, '_>,
    name: &str,
    out: &mut String,
    state: &mut State,
) -> Fallible<'i> {
    let nested = Context {
        no_elements: true,
        ..Default::default()
    };
    skip_whitespace(input);
    match name {
        "slotted" => {
            let nested = Context {
                compound: true,
                ..nested
            };
            complex(input, nested, out, state)?;
        }
        "cue" => joined(out, list(input, nested, false, state)?),
        "part" => {
            let mut names = Vec::new();
            while !input.is_exhausted() {
                let start = input.position();
                identifier(input)?;
                names.push(input.slice_from(start).to_owned());
                skip_whitespace(input);
            }
            if names.is_empty() {
                return fail(input);
            }
            out.push_str(&names.join(" "));
        }
        "highlight" => {
            let start = input.position();
            identifier(input)?;
            out.push_str(input.slice_from(start));
        }
        "picker" => {
            if !identifier(input)?.eq_ignore_ascii_case("select") {
                return fail(input);
            }
            out.push_str("select");
        }
        "scroll-button" => {
            if delim(input, '*') {
                out.push('*');
            } else {
                let value = lower(&identifier(input)?);
                if !SCROLL_BUTTONS.contains(&value.as_str()) {
                    return fail(input);
                }
                out.push_str(&value);
            }
        }
        // <pt-name-selector>: `*` or a name, then optional `.class` tags.
        _ => {
            let start = input.position();
            let named = delim(input, '*') || {
                let before = input.state();
                let ident = matches!(input.next_including_whitespace(), Ok(Token::Ident(_)));
                if !ident {
                    input.reset(&before);
                }
                ident
            };
            let head = input.slice_from(start).to_owned();
            let classes = input.position();
            let mut tagged = false;
            while delim(input, '.') {
                identifier(input)?;
                tagged = true;
            }
            if !named && !tagged {
                return fail(input);
            }
            out.push_str(if named { &head } else { "*" });
            out.push_str(input.slice_from(classes));
        }
    }
    skip_whitespace(input);
    input.expect_exhausted().map_err(basic)
}

fn admit(source: &str) -> Option<Admitted> {
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    let mut state = State { supported: true };
    let items = input
        .parse_entirely(|input| list(input, Context::default(), false, &mut state))
        .ok()?;
    Some(Admitted {
        text: items.join(", "),
        supported: state.supported,
        complex: items.len(),
    })
}

/// Validate and serialize an author selector list. Validation is pure, so the
/// bounded memo only avoids re-tokenizing selectors repeated by DOM queries.
pub(super) fn selector_list(source: &str) -> Option<Admitted> {
    thread_local! {
        static MEMO: RefCell<HashMap<String, Option<Admitted>>> = RefCell::default();
    }
    if let Some(hit) = MEMO.with(|memo| memo.borrow().get(source).cloned()) {
        return hit;
    }
    let admitted = admit(source);
    MEMO.with(|memo| {
        let mut memo = memo.borrow_mut();
        if memo.len() >= 512 {
            memo.clear();
        }
        memo.insert(source.to_owned(), admitted.clone());
    });
    admitted
}

/// `CSS.supports("selector(...)")`: one complex selector that Chrome parses
/// without forgiving drops or compatibility pseudo-elements. turbo-test has no
/// CSS namespace; hosts that install `CSS.supports` share this predicate.
#[cfg_attr(not(test), allow(dead_code))]
pub fn supports_selector(source: &str) -> bool {
    selector_list(source).is_some_and(|admitted| admitted.supported && admitted.complex == 1)
}

#[cfg(test)]
mod tests {
    use super::{selector_list, supports_selector};

    fn text(source: &str) -> Option<String> {
        selector_list(source).map(|admitted| admitted.text)
    }

    #[test]
    fn unknown_pseudos_invalidate_unforgiving_lists_only() {
        assert_eq!(text(".target, :definitely-unknown"), None);
        assert_eq!(text(".target, ???"), None);
        assert_eq!(
            text(":is(.a, :definitely-unknown)").as_deref(),
            Some(":is(.a)")
        );
        assert_eq!(text(":where(::before)").as_deref(), Some(":where()"));
        assert_eq!(text(":has(:definitely-unknown)"), None);
        assert_eq!(text(":not(.a, :definitely-unknown)"), None);
        assert_eq!(text(":has(:is(:has(.a)))").as_deref(), Some(":has(:is())"));
        assert!(!supports_selector(":is(.a, :definitely-unknown)"));
        assert!(supports_selector(":is(:hover, :-webkit-autofill)"));
    }

    #[test]
    fn vendor_compatibility_and_placement_follow_chrome() {
        assert!(text("::-webkit-definitely-unknown").is_some());
        assert!(!supports_selector("::-webkit-definitely-unknown"));
        assert!(supports_selector(
            "::-webkit-scrollbar-thumb:horizontal:active"
        ));
        assert_eq!(text("::-webkit-full-page-media"), None);
        assert_eq!(text(":-moz-focusring"), None);
        assert_eq!(text("::before:hover"), None);
        assert_eq!(text("::before .a"), None);
        assert_eq!(
            text("::before:is(:hover)").as_deref(),
            Some("::before:is()")
        );
        assert_eq!(text(":BEFORE").as_deref(), Some("::before"));
        assert_eq!(text(":NTH-CHILD(odd)").as_deref(), Some(":nth-child(2n+1)"));
        assert_eq!(text("A.B>.c").as_deref(), Some("a.B > .c"));
    }
}

//! The style system (§4): CSS parsing, selector matching on turbo-dom's selector
//! engine, named style tokens,
//! render-time node styles, and cascade + inheritance into a styled tree from a
//! turbo-dom [`Tree`](turbo_dom::Tree) ([`style_input`] over a [`TreeView`]; the
//! live-DOM input is [`crate::live`]).

mod cascade;
mod parser;
mod quad_shorthand;
mod token;

pub(crate) use cascade::presentational_declarations_of;
pub use cascade::{
    presentational_declarations, resolve_matched_style, style_input, Cascade, CascadeOrigin,
    ComputedStyle, LeveledRule, MatchedDeclaration, Navigator, Roots, SiblingPosition, StyleInput,
    StyleOptions, StyledElement, StyledNode, StyledText, TreeView,
};
pub use parser::{
    parse_declarations, parse_stylesheet, AtRule, Declaration, Rule, RuleSelectors, Stylesheet,
};
pub use token::{StyleToken, TokenSet};

/// A minimal user-agent stylesheet (§4.2): the defaults the cascade starts from.
/// Block elements need no rule (display resolves to `block` by default); this
/// declares the inline, table, and list-item defaults plus a few typographic
/// ones so real HTML lays out without author CSS.
///
/// The `font-family` defaults pin the document to the CSS generic families so a
/// doc renders with no author CSS: `body` resolves to `sans-serif` (inherited by
/// everything) and code-ish elements to `monospace`. With the default
/// `bundled-fonts` feature the [`FontRegistry`](crate::FontRegistry) expands
/// `sans-serif` → Inter → Roboto, `serif` → Liberation Serif → PT Serif, and
/// `monospace` → Fira Code → IBM Plex Mono, so these resolve to embedded faces
/// with no caller fonts.
///
/// Chrome 156's `html.css` makes `button`, `select`, `meter`, `progress` and the
/// button-like / checkable / color / search inputs `box-sizing: border-box`
/// (miaskiewicz/turbo-dom#193); text inputs, `input[type=file]` and `textarea` stay
/// content-box.
const UA_CSS: &str = "
body { font-family: sans-serif }
code, kbd, samp, pre { font-family: monospace }
pre, listing, xmp, plaintext { white-space: pre }
b { font-weight: bold }
strong { font-weight: bold }
i { font-style: italic }
em { font-style: italic }
a { color: #0000ee; display: inline }
h1 { font-weight: bold; font-size: 2em; margin: 0.67em 0 }
h2 { font-weight: bold; font-size: 1.5em; margin: 0.83em 0 }
h3 { font-weight: bold; font-size: 1.17em; margin: 1em 0 }
h4 { font-weight: bold; margin: 1.33em 0 }
h5 { font-weight: bold; font-size: 0.83em; margin: 1.67em 0 }
h6 { font-weight: bold; font-size: 0.67em; margin: 2.33em 0 }
p { margin: 1em 0 }
ul, ol { margin: 1em 0; padding-left: 40px }
small { font-size: 0.8em; display: inline }
span { display: inline }
b, strong, i, em { display: inline }
code, kbd, samp { display: inline }
sub, sup, abbr, cite, q, mark, u, s, label, time { display: inline }
sub, sup { font-size: 0.83em }
sub { vertical-align: sub }
sup { vertical-align: super }
table { display: table; text-align: left; box-sizing: border-box; border-spacing: 2px }
thead { display: table-header-group }
tfoot { display: table-footer-group }
tr { display: table-row }
caption { display: table-caption; text-align: -webkit-center }
td, th { display: table-cell; padding: 1px }
th { font-weight: bold; text-align: center }
li { display: list-item }
center { display: block; text-align: -webkit-center }
hr { display: block; height: 1px; background-color: #cccccc }
input { display: inline-block; width: 150px; height: 1.4em; border: 1px solid #a9a9a9; background-color: #ffffff }
input[type=hidden] { display: none }
textarea { display: inline-block; width: 200px; height: 3em; border: 1px solid #a9a9a9; background-color: #ffffff }
select, button { display: inline-block; border: 1px solid #a9a9a9; background-color: #efefef }
button, select, meter, progress, input[type=button], input[type=submit], input[type=reset], input[type=color], input[type=checkbox], input[type=radio], input[type=search] { box-sizing: border-box }
template { display: none }
";

/// The user-agent rules a whole HTML document adds to [`UA_CSS`] when it is styled
/// from its root element ([`Roots::Document`], the HTML drive): Chrome's `<head>` is
/// not rendered and `<body>` has an 8px margin. The flows html2pdf styles detached
/// (template bodies, notes, regions) never contain these elements, and the live-DOM
/// input brings its own supplement, so these rules stay out of [`UA_CSS`].
const DOCUMENT_UA_CSS: &str = "
head { display: none }
body { margin: 8px }
";

static DOCUMENT_UA_SHEET: std::sync::LazyLock<Stylesheet> =
    std::sync::LazyLock::new(|| parse_stylesheet(DOCUMENT_UA_CSS));

/// Chrome 156's user-agent rules for the fullscreen element (its `fullscreen.css`,
/// miaskiewicz/turbo-dom#82): fixed to the viewport at 100% x 100%, border-box, no margin
/// or transform, its min / max sizes reset, `object-fit: contain`. The internal
/// `-internal-top-layer` value puts its box in the top layer (laid out above everything
/// else as a child of the initial containing block, with its `::backdrop`). They match
/// only an element in the tree's fullscreen stack (`Tree::fullscreen_stack`), so a
/// cascade carries them only when that stack is non-empty
/// ([`Cascade::with_fullscreen`], [`fullscreen_user_agent_css`]): a document without
/// fullscreen pays nothing for them.
const FULLSCREEN_UA_CSS: &str = ":not(:root):fullscreen { object-fit: contain; \
position: fixed !important; inset: 0 !important; margin: 0 !important; \
box-sizing: border-box !important; min-width: 0 !important; max-width: none !important; \
min-height: 0 !important; max-height: none !important; width: 100% !important; \
height: 100% !important; transform: none !important; \
-internal-top-layer: fullscreen !important }";

static FULLSCREEN_UA_SHEET: std::sync::LazyLock<Stylesheet> =
    std::sync::LazyLock::new(|| parse_stylesheet(FULLSCREEN_UA_CSS));

/// The user-agent rules an embedder adds to [`user_agent_css`] while its tree's
/// fullscreen stack is non-empty (miaskiewicz/turbo-dom#82; see [`Cascade::with_fullscreen`]).
#[must_use]
pub fn fullscreen_user_agent_css() -> &'static str {
    FULLSCREEN_UA_CSS
}

impl Cascade {
    /// This cascade for a whole HTML document styled from its root element: the
    /// document UA rules (`head { display: none }`, `body { margin: 8px }`) join the UA
    /// level, and `document_css` — the document's own `<style>` sheets in document order,
    /// plus any linked sheets the caller fetched — is author CSS ordered BEFORE this
    /// cascade's own author rules (a caller's extra CSS keeps winning ties). `@media` /
    /// `@supports` blocks in `document_css` are evaluated against the current viewport
    /// (see [`build_cascade_with_width`]).
    #[must_use]
    pub fn for_document(&self, document_css: &str) -> Cascade {
        let mut rules = Vec::with_capacity(self.rules.len());
        let mut order = 0;
        let (ua, rest): (Vec<_>, Vec<_>) = self.rules.iter().partition(|r| r.level == 0);
        for r in ua {
            rules.push(LeveledRule { order, ..r.clone() });
            order += 1;
        }
        add_leveled(&mut rules, &mut order, 0, DOCUMENT_UA_SHEET.clone());
        add_leveled(&mut rules, &mut order, 1, parse_stylesheet(document_css));
        for r in rest {
            rules.push(LeveledRule { order, ..r.clone() });
            order += 1;
        }
        Cascade {
            rules,
            tokens: self.tokens.clone(),
        }
    }
}

impl Cascade {
    /// This cascade plus the fullscreen element's user-agent rules (at the UA level,
    /// after its other UA rules) — for a tree whose fullscreen stack is non-empty
    /// (miaskiewicz/turbo-dom#82). The HTML drive adds them itself.
    #[must_use]
    pub fn with_fullscreen(&self) -> Cascade {
        let mut rules = self.rules.clone();
        // A cascade numbers its rules 0, 1, ... in order: the fullscreen rules follow.
        let mut order = rules.len();
        add_leveled(&mut rules, &mut order, 0, FULLSCREEN_UA_SHEET.clone());
        Cascade {
            rules,
            tokens: self.tokens.clone(),
        }
    }
}

/// The same UA declarations used by the ordinary cascade. A DOM embedder can
/// match these against each original tree scope before resolving matched styles.
#[must_use]
pub fn user_agent_stylesheet() -> Stylesheet {
    parse_stylesheet(UA_CSS)
}

/// Original UA source for embedders that use a different selector parser.
#[must_use]
pub fn user_agent_css() -> &'static str {
    UA_CSS
}

/// Add `sheet`'s rules at `level` in source order. A `@media` / `@supports` block
/// whose condition holds contributes its rules (recursively, so nested blocks too)
/// at the block's own source position, as Chrome orders every style rule by where
/// it appears whether or not it is nested (miaskiewicz/turbo-dom#24): a conditional
/// rule before a plain rule of equal specificity loses to it. Non-matching /
/// non-`screen` blocks and every other at-rule are dropped.
fn add_leveled(rules: &mut Vec<LeveledRule>, order: &mut usize, level: u8, sheet: Stylesheet) {
    let positions: Vec<usize> = (0..sheet.at_rules.len())
        .map(|i| sheet.at_rule_position(i))
        .collect();
    let mut at_rules = sheet.at_rules.into_iter().zip(positions).peekable();
    for (index, rule) in sheet.rules.into_iter().enumerate() {
        while let Some((at, _)) = at_rules.next_if(|(_, position)| *position <= index) {
            add_conditional(rules, order, level, &at);
        }
        rules.push(LeveledRule {
            level,
            order: *order,
            rule,
        });
        *order += 1;
    }
    for (at, _) in at_rules {
        add_conditional(rules, order, level, &at);
    }
}

/// Add a conditional group rule's contents when its condition holds (see
/// [`add_leveled`]).
fn add_conditional(rules: &mut Vec<LeveledRule>, order: &mut usize, level: u8, at: &AtRule) {
    let apply = match at.name.as_str() {
        "media" => media_matches(&at.prelude, VIEWPORT_WIDTH.get(), VIEWPORT_HEIGHT.get()),
        "supports" => supports_matches(&at.prelude),
        _ => false,
    };
    if apply {
        add_leveled(rules, order, level, parse_stylesheet(&at.body));
    }
}

/// Whether an `@supports` condition holds for this engine. We render like a modern
/// browser, so a bare feature test `(prop: value)` is treated as supported; `not`
/// negates and `and`/`or` combine. This makes progressive-enhancement blocks —
/// e.g. Wikipedia's `@supports (display:grid){ .vector-pinned-container{display:
/// block} }`, which un-hides the sidebar TOC + appearance panel — actually apply,
/// while their `not(...)` legacy fallbacks stay dropped.
fn supports_matches(cond: &str) -> bool {
    supports_or(&cond.trim().to_ascii_lowercase())
}

/// Split the (already lowercased) `s` on the whole word `op` (space-delimited) at
/// paren depth 0.
fn split_top<'a>(s: &'a str, op: &str) -> Vec<&'a str> {
    let needle = format!(" {op} ");
    let (mut out, mut depth, mut start, bytes) = (Vec::new(), 0i32, 0usize, s.as_bytes());
    let mut i = 0;
    while i < s.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth == 0 && s[i..].starts_with(&needle) {
            out.push(&s[start..i]);
            i += needle.len();
            start = i;
            continue;
        }
        i += 1;
    }
    out.push(&s[start..]);
    out
}

fn supports_or(s: &str) -> bool {
    let parts = split_top(s, "or");
    if parts.len() > 1 {
        return parts.iter().any(|p| supports_and(p.trim()));
    }
    supports_and(s)
}

fn supports_and(s: &str) -> bool {
    let parts = split_top(s, "and");
    if parts.len() > 1 {
        return parts.iter().all(|p| supports_unary(p.trim()));
    }
    supports_unary(s)
}

fn supports_unary(s: &str) -> bool {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("not ").or_else(|| s.strip_prefix("not(")) {
        // `not(` keeps the paren the strip consumed only for the spaced form; re-add
        // it for the parenless `not(...)` case so the group parses.
        let inner = if s.starts_with("not(") { &s[3..] } else { rest };
        return !supports_unary(inner);
    }
    match strip_group(s) {
        // A parenthesised group holding an operator expression → recurse.
        Some(inner) if is_group_expr(inner) => supports_or(inner),
        // A leaf `(property: value)` → assume supported; any other parenthesised text
        // (`(not foo)`, `(foo)`) is <general-enclosed>, which is false (CSS Conditional
        // 3 §2.1; Chrome 156, miaskiewicz/turbo-dom#123).
        Some(inner) => is_declaration(inner),
        // Parenthesised text that is not one group (`(not a) (b)`) does not parse, so
        // Chrome drops the rule; a bare function (`selector(...)`) → assume supported.
        None => !s.starts_with('('),
    }
}

/// Whether a leaf group's inside is a `property: value` declaration: an identifier
/// (custom properties included) before the colon.
fn is_declaration(inner: &str) -> bool {
    inner.split_once(':').is_some_and(|(name, _)| {
        let name = name.trim();
        !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    })
}

/// Whether a group's inside is a nested operator expression (holds a sub-group or a
/// top-level `and`/`or`) rather than a single `(property: value)` leaf.
fn is_group_expr(inner: &str) -> bool {
    inner.contains('(') || split_top(inner, "and").len() > 1 || split_top(inner, "or").len() > 1
}

/// The inside of `s` if it is exactly one matching-paren group, else `None`.
fn strip_group(s: &str) -> Option<&str> {
    let inner = s.trim().strip_prefix('(')?.strip_suffix(')')?;
    // Confirm the opening paren matches the closing one (not `(a) and (b)`).
    let mut depth = 0i32;
    for ch in inner.chars() {
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => return None,
            ')' => depth -= 1,
            _ => {}
        }
    }
    (depth == 0).then_some(inner)
}

/// The viewport width (px) `@media` width conditions are evaluated against for the
/// current cascade build. A thread-local so `build_cascade`'s signature (used by
/// many callers) stays unchanged; the screenshot path sets it per render via
/// [`build_cascade_with_width`].
mod viewport {
    use std::cell::Cell;
    thread_local!(static WIDTH: Cell<f32> = const { Cell::new(1280.0) });
    thread_local!(static HEIGHT: Cell<f32> = const { Cell::new(800.0) });
    pub(super) struct ViewportWidth;
    #[allow(clippy::unused_self)] // zero-sized handle: the method form mirrors the thread-local it wraps
    impl ViewportWidth {
        pub(super) fn get(&self) -> f32 {
            WIDTH.with(Cell::get)
        }
        pub(super) fn set(&self, w: f32) {
            WIDTH.with(|c| c.set(w));
        }
    }
    pub(super) struct ViewportHeight;
    #[allow(clippy::unused_self)] // zero-sized handle: the method form mirrors the thread-local it wraps
    impl ViewportHeight {
        pub(super) fn get(&self) -> f32 {
            HEIGHT.with(Cell::get)
        }
        pub(super) fn set(&self, h: f32) {
            HEIGHT.with(|c| c.set(h));
        }
    }
}
static VIEWPORT_WIDTH: viewport::ViewportWidth = viewport::ViewportWidth;

thread_local!(static ROOT_FONT: std::cell::Cell<f32> = const { std::cell::Cell::new(16.0) });

/// The root element's computed font size (px) that `rem` lengths resolve against on
/// this thread: the initial 16px unless a [`root_font_scope`] is active.
#[must_use]
pub fn root_font_px() -> f32 {
    ROOT_FONT.with(std::cell::Cell::get)
}

/// Restores the previous root font size when dropped (see [`root_font_scope`]).
#[must_use = "the root font size is restored when the scope is dropped"]
pub struct RootFontScope(f32);

impl Drop for RootFontScope {
    fn drop(&mut self) {
        ROOT_FONT.with(|c| c.set(self.0));
    }
}

/// Make `rem` resolve against `px` on this thread until the returned guard drops —
/// the root element's computed font size while its document is styled and laid out.
/// Styling a [`Roots::Document`] scope sets it for the root element's descendants;
/// the HTML drive (and html2pdf's document render) holds one around layout, where
/// `rem` lengths resolve ([`document_root_font_px`]).
pub fn root_font_scope(px: f32) -> RootFontScope {
    RootFontScope(ROOT_FONT.with(|c| c.replace(px)))
}

/// The computed font size (px) of a styled document's root element — the top-level
/// `<html>` element of a [`Roots::Document`] styling — or the initial 16px when the
/// styled tree has none (a detached flow).
#[must_use]
pub fn document_root_font_px(styled: &[StyledNode]) -> f32 {
    styled
        .iter()
        .filter_map(StyledNode::as_element)
        .find(|e| matches!(&e.tag, crate::tag::Tag::Html(name) if name == "html"))
        .map_or(16.0, |e| cascade::computed_font_px(&e.style))
}
static VIEWPORT_HEIGHT: viewport::ViewportHeight = viewport::ViewportHeight;

/// Whether a `@media` query's condition matches `width` (px). Supports a comma
/// list (any clause matches), the `screen`/`all` types (`print` never matches;
/// no/other type is treated as applicable), and `min-width`/`max-width` features
/// in `px`/`em`/`rem` (`em`/`rem` against a 16px root). Other features are ignored
/// (a clause with only unknown features still matches if its type does).
fn media_matches(query: &str, width: f32, height: f32) -> bool {
    let q = query.to_ascii_lowercase();
    q.split(',')
        .any(|clause| clause_matches(clause.trim(), width, height))
}

fn clause_matches(clause: &str, width: f32, height: f32) -> bool {
    if clause.contains("print") {
        return false;
    }
    // A clause matches unless one of its `and`-joined length features excludes the
    // viewport. Height features (Google hides its tall search box below
    // `max-height:575px`) matter as much as width — evaluate both.
    !clause
        .split("and")
        .any(|feat| feature_excludes(feat.trim(), width, height))
}

/// Whether a single media feature's `(min|max)-(width|height)` bound excludes the
/// viewport (so its clause must NOT match). Unknown features never exclude.
fn feature_excludes(feat: &str, width: f32, height: f32) -> bool {
    // (feature name, is-min bound, is-width axis).
    const CHECKS: [(&str, bool, bool); 4] = [
        ("min-width", true, true),
        ("max-width", false, true),
        ("min-height", true, false),
        ("max-height", false, false),
    ];
    CHECKS.iter().any(|&(name, is_min, is_width)| {
        feature_len(feat, name).is_some_and(|bound| {
            let v = if is_width { width } else { height };
            if is_min {
                v < bound
            } else {
                v > bound
            }
        })
    })
}

/// The px value of a `(min-width: …)` / `(max-width: …)` feature in `clause`.
fn feature_len(feat: &str, name: &str) -> Option<f32> {
    let rest = feat.split_once(name)?.1.trim_start();
    // Drop the leading `:` and the feature's own closing paren; a `calc(...)` value
    // keeps its own parens, so strip exactly one trailing `)` (the feature's).
    let val = rest.strip_prefix(':')?.trim();
    let val = val.strip_suffix(')').unwrap_or(val).trim();
    if let Some(inner) = val.strip_prefix("calc(").and_then(|s| s.strip_suffix(')')) {
        return eval_calc(inner);
    }
    parse_len(val)
}

/// A single length token → px (`px` bare, or `em`/`rem` against a 16px root).
fn parse_len(val: &str) -> Option<f32> {
    let val = val.trim();
    if let Some(em) = val.strip_suffix("rem").or_else(|| val.strip_suffix("em")) {
        return em.trim().parse::<f32>().ok().map(|n| n * 16.0);
    }
    val.trim_end_matches("px").trim().parse::<f32>().ok()
}

/// Evaluate a minimal `calc()` body of two length terms — `A + B` / `A - B` (as in
/// Vector's `calc(1120px - 1px)` breakpoint). Falls back to the single term, or a
/// bare parse, so unknown shapes don't silently drop the whole media condition.
fn eval_calc(inner: &str) -> Option<f32> {
    for (op, sign) in [(" - ", -1.0), (" + ", 1.0)] {
        if let Some((a, b)) = inner.split_once(op) {
            return Some(parse_len(a)? + sign * parse_len(b)?);
        }
    }
    parse_len(inner)
}

/// Set the validated continuous-media viewport for layout unit resolution.
pub(crate) fn set_layout_viewport(width: f32, height: f32) {
    VIEWPORT_WIDTH.set(width);
    VIEWPORT_HEIGHT.set(height);
}

/// The user-agent stylesheet, parsed once. It is a fixed constant, so parsing it
/// on every `build_cascade` (i.e. every render) is pure waste; cache the parse
/// and clone its rules into each cascade instead.
static UA_SHEET: std::sync::LazyLock<Stylesheet> =
    std::sync::LazyLock::new(|| parse_stylesheet(UA_CSS));

/// Build a [`Cascade`] from author CSS, render-time node-style CSS, and tokens.
/// The UA stylesheet is layered underneath automatically (§4.2).
pub fn build_cascade(author_css: &str, node_style_css: &str, tokens: TokenSet) -> Cascade {
    let mut rules = Vec::new();
    let mut order = 0;
    add_leveled(&mut rules, &mut order, 0, UA_SHEET.clone());
    add_leveled(&mut rules, &mut order, 1, parse_stylesheet(author_css));
    add_leveled(&mut rules, &mut order, 3, parse_stylesheet(node_style_css));
    Cascade { rules, tokens }
}

/// Like [`build_cascade`] but evaluates `@media` width conditions against
/// `viewport_width` px (instead of the 1280px desktop default). The screenshot
/// tier passes its viewport so responsive stylesheets pick the right breakpoint.
#[must_use]
pub fn build_cascade_with_width(
    author_css: &str,
    node_style_css: &str,
    tokens: TokenSet,
    viewport_width: f32,
) -> Cascade {
    VIEWPORT_WIDTH.set(viewport_width);
    build_cascade(author_css, node_style_css, tokens)
    // NB: the viewport width is left set (not reset to the 1280 default) so the same
    // value backs `vw`/`vmin`/`vmax` unit resolution during the layout pass that
    // follows this cascade build (see `viewport_px`).
}

/// The layout viewport `(width, height)` px that `vw`/`vh`/`vmin`/`vmax` units
/// resolve against — the last values set by [`build_cascade_with_width`] +
/// [`set_media_viewport_height`] (the screenshot tier's canvas size), or the
/// 1280x800 desktop default.
pub(crate) fn viewport_px() -> (f32, f32) {
    (VIEWPORT_WIDTH.get(), VIEWPORT_HEIGHT.get())
}

/// Set the viewport WIDTH (px) `@media` width conditions and `vw` units evaluate against
/// for the next cascade build and layout on this thread, as [`build_cascade_with_width`]
/// does — for embedders that resolve their own media rules (the live-DOM input).
pub fn set_media_viewport_width(viewport_width: f32) {
    VIEWPORT_WIDTH.set(viewport_width);
}

/// Set the viewport HEIGHT (px) `@media (min-height:…)`/`(max-height:…)` conditions
/// evaluate against for the next cascade build on this thread. Defaults to 800.
/// The screenshot tier sets this to its canvas height so height-gated responsive
/// variants (e.g. Google's tall vs compact search box) pick the right one; without
/// it every height feature was ignored and its rules always applied.
pub fn set_media_viewport_height(viewport_height: f32) {
    if viewport_height > 0.0 {
        VIEWPORT_HEIGHT.set(viewport_height);
    }
}

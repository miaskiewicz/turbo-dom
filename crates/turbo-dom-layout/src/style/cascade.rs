//! Cascade + computed-style resolution (§4.2–4.3). Produces a styled tree from
//! the node tree by matching selectors, applying style tokens and render-time
//! node styles, and folding in inheritance. Cascade order (low→high):
//! UA < author sheets < tokens < node styles < inline < `!important` (AC-4.9).

use std::collections::BTreeMap;

use super::parser::{parse_declarations, Declaration, Rule};
use super::token::{resolve_tokens, TokenSet};
use crate::node::{Attr, Element, Node, Tag};
use turbo_dom::rtdom::query::Specificity;
use turbo_dom::Handle;

/// Computed property values for one element.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComputedStyle {
    map: BTreeMap<String, String>,
}

impl ComputedStyle {
    /// The computed value of a property, if set.
    pub fn get(&self, property: &str) -> Option<&str> {
        self.map.get(property).map(String::as_str)
    }

    /// Every computed property and value, in property-name order. Consumers that
    /// derive used values (for example CSS `zoom` scaling) rebuild a style from
    /// these pairs with [`ComputedStyle::from_pairs`].
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.map
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// Anonymous live formatting boxes inherit text properties, never the
    /// parent's frame/transform/position a second time.
    pub(crate) fn inherited_only(&self) -> Self {
        Self::from_pairs(
            INHERITED
                .iter()
                .filter_map(|&name| self.get(name).map(|value| (name, value))),
        )
    }

    pub(crate) fn inline_fragment_style(&self, first: bool, last: bool) -> Self {
        let mut style = self.clone();
        for (side, retained) in [("left", first), ("right", last)] {
            if !retained {
                for property in [
                    format!("margin-{side}"),
                    format!("padding-{side}"),
                    format!("border-{side}-width"),
                ] {
                    style.map.insert(property, "0px".into());
                }
            }
        }
        style
    }

    /// Whether no value uses a containing-block-relative (`%`) or font-relative
    /// (`em`/`rem`) unit. When true, the box's resolved metrics do not depend on
    /// the layout context, so they can be resolved once and cached (a `font-size`
    /// is still required to be absolute — see `is_ctx_independent`). Conservative:
    /// any `%`/`em` substring (even inside a value like `lemonchiffon`) just
    /// disables the cache, which is always safe.
    pub fn has_no_relative_units(&self) -> bool {
        self.map
            .values()
            .all(|v| !v.contains('%') && !v.contains("em"))
    }

    /// Build a computed style directly from property/value pairs. Useful for
    /// programmatic node styles and for layout-level testing.
    pub fn from_pairs<I, K, V>(pairs: I) -> ComputedStyle
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let map = pairs
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        ComputedStyle { map }
    }
}

/// A styled element node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledElement {
    pub tag: Tag,
    pub attrs: Vec<Attr>,
    pub style: ComputedStyle,
    pub children: Vec<StyledNode>,
    /// The turbo-dom node this element was styled from (`None` for the legacy
    /// [`Node`] input and synthetic elements). Layout's source-recording mode
    /// maps box ids back to it.
    pub source: Option<Handle>,
}

/// A styled text node: its text plus the turbo-dom text node it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledText {
    pub text: String,
    /// The source text node (`None` for the legacy [`Node`] input and synthetic text).
    pub source: Option<Handle>,
}

impl std::ops::Deref for StyledText {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}

impl From<String> for StyledText {
    fn from(text: String) -> Self {
        StyledText { text, source: None }
    }
}

impl From<&str> for StyledText {
    fn from(text: &str) -> Self {
        StyledText {
            text: text.to_string(),
            source: None,
        }
    }
}

/// A node in the styled tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyledNode {
    Element(StyledElement),
    Text(StyledText),
}

impl StyledNode {
    /// Borrow the styled element if this node is one.
    pub fn as_element(&self) -> Option<&StyledElement> {
        match self {
            StyledNode::Element(e) => Some(e),
            StyledNode::Text(_) => None,
        }
    }

    /// This subtree with every `source` cleared — the comparison key between the
    /// legacy [`Node`] input and the turbo-dom [`TreeView`](super::TreeView) input,
    /// which differ only in that provenance.
    #[must_use]
    pub fn without_sources(&self) -> StyledNode {
        match self {
            StyledNode::Element(e) => StyledNode::Element(StyledElement {
                tag: e.tag.clone(),
                attrs: e.attrs.clone(),
                style: e.style.clone(),
                children: e.children.iter().map(StyledNode::without_sources).collect(),
                source: None,
            }),
            StyledNode::Text(t) => StyledNode::Text(t.text.as_str().into()),
        }
    }
}

/// A rule tagged with its cascade level and source order.
#[derive(Debug, Clone)]
pub struct LeveledRule {
    pub level: u8,
    pub order: usize,
    pub rule: Rule,
}

/// Resolved cascade input shared across a render.
#[derive(Debug, Clone, Default)]
pub struct Cascade {
    pub rules: Vec<LeveledRule>,
    pub tokens: TokenSet,
}

/// Properties that inherit by default (§4.2).
const INHERITED: &[&str] = &[
    "color",
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "line-height",
    "text-align",
    "text-indent",
    "letter-spacing",
    "word-spacing",
    "white-space",
    "text-transform",
    "hyphens",
    "list-style-type",
    "list-style-position",
    "visibility",
];

// --------------------------------------------------------------------------
// the element being styled
// --------------------------------------------------------------------------

/// Where the non-selector cascade inputs (presentational hints, tokens, inline style,
/// `cellpadding`) read an element's attributes: the legacy owned node's list, or a
/// turbo-dom element (by local attribute name, in attribute order — the same identities
/// html5ever's `name.local` gave the legacy conversion).
#[derive(Clone, Copy)]
pub(crate) enum Attrs<'a> {
    Node(&'a [Attr]),
    Tree(&'a turbo_dom::Tree, Handle),
}

impl<'a> Attrs<'a> {
    /// The first attribute named exactly `name`.
    pub(crate) fn get(self, name: &str) -> Option<&'a str> {
        match self {
            Attrs::Node(attrs) => attrs
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.value.as_str()),
            Attrs::Tree(tree, h) => tree.get_attribute_local(h, name),
        }
    }

    /// The first attribute whose name equals `name` ignoring ASCII case.
    pub(crate) fn get_ignore_case(self, name: &str) -> Option<&'a str> {
        match self {
            Attrs::Node(attrs) => attrs
                .iter()
                .find(|a| a.name.eq_ignore_ascii_case(name))
                .map(|a| a.value.as_str()),
            Attrs::Tree(tree, h) => tree
                .local_attributes(h)
                .find(|(local, _)| local.eq_ignore_ascii_case(name))
                .map(|(_, value)| value),
        }
    }
}

/// The element being styled, as the non-selector inputs see it: its local name (`None`
/// for a marker) and attributes.
#[derive(Clone, Copy)]
pub(crate) struct Subject<'a> {
    pub(crate) tag: Option<&'a str>,
    pub(crate) attrs: Attrs<'a>,
}

impl<'a> Subject<'a> {
    pub(crate) fn node(element: &'a Element) -> Self {
        Subject {
            tag: match &element.tag {
                Tag::Html(name) => Some(name),
                Tag::Marker(_) => None,
            },
            attrs: Attrs::Node(&element.attrs),
        }
    }
}

/// The `nth-of-type` grouping key: the element's local name, or a per-kind key for
/// markers that no element name can equal.
fn tag_key(tag: &Tag) -> String {
    match tag {
        Tag::Html(name) => name.clone(),
        Tag::Marker(kind) => marker_key(*kind),
    }
}

fn marker_key(kind: crate::node::MarkerKind) -> String {
    format!("\u{1}marker:{}", kind.id)
}

// --------------------------------------------------------------------------
// candidate collection + winner selection
// --------------------------------------------------------------------------

type Key = (bool, u8, Specificity, usize);

struct Cand {
    key: Key,
    property: String,
    value: String,
}

fn push_decls(
    decls: &[Declaration],
    level: u8,
    spec: Specificity,
    base: usize,
    out: &mut Vec<Cand>,
) {
    for (i, d) in decls.iter().enumerate() {
        let key = (d.important, level, spec, base + i);
        out.push(Cand {
            key,
            property: d.property.clone(),
            value: d.value.clone(),
        });
    }
}

fn collect_rules(cascade: &Cascade, nav: &Navigator, h: Handle, out: &mut Vec<Cand>) {
    for lr in &cascade.rules {
        if let Some(spec) = lr.rule.selectors.best_match(nav, h) {
            push_decls(&lr.rule.declarations, lr.level, spec, lr.order, out);
        }
    }
}

fn collect_tokens(element: Subject, cascade: &Cascade, out: &mut Vec<Cand>) {
    if let Some(t_style) = element.attrs.get("t:style") {
        let decls = resolve_tokens(t_style, &cascade.tokens);
        push_decls(&decls, 2, (0, 0, 0), 0, out);
    }
}

fn collect_inline(element: Subject, out: &mut Vec<Cand>) {
    if let Some(style) = element.attrs.get("style") {
        let decls = parse_declarations(style);
        push_decls(&decls, 4, (0, 0, 0), 0, out);
    }
}

/// Legacy presentational attributes mapped to CSS declarations (`bgcolor`,
/// `width`/`height`, `<font color>`). These are "presentational hints": they sit
/// just above the UA sheet and below any real author rule (level 1, specificity
/// 0), so an author stylesheet always overrides them. Old table-layout sites
/// (Hacker News' orange header `<td bgcolor>`, sized `<img>`) depend on them.
fn collect_presentational(element: Subject, out: &mut Vec<Cand>) {
    // Order matters (bgcolor, width, height, then `<font color>`); each entry is
    // present only when its attribute is set and usable.
    let decls: Vec<Declaration> = [
        bgcolor_decl(element),
        element
            .attrs
            .get("width")
            .and_then(attr_length)
            .map(width_decl),
        element
            .attrs
            .get("height")
            .and_then(attr_length)
            .map(height_decl),
        font_color_decl(element),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !decls.is_empty() {
        push_decls(&decls, 1, (0, 0, 0), 0, out);
    }
}

/// A presentational-hint declaration (level 1, never `important`).
fn pres_decl(property: &str, value: String) -> Declaration {
    Declaration {
        property: property.to_string(),
        value,
        important: false,
    }
}

fn width_decl(value: String) -> Declaration {
    pres_decl("width", value)
}

fn height_decl(value: String) -> Declaration {
    pres_decl("height", value)
}

/// `bgcolor="..."` → `background-color`, when non-empty.
fn bgcolor_decl(element: Subject) -> Option<Declaration> {
    let bg = element
        .attrs
        .get("bgcolor")
        .map(str::trim)
        .filter(|v| !v.is_empty())?;
    Some(pres_decl("background-color", bg.to_string()))
}

/// `<font color="...">` → `color`, when non-empty (only on the `<font>` tag).
fn font_color_decl(element: Subject) -> Option<Declaration> {
    if !is_tag(element, "font") {
        return None;
    }
    let c = element
        .attrs
        .get("color")
        .map(str::trim)
        .filter(|v| !v.is_empty())?;
    Some(pres_decl("color", c.to_string()))
}

/// A presentational length attribute → a CSS length value: a bare number is `px`,
/// a `N%` passes through; anything else (e.g. `width="*"`) is ignored.
fn attr_length(v: &str) -> Option<String> {
    let v = v.trim();
    if let Some(pct) = v.strip_suffix('%') {
        return valid_attr_length(pct).map(|_| v.to_string());
    }
    valid_attr_length(v).map(|n| format!("{n}px"))
}

fn valid_attr_length(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn is_tag(element: Subject, name: &str) -> bool {
    element.tag.is_some_and(|t| t.eq_ignore_ascii_case(name))
}

/// The `cellpadding` (px) of the nearest ancestor `<table>` (within the matching scope,
/// shells included) — a legacy table attribute that pads every cell (e.g. HN's
/// `cellpadding="1"` divider bar, an empty coloured `<td>` that must reserve its padding
/// to show as a line).
fn ancestor_cellpadding(nav: &Navigator, h: Handle) -> Option<f32> {
    let mut ancestor = nav.parent(h);
    while let Some(a) = ancestor {
        let subject = nav.subject(a);
        if subject.tag == Some("table") {
            let value = subject.attrs.get_ignore_case("cellpadding")?;
            return value.trim().parse::<f32>().ok();
        }
        ancestor = nav.parent(a);
    }
    None
}

/// Apply a table's `cellpadding` to its cells as a presentational padding hint.
fn collect_cellpadding(element: Subject, nav: &Navigator, h: Handle, out: &mut Vec<Cand>) {
    if !(is_tag(element, "td") || is_tag(element, "th")) {
        return;
    }
    if let Some(pad) = ancestor_cellpadding(nav, h) {
        let decl = Declaration {
            property: "padding".to_string(),
            value: format!("{pad}px"),
            important: false,
        };
        push_decls(&[decl], 1, (0, 0, 0), 0, out);
    }
}

fn pick_winners(cands: Vec<Cand>) -> BTreeMap<String, String> {
    let mut best: BTreeMap<String, (Key, String)> = BTreeMap::new();
    for cand in cands {
        let beats = best.get(&cand.property).is_none_or(|(k, _)| cand.key >= *k);
        if beats {
            best.insert(cand.property, (cand.key, cand.value));
        }
    }
    best.into_iter().map(|(k, (_, v))| (k, v)).collect()
}

/// The document root font size (px). CSS `rem` and the initial `font-size` both
/// resolve against it; turbo does not honor an `html { font-size }` override.
const ROOT_FONT_PX: f32 = 16.0;

/// Resolve `font-size` to an absolute px value against the parent's (already
/// absolute) font size, then store it back as `"<n>px"`. Font-relative units
/// resolve at the element that *declares* them (§4.2): if we left `em`/`%` as a
/// string, every descendant that inherits it would re-multiply against the
/// parent's px — e.g. `h1{font-size:1.8em}` compounding to 1.8²·16 on its text.
fn resolve_font_size(map: &mut BTreeMap<String, String>, parent: &ComputedStyle) {
    let parent_px = parent
        .get("font-size")
        .and_then(parse_abs_px)
        .unwrap_or(ROOT_FONT_PX);
    let Some(raw) = map.get("font-size") else {
        return;
    };
    let px = font_size_px(raw, parent_px);
    map.insert("font-size".to_string(), format!("{px}px"));
}

/// Parse an already-absolute px string (`"28.8px"` / `"16"`); `None` for anything
/// still carrying a relative unit.
fn parse_abs_px(v: &str) -> Option<f32> {
    let t = v.trim();
    t.strip_suffix("px").unwrap_or(t).trim().parse::<f32>().ok()
}

/// Resolve one `font-size` value to px against `parent_px`: `px`/`pt`/`pc`/`in`/
/// `cm`/`mm` absolute, `em`/`%` parent-relative, `rem` root-relative, and the
/// `larger`/`smaller`/`xx-small…xx-large` keywords (coarse steps).
fn font_size_px(value: &str, parent_px: f32) -> f32 {
    let t = value.trim();
    let kw = match t {
        "larger" => Some(parent_px * 1.2),
        "smaller" => Some(parent_px / 1.2),
        "xx-small" => Some(ROOT_FONT_PX * 0.6),
        "x-small" => Some(ROOT_FONT_PX * 0.75),
        "small" => Some(ROOT_FONT_PX * 0.89),
        "medium" => Some(ROOT_FONT_PX),
        "large" => Some(ROOT_FONT_PX * 1.2),
        "x-large" => Some(ROOT_FONT_PX * 1.5),
        "xx-large" => Some(ROOT_FONT_PX * 2.0),
        _ => None,
    };
    if let Some(px) = kw {
        return px;
    }
    let split = t
        .char_indices()
        .find(|(_, c)| c.is_ascii_alphabetic() || *c == '%')
        .map(|(i, _)| i)
        .unwrap_or(t.len());
    let Ok(n) = t[..split].parse::<f32>() else {
        return parent_px;
    };
    match t[split..].trim() {
        "" | "px" => n,
        "pt" => n * 96.0 / 72.0,
        "pc" => n * 16.0,
        "in" => n * 96.0,
        "cm" => n * 96.0 / 2.54,
        "mm" => n * 96.0 / 25.4,
        "em" => n * parent_px,
        "rem" => n * ROOT_FONT_PX,
        "%" => n / 100.0 * parent_px,
        _ => parent_px,
    }
}

fn inherit(own: BTreeMap<String, String>, parent: &ComputedStyle) -> ComputedStyle {
    let mut map = BTreeMap::new();
    for prop in INHERITED {
        if let Some(v) = parent.get(prop) {
            map.insert((*prop).to_string(), v.to_string());
        }
    }
    // Custom properties (`--*`) inherit by default (they're how `var()` cascades).
    for (k, v) in &parent.map {
        if k.starts_with("--") {
            map.insert(k.clone(), v.clone());
        }
    }
    map.extend(own);
    resolve_explicit_inherit(&mut map, parent);
    resolve_var_refs(&mut map);
    resolve_font_size(&mut map, parent);
    ComputedStyle { map }
}

/// Resolve the explicit `inherit` keyword: a property declared `inherit` takes the
/// parent's computed value (or drops if the parent has none). Nike's editorial cards
/// set `box-sizing:inherit`, inheriting the global `*{box-sizing:border-box}` —
/// without this their `width:50%` resolved as a content width plus padding, so two
/// cards no longer fit a row and every hero image stacked into a half-width column.
fn resolve_explicit_inherit(map: &mut BTreeMap<String, String>, parent: &ComputedStyle) {
    let keys: Vec<String> = map
        .iter()
        .filter(|(_, v)| v.trim() == "inherit")
        .map(|(k, _)| k.clone())
        .collect();
    for k in keys {
        match parent.get(&k) {
            Some(pv) => drop(map.insert(k, pv.to_string())),
            None => drop(map.remove(&k)),
        }
    }
}

/// Substitute every `var(--name, fallback)` in the map's values with the resolved
/// custom-property value (or the fallback). Custom properties are read from the
/// same map (already inherited), so `var()` sees the cascaded value. Runs after
/// inheritance so a child's `var()` picks up an ancestor's custom property.
fn resolve_var_refs(map: &mut BTreeMap<String, String>) {
    let vars: BTreeMap<String, String> = map
        .iter()
        .filter(|(k, _)| k.starts_with("--"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (key, value) in map.iter_mut() {
        if value.contains("var(") {
            *value = substitute_vars(value, &vars, 0, &[]);
            let _ = key;
        }
    }
}

/// Replace `var(--name[, fallback])` references in `value` using `vars`. Balanced
/// parens (a fallback may itself contain `var()` / functions); a missing or empty
/// custom property falls back. Depth-limited against a cyclic `--a: var(--a)`.
fn substitute_vars(
    value: &str,
    vars: &BTreeMap<String, String>,
    depth: u8,
    chain: &[&str],
) -> String {
    if depth > 16 || !value.contains("var(") {
        return value.to_string();
    }
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < bytes.len() {
        if value[i..].starts_with("var(") {
            let inner_start = i + 4;
            let Some(close) = matching_paren(value, inner_start) else {
                out.push_str(&value[i..]);
                break;
            };
            let inner = &value[inner_start..close];
            out.push_str(&resolve_one_var(inner, vars, depth, chain));
            i = close + 1;
        } else {
            let ch = value[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// The index of the `)` matching the `(` that opened just before `open` (i.e. the
/// close of the `var(` at `open-4`), accounting for nested parens.
fn matching_paren(s: &str, open: usize) -> Option<usize> {
    let mut depth = 1i32;
    for (rel, ch) in s[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + rel);
                }
            }
            _ => {}
        }
    }
    None
}

/// Resolve one `var()` body (`--name` or `--name, fallback`) to its value. `chain`
/// is the stack of custom properties currently being resolved: a name that appears
/// again is a reference cycle (`--a: var(--a, 1rem)` — Codex's "redefine a token
/// from its inherited value" idiom, which we flatten and so cannot follow up the
/// tree), so it resolves to the fallback instead of spinning to the depth cap and
/// leaving an unresolved `var()` that breaks the surrounding `calc()`.
fn resolve_one_var(
    inner: &str,
    vars: &BTreeMap<String, String>,
    depth: u8,
    chain: &[&str],
) -> String {
    let (name, fallback) = match inner.find(',') {
        Some(c) => (inner[..c].trim(), inner[c + 1..].trim()),
        None => (inner.trim(), ""),
    };
    let cyclic = chain.contains(&name);
    match vars.get(name).map(|v| v.trim()).filter(|v| !v.is_empty()) {
        Some(v) if !cyclic => {
            let mut next: Vec<&str> = chain.to_vec();
            next.push(name);
            substitute_vars(v, vars, depth + 1, &next)
        }
        _ => substitute_vars(fallback, vars, depth + 1, chain),
    }
}

fn quad_variable_context(cands: &[Cand], parent: &ComputedStyle) -> BTreeMap<String, String> {
    let custom: Vec<_> = cands
        .iter()
        .filter(|cand| cand.property.starts_with("--"))
        .map(|cand| Cand {
            key: cand.key,
            property: cand.property.clone(),
            value: cand.value.clone(),
        })
        .collect();
    let mut variables: BTreeMap<_, _> = parent
        .map
        .iter()
        .filter(|(name, _)| name.starts_with("--"))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    variables.extend(pick_winners(custom));
    resolve_explicit_inherit(&mut variables, parent);
    variables
}

fn expand_quad_candidates(cands: Vec<Cand>, parent: &ComputedStyle) -> Vec<Cand> {
    if !cands
        .iter()
        .any(|cand| super::quad_shorthand::is_quad(&cand.property))
    {
        return cands;
    }
    let variables = quad_variable_context(&cands, parent);
    let mut expanded = Vec::new();
    for cand in cands {
        expanded.extend(expanded_quad(&cand, &variables).into_iter().flatten());
        expanded.push(cand);
    }
    expanded
}

fn expanded_quad(cand: &Cand, variables: &BTreeMap<String, String>) -> Option<[Cand; 4]> {
    if !super::quad_shorthand::is_quad(&cand.property) {
        return None;
    }
    let value = substitute_vars(&cand.value, variables, 0, &[]);
    resolved_quad(&cand.property, &cand.value, &value).map(|longhands| {
        longhands.map(|(property, value)| Cand {
            key: cand.key,
            property: property.into(),
            value: value.into(),
        })
    })
}

fn resolved_quad<'a>(
    property: &str,
    original: &str,
    value: &'a str,
) -> Option<[(&'static str, &'a str); 4]> {
    super::quad_shorthand::expand(property, value).or_else(|| {
        // Invalid-at-computed-value-time variables unset the winning longhands;
        // they do not resurrect earlier declarations.
        original
            .contains("var(")
            .then(|| super::quad_shorthand::expand(property, "unset"))
            .flatten()
    })
}

/// Resolve the computed style of element `h`, matching through `nav` (its scope).
fn resolve_style(
    nav: &Navigator,
    h: Handle,
    parent: &ComputedStyle,
    cascade: &Cascade,
) -> ComputedStyle {
    let element = nav.subject(h);
    let mut cands = Vec::new();
    collect_presentational(element, &mut cands);
    collect_cellpadding(element, nav, h, &mut cands);
    collect_rules(cascade, nav, h, &mut cands);
    collect_tokens(element, cascade, &mut cands);
    collect_inline(element, &mut cands);
    inherit(pick_winners(expand_quad_candidates(cands, parent)), parent)
}

// --------------------------------------------------------------------------
// the legacy Node input (reference + turbo-html2pdf-core's Node-literal tests)
// --------------------------------------------------------------------------

/// The element name a [`Tag::Marker`] gets in the turbo-dom copy of a legacy forest (no
/// parsed or authored element can have it), and its inverse.
fn bridge_marker_name(kind: crate::node::MarkerKind) -> String {
    format!("\u{1}marker:{}:{}", kind.id, u8::from(kind.inline))
}

fn bridge_marker(local: &str) -> Option<crate::node::MarkerKind> {
    let (id, inline) = local.strip_prefix("\u{1}marker:")?.split_once(':')?;
    Some(crate::node::MarkerKind {
        id: id.parse().ok()?,
        inline: inline == "1",
    })
}

/// Style a legacy forest: copy it into a turbo-dom [`Tree`](turbo_dom::Tree)
/// ([`tree_from_nodes`](crate::tree_from_nodes)) and style the copy like any other Tree
/// input, so both inputs share one selector engine. `shells` (outermost first) become
/// match-only ancestors. The result carries no `source` handles.
fn style_forest(nodes: &[Node], cascade: &Cascade, shells: &[Element]) -> Vec<StyledNode> {
    let mut copy = crate::tree_from_nodes(nodes, &bridge_marker_name);
    let mut shell_handles = Vec::with_capacity(shells.len());
    for shell in shells {
        let name = match &shell.tag {
            Tag::Html(name) => name.clone(),
            Tag::Marker(kind) => bridge_marker_name(*kind),
        };
        let h = copy
            .tree
            .create_element_ns(&name, turbo_dom::rtdom::tree::Namespace::Html);
        for attr in &shell.attrs {
            copy.tree.set_attribute(h, &attr.name, &attr.value);
        }
        shell_handles.push(h);
    }
    let view = TreeView::new(&copy.tree).markers(&bridge_marker);
    let nav = Navigator::with_shells(&view, shell_handles, copy.roots.clone());
    tree_view::style_scope(&nav, cascade)
        .iter()
        .map(StyledNode::without_sources)
        .collect()
}

/// Build the styled tree for a flow of nodes under the given cascade.
pub fn style_tree(nodes: &[Node], cascade: &Cascade) -> Vec<StyledNode> {
    style_forest(nodes, cascade, &[])
}

/// [`style_tree`] with explicit [`StyleOptions`] (the legacy-input twin of
/// [`style_input`]'s options).
pub fn style_tree_with_options(
    nodes: &[Node],
    cascade: &Cascade,
    options: &StyleOptions,
) -> Vec<StyledNode> {
    let _cmyk = crate::layout::value::cmyk_scope(options.accept_cmyk);
    style_tree(nodes, cascade)
}

/// [`style_tree`] with `<html>`/`<body>` ancestor shells seeded as match-only
/// contexts, so descendant selectors gated on their classes resolve. The shells
/// are not laid out; they only participate in selector matching. Each shell's own
/// style resolves down the chain so the content inherits from the real `<body>`
/// computed style (nike's `html{box-sizing:border-box}` + `*{box-sizing:inherit}`).
pub fn style_tree_with_roots(
    nodes: &[Node],
    cascade: &Cascade,
    roots: &[Element],
) -> Vec<StyledNode> {
    style_forest(nodes, cascade, roots)
}

/// [`style_tree_with_roots`] with explicit [`StyleOptions`].
pub fn style_tree_with_roots_options(
    nodes: &[Node],
    cascade: &Cascade,
    roots: &[Element],
    options: &StyleOptions,
) -> Vec<StyledNode> {
    let _cmyk = crate::layout::value::cmyk_scope(options.accept_cmyk);
    style_tree_with_roots(nodes, cascade, roots)
}

#[path = "matched.rs"]
mod matched;

#[path = "tree_view.rs"]
mod tree_view;
pub(crate) use matched::presentational_declarations_of;
pub use matched::{
    presentational_declarations, resolve_matched_style, CascadeOrigin, MatchedDeclaration,
};
pub(crate) use tree_view::is_non_visual;
pub use tree_view::{
    style_input, Navigator, Roots, SiblingPosition, StyleInput, StyleOptions, TreeView,
};

#[cfg(test)]
mod dimension_hint_tests {
    #[test]
    fn presentational_dimensions_ignore_invalid_values_and_preserve_zero_and_percentages() {
        for value in ["-1", "-0.5%", "NaN", "inf", "Infinity%", "*", "2px"] {
            assert_eq!(super::attr_length(value), None, "{value}");
        }
        for (value, expected) in [
            ("0", "0px"),
            (" 12.5 ", "12.5px"),
            ("0%", "0%"),
            ("50%", "50%"),
        ] {
            assert_eq!(
                super::attr_length(value).as_deref(),
                Some(expected),
                "{value}"
            );
        }
    }
}

#[cfg(test)]
mod iteration_tests {
    use super::ComputedStyle;

    #[test]
    fn iteration_yields_every_pair_in_name_order_and_round_trips() {
        let style = ComputedStyle::from_pairs([("width", "10px"), ("--x", "1"), ("color", "red")]);
        let pairs: Vec<_> = style.iter().collect();
        assert_eq!(pairs, [("--x", "1"), ("color", "red"), ("width", "10px")]);
        assert_eq!(ComputedStyle::from_pairs(style.iter()), style);
        assert_eq!(ComputedStyle::default().iter().count(), 0);
    }
}

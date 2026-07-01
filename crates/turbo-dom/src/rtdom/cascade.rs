//! Partial computed-style cascade — pure-Rust port of `src/runtime/cascade.mjs`.
//!
//! Resolves `getComputedStyle` from injected `<style>` sheets (emotion/MUI
//! `.css-HASH { … }`), inline styles, and specificity ordering.
//!
//! HONEST: this only ever returns values that come from a REAL matched rule or
//! inline style (or inherited from one — see `INHERITED`). A property no
//! stylesheet/inline set still reads `""` — it never invents layout/cascade
//! numbers or initial values. Out of scope (return `""`): @media/@supports/
//! @keyframes, :hover & other stateful pseudo-classes, pseudo-elements, the
//! `inherit`/`initial`/`unset` keywords, CSS custom-property resolution. Colors
//! ARE canonicalized to `rgb()/rgba()` (`color.rs`) and bare-0 lengths to `0px`,
//! matching what a browser serializes.

use crate::rtdom::color;
use crate::rtdom::tree::{Handle, NodeType, Tree};
use std::collections::HashMap;

/// camelCase → kebab-case property name (only ever fed kebab from CSS, but kept
/// for parity with the JS `kebab` used at the proxy boundary).
#[must_use]
pub fn kebab(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Longhand → shorthand fallback for single-token shorthands (mirrors dom.mjs styleGet).
fn shorthand_of(prop: &str) -> Option<&'static str> {
    match prop {
        "background-color" => Some("background"),
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => Some("margin"),
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => Some("padding"),
        _ => None,
    }
}

/// Length properties whose bare `0` real browsers serialize as `0px`.
const LENGTH_PROPS: &[&str] = &[
    "width",
    "height",
    "min-width",
    "max-width",
    "min-height",
    "max-height",
    "top",
    "right",
    "bottom",
    "left",
    "flex-basis",
    "gap",
    "row-gap",
    "column-gap",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "border-width",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "font-size",
    "letter-spacing",
    "word-spacing",
    "text-indent",
];

fn is_length_prop(p: &str) -> bool {
    LENGTH_PROPS.contains(&p)
}

const BORDER_STYLES: &[&str] = &[
    "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
];

/// Inheritable properties propagated down the flattened tree (curated set).
const INHERITED: &[&str] = &[
    "color",
    "cursor",
    "direction",
    "font",
    "font-family",
    "font-size",
    "font-style",
    "font-variant",
    "font-weight",
    "letter-spacing",
    "line-height",
    "list-style",
    "list-style-type",
    "text-align",
    "text-indent",
    "text-transform",
    "visibility",
    "white-space",
    "word-spacing",
    "quotes",
];

/// `<token>` looks like a CSS length/number ("1px", "2.5em", "0", "50%").
fn looks_like_width_token(p: &str) -> bool {
    if p == "thin" || p == "medium" || p == "thick" {
        return true;
    }
    // /^[\d.]+(px|em|rem|%|pt|vh|vw)?$/
    let units = ["px", "em", "rem", "%", "pt", "vh", "vw"];
    let mut num_end = 0;
    for (i, c) in p.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if num_end == 0 {
        return false; // no leading numeric part
    }
    let rest = &p[num_end..];
    rest.is_empty() || units.contains(&rest)
}

/// Mirror of the JS `setProp`: store the declaration AND expand common shorthands
/// into longhands so longhand computed getters resolve.
fn set_prop(map: &mut HashMap<String, String>, name: &str, val: &str) {
    map.insert(name.to_string(), val.to_string());
    if name == "margin" || name == "padding" {
        let t: Vec<&str> = val.split_whitespace().collect();
        let top = t.first().copied().unwrap_or("");
        let right = t.get(1).copied().unwrap_or(top);
        let bottom = t.get(2).copied().unwrap_or(top);
        let left = t.get(3).copied().unwrap_or(right);
        map.insert(format!("{name}-top"), top.to_string());
        map.insert(format!("{name}-right"), right.to_string());
        map.insert(format!("{name}-bottom"), bottom.to_string());
        map.insert(format!("{name}-left"), left.to_string());
    } else if name == "border" {
        let mut width: Option<&str> = None;
        let mut style: Option<&str> = None;
        let mut color_v: Option<&str> = None;
        for p in val.split_whitespace() {
            if BORDER_STYLES.contains(&p) {
                style = Some(p);
            } else if looks_like_width_token(p) {
                width = Some(p);
            } else {
                color_v = Some(p);
            }
        }
        if let Some(w) = width {
            map.insert("border-width".to_string(), w.to_string());
            for s in ["top", "right", "bottom", "left"] {
                map.insert(format!("border-{s}-width"), w.to_string());
            }
        }
        if let Some(s) = style {
            map.insert("border-style".to_string(), s.to_string());
        }
        if let Some(c) = color_v {
            map.insert("border-color".to_string(), c.to_string());
        }
    } else if name == "background" && !val.trim().contains(char::is_whitespace) {
        map.insert("background-color".to_string(), val.trim().to_string());
    }
}

/// Strip a trailing `!important` (case-insensitive, surrounding whitespace) from a value.
fn strip_important(v: &str) -> String {
    let trimmed = v.trim_end();
    let lower = trimmed.to_ascii_lowercase();
    // match /\s*!\s*important\s*$/i — but the JS applied it after .trim() of the
    // value; here we replicate `!<ws>important` at the (already trimmed) end.
    if let Some(bang) = lower.rfind('!') {
        let after = lower[bang + 1..].trim_start();
        if after == "important" {
            return trimmed[..bang].trim_end().to_string();
        }
    }
    trimmed.to_string()
}

/// Parse a `prop:val;prop:val` declaration block into `map` (later wins).
fn parse_decls(text: &str, map: &mut HashMap<String, String>) {
    for decl in text.split(';') {
        let Some(c) = decl.find(':') else { continue };
        let name = decl[..c].trim().to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        let raw = decl[c + 1..].trim();
        let val = strip_important(raw);
        set_prop(map, &name, &val);
    }
}

/// Rough WHATWG specificity: a*10000 + b*100 + c (ids, classes/attrs/pseudos, types).
fn specificity(sel: &str) -> u32 {
    let bytes = sel.as_bytes();
    let n = bytes.len();
    let is_ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-';

    // a: count of `#[\w-]+`
    let mut a = 0u32;
    let mut i = 0;
    while i < n {
        if bytes[i] == b'#' {
            let mut j = i + 1;
            while j < n && is_ident(bytes[j]) {
                j += 1;
            }
            if j > i + 1 {
                a += 1;
            }
            i = j;
        } else {
            i += 1;
        }
    }

    // b: count of `\.[\w-]+ | \[[^\]]*\] | :[\w-]+`
    let mut b = 0u32;
    i = 0;
    while i < n {
        match bytes[i] {
            b'.' | b':' => {
                let mut j = i + 1;
                while j < n && is_ident(bytes[j]) {
                    j += 1;
                }
                if j > i + 1 {
                    b += 1;
                    i = j;
                } else {
                    i += 1;
                }
            }
            b'[' => {
                let mut j = i + 1;
                while j < n && bytes[j] != b']' {
                    j += 1;
                }
                // `[^\]]*` matches zero or more, so `[]` still counts.
                b += 1;
                i = if j < n { j + 1 } else { n };
            }
            _ => i += 1,
        }
    }

    // c: count of `(^|[\s>+~])[a-zA-Z][\w-]*` — a type selector at start or after
    // a combinator/whitespace.
    let mut c = 0u32;
    i = 0;
    while i < n {
        let at_boundary = i == 0
            || matches!(bytes[i - 1], b' ' | b'\t' | b'\n' | b'\r' | b'>' | b'+' | b'~');
        if at_boundary && bytes[i].is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < n && is_ident(bytes[j]) {
                j += 1;
            }
            c += 1;
            i = j;
        } else {
            i += 1;
        }
    }

    a * 10000 + b * 100 + c
}

/// Strip `/* … */` comments.
fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let bytes = css.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else {
            // push the char (handle UTF-8 boundaries via the str slice)
            let ch_len = utf8_len(bytes[i]);
            out.push_str(&css[i..(i + ch_len).min(n)]);
            i += ch_len;
        }
    }
    out
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

#[derive(Clone)]
pub(crate) struct Rule {
    selector: String,
    decls: HashMap<String, String>,
    spec: u32,
    order: u32,
}

/// Version-keyed cascade cache held on the `Tree` (mirrors the JS `__computedStyle`
/// memo keyed on `Document.__version`). `build_index` runs once per version, and
/// each element's resolved map is memoized — without this, `getComputedStyle`
/// re-parses every `<style>` per call AND per inheritance-recursion ancestor.
#[derive(Default)]
pub(crate) struct CssCache {
    pub version: u64,
    pub rules: Option<std::rc::Rc<Vec<Rule>>>,
    pub computed: HashMap<Handle, std::rc::Rc<HashMap<String, String>>>,
}

/// Brace-depth scan: emit only depth-1 rules whose selector isn't an at-rule;
/// the bodies of @media/@keyframes (nested braces) are skipped wholesale.
fn parse_stylesheet(css: &str, start_order: u32, rules: &mut Vec<Rule>) -> u32 {
    let css = strip_comments(css);
    let bytes = css.as_bytes();
    let n = bytes.len();
    let mut order = start_order;
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && bytes[j] != b'{' && bytes[j] != b'}' && bytes[j] != b';' {
            j += 1;
        }
        if j >= n {
            break;
        }
        if bytes[j] == b';' {
            // A `;` outside any block terminates a statement at-rule (@import/@charset/
            // @namespace) or a stray semicolon — it has no block, so skip just the
            // statement and keep scanning for the next real selector. Without this the
            // statement would be glued onto the following selector and (starting with
            // `@`) drag that whole rule into the skip.
            order += 1;
            i = j + 1;
            continue;
        }
        if bytes[j] == b'}' {
            i = j + 1;
            continue;
        }
        let sel = css[i..j].trim().to_string();
        let mut depth = 1i32;
        let mut k = j + 1;
        while k < n && depth > 0 {
            match bytes[k] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            k += 1;
        }
        if !sel.is_empty() && !sel.starts_with('@') {
            let body = &css[j + 1..k.saturating_sub(1)];
            let mut decls = HashMap::new();
            parse_decls(body, &mut decls);
            if !decls.is_empty() {
                for part in sel.split(',') {
                    let t = part.trim();
                    if !t.is_empty() {
                        rules.push(Rule {
                            selector: t.to_string(),
                            decls: decls.clone(),
                            spec: specificity(t),
                            order,
                        });
                        order += 1;
                    }
                }
            }
        } else {
            order += 1;
        }
        i = k;
    }
    order
}

/// Build the rule index from every `<style>` element's text content in the doc.
fn build_index(tree: &Tree) -> Vec<Rule> {
    let styles = tree.get_elements_by_tag_name("style");
    let mut rules = Vec::new();
    let mut order = 0u32;
    for h in styles {
        let text = tree.text_content(h);
        order = parse_stylesheet(&text, order, &mut rules);
        // NOTE: the JS also reads emotion `__sheet.cssRules` (speedy-mode injected
        // via insertRule, never written to textContent). The Rust Tree has no
        // CSSOM sheet overlay yet — only authored <style> text is available here.
        // TODO: insertRule-injected rules (emotion speedy mode) once Tree grows a
        // stylesheet overlay.
    }
    rules
}

/// `:host` / `::slotted` selectors that never match plain elements.
fn is_shadow_only_selector(sel: &str) -> bool {
    sel.starts_with(":host") || sel.contains("::slotted(")
}

/// Collect rules whose selector matches `h` (kind === 'normal' only — shadow
/// scoping is out of scope for v1).
fn collect_matched(tree: &Tree, h: Handle, rules: &[Rule], into: &mut Vec<Rule>) {
    for r in rules {
        // TODO: shadow scoping — the JS handles :host/::slotted ('host'/'slotted'
        // kinds). For v1 we only do the light-DOM 'normal' kind and skip
        // shadow-only selectors (they never match a plain element anyway).
        if is_shadow_only_selector(&r.selector) {
            continue;
        }
        if tree.matches(h, &r.selector) {
            into.push(r.clone());
        }
    }
}

/// Sort by (spec, order) then apply in cascade order (later wins).
fn apply_matched(matched: &mut [Rule], out: &mut HashMap<String, String>) {
    matched.sort_by(|x, y| x.spec.cmp(&y.spec).then(x.order.cmp(&y.order)));
    for r in matched.iter() {
        for (k, v) in &r.decls {
            out.insert(k.clone(), v.clone());
        }
    }
}

/// Flattened-tree parent for inheritance. Crosses the shadow boundary: a
/// top-level shadow child inherits from the HOST (the JS hops shadow-root→host).
fn flattened_parent(tree: &Tree, h: Handle) -> Option<Handle> {
    let p = tree.parent(h)?;
    if tree.is_shadow_root(p) {
        return tree.shadow_host(p); // shadow-root→host hop
    }
    if tree.node_type(p) == NodeType::Element {
        Some(p)
    } else {
        None
    }
}

/// Parse every `<style>` inside a shadow root into scoped rules.
fn shadow_rules(tree: &Tree, shadow_root: Handle) -> Vec<Rule> {
    let mut rules = Vec::new();
    let mut order = 0u32;
    for h in tree.descendants(shadow_root) {
        if tree.local_name(h) == Some("style") {
            let text = tree.text_content(h);
            order = parse_stylesheet(&text, order, &mut rules);
        }
    }
    rules
}

/// `:host` → Some(""), `:host(sel)` → Some(sel), else None.
fn host_selector(sel: &str) -> Option<&str> {
    let s = sel.trim();
    if let Some(rest) = s.strip_prefix(":host(") {
        rest.strip_suffix(')')
    } else if s == ":host" {
        Some("")
    } else {
        None
    }
}

/// `::slotted(sel)` → Some(sel), else None.
fn slotted_selector(sel: &str) -> Option<&str> {
    let i = sel.find("::slotted(")?;
    let rest = &sel[i + "::slotted(".len()..];
    rest.find(')').map(|j| rest[..j].trim())
}

/// Look up a resolved property from the cascade map, applying shorthand fallback,
/// bare-0 → `0px`, font-family comma spacing, and color canonicalization.
fn lookup(map: &HashMap<String, String>, prop: &str) -> String {
    let mut v: Option<String> = map.get(prop).cloned();
    if v.is_none() {
        if let Some(sh) = shorthand_of(prop) {
            if let Some(s) = map.get(sh) {
                if !s.trim().contains(char::is_whitespace) {
                    v = Some(s.clone());
                }
            }
        }
    }
    let Some(v) = v else { return String::new() };
    // px-normalize a bare `0` for length properties (browsers report `0px`)
    if v == "0" && is_length_prop(prop) {
        return "0px".to_string();
    }
    // font-family: browsers serialize the list with ", " regardless of source spacing.
    if prop == "font-family" {
        return normalize_font_family(&v);
    }
    // <color> properties → rgb()/rgba() (include_named=true). Unrecognized → passthrough.
    if color::is_color_prop(prop) {
        if let Some(c) = color::canonicalize_color(&v, true) {
            return c;
        }
    }
    v
}

/// Replace `\s*,\s*` with `, ` (scoped to font-family).
fn normalize_font_family(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ',' {
            // drop any whitespace we'd already pushed before the comma
            while out.ends_with(char::is_whitespace) {
                out.pop();
            }
            out.push_str(", ");
            // skip following whitespace
            while matches!(chars.peek(), Some(w) if w.is_whitespace()) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Resolve the full computed-style map for element `h`.
///
/// Mirrors `makeGetComputedStyle`'s gcs(el): collect matched `<style>` rules,
/// sort by (spec, order), apply; overlay inline `style`; then inherit the
/// curated `INHERITED` set from the flattened parent (only values a REAL
/// rule/inline set, never an initial value).
pub fn computed_style(tree: &Tree, h: Handle) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    if tree.node_type(h) != NodeType::Element {
        return map;
    }

    // --- version-keyed memo: return a cached result, and build the rule index
    // at most once per version (not per call / per inheritance-recursion hop). ---
    let version = tree.version;
    let light_rules: std::rc::Rc<Vec<Rule>> = {
        let mut c = tree.css_cache.borrow_mut();
        if c.version != version {
            c.version = version;
            c.rules = None;
            c.computed.clear();
        }
        if let Some(cached) = c.computed.get(&h) {
            return (**cached).clone();
        }
        c.rules
            .get_or_insert_with(|| std::rc::Rc::new(build_index(tree)))
            .clone()
    };

    let mut matched = Vec::new();

    // Document <style> rules apply to LIGHT-DOM elements only (encapsulation).
    let in_shadow = tree.shadow_root_of(h);
    if in_shadow.is_none() {
        collect_matched(tree, h, &light_rules, &mut matched);
    } else if let Some(sr) = in_shadow {
        // shadow-internal element: normal selectors from the shadow's own <style>.
        for r in shadow_rules(tree, sr) {
            if !is_shadow_only_selector(&r.selector) && tree.matches(h, &r.selector) {
                matched.push(r);
            }
        }
    }

    // :host — if `h` hosts a shadow root, its shadow's :host rules style the host.
    if let Some(sr) = tree.shadow_root(h) {
        for r in shadow_rules(tree, sr) {
            if let Some(sel) = host_selector(&r.selector) {
                if sel.is_empty() || tree.matches(h, sel) {
                    matched.push(r);
                }
            }
        }
    }

    // ::slotted(sel) — if `h` is a light child slotted into a shadow host, that
    // shadow's ::slotted rules style the distributed node.
    if let Some(parent) = tree.parent(h) {
        if let Some(sr) = tree.shadow_root(parent) {
            for r in shadow_rules(tree, sr) {
                if let Some(inner) = slotted_selector(&r.selector) {
                    if inner.is_empty() || tree.matches(h, inner) {
                        matched.push(r);
                    }
                }
            }
        }
    }

    apply_matched(&mut matched, &mut map);

    // inline style wins over stylesheet rules
    if let Some(inline) = tree.get_attribute(h, "style") {
        if !inline.is_empty() {
            let inline = inline.to_string();
            parse_decls(&inline, &mut map);
        }
    }

    // Inheritance: inherit unset INHERITED props from the flattened parent, using
    // the parent's already-resolved value (only when a REAL rule/inline set it).
    if let Some(parent) = flattened_parent(tree, h) {
        let ps = computed_style(tree, parent);
        for prop in INHERITED {
            if !map.contains_key(*prop) {
                let pv = lookup(&ps, prop);
                if !pv.is_empty() {
                    // store the resolved value so child lookups stay consistent
                    map.insert((*prop).to_string(), pv);
                }
            }
        }
    }

    // memoize for this (handle, version) — inheritance recursion + repeat calls hit this.
    let result = std::rc::Rc::new(map);
    tree.css_cache.borrow_mut().computed.insert(h, result.clone());
    (*result).clone()
}

/// Resolve one property from a computed-style map — `""` for absent (honest).
#[must_use]
pub fn get_property_value(map: &HashMap<String, String>, prop: &str) -> String {
    lookup(map, &prop.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdom::tree::Tree;

    fn gcs_prop(html: &str, selector: &str, prop: &str) -> String {
        let tree = Tree::parse(html);
        let h = tree.query_selector_all(selector)[0];
        let map = computed_style(&tree, h);
        get_property_value(&map, prop)
    }

    #[test]
    fn inline_color_canonicalized() {
        // (a) inline style="color:red" → rgb(255, 0, 0)
        let got = gcs_prop(
            "<div id=x style=\"color:red\">hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(255, 0, 0)");
    }

    #[test]
    fn shadow_host_slotted_and_inheritance() {
        // host with a shadow root: :host styles host, a shadow-internal element
        // gets shadow <style> rules, ::slotted styles distributed light nodes,
        // and inheritance hops shadow-root→host.
        let mut tree = Tree::parse("<my-box style=\"color:green\"><span slot=t>S</span></my-box>");
        let host = tree.query_selector("my-box").unwrap();
        let sr = tree.attach_shadow(host);
        tree.set_inner_html(
            sr,
            "<style>:host{padding:4px} p{color:blue} ::slotted(span){font-weight:bold}</style><p>inner</p><slot name=t></slot>",
        );

        // :host applies to the host
        let host_cs = computed_style(&tree, host);
        assert_eq!(get_property_value(&host_cs, "padding"), "4px");

        // shadow-internal <p> matches the shadow's `p{color:blue}`
        let p = tree.descendants(sr).into_iter().find(|&h| tree.local_name(h) == Some("p")).unwrap();
        let p_cs = computed_style(&tree, p);
        assert_eq!(get_property_value(&p_cs, "color"), "rgb(0, 0, 255)");
        // ...and inherits color from the host across the boundary (host has color:green inline)
        let p_color_inherited = {
            // <p> sets its own color (blue), so test inheritance on a node that does NOT:
            // the slot itself inherits host green via flattened parent
            let slot = tree.descendants(sr).into_iter().find(|&h| tree.local_name(h) == Some("slot")).unwrap();
            get_property_value(&computed_style(&tree, slot), "color")
        };
        assert_eq!(p_color_inherited, "rgb(0, 128, 0)"); // green from host, crossed boundary

        // ::slotted(span) styles the distributed light <span>
        let span = tree.query_selector("span").unwrap();
        let span_cs = computed_style(&tree, span);
        assert_eq!(get_property_value(&span_cs, "font-weight"), "bold");
    }

    #[test]
    fn matched_class_rule_color() {
        // (b) <style>.card{color:blue}</style> + <div class=card> → blue canonicalized
        let got = gcs_prop(
            "<style>.card{color:blue}</style><div class=card>hi</div>",
            ".card",
            "color",
        );
        assert_eq!(got, "rgb(0, 0, 255)");
    }

    #[test]
    fn id_beats_class_specificity() {
        // (c) #id{color:green} beats .card{color:blue}
        let got = gcs_prop(
            "<style>.card{color:blue} #x{color:green}</style><div id=x class=card>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(0, 128, 0)"); // green
    }

    #[test]
    fn inheritance_cascades_color() {
        // (d) body{color:red} cascades to a child <p> (color in INHERITED)
        let got = gcs_prop(
            "<style>body{color:red}</style><body><p id=p>hi</p></body>",
            "#p",
            "color",
        );
        assert_eq!(got, "rgb(255, 0, 0)");
    }

    #[test]
    fn unmatched_prop_empty() {
        // (e) unmatched prop → ""
        let got = gcs_prop(
            "<div id=x style=\"color:red\">hi</div>",
            "#x",
            "font-weight",
        );
        assert_eq!(got, "");
    }

    #[test]
    fn bare_zero_length_to_px() {
        // (f) bare-0 length → 0px
        let got = gcs_prop(
            "<div id=x style=\"margin:0\">hi</div>",
            "#x",
            "margin-top",
        );
        assert_eq!(got, "0px");
    }

    #[test]
    fn specificity_ordering() {
        assert!(specificity("#id") > specificity(".card"));
        assert!(specificity(".card") > specificity("div"));
        assert_eq!(specificity("#id"), 10000);
        assert_eq!(specificity(".card"), 100);
        assert_eq!(specificity("div"), 1);
        // descendant of two types + a class
        assert_eq!(specificity("div .card span"), 100 + 2);
    }

    #[test]
    fn kebab_camel_to_kebab() {
        // direct: camelCase → kebab (parity helper, only ever fed from the JS proxy)
        assert_eq!(kebab("backgroundColor"), "background-color");
        assert_eq!(kebab("color"), "color"); // already lowercase, no dashes
        assert_eq!(kebab("WebkitBoxShadow"), "-webkit-box-shadow");
    }

    #[test]
    fn looks_like_width_token_keywords_and_units() {
        // keyword widths
        assert!(looks_like_width_token("thin"));
        assert!(looks_like_width_token("medium"));
        assert!(looks_like_width_token("thick"));
        // numeric with units
        assert!(looks_like_width_token("1px"));
        assert!(looks_like_width_token("2.5em"));
        assert!(looks_like_width_token("0"));
        assert!(looks_like_width_token("50%"));
        // not numeric → false (no leading numeric part)
        assert!(!looks_like_width_token("red"));
        // numeric but unknown unit → false (rest not in units)
        assert!(!looks_like_width_token("10foo"));
    }

    #[test]
    fn border_shorthand_assembles_longhands() {
        // border: 1px solid red → border-width/style/color + per-side widths
        let mut map = HashMap::new();
        set_prop(&mut map, "border", "1px solid red");
        assert_eq!(map.get("border-width").unwrap(), "1px");
        assert_eq!(map.get("border-top-width").unwrap(), "1px");
        assert_eq!(map.get("border-left-width").unwrap(), "1px");
        assert_eq!(map.get("border-style").unwrap(), "solid");
        assert_eq!(map.get("border-color").unwrap(), "red");
        // through gcs: border-style resolves
        let got = gcs_prop(
            "<div id=x style=\"border:2px dashed blue\">hi</div>",
            "#x",
            "border-style",
        );
        assert_eq!(got, "dashed");
    }

    #[test]
    fn background_single_token_shorthand_to_color() {
        // background:red (no whitespace) → background-color fallback via lookup
        let got = gcs_prop(
            "<div id=x style=\"background:red\">hi</div>",
            "#x",
            "background-color",
        );
        assert_eq!(got, "rgb(255, 0, 0)");
    }

    #[test]
    fn specificity_attribute_selector() {
        // `[...]` branch: attribute selector counts as a `b`
        assert_eq!(specificity("[data-x]"), 100);
        // empty `[]` still counts (per the `[^\]]*` zero-or-more comment)
        assert_eq!(specificity("[]"), 100);
        // unterminated `[` runs to end of string
        assert_eq!(specificity("div["), 100 + 1);
        // bare `.` / `:` with no ident don't count
        assert_eq!(specificity(". :"), 0);
    }

    #[test]
    fn strip_comments_and_multibyte() {
        // CSS comment is stripped; a multibyte char survives (utf8_len path)
        let got = gcs_prop(
            "<style>/* note: \u{00e9}\u{4e2d} */ #x { color: red }</style><div id=x>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(255, 0, 0)");
        // multibyte content selector text after a comment — content="café" 中
        let got2 = gcs_prop(
            "<style>/* x */ .caf\u{00e9} { color: blue }</style><div id=x class=\"caf\u{00e9}\">hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got2, "rgb(0, 0, 255)");
    }

    #[test]
    fn at_rule_and_stray_brace_skipped() {
        // @media body is skipped wholesale; the trailing real rule still applies.
        let got = gcs_prop(
            "<style>@media (max-width:600px){ #x{color:red} } #x{color:green}</style><div id=x>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(0, 128, 0)"); // green from the depth-1 rule, not the @media one
        // a stray leading `}` (depth underflow) is skipped, real rule still parses
        let got2 = gcs_prop(
            "<style>} #x{color:blue}</style><div id=x>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got2, "rgb(0, 0, 255)");
    }

    #[test]
    fn statement_at_rule_does_not_swallow_following_rule() {
        // A `;`-terminated statement at-rule (@import / @charset / @namespace) has no
        // block. The rule that follows it must still apply — only the at-statement is
        // skipped, not the next real selector.
        let got = gcs_prop(
            "<style>@import url(\"reset.css\"); #x{color:green}</style><div id=x>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(0, 128, 0)");
        // @charset followed immediately by a rule
        let got2 = gcs_prop(
            "<style>@charset \"utf-8\"; .card{color:blue}</style><div id=x class=card>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got2, "rgb(0, 0, 255)");
        // a block at-rule (@media) is still skipped wholesale, and a statement at-rule
        // BEFORE it does not disturb the later depth-1 rule
        let got3 = gcs_prop(
            "<style>@import url(a); @media print { #x{color:red} } #x{color:green}</style><div id=x>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got3, "rgb(0, 128, 0)");
    }

    #[test]
    fn trailing_selector_without_brace_breaks() {
        // a selector with no `{` (j reaches n) breaks the scan; earlier rule applied.
        let got = gcs_prop(
            "<style>#x{color:red} #y</style><div id=x>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(255, 0, 0)");
    }

    #[test]
    fn shadow_only_selector_skipped_in_light_dom() {
        // light-DOM element: a `:host` rule in a doc <style> must NOT match it.
        let got = gcs_prop(
            "<style>:host{color:red} #x{color:green}</style><div id=x>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(0, 128, 0)"); // green, :host skipped
        assert!(is_shadow_only_selector(":host"));
        assert!(is_shadow_only_selector("::slotted(span)"));
        assert!(!is_shadow_only_selector(".card"));
    }

    #[test]
    fn host_selector_with_arg() {
        // :host(sel) → Some(sel); :host → Some(""); other → None
        assert_eq!(host_selector(":host(.foo)"), Some(".foo"));
        assert_eq!(host_selector(":host"), Some(""));
        assert_eq!(host_selector(".card"), None);
    }

    #[test]
    fn normalize_font_family_comma_spacing() {
        // through lookup/gcs: comma spacing canonicalized to ", "
        let got = gcs_prop(
            "<div id=x style='font-family: \"Helvetica Neue\",Arial , sans-serif'>hi</div>",
            "#x",
            "font-family",
        );
        assert_eq!(got, "\"Helvetica Neue\", Arial, sans-serif");
        // direct helper
        assert_eq!(normalize_font_family("a ,b"), "a, b");
        assert_eq!(normalize_font_family("solo"), "solo");
    }

    #[test]
    fn margin_shorthand_fallback_via_lookup() {
        // margin-top resolves via the margin shorthand longhand expansion
        let got = gcs_prop(
            "<div id=x style=\"margin:10px 20px\">hi</div>",
            "#x",
            "margin-top",
        );
        assert_eq!(got, "10px");
    }

    #[test]
    fn get_property_value_mixed_case_prop() {
        // get_property_value lowercases the requested prop name
        let tree = Tree::parse("<div id=x style=\"color:red\">hi</div>");
        let h = tree.query_selector_all("#x")[0];
        let map = computed_style(&tree, h);
        assert_eq!(get_property_value(&map, "COLOR"), "rgb(255, 0, 0)");
        assert_eq!(get_property_value(&map, "Color"), "rgb(255, 0, 0)");
    }

    #[test]
    fn important_stripped() {
        // !important is stripped from the value (strip_important branch)
        let got = gcs_prop(
            "<div id=x style=\"color:red !important\">hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(255, 0, 0)");
        // direct helper coverage of the matched branch + the non-important passthrough
        assert_eq!(strip_important("10px ! important"), "10px");
        assert_eq!(strip_important("10px"), "10px");
    }

    #[test]
    fn parse_decls_skips_malformed() {
        // a decl with no colon is skipped, and an empty-name decl is skipped;
        // the valid one still applies.
        let got = gcs_prop(
            "<div id=x style=\"garbage; :nope; color:red\">hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(255, 0, 0)");
    }

    #[test]
    fn utf8_len_three_and_four_byte() {
        // direct helper: 1/2/3/4-byte lead bytes
        assert_eq!(utf8_len(b'a'), 1);
        assert_eq!(utf8_len(0xC3), 2); // é lead
        assert_eq!(utf8_len(0xE4), 3); // 中 lead
        assert_eq!(utf8_len(0xF0), 4); // 😀 lead
        // and through strip_comments: a 4-byte emoji outside a comment survives
        let got = gcs_prop(
            "<style>/* c */ #x{content:\"\u{1F600}\"; color:red}</style><div id=x>hi</div>",
            "#x",
            "color",
        );
        assert_eq!(got, "rgb(255, 0, 0)");
    }

    #[test]
    fn lookup_shorthand_fallback_accepts_singletoken() {
        // Direct lookup with a hand-built map that has the single-token shorthand
        // present but the longhand ABSENT — exercises the fallback success branch.
        // (In the full pipeline set_prop pre-expands single-token shorthands, so
        // this state only arises by direct construction.)
        let mut map = HashMap::new();
        map.insert("margin".to_string(), "7px".to_string());
        assert_eq!(lookup(&map, "margin-top"), "7px");
    }

    #[test]
    fn shorthand_fallback_rejects_multitoken() {
        // background:red url(x) (whitespace) → background-color fallback REJECTED,
        // so background-color reads "" (lookup 486-487 false branch).
        let got = gcs_prop(
            "<div id=x style=\"background:red url(x.png)\">hi</div>",
            "#x",
            "background-color",
        );
        assert_eq!(got, "");
    }

    #[test]
    fn non_element_node_empty_map() {
        // computed_style on a non-element (text node) returns an empty map
        let tree = Tree::parse("<div id=x>hello</div>");
        let div = tree.query_selector_all("#x")[0];
        let text = tree.descendants(div).into_iter().find(|&h| tree.node_type(h) != NodeType::Element);
        if let Some(t) = text {
            let map = computed_style(&tree, t);
            assert!(map.is_empty());
        }
    }
}

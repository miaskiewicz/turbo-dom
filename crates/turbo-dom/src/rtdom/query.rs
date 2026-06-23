//! Selector matching + query API over `Tree`. Mirrors the JS runtime's
//! alloc-free matcher discipline (whole-word class scan, index loops, no regex)
//! and version-keyed result caching (`Document.__version` → `Tree.version`).
//!
//! Selector support: comma lists, descendant (` `) and child (`>`) combinators,
//! compounds of `tag` / `.class` / `#id` / `[attr]` / `[attr=val]`. Enough for
//! RTL-style queries; extend as the gauntlet demands.

use super::tree::{Handle, NodeType, Tree};

#[derive(Debug, Clone, Copy, PartialEq)]
enum AttrOp {
    Presence, // [attr]
    Exact,    // [attr=v]
    Includes, // [attr~=v]  whitespace-separated word
    Dash,     // [attr|=v]  v or v-...
    Prefix,   // [attr^=v]
    Suffix,   // [attr$=v]
    Substr,   // [attr*=v]
}

#[derive(Debug, Clone)]
struct Attr {
    name: String,
    op: AttrOp,
    value: String, // empty for Presence
}

#[derive(Debug, Clone, Default)]
struct Compound {
    tag: Option<String>, // None or "*" = any
    id: Option<String>,
    classes: Vec<String>,
    attrs: Vec<Attr>,
    pseudos: Vec<Pseudo>,
}

/// Supported pseudo-classes. Mirrors `src/runtime/selectors.mjs` `matchPseudo`.
/// `Unknown` (anything not parsed into a variant) NEVER matches — honest, not a
/// silent true — matching the JS `default: return false`.
#[derive(Debug, Clone)]
enum Pseudo {
    // Structural
    FirstChild,
    LastChild,
    OnlyChild,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    NthChild(i64, i64),       // (a, b) from `an+b`
    NthLastChild(i64, i64),
    NthOfType(i64, i64),
    NthLastOfType(i64, i64),
    Empty,
    Root,
    // Form-state
    Checked,
    Disabled,
    Enabled,
    Required,
    Optional,
    ReadOnly,
    ReadWrite,
    Selected,
    // Negation of a simple compound
    Not(Box<Compound>),
    // Unrecognized / unparseable → never matches
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Combinator {
    Descendant,
    Child,
}

/// A complex selector = a chain of (combinator, compound), left-to-right.
/// The first compound has no meaningful combinator (use Descendant).
#[derive(Debug, Clone)]
struct Complex {
    parts: Vec<(Combinator, Compound)>,
}

/// is byte a compound-part boundary (start of the next `.`/`#`/`[`/`:` token)?
#[inline]
fn is_part_start(byte: u8) -> bool {
    byte == b'.' || byte == b'#' || byte == b'[' || byte == b':'
}

fn parse_compound(src: &str) -> Compound {
    let mut c = Compound::default();
    let b = src.as_bytes();
    let mut i = 0;
    // type selector (stops at the first `.`/`#`/`[`/`:`)
    if i < b.len() && !is_part_start(b[i]) {
        let start = i;
        while i < b.len() && !is_part_start(b[i]) {
            i += 1;
        }
        let tag = &src[start..i];
        if tag != "*" {
            c.tag = Some(tag.to_string());
        }
    }
    while i < b.len() {
        match b[i] {
            b'.' => {
                i += 1;
                let start = i;
                while i < b.len() && !is_part_start(b[i]) {
                    i += 1;
                }
                c.classes.push(src[start..i].to_string());
            }
            b'#' => {
                i += 1;
                let start = i;
                while i < b.len() && !is_part_start(b[i]) {
                    i += 1;
                }
                c.id = Some(src[start..i].to_string());
            }
            b'[' => {
                i += 1;
                let start = i;
                while i < b.len() && b[i] != b']' {
                    i += 1;
                }
                let inner = &src[start..i];
                if i < b.len() {
                    i += 1; // skip ]
                }
                c.attrs.push(parse_attr(inner));
            }
            b':' => {
                // `:name` then optional `(...)` arg (balanced parens, so a
                // nested `:not(.a)` arg survives intact for recursive parse).
                i += 1;
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphabetic() || b[i] == b'-') {
                    i += 1;
                }
                let name = &src[start..i];
                let mut arg: Option<&str> = None;
                if i < b.len() && b[i] == b'(' {
                    let mut depth = 1usize;
                    let astart = i + 1;
                    i += 1;
                    while i < b.len() && depth > 0 {
                        match b[i] {
                            b'(' => depth += 1,
                            b')' => depth -= 1,
                            _ => {}
                        }
                        i += 1;
                    }
                    // i now points just past the closing ')'
                    let aend = if i > astart { i - 1 } else { astart };
                    arg = Some(&src[astart..aend]);
                }
                c.pseudos.push(parse_pseudo(name, arg));
            }
            _ => {
                i += 1;
            }
        }
    }
    c
}

/// Map a pseudo `name` + optional `arg` to a `Pseudo`. Unrecognized → `Unknown`
/// (never matches). Mirrors selectors.mjs `matchPseudo` switch.
fn parse_pseudo(name: &str, arg: Option<&str>) -> Pseudo {
    match name {
        "first-child" => Pseudo::FirstChild,
        "last-child" => Pseudo::LastChild,
        "only-child" => Pseudo::OnlyChild,
        "first-of-type" => Pseudo::FirstOfType,
        "last-of-type" => Pseudo::LastOfType,
        "only-of-type" => Pseudo::OnlyOfType,
        "empty" => Pseudo::Empty,
        "root" => Pseudo::Root,
        "checked" => Pseudo::Checked,
        "disabled" => Pseudo::Disabled,
        "enabled" => Pseudo::Enabled,
        "required" => Pseudo::Required,
        "optional" => Pseudo::Optional,
        "read-only" => Pseudo::ReadOnly,
        "read-write" => Pseudo::ReadWrite,
        "selected" => Pseudo::Selected,
        "nth-child" => {
            let (a, b) = parse_nth(arg.unwrap_or(""));
            Pseudo::NthChild(a, b)
        }
        "nth-last-child" => {
            let (a, b) = parse_nth(arg.unwrap_or(""));
            Pseudo::NthLastChild(a, b)
        }
        "nth-of-type" => {
            let (a, b) = parse_nth(arg.unwrap_or(""));
            Pseudo::NthOfType(a, b)
        }
        "nth-last-of-type" => {
            let (a, b) = parse_nth(arg.unwrap_or(""));
            Pseudo::NthLastOfType(a, b)
        }
        "not" => Pseudo::Not(Box::new(parse_compound(arg.unwrap_or("").trim()))),
        _ => Pseudo::Unknown,
    }
}

/// Parse an `An+B` / `odd` / `even` / integer argument into `(a, b)` coefficients
/// (so the predicate is `nth_match(a, b, index)`). Mirrors selectors.mjs `nthMatch`
/// keyword handling. A pure integer `k` → `(0, k)`; `odd` → `(2, 1)`; `even` → `(2, 0)`.
/// Unparseable → `(0, 0)` which matches index 0 (never, since indices are 1-based).
fn parse_nth(arg: &str) -> (i64, i64) {
    let a0: String = arg.trim().to_ascii_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    if a0 == "odd" {
        return (2, 1);
    }
    if a0 == "even" {
        return (2, 0);
    }
    // An+B form
    if let Some(npos) = a0.find('n') {
        let acoef = &a0[..npos];
        let a = match acoef {
            "" | "+" => 1,
            "-" => -1,
            s => s.parse::<i64>().unwrap_or(0),
        };
        let brest = &a0[npos + 1..];
        let b = if brest.is_empty() {
            0
        } else {
            brest.parse::<i64>().unwrap_or(0)
        };
        return (a, b);
    }
    // bare integer
    match a0.parse::<i64>() {
        Ok(n) => (0, n),
        Err(_) => (0, 0),
    }
}

/// Does 1-based `index` satisfy `an+b`? Mirrors selectors.mjs `nthMatch`:
/// for a==0, exact match; otherwise `(index-b) % a == 0 && (index-b)/a >= 0`.
fn nth_match(a: i64, b: i64, index: i64) -> bool {
    if a == 0 {
        return index == b;
    }
    let d = index - b;
    d % a == 0 && d / a >= 0
}

fn parse_attr(inner: &str) -> Attr {
    match inner.find('=') {
        Some(eq) => {
            // operator char (if any) sits immediately before '='
            let raw_name = &inner[..eq];
            let (op, name) = match raw_name.as_bytes().last() {
                Some(b'~') => (AttrOp::Includes, &raw_name[..raw_name.len() - 1]),
                Some(b'|') => (AttrOp::Dash, &raw_name[..raw_name.len() - 1]),
                Some(b'^') => (AttrOp::Prefix, &raw_name[..raw_name.len() - 1]),
                Some(b'$') => (AttrOp::Suffix, &raw_name[..raw_name.len() - 1]),
                Some(b'*') => (AttrOp::Substr, &raw_name[..raw_name.len() - 1]),
                _ => (AttrOp::Exact, raw_name),
            };
            let mut val = inner[eq + 1..].trim();
            if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                val = &val[1..val.len() - 1];
            }
            Attr { name: name.trim().to_string(), op, value: val.to_string() }
        }
        None => Attr { name: inner.trim().to_string(), op: AttrOp::Presence, value: String::new() },
    }
}

fn attr_op_matches(op: AttrOp, want: &str, got: &str) -> bool {
    match op {
        AttrOp::Presence => true,
        AttrOp::Exact => got == want,
        AttrOp::Prefix => !want.is_empty() && got.starts_with(want),
        AttrOp::Suffix => !want.is_empty() && got.ends_with(want),
        AttrOp::Substr => !want.is_empty() && got.contains(want),
        AttrOp::Includes => !want.is_empty() && got.split_ascii_whitespace().any(|w| w == want),
        AttrOp::Dash => got == want || got.starts_with(&format!("{want}-")),
    }
}

fn parse_complex(src: &str) -> Complex {
    let mut parts = Vec::new();
    let mut combinator = Combinator::Descendant;
    for tok in tokenize_complex(src) {
        if tok == ">" {
            combinator = Combinator::Child;
        } else {
            parts.push((combinator, parse_compound(&tok)));
            combinator = Combinator::Descendant;
        }
    }
    Complex { parts }
}

/// split on whitespace but keep `>` as its own token (with or without surrounding ws).
/// Whitespace and `>` inside an attribute selector `[...]` or a quoted string are NOT separators —
/// e.g. `svg[viewBox="0 0 10 10"]` is one token, not five.
fn tokenize_complex(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth: i32 = 0; // inside [...]
    let mut quote: Option<char> = None;
    for ch in src.chars() {
        if let Some(q) = quote {
            cur.push(ch);
            if ch == q { quote = None; }
            continue;
        }
        match ch {
            '"' | '\'' => { quote = Some(ch); cur.push(ch); }
            '[' => { depth += 1; cur.push(ch); }
            ']' => { depth -= 1; cur.push(ch); }
            c if depth > 0 => cur.push(c),
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            '>' => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                out.push(">".to_string());
            }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// whole-word class scan, alloc-free — mirrors JS `hasClass`.
fn has_class(class_attr: &str, cls: &str) -> bool {
    if cls.is_empty() {
        return false;
    }
    let bytes = class_attr.as_bytes();
    let mut search = 0;
    while let Some(rel) = class_attr[search..].find(cls) {
        let pos = search + rel;
        let before_ok = pos == 0 || bytes[pos - 1].is_ascii_whitespace();
        let end = pos + cls.len();
        let after_ok = end == bytes.len() || bytes[end].is_ascii_whitespace();
        if before_ok && after_ok {
            return true;
        }
        search = end;
    }
    false
}

impl Tree {
    fn matches_compound(&self, h: Handle, c: &Compound) -> bool {
        if self.node_type(h) != NodeType::Element {
            return false;
        }
        if let Some(tag) = &c.tag {
            match self.local_name(h) {
                Some(ln) if ln.eq_ignore_ascii_case(tag) => {}
                _ => return false,
            }
        }
        if let Some(id) = &c.id {
            if self.get_attribute(h, "id") != Some(id.as_str()) {
                return false;
            }
        }
        if !c.classes.is_empty() {
            let cv = match self.get_attribute(h, "class") {
                Some(v) => v,
                None => return false,
            };
            for cls in &c.classes {
                if !has_class(cv, cls) {
                    return false;
                }
            }
        }
        for a in &c.attrs {
            match self.get_attribute(h, &a.name) {
                Some(got) if attr_op_matches(a.op, &a.value, got) => {}
                _ => return false,
            }
        }
        for p in &c.pseudos {
            if !self.matches_pseudo(h, p) {
                return false;
            }
        }
        true
    }

    /// Element-only siblings of `h` (children of its parent that are elements).
    /// Returns `[h]` if `h` has no parent — mirrors selectors.mjs `elementSiblings`.
    fn element_siblings(&self, h: Handle) -> Vec<Handle> {
        match self.parent(h) {
            Some(p) => self
                .children(p)
                .into_iter()
                .filter(|&c| self.node_type(c) == NodeType::Element)
                .collect(),
            None => vec![h],
        }
    }

    fn matches_pseudo(&self, h: Handle, p: &Pseudo) -> bool {
        match p {
            Pseudo::FirstChild => {
                let sibs = self.element_siblings(h);
                sibs.first() == Some(&h)
            }
            Pseudo::LastChild => {
                let sibs = self.element_siblings(h);
                sibs.last() == Some(&h)
            }
            Pseudo::OnlyChild => self.element_siblings(h).len() == 1,
            Pseudo::FirstOfType => {
                let ln = self.local_name(h);
                self.element_siblings(h)
                    .into_iter()
                    .find(|&c| self.local_name(c) == ln)
                    == Some(h)
            }
            Pseudo::LastOfType => {
                let ln = self.local_name(h);
                self.element_siblings(h)
                    .into_iter()
                    .rev()
                    .find(|&c| self.local_name(c) == ln)
                    == Some(h)
            }
            Pseudo::OnlyOfType => {
                let ln = self.local_name(h);
                self.element_siblings(h)
                    .into_iter()
                    .filter(|&c| self.local_name(c) == ln)
                    .count()
                    == 1
            }
            // `position` is always Some (h is among its own element siblings); the
            // map_or default is unreachable but keeps each arm a single covered line.
            Pseudo::NthChild(a, b) => {
                let sibs = self.element_siblings(h);
                sibs.iter().position(|&c| c == h).map_or(false, |idx| nth_match(*a, *b, idx as i64 + 1))
            }
            Pseudo::NthLastChild(a, b) => {
                let sibs = self.element_siblings(h);
                sibs.iter()
                    .position(|&c| c == h)
                    .map_or(false, |idx| nth_match(*a, *b, sibs.len() as i64 - idx as i64))
            }
            Pseudo::NthOfType(a, b) => {
                let ln = self.local_name(h);
                let same: Vec<Handle> =
                    self.element_siblings(h).into_iter().filter(|&c| self.local_name(c) == ln).collect();
                same.iter().position(|&c| c == h).map_or(false, |idx| nth_match(*a, *b, idx as i64 + 1))
            }
            Pseudo::NthLastOfType(a, b) => {
                let ln = self.local_name(h);
                let same: Vec<Handle> =
                    self.element_siblings(h).into_iter().filter(|&c| self.local_name(c) == ln).collect();
                same.iter()
                    .position(|&c| c == h)
                    .map_or(false, |idx| nth_match(*a, *b, same.len() as i64 - idx as i64))
            }
            // :empty — no child nodes at all (any node type counts in the JS, which
            // checks childNodes.length). We have only element children via children();
            // mirror "no children" honestly.
            Pseudo::Empty => self.children(h).is_empty(),
            Pseudo::Root => match self.parent(h) {
                None => true,
                // parent is the document/root container, not an element
                Some(p) => self.node_type(p) != NodeType::Element,
            },
            // Form-state. selectors.mjs reads the live DOM property (el.checked etc.);
            // this Rust port reads the corresponding attribute.
            // TODO: live prop — JS reads el.checked / el.selected (set by React).
            Pseudo::Checked => {
                let ln = self.local_name(h);
                if ln == Some("option") {
                    self.has_attribute(h, "selected")
                } else {
                    self.has_attribute(h, "checked")
                }
            }
            // TODO: live prop — JS reads el.disabled.
            Pseudo::Disabled => self.has_attribute(h, "disabled"),
            // TODO: live prop — JS reads el.disabled.
            Pseudo::Enabled => {
                matches!(
                    self.local_name(h),
                    Some("input") | Some("button") | Some("select") | Some("textarea")
                        | Some("optgroup") | Some("option") | Some("fieldset")
                ) && !self.has_attribute(h, "disabled")
            }
            // TODO: live prop — JS reads el.required.
            Pseudo::Required => self.has_attribute(h, "required"),
            // TODO: live prop — JS reads el.required.
            Pseudo::Optional => {
                matches!(
                    self.local_name(h),
                    Some("input") | Some("select") | Some("textarea")
                ) && !self.has_attribute(h, "required")
            }
            // TODO: live prop — JS reads el.readOnly.
            Pseudo::ReadOnly => self.has_attribute(h, "readonly"),
            // TODO: live prop — JS reads el.readOnly.
            Pseudo::ReadWrite => !self.has_attribute(h, "readonly"),
            // TODO: live prop — JS reads el.selected.
            Pseudo::Selected => self.has_attribute(h, "selected"),
            Pseudo::Not(inner) => !self.matches_compound(h, inner),
            Pseudo::Unknown => false,
        }
    }

    /// Does `h` match the full complex selector, anchored at its rightmost compound?
    fn matches_complex(&self, h: Handle, cx: &Complex) -> bool {
        let n = cx.parts.len();
        let (_, last) = &cx.parts[n - 1];
        if !self.matches_compound(h, last) {
            return false;
        }
        // walk leftwards matching ancestors
        let mut cur = h;
        for k in (0..n - 1).rev() {
            let (combinator, compound) = &cx.parts[k];
            match combinator_for(cx, k + 1) {
                Combinator::Child => {
                    let p = match self.parent(cur) {
                        Some(p) => p,
                        None => return false,
                    };
                    if !self.matches_compound(p, compound) {
                        return false;
                    }
                    cur = p;
                }
                Combinator::Descendant => {
                    let mut anc = self.parent(cur);
                    let mut found = None;
                    while let Some(a) = anc {
                        if self.matches_compound(a, compound) {
                            found = Some(a);
                            break;
                        }
                        anc = self.parent(a);
                    }
                    match found {
                        Some(a) => cur = a,
                        None => return false,
                    }
                }
            }
            let _ = combinator; // combinator stored on the RIGHT part; see combinator_for
        }
        true
    }

    pub fn matches(&self, h: Handle, selector: &str) -> bool {
        selector.split(',').any(|s| {
            let cx = parse_complex(s.trim());
            !cx.parts.is_empty() && self.matches_complex(h, &cx)
        })
    }

    /// querySelectorAll, document-order, version-cached by selector string. Returns a
    /// shared `Rc<[Handle]>` so a cache hit (the common RTL repeated-query case) is a
    /// pointer bump — no Vec copy. Deref to `&[Handle]` for `.len()`/`[i]`/`.iter()`.
    pub fn query_selector_all(&self, selector: &str) -> std::rc::Rc<[Handle]> {
        {
            let mut cache = self.qcache.borrow_mut();
            if cache.version != self.version {
                cache.version = self.version;
                cache.map.clear();
            }
            if let Some(hit) = cache.map.get(selector) {
                return hit.clone();
            }
        }
        let selectors: Vec<Complex> = selector
            .split(',')
            .map(|s| parse_complex(s.trim()))
            .filter(|cx| !cx.parts.is_empty())
            .collect();
        let mut out = Vec::new();
        let mut stack = vec![self.root()];
        // document-order DFS
        let mut order = Vec::new();
        while let Some(h) = stack.pop() {
            order.push(h);
            let kids = self.children(h);
            for &c in kids.iter().rev() {
                stack.push(c);
            }
        }
        for h in order {
            if self.node_type(h) == NodeType::Element
                && selectors.iter().any(|cx| self.matches_complex(h, cx))
            {
                out.push(h);
            }
        }
        let rc: std::rc::Rc<[Handle]> = out.into();
        self.qcache.borrow_mut().map.insert(selector.to_string(), rc.clone());
        rc
    }

    pub fn query_selector(&self, selector: &str) -> Option<Handle> {
        self.query_selector_all(selector).first().copied()
    }

    /// getElementById, version-cached. Shares `qcache` with querySelectorAll under a
    /// `\u{1}`-prefixed key (never a valid selector char, so no collision). Result is
    /// stored as a 0- or 1-element Vec. Mirrors the JS `__idCache`.
    pub fn get_element_by_id(&self, id: &str) -> Option<Handle> {
        let key = {
            let mut k = String::with_capacity(id.len() + 1);
            k.push('\u{1}');
            k.push_str(id);
            k
        };
        {
            let mut cache = self.qcache.borrow_mut();
            if cache.version != self.version {
                cache.version = self.version;
                cache.map.clear();
            }
            if let Some(hit) = cache.map.get(&key) {
                return hit.first().copied();
            }
        }
        let mut found = None;
        let mut stack = vec![self.root()];
        while let Some(h) = stack.pop() {
            if self.node_type(h) == NodeType::Element && self.get_attribute(h, "id") == Some(id) {
                found = Some(h);
                break;
            }
            for &c in self.children(h).iter().rev() {
                stack.push(c);
            }
        }
        self.qcache.borrow_mut().map.insert(key, found.into_iter().collect());
        found
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<Handle> {
        let any = tag == "*";
        let mut out = Vec::new();
        let mut stack = vec![self.root()];
        let mut order = Vec::new();
        while let Some(h) = stack.pop() {
            order.push(h);
            for &c in self.children(h).iter().rev() {
                stack.push(c);
            }
        }
        for h in order {
            if self.node_type(h) == NodeType::Element
                && (any || self.local_name(h).map_or(false, |ln| ln.eq_ignore_ascii_case(tag)))
            {
                out.push(h);
            }
        }
        out
    }
}

/// The combinator that connects part `k` to part `k-1` is stored on part `k`.
fn combinator_for(cx: &Complex, k: usize) -> Combinator {
    cx.parts[k].0
}

#[cfg(test)]
mod tests {
    use super::super::tree::Tree;

    #[test]
    fn qsa_compound_and_descendant() {
        let tree = Tree::parse(
            "<main class=grid><div class='card x'><span class=t>a</span></div><div class=card><b>b</b></div></main>",
        );
        assert_eq!(tree.query_selector_all("div.card").len(), 2);
        assert_eq!(tree.query_selector_all(".grid .t").len(), 1);
        assert_eq!(tree.query_selector_all("main > div").len(), 2);
        assert_eq!(tree.query_selector_all("main > span").len(), 0);
    }

    #[test]
    fn qsa_id_and_attr() {
        let tree = Tree::parse("<a id=home href='/x' data-k=v>hi</a><a href='/y'>z</a>");
        assert_eq!(tree.query_selector("#home").is_some(), true);
        assert_eq!(tree.query_selector_all("a[href]").len(), 2);
        assert_eq!(tree.query_selector_all("a[data-k=v]").len(), 1);
        assert_eq!(tree.query_selector_all("[data-k='v']").len(), 1);
    }

    #[test]
    fn attr_operators() {
        let tree = Tree::parse(
            "<a href='/docs/intro' lang='en-US' class='btn primary' data-x='foobar'>1</a>\
             <a href='/help' lang='en' class='btn' data-x='bazfoo'>2</a>",
        );
        assert_eq!(tree.query_selector_all("a[href^='/docs']").len(), 1);
        assert_eq!(tree.query_selector_all("a[href$='intro']").len(), 1);
        assert_eq!(tree.query_selector_all("a[data-x*='oba']").len(), 1); // foobar
        assert_eq!(tree.query_selector_all("a[class~='primary']").len(), 1);
        assert_eq!(tree.query_selector_all("a[class~='btn']").len(), 2);
        assert_eq!(tree.query_selector_all("a[lang|='en']").len(), 2); // en + en-US
        assert_eq!(tree.query_selector_all("a[href^='/x']").len(), 0);
    }

    #[test]
    fn get_element_by_id_caches_and_invalidates() {
        let mut tree = Tree::parse("<div id=a></div><div id=b></div>");
        let a = tree.get_element_by_id("a").unwrap(); // miss → walk → cache
        assert_eq!(tree.get_element_by_id("a"), Some(a)); // hit path
        assert_eq!(tree.get_element_by_id("missing"), None); // negative cached
        assert_eq!(tree.get_element_by_id("missing"), None); // hit of negative
        // mutation bumps version → cache cleared, new id visible
        let root = tree.root();
        let c = tree.create_element("div");
        tree.set_attribute(c, "id", "c");
        tree.append_child(root, c);
        assert_eq!(tree.get_element_by_id("c"), Some(c));
    }

    #[test]
    fn cache_invalidates_on_mutation() {
        let mut tree = Tree::parse("<ul><li class=x>1</li></ul>");
        assert_eq!(tree.query_selector_all(".x").len(), 1);
        let ul = tree.get_elements_by_tag_name("ul")[0];
        let li = tree.create_element("li");
        tree.set_attribute(li, "class", "x");
        tree.append_child(ul, li);
        assert_eq!(tree.query_selector_all(".x").len(), 2); // cache cleared by version bump
    }

    #[test]
    fn matches_and_get_by_id() {
        let tree = Tree::parse("<div id=root><p class=lead>hi</p></div>");
        let p = tree.query_selector("p.lead").unwrap();
        assert!(tree.matches(p, "div#root p.lead"));
        assert!(tree.matches(p, ".lead"));
        assert!(!tree.matches(p, "span"));
        assert_eq!(tree.get_element_by_id("root").is_some(), true);
    }

    #[test]
    fn pseudo_structural_first_last() {
        let tree = Tree::parse("<ul><li>1</li><li>2</li><li>3</li></ul>");
        assert_eq!(tree.query_selector_all("li:first-child").len(), 1);
        assert_eq!(tree.query_selector_all("li:last-child").len(), 1);
        // the first li matches :first-child and contains text "1"
        let first = tree.query_selector("li:first-child").unwrap();
        assert_eq!(tree.text_content(first), "1");
        let last = tree.query_selector("li:last-child").unwrap();
        assert_eq!(tree.text_content(last), "3");
    }

    #[test]
    fn pseudo_nth_child() {
        let tree = Tree::parse("<ul><li>1</li><li>2</li><li>3</li><li>4</li></ul>");
        // nth-child(2) → exactly the 2nd
        let two = tree.query_selector_all("li:nth-child(2)");
        assert_eq!(two.len(), 1);
        assert_eq!(tree.text_content(two[0]), "2");
        // odd → 1st and 3rd
        let odd = tree.query_selector_all("li:nth-child(odd)");
        assert_eq!(odd.len(), 2);
        assert_eq!(tree.text_content(odd[0]), "1");
        assert_eq!(tree.text_content(odd[1]), "3");
        // even → 2nd and 4th
        assert_eq!(tree.query_selector_all("li:nth-child(even)").len(), 2);
        // 2n+1 == odd
        assert_eq!(tree.query_selector_all("li:nth-child(2n+1)").len(), 2);
    }

    #[test]
    fn pseudo_of_type() {
        let tree = Tree::parse("<div><span>a</span><b>x</b><span>b</span></div>");
        assert_eq!(tree.query_selector_all("span:first-of-type").len(), 1);
        assert_eq!(tree.query_selector_all("span:last-of-type").len(), 1);
        assert_eq!(tree.query_selector_all("b:only-of-type").len(), 1);
        assert_eq!(tree.query_selector_all("span:only-of-type").len(), 0);
    }

    #[test]
    fn attr_value_with_spaces() {
        // quoted attribute value containing spaces must not be split by the complex tokenizer
        let tree = Tree::parse("<svg viewBox=\"0 0 10 10\"></svg>");
        assert_eq!(tree.query_selector_all("svg[viewBox=\"0 0 10 10\"]").len(), 1);
        assert_eq!(tree.query_selector_all("[viewBox=\"0 0 10 10\"]").len(), 1);
        assert_eq!(tree.query_selector_all("svg[viewBox=\"9 9 9 9\"]").len(), 0);
    }

    #[test]
    fn pseudo_checked() {
        let tree = Tree::parse("<form><input checked><input></form>");
        let checked = tree.query_selector_all("input:checked");
        assert_eq!(checked.len(), 1);
        // option[selected] also counts as :checked
        let t2 = Tree::parse("<select><option>a</option><option selected>b</option></select>");
        assert_eq!(t2.query_selector_all("option:checked").len(), 1);
    }

    #[test]
    fn pseudo_disabled_required() {
        let tree = Tree::parse("<form><input disabled><input required><button></button></form>");
        assert_eq!(tree.query_selector_all(":disabled").len(), 1);
        assert_eq!(tree.query_selector_all("input:required").len(), 1);
        // enabled: input/button that isn't disabled (the required input + the button)
        assert_eq!(tree.query_selector_all(":enabled").len(), 2);
    }

    #[test]
    fn pseudo_not() {
        let tree = Tree::parse("<ul><li class=x>1</li><li>2</li><li class=x>3</li></ul>");
        let not_x = tree.query_selector_all("li:not(.x)");
        assert_eq!(not_x.len(), 1);
        assert_eq!(tree.text_content(not_x[0]), "2");
        // :not with a tag
        assert_eq!(tree.query_selector_all("li:not(li)").len(), 0);
    }

    #[test]
    fn pseudo_empty() {
        let tree = Tree::parse("<ul><li></li><li>x</li><li></li></ul>");
        assert_eq!(tree.query_selector_all("li:empty").len(), 2);
    }

    #[test]
    fn pseudo_unknown_matches_nothing() {
        let tree = Tree::parse("<div><p>a</p><p>b</p></div>");
        // an unsupported pseudo never matches (honest)
        assert_eq!(tree.query_selector_all("p:hover").len(), 0);
        assert_eq!(tree.query_selector_all("p:focus-within").len(), 0);
        // but the rest of the compound is still parsed: p without the pseudo matches
        assert_eq!(tree.query_selector_all("p").len(), 2);
    }

    #[test]
    fn pseudo_combined_with_combinator() {
        let tree = Tree::parse(
            "<ul><li class=a>1</li><li class=a>2</li></ul><ol><li>x</li></ol>",
        );
        // descendant + pseudo
        assert_eq!(tree.query_selector_all("ul li:first-child").len(), 1);
        assert_eq!(tree.query_selector_all("ul > li:last-child.a").len(), 1);
    }

    #[test]
    fn pseudo_nth_of_type_and_last() {
        // 2 spans then a b then a span — nth-of-type counts only same-tag siblings
        let tree = Tree::parse(
            "<div><span>1</span><span>2</span><b>x</b><span>3</span></div>",
        );
        // nth-of-type(2) → the 2nd span
        let nt = tree.query_selector_all("span:nth-of-type(2)");
        assert_eq!(nt.len(), 1);
        assert_eq!(tree.text_content(nt[0]), "2");
        // nth-last-of-type(1) → the last span (the 3rd)
        let nlt = tree.query_selector_all("span:nth-last-of-type(1)");
        assert_eq!(nlt.len(), 1);
        assert_eq!(tree.text_content(nlt[0]), "3");
        // nth-last-child(1) → last child of each parent
        let nlc = tree.query_selector_all("span:nth-last-child(1)");
        assert_eq!(nlc.len(), 1);
        assert_eq!(tree.text_content(nlc[0]), "3");
        // nth-of-type with An+B coefficient form covering nth-of-type arm
        assert_eq!(tree.query_selector_all("span:nth-of-type(2n)").len(), 1); // the 2nd span
        // nth-last-of-type An+B form
        assert_eq!(tree.query_selector_all("span:nth-last-of-type(2n+1)").len(), 2);
    }

    #[test]
    fn pseudo_only_child() {
        let tree = Tree::parse("<div><p>solo</p></div><div><p>a</p><p>b</p></div>");
        // only the first div's p is an only-child
        let only = tree.query_selector_all("p:only-child");
        assert_eq!(only.len(), 1);
        assert_eq!(tree.text_content(only[0]), "solo");
    }

    #[test]
    fn pseudo_root() {
        // <html> is the root element (parent is the document container, not an element)
        let tree = Tree::parse("<div>x</div>");
        let root = tree.query_selector_all(":root");
        assert_eq!(root.len(), 1);
        assert_eq!(tree.local_name(root[0]), Some("html"));
        // a nested element is NOT :root (its parent IS an element)
        assert_eq!(tree.query_selector_all("div:root").len(), 0);
    }

    #[test]
    fn pseudo_optional_readonly_readwrite_selected() {
        let tree = Tree::parse(
            "<form>\
               <input>\
               <input required>\
               <input readonly>\
               <select><option>a</option><option selected>b</option></select>\
             </form>",
        );
        // :optional — form fields without `required`: the bare input, the readonly input, and the select
        assert_eq!(tree.query_selector_all("input:optional").len(), 2);
        // :read-only — has the readonly attribute
        assert_eq!(tree.query_selector_all(":read-only").len(), 1);
        // :read-write — does NOT have readonly (all the non-readonly inputs + others)
        let rw = tree.query_selector_all("input:read-write");
        assert_eq!(rw.len(), 2);
        // :selected — the selected option
        let sel = tree.query_selector_all("option:selected");
        assert_eq!(sel.len(), 1);
        assert_eq!(tree.text_content(sel[0]), "b");
    }

    #[test]
    fn nth_no_position_match_via_not() {
        // nth-* of an element with no parent isn't reachable through qSA (root always
        // has a container parent); but exercise the bare-integer-out-of-range nth.
        let tree = Tree::parse("<ul><li>1</li><li>2</li></ul>");
        // nth-child(0) never matches (1-based indices)
        assert_eq!(tree.query_selector_all("li:nth-child(0)").len(), 0);
        // nth-child() with empty arg → parse_nth("") bare-integer Err branch → (0,0) → matches index 0 → never
        assert_eq!(tree.query_selector_all("li:nth-child()").len(), 0);
        // nth-child(n) → a=1,b=0 (brest empty branch): matches every index
        assert_eq!(tree.query_selector_all("li:nth-child(n)").len(), 2);
    }

    #[test]
    fn nested_not_balanced_parens() {
        // a nested paren inside :not(...) exercises the depth-increment parse branch
        // (the inner `(2)` raises paren depth before the outer `)` closes it).
        // :not(:nth-child(2)) → all <p> EXCEPT the 2nd → here only the 1st.
        let tree = Tree::parse("<div><p>a</p><p>b</p></div>");
        let r = tree.query_selector_all("p:not(:nth-child(2))");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.text_content(r[0]), "a");
    }

    #[test]
    fn compound_trailing_non_part_byte() {
        // After consuming a `[...]` attr the next byte is a plain char (not .#[:),
        // exercising the catch-all `_ => i += 1` arm in parse_compound (it skips
        // the stray byte). `div[id=x]y` → the trailing `y` is ignored.
        let tree = Tree::parse("<div id=x>hit</div>");
        assert_eq!(tree.query_selector_all("div[id=x]y").len(), 1);
    }

    #[test]
    fn tokenize_child_combinator_no_spaces() {
        // `a>b` with no surrounding whitespace: tokenize must flush the non-empty
        // `cur` ("section") when it hits `>`.
        let tree = Tree::parse("<section><div>x</div></section>");
        assert_eq!(tree.query_selector_all("section>div").len(), 1);
    }

    #[test]
    fn has_class_multi_word_scan() {
        // class value where the target word appears only after a non-matching prefix
        // substring — exercises the search-advance loop in has_class.
        let tree = Tree::parse("<div class='cardx card'>a</div><div class='cardx'>b</div>");
        // ".card" must whole-word match only the first (cardx is not "card")
        assert_eq!(tree.query_selector_all("div.card").len(), 1);
        // empty class selector never matches (has_class empty-cls guard) — via attr form
        // a selector like ".." parses an empty class then a second empty class
        assert_eq!(tree.query_selector_all(".").len(), 0);
    }

    #[test]
    fn child_combinator_fails_at_ancestor() {
        // `a > b` where b exists but its parent isn't `a` → Child combinator returns
        // false at the parent check.
        let tree = Tree::parse("<section><div><span>x</span></div></section>");
        // span's parent is div, not section → no match
        assert_eq!(tree.query_selector_all("section > span").len(), 0);
        // chain where the leftmost child combinator's parent is the root container
        // (parent() is Some but not matching) is covered above; force the parent()=None
        // path: `x > html` — html's parent is the document container, matches_compound
        // on a non-element parent returns false (handled), here parent IS Some.
        assert_eq!(tree.query_selector_all("span > nothing").len(), 0);
    }

    #[test]
    fn descendant_combinator_fails() {
        // `nomatch span` — no ancestor matches `nomatch` → descendant walk exhausts.
        let tree = Tree::parse("<div><span>x</span></div>");
        assert_eq!(tree.query_selector_all("nomatch span").len(), 0);
    }

    #[test]
    fn get_element_by_id_hit_and_miss() {
        let tree = Tree::parse("<div id=present><p>x</p></div>");
        assert!(tree.get_element_by_id("present").is_some());
        assert!(tree.get_element_by_id("absent").is_none());
    }

    #[test]
    fn detached_element_pseudos_and_child_parent_none() {
        // A freshly created element has no parent. `matches` drives the no-parent
        // branches directly:
        let mut tree = Tree::parse("<div>x</div>");
        let orphan = tree.create_element("p");
        // element_siblings → vec![h] fallback (no parent), so :first-child is true
        assert!(tree.matches(orphan, "p:first-child"));
        assert!(tree.matches(orphan, "p:last-child"));
        assert!(tree.matches(orphan, "p:only-child"));
        // :root true via parent()==None
        assert!(tree.matches(orphan, ":root"));
        // child combinator whose left side needs a parent that doesn't exist → false
        assert!(!tree.matches(orphan, "div > p"));
    }

    #[test]
    fn get_elements_by_tag_name_wildcard() {
        let tree = Tree::parse("<div><p>a</p><span>b</span></div>");
        // "*" returns every element (html, head, body, div, p, span)
        let all = tree.get_elements_by_tag_name("*");
        assert!(all.len() >= 4);
        // a specific tag, case-insensitive
        assert_eq!(tree.get_elements_by_tag_name("P").len(), 1);
        assert_eq!(tree.get_elements_by_tag_name("nope").len(), 0);
    }
}

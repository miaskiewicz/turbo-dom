//! Selector matching + query API over `Tree`. Mirrors the JS runtime's
//! alloc-free matcher discipline (whole-word class scan, index loops, no regex)
//! and version-keyed result caching (`Document.__version` → `Tree.version`).
//!
//! Selector support: comma lists, descendant (` `), child (`>`), adjacent (`+`)
//! and general-sibling (`~`) combinators, compounds of `tag` / `.class` / `#id` /
//! `[attr]` / `[attr=val]`, the `:is()` / `:where()` / `:not()` selector-list
//! pseudo-classes, and the relational `:has()`. Enough for RTL-style queries;
//! extend as the gauntlet demands.

use super::tree::{Handle, NodeType, Tree};

/// How an `[attr...]` test compares against the element's attribute value. The
/// operator and its operand are ONE inseparable thing: a presence test `[attr]`
/// carries no value, and every value-bearing operator carries its value. This
/// makes "presence with a value" / "equals with no value" unrepresentable — the
/// old `Attr { op, value }` could express both.
#[derive(Debug, Clone)]
enum AttrMatch {
    Present,          // [attr]
    Equals(String),   // [attr=v]
    Includes(String), // [attr~=v]  whitespace-separated word
    Dash(String),     // [attr|=v]  v or v-...
    Prefix(String),   // [attr^=v]
    Suffix(String),   // [attr$=v]
    Substr(String),   // [attr*=v]
}

impl AttrMatch {
    /// Does `got` (the element's attribute value) satisfy this test? SAME
    /// semantics as the old `attr_op_matches`: the `!want.is_empty()` guards on
    /// prefix/suffix/substr/includes are preserved, `Dash` = `v` or `v-…`.
    fn matches(&self, got: &str) -> bool {
        match self {
            AttrMatch::Present => true,
            AttrMatch::Equals(want) => got == want,
            AttrMatch::Prefix(want) => !want.is_empty() && got.starts_with(want.as_str()),
            AttrMatch::Suffix(want) => !want.is_empty() && got.ends_with(want.as_str()),
            AttrMatch::Substr(want) => !want.is_empty() && got.contains(want.as_str()),
            AttrMatch::Includes(want) => {
                !want.is_empty() && got.split_ascii_whitespace().any(|w| w == want)
            }
            AttrMatch::Dash(want) => got == want || got.starts_with(&format!("{want}-")),
        }
    }
}

#[derive(Debug, Clone)]
struct Attr {
    name: String,
    m: AttrMatch,
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
    // Matching pseudos whose argument is a SELECTOR LIST (comma-separated complex
    // selectors), each anchored at the element under test. `:where` matches
    // identically to `:is` (the specificity difference is a cascade concern, out of
    // scope here); `:not` is the negation (NONE of the list may match).
    Is(Vec<Complex>),
    Where(Vec<Complex>),
    Not(Vec<Complex>),
    // `:has(<relative-selector-list>)` — relational. Each entry is a leading
    // combinator (None = descendant) paired with the relative complex selector.
    // `el:has(S)` is true iff SOME element reachable from `el` per S's leading
    // combinator matches S, scoped to `el`. Empty list → never matches.
    Has(Vec<RelativeSelector>),
    // Unrecognized / unparseable → never matches
    Unknown,
}

/// One relative selector inside `:has()`: an optional leading combinator (None =
/// descendant) and the complex selector that follows. `> li.active` → `(Child,
/// li.active)`; `.a .b` → `(Descendant, .a .b)`.
#[derive(Debug, Clone)]
struct RelativeSelector {
    combinator: Combinator,
    complex: Complex,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Combinator {
    Descendant,
    Child,
    Adjacent,       // + immediately-preceding element sibling
    GeneralSibling, // ~ any preceding element sibling
}

/// A complex selector = a non-empty head compound + a tail of
/// `(combinator, compound)` pairs, left-to-right. In `rest`, entry `i`'s
/// combinator connects its compound to the one on its LEFT (`rest[i-1].1`, or
/// `first` when `i==0`). Modeling the head separately makes "empty complex" and
/// "meaningless leading combinator" unrepresentable — `parse_complex` returns
/// `Option<Complex>`, so a `Complex` is always non-empty by construction.
#[derive(Debug, Clone)]
struct Complex {
    first: Compound,
    rest: Vec<(Combinator, Compound)>,
}

/// A lexical token. The lexer is the ONE pass that centrally handles quoted
/// strings and balanced `[...]` / `(...)`; everything the grammar sees is already
/// well-formed tokens, so the recursive-descent parser never tracks bracket/paren
/// depth itself (the class of bug this refactor exists to kill).
///
/// `*` and digits are ordinary identifier characters here, so a leading `*` lexes
/// as `Ident("*")` (the parser maps it to "any tag") and `*foo` stays `Ident("*foo")`
/// — preserving the old substring-tag semantics. `[..]` / `(..)` carry their already
/// quote-/depth-balanced inner text, so `svg[viewBox="0 0 10 10"]` is one `Attr` token
/// and `:not(:nth-child(2))` is one `Paren` token.
enum Token {
    Ident(String), // run of non-special chars: tag / class / id / pseudo-name / stray
    Hash,          // #
    Dot,           // .
    Colon,         // :
    Attr(String),  // inner text of a balanced `[ ... ]`
    Paren(String), // inner text of a balanced `( ... )` (a pseudo argument)
    Ws,            // a run of significant whitespace (descendant separator)
    Gt,            // > child combinator
    Plus,          // + adjacent-sibling combinator
    Tilde,         // ~ general-sibling combinator
}

/// is `ch` a token boundary (the lexer can never put it inside an `Ident`)?
#[inline]
fn is_special(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '>' | '+' | '~' | '#' | '.' | ':' | '[' | ']' | '(' | ')')
}

/// Scan from just past an opening `open` to its matching `close`, honoring quotes
/// and nesting. Returns the inner text and leaves `*i` on the closing delimiter
/// (or at end if unterminated). Centralizing this is the whole point: `[...]` and
/// `(...)` share one balanced, quote-aware scanner instead of two ad-hoc loops.
fn scan_balanced(chars: &[char], i: &mut usize, open: char, close: char) -> String {
    let start = *i;
    let mut depth = 1i32;
    let mut quote: Option<char> = None;
    while *i < chars.len() {
        let c = chars[*i];
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        *i += 1;
    }
    chars[start..*i].iter().collect()
}

/// Lex one complex selector (already comma-split + trimmed) into tokens.
fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let ch = chars[i];
        match ch {
            c if c.is_whitespace() => {
                // collapse a whitespace run into a single descendant separator
                while i < n && chars[i].is_whitespace() {
                    i += 1;
                }
                out.push(Token::Ws);
            }
            '>' => {
                out.push(Token::Gt);
                i += 1;
            }
            '+' => {
                out.push(Token::Plus);
                i += 1;
            }
            '~' => {
                out.push(Token::Tilde);
                i += 1;
            }
            '#' => {
                out.push(Token::Hash);
                i += 1;
            }
            '.' => {
                out.push(Token::Dot);
                i += 1;
            }
            ':' => {
                out.push(Token::Colon);
                i += 1;
            }
            '[' => {
                i += 1; // past '['
                let inner = scan_balanced(&chars, &mut i, '[', ']');
                if i < n {
                    i += 1; // past ']'
                }
                out.push(Token::Attr(inner));
            }
            '(' => {
                i += 1; // past '('
                let inner = scan_balanced(&chars, &mut i, '(', ')');
                if i < n {
                    i += 1; // past ')'
                }
                out.push(Token::Paren(inner));
            }
            // a stray closer at top level isn't part of any token — drop it
            ']' | ')' => i += 1,
            _ => {
                let start = i;
                while i < n && !is_special(chars[i]) {
                    i += 1;
                }
                out.push(Token::Ident(chars[start..i].iter().collect()));
            }
        }
    }
    out
}

/// Consume the `Ident` at `*i` (advancing past it) or `""` if the current token
/// isn't an `Ident`. Lets `.`/`#`/`:` with no following name yield an empty
/// class/id/pseudo-name — exactly the old "read until the next part-start" result.
fn take_ident(toks: &[Token], i: &mut usize) -> String {
    if let Some(Token::Ident(s)) = toks.get(*i) {
        let s = s.clone();
        *i += 1;
        s
    } else {
        String::new()
    }
}

/// Parse a single compound starting at `*i`, stopping at a combinator (`Ws`/`Gt`)
/// or end. Leniency preserved: a bare `Ident` in the suffix position (e.g. the
/// trailing `y` of `div[id=x]y`) is skipped, an unknown pseudo becomes
/// `Pseudo::Unknown`, and a leading `Ident` of `"*"` means "any tag".
fn parse_compound_tokens(toks: &[Token], i: &mut usize) -> Compound {
    let mut c = Compound::default();
    // optional leading type selector
    if let Some(Token::Ident(s)) = toks.get(*i) {
        if s != "*" {
            c.tag = Some(s.clone());
        }
        *i += 1;
    }
    while let Some(tok) = toks.get(*i) {
        match tok {
            Token::Ws | Token::Gt | Token::Plus | Token::Tilde => break,
            Token::Dot => {
                *i += 1;
                c.classes.push(take_ident(toks, i));
            }
            Token::Hash => {
                *i += 1;
                c.id = Some(take_ident(toks, i));
            }
            Token::Colon => {
                *i += 1;
                let name = take_ident(toks, i);
                // borrow the paren's argument text — parse_pseudo takes &str, so the
                // old `.clone()` here only to immediately re-borrow was pure waste.
                let arg = match toks.get(*i) {
                    Some(Token::Paren(p)) => {
                        *i += 1;
                        Some(p.as_str())
                    }
                    _ => None,
                };
                c.pseudos.push(parse_pseudo(&name, arg));
            }
            Token::Attr(inner) => {
                c.attrs.push(parse_attr(inner));
                *i += 1;
            }
            // bare ident (lenient trailing junk) or stray paren → ignore
            Token::Ident(_) | Token::Paren(_) => *i += 1,
        }
    }
    c
}

/// Split a selector string on TOP-LEVEL commas only — commas inside `[...]`,
/// `(...)`, or quoted strings are NOT separators. Mirrors the JS `splitTopLevel`
/// and is required now that `:is()/:where()/:not()` arguments hold comma-separated
/// lists: a naive `split(',')` would break `li:not(.a, .b)` mid-paren.
fn split_top_level_commas(src: &str) -> Vec<&str> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    let mut last = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            quote = Some(b);
        } else if b == b'(' || b == b'[' {
            depth += 1;
        } else if b == b')' || b == b']' {
            depth -= 1;
        } else if b == b',' && depth == 0 {
            out.push(&src[last..i]);
            last = i + 1;
        }
    }
    out.push(&src[last..]);
    out
}

/// Parse a pseudo argument as a SELECTOR LIST (comma-separated complex selectors)
/// for `:is(...)` / `:where(...)` / `:not(...)`. Each non-empty segment becomes a
/// `Complex` (reusing `parse_complex`); empty segments are dropped. A single-item
/// list is the degenerate `:not(.x)` case, so existing behavior is preserved.
fn parse_selector_list_str(src: &str) -> Vec<Complex> {
    split_top_level_commas(src).into_iter().filter_map(|s| parse_complex(s.trim())).collect()
}

/// Parse a `:has()` argument into a list of relative selectors. Each top-level
/// comma segment may begin with a combinator (`>`/`+`/`~`); absent ⇒ descendant.
/// The remainder is parsed as a normal complex (reusing `parse_complex`). Segments
/// that yield no complex (empty, or a lone combinator) are dropped.
fn parse_relative_selector_list(src: &str) -> Vec<RelativeSelector> {
    split_top_level_commas(src)
        .into_iter()
        .filter_map(|seg| {
            let seg = seg.trim_start();
            let (combinator, rest) = match seg.as_bytes().first() {
                Some(b'>') => (Combinator::Child, &seg[1..]),
                Some(b'+') => (Combinator::Adjacent, &seg[1..]),
                Some(b'~') => (Combinator::GeneralSibling, &seg[1..]),
                _ => (Combinator::Descendant, seg),
            };
            parse_complex(rest.trim()).map(|complex| RelativeSelector { combinator, complex })
        })
        .collect()
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
        "is" => Pseudo::Is(parse_selector_list_str(arg.unwrap_or(""))),
        "where" => Pseudo::Where(parse_selector_list_str(arg.unwrap_or(""))),
        "not" => Pseudo::Not(parse_selector_list_str(arg.unwrap_or(""))),
        "has" => Pseudo::Has(parse_relative_selector_list(arg.unwrap_or(""))),
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
            let mut val = inner[eq + 1..].trim();
            if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                val = &val[1..val.len() - 1];
            }
            let val = val.to_string();
            let (m, name) = match raw_name.as_bytes().last() {
                Some(b'~') => (AttrMatch::Includes(val), &raw_name[..raw_name.len() - 1]),
                Some(b'|') => (AttrMatch::Dash(val), &raw_name[..raw_name.len() - 1]),
                Some(b'^') => (AttrMatch::Prefix(val), &raw_name[..raw_name.len() - 1]),
                Some(b'$') => (AttrMatch::Suffix(val), &raw_name[..raw_name.len() - 1]),
                Some(b'*') => (AttrMatch::Substr(val), &raw_name[..raw_name.len() - 1]),
                _ => (AttrMatch::Equals(val), raw_name),
            };
            Attr { name: name.trim().to_string(), m }
        }
        None => Attr { name: inner.trim().to_string(), m: AttrMatch::Present },
    }
}

/// Parse one complex selector (already comma-split + trimmed) into an
/// `Option<Complex>` — `None` when the segment yields no compound at all (empty
/// or combinator-only), so empty segments are dropped at PARSE time and a built
/// `Complex` is always non-empty. A `Gt` between two compounds is a child
/// combinator; otherwise (whitespace or nothing) it is descendant — matching the
/// old "default descendant, `>` ⇒ child" rule exactly. Leading/trailing/extra
/// `Ws` are inert separators, and a leading/trailing combinator simply never
/// lands in `rest` (so `a >` parses to `Complex { first: a, rest: [] }` and
/// matches exactly what `a` matches).
fn parse_complex(src: &str) -> Option<Complex> {
    let toks = tokenize(src);
    let mut first: Option<Compound> = None;
    let mut rest: Vec<(Combinator, Compound)> = Vec::new();
    let mut combinator = Combinator::Descendant;
    let mut i = 0;
    while i < toks.len() {
        match toks[i] {
            Token::Ws => i += 1,
            Token::Gt => {
                combinator = Combinator::Child;
                i += 1;
            }
            Token::Plus => {
                combinator = Combinator::Adjacent;
                i += 1;
            }
            Token::Tilde => {
                combinator = Combinator::GeneralSibling;
                i += 1;
            }
            _ => {
                let c = parse_compound_tokens(&toks, &mut i);
                if first.is_none() {
                    // the head carries no combinator (a leading `>` is inert)
                    first = Some(c);
                } else {
                    rest.push((combinator, c));
                }
                combinator = Combinator::Descendant;
            }
        }
    }
    first.map(|first| Complex { first, rest })
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
                Some(got) if a.m.matches(got) => {}
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

    /// The immediately-preceding ELEMENT sibling of `h` (walking `previous_sibling`
    /// and skipping text/comment nodes). Mirrors selectors.mjs `previousElement`.
    fn previous_element_sibling(&self, h: Handle) -> Option<Handle> {
        let mut n = self.previous_sibling(h);
        while let Some(s) = n {
            if self.node_type(s) == NodeType::Element {
                return Some(s);
            }
            n = self.previous_sibling(s);
        }
        None
    }

    /// The immediately-FOLLOWING element sibling of `h` (walking `next_sibling`,
    /// skipping text/comment nodes). Mirrors selectors.mjs `nextElement`.
    fn next_element_sibling(&self, h: Handle) -> Option<Handle> {
        let mut n = self.next_sibling(h);
        while let Some(s) = n {
            if self.node_type(s) == NodeType::Element {
                return Some(s);
            }
            n = self.next_sibling(s);
        }
        None
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
            // selector-list pseudos: each Complex is anchored at `h` (matches_complex
            // tests the rightmost compound against `h` and walks left).
            Pseudo::Is(list) | Pseudo::Where(list) => {
                list.iter().any(|cx| self.matches_complex(h, cx))
            }
            Pseudo::Not(list) => !list.iter().any(|cx| self.matches_complex(h, cx)),
            // relational: SOME relative selector finds a matching element reachable
            // from `h` per its leading combinator, scoped to `h`'s subtree.
            Pseudo::Has(list) => list.iter().any(|rel| self.has_match(h, rel)),
            Pseudo::Unknown => false,
        }
    }

    /// The compound at position `k` in the chain: `k==0` is the head (`first`),
    /// `k>0` is `rest[k-1].1`. (`rest` entry `i` pairs the combinator on its left
    /// with the compound at chain position `i+1`.)
    fn compound_at(cx: &Complex, k: usize) -> &Compound {
        if k == 0 {
            &cx.first
        } else {
            &cx.rest[k - 1].1
        }
    }

    /// Does `h` match the full complex selector, anchored at its rightmost compound?
    /// Recursive with backtracking: a descendant combinator tries EVERY matching
    /// ancestor, not just the nearest, so a mixed chain such as `.a > .b .c` — where
    /// the `.b` that is a direct child of `.a` is *farther* from `.c` than another
    /// `.b` — is matched correctly. A greedy nearest-ancestor walk would miss it.
    fn matches_complex(&self, h: Handle, cx: &Complex) -> bool {
        self.matches_chain(h, cx, cx.rest.len())
    }

    /// Match the chain prefix ending at position `k` against element `h`. The
    /// combinator in `rest[k-1]` connects position `k` to position `k-1`
    /// (see `parse_complex`); position 0 (the head) has no combinator.
    fn matches_chain(&self, h: Handle, cx: &Complex, k: usize) -> bool {
        if !self.matches_compound(h, Self::compound_at(cx, k)) {
            return false;
        }
        if k == 0 {
            return true; // matched the leftmost compound — the whole chain is satisfied
        }
        match cx.rest[k - 1].0 {
            // the direct parent must match the remaining prefix
            Combinator::Child => match self.parent(h) {
                Some(p) => self.matches_chain(p, cx, k - 1),
                None => false,
            },
            // ANY ancestor may match the remaining prefix — try each, backtracking
            Combinator::Descendant => {
                let mut anc = self.parent(h);
                while let Some(a) = anc {
                    if self.matches_chain(a, cx, k - 1) {
                        return true;
                    }
                    anc = self.parent(a);
                }
                false
            }
            // the IMMEDIATELY-preceding element sibling must match the prefix
            // (single step — mirrors the JS `'+'` arm).
            Combinator::Adjacent => match self.previous_element_sibling(h) {
                Some(prev) => self.matches_chain(prev, cx, k - 1),
                None => false,
            },
            // ANY preceding element sibling may match the prefix — try each,
            // backtracking (like Descendant but over previous element siblings;
            // mirrors the JS `'~'` arm).
            Combinator::GeneralSibling => {
                let mut prev = self.previous_element_sibling(h);
                while let Some(p) = prev {
                    if self.matches_chain(p, cx, k - 1) {
                        return true;
                    }
                    prev = self.previous_element_sibling(p);
                }
                false
            }
        }
    }

    /// `:has()` forward existence search. Given the scope element `h` and one
    /// relative selector, enumerate the candidate set per the leading combinator
    /// and test the relative complex FORWARD (the opposite direction from the normal
    /// right-to-left matcher), confined to `h`'s scope where applicable:
    ///   - Descendant: SOME descendant of `h` matches (ancestor walk capped at `h`).
    ///   - Child (`>`): SOME direct element child matches.
    ///   - Adjacent (`+`): `h`'s next element sibling matches.
    ///   - GeneralSibling (`~`): SOME following element sibling matches.
    fn has_match(&self, h: Handle, rel: &RelativeSelector) -> bool {
        match rel.combinator {
            Combinator::Descendant => {
                // every descendant of `h`, capping the relative complex's own
                // ancestor walk at `h` so a multi-compound `.a .b` stays in scope.
                self.descendants(h)
                    .into_iter()
                    .any(|d| self.matches_complex_scoped(d, &rel.complex, Some(h)))
            }
            Combinator::Child => self
                .children(h)
                .into_iter()
                .filter(|&c| self.node_type(c) == NodeType::Element)
                .any(|c| self.matches_complex_scoped(c, &rel.complex, Some(h))),
            Combinator::Adjacent => match self.next_element_sibling(h) {
                // the sibling is outside `h`'s subtree → no scope cap
                Some(sib) => self.matches_complex_scoped(sib, &rel.complex, None),
                None => false,
            },
            Combinator::GeneralSibling => {
                let mut sib = self.next_element_sibling(h);
                while let Some(s) = sib {
                    if self.matches_complex_scoped(s, &rel.complex, None) {
                        return true;
                    }
                    sib = self.next_element_sibling(s);
                }
                false
            }
        }
    }

    /// Like `matches_complex`, but ancestor/parent steps (Descendant/Child) must
    /// stay strictly BELOW `boundary` (exclusive) when one is given — so a `:has()`
    /// relative complex cannot escape the scope element's subtree. With `None`
    /// boundary this is exactly `matches_complex` (used for sibling candidates).
    fn matches_complex_scoped(&self, h: Handle, cx: &Complex, boundary: Option<Handle>) -> bool {
        self.matches_chain_scoped(h, cx, cx.rest.len(), boundary)
    }

    fn matches_chain_scoped(
        &self,
        h: Handle,
        cx: &Complex,
        k: usize,
        boundary: Option<Handle>,
    ) -> bool {
        if !self.matches_compound(h, Self::compound_at(cx, k)) {
            return false;
        }
        if k == 0 {
            return true;
        }
        // a parent step that would reach `boundary` (the scope element) or above
        // leaves the scope → reject (the relative selector must stay inside).
        let within = |p: Handle| boundary != Some(p);
        match cx.rest[k - 1].0 {
            Combinator::Child => match self.parent(h) {
                Some(p) if within(p) => self.matches_chain_scoped(p, cx, k - 1, boundary),
                _ => false,
            },
            Combinator::Descendant => {
                let mut anc = self.parent(h);
                while let Some(a) = anc {
                    if !within(a) {
                        break; // reached the scope boundary — stop ascending
                    }
                    if self.matches_chain_scoped(a, cx, k - 1, boundary) {
                        return true;
                    }
                    anc = self.parent(a);
                }
                false
            }
            Combinator::Adjacent => match self.previous_element_sibling(h) {
                Some(prev) => self.matches_chain_scoped(prev, cx, k - 1, boundary),
                None => false,
            },
            Combinator::GeneralSibling => {
                let mut prev = self.previous_element_sibling(h);
                while let Some(p) = prev {
                    if self.matches_chain_scoped(p, cx, k - 1, boundary) {
                        return true;
                    }
                    prev = self.previous_element_sibling(p);
                }
                false
            }
        }
    }

    pub fn matches(&self, h: Handle, selector: &str) -> bool {
        // empty/combinator-only segments are dropped at parse time (filter_map),
        // so every `cx` reaching the matcher is a non-empty `Complex`.
        split_top_level_commas(selector)
            .into_iter()
            .filter_map(|s| parse_complex(s.trim()))
            .any(|cx| self.matches_complex(h, &cx))
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
        let selectors: Vec<Complex> = split_top_level_commas(selector)
            .into_iter()
            .filter_map(|s| parse_complex(s.trim()))
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

    #[test]
    fn nth_child_with_whitespace_in_arg() {
        // CSS `An+B` notation permits whitespace: `2n + 1` is valid and means "odd".
        // The complex tokenizer must not split on the spaces inside the pseudo's
        // parentheses (it already protects `[...]` and quotes; parens are the same).
        let tree = Tree::parse("<ul><li>1</li><li>2</li><li>3</li><li>4</li></ul>");
        let odd = tree.query_selector_all("li:nth-child(2n + 1)");
        assert_eq!(odd.len(), 2);
        assert_eq!(tree.text_content(odd[0]), "1");
        assert_eq!(tree.text_content(odd[1]), "3");
    }

    #[test]
    fn parser_parity_corpus() {
        // The tricky-case corpus that the tokenizer + recursive-descent parser must
        // handle identically to the old ad-hoc scanner (mirrors the parallel JS effort).
        // Each selector is exercised against a fixture chosen to make it match exactly once.
        let cases: &[(&str, &str)] = &[
            ("div.card", "<div class='card'>x</div>"),
            (".grid .t", "<div class='grid'><span class='t'>x</span></div>"),
            ("main > div", "<main><div>x</div></main>"),
            ("a[href]", "<a href='/x'>x</a>"),
            ("a[data-k=v]", "<a data-k='v'>x</a>"),
            ("[data-k='v']", "<a data-k='v'>x</a>"),
            ("a[href^='/docs']", "<a href='/docs/intro'>x</a>"),
            ("a[data-x*='oba']", "<a data-x='foobar'>x</a>"),
            ("a[class~='primary']", "<a class='btn primary'>x</a>"),
            ("a[lang|='en']", "<a lang='en-US'>x</a>"),
            ("svg[viewBox=\"0 0 10 10\"]", "<svg viewBox=\"0 0 10 10\"></svg>"),
            ("li:nth-child(2n + 1)", "<ul><li>1</li></ul>"),
            (".a > .b .c", "<div class='a'><div class='b'><div class='c'>c</div></div></div>"),
            ("div[id=x]y", "<div id='x'>hit</div>"), // lenient: trailing `y` ignored → matches
        ];
        for (sel, html) in cases {
            let tree = Tree::parse(html);
            assert_eq!(
                tree.query_selector_all(sel).len(),
                1,
                "selector {sel:?} should match exactly once in {html:?}"
            );
        }
        // :not with a NESTED pseudo whose argument itself contains balanced parens.
        let tree = Tree::parse("<div><p>a</p><p>b</p></div>");
        let r = tree.query_selector_all(":not(:nth-child(2))");
        // every element except the 2nd child of its parent: html, head, body, div, the 1st p
        assert!(r.iter().any(|&h| tree.local_name(h) == Some("p")));
        assert_eq!(
            tree.query_selector_all("p:not(:nth-child(2))").len(),
            1,
            "only the first <p> survives :not(:nth-child(2))"
        );
    }

    #[test]
    fn tokenizer_lenient_edges() {
        let tree = Tree::parse("<div><p>x</p></div>");
        // universal `*` selector: parsed compound has tag None → matches every element
        // (exercises the `s == \"*\"` any-tag branch of parse_compound_tokens).
        let all = tree.query_selector_all("*");
        assert!(all.len() >= 4); // html, head, body, div, p
        assert_eq!(tree.query_selector_all("*.nope").len(), 0); // `*` + a class that matches nothing
        // a stray top-level closer is not part of any token — it is dropped, so the
        // selector degrades to the rest (exercises the `']' | ')'` arm of tokenize;
        // without that arm the ident scanner would stall on the special char).
        assert_eq!(tree.query_selector_all("div]").len(), 1);
        assert_eq!(tree.query_selector_all("p)").len(), 1);
    }

    #[test]
    fn descendant_then_child_backtracks() {
        // `.a > .b .c`: a `.c` inside a `.b` that is a DIRECT child of `.a`.
        // The matching `.b` (the outer one) is a child of `.a`; a *nearer* `.b`
        // (the inner one) is NOT a child of `.a`. A greedy nearest-ancestor matcher
        // picks the inner `.b`, fails the `> .a` child check, and wrongly reports no
        // match. The correct answer is 1 — the outer `.b` satisfies the chain.
        let tree = Tree::parse(
            "<div class=a>\
               <div class=b>\
                 <div class=x>\
                   <div class=b>\
                     <div class=c>c</div>\
                   </div>\
                 </div>\
               </div>\
             </div>",
        );
        assert_eq!(tree.query_selector_all(".a > .b .c").len(), 1);
    }

    #[test]
    fn dangling_trailing_combinator_matches_head_only() {
        // A trailing combinator with nothing after it (`a >`) is inert: it never
        // lands in `Complex.rest`, so the selector is exactly `a`. This preserves
        // the old behavior BY RESULT now that "leading/trailing combinator" is
        // unrepresentable in the AST (head+tail).
        let tree = Tree::parse(
            "<section><div><a>1</a></div><a>2</a></section>",
        );
        let plain = tree.query_selector_all("a");
        let dangling = tree.query_selector_all("a >");
        assert_eq!(dangling.len(), plain.len());
        assert_eq!(*dangling, *plain); // same handles, same document order
        // and `matches` agrees element-by-element
        for &h in plain.iter() {
            assert!(tree.matches(h, "a >"));
        }
        // a leading combinator is likewise inert: `> a` == `a`
        assert_eq!(*tree.query_selector_all("> a"), *plain);
    }

    #[test]
    fn adjacent_sibling_combinator() {
        // `a + b`: b must be the element IMMEDIATELY after an `a` sibling. A non-
        // element node (text) between them is skipped (previous_element_sibling).
        let tree = Tree::parse(
            "<div><h2>t</h2> <p>after</p><span>x</span><p>not-after-h2</p></div>",
        );
        // the first <p> is the immediate element sibling after <h2> → matches once
        let r = tree.query_selector_all("h2 + p");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.text_content(r[0]), "after");
        // `span + p` → the 2nd <p> immediately follows the <span>
        let r2 = tree.query_selector_all("span + p");
        assert_eq!(r2.len(), 1);
        assert_eq!(tree.text_content(r2[0]), "not-after-h2");
        // no previous element sibling → no match (the <h2> is first)
        assert_eq!(tree.query_selector_all("p + h2").len(), 0);
    }

    #[test]
    fn general_sibling_combinator() {
        // `a ~ b`: b is ANY element preceded (at any distance) by a matching `a`.
        let tree = Tree::parse(
            "<div><span class=mark>m</span><p>1</p><b>x</b><p>2</p></div>",
        );
        // both <p> follow the .mark span → 2 matches
        assert_eq!(tree.query_selector_all(".mark ~ p").len(), 2);
        // a sibling with no preceding .mark → exhausts to false
        let t2 = Tree::parse("<div><p>1</p><span class=mark>m</span></div>");
        assert_eq!(t2.query_selector_all(".mark ~ p").len(), 0);
    }

    #[test]
    fn general_sibling_backtracks() {
        // Mirrors the descendant backtracking case but over PREVIOUS element
        // siblings: `.a ~ .b ~ .c` must try every preceding sibling, not just the
        // nearest. Here .c is preceded by .b which is preceded by .a, but a SECOND
        // .b sits between .a and the first .b with no .a before it directly — the
        // matcher must backtrack across siblings to satisfy the full chain.
        let tree = Tree::parse(
            "<ul>               <li class=a>a</li>               <li class=q>q</li>               <li class=b>b1</li>               <li class=c>c</li>             </ul>",
        );
        // .a ~ .b ~ .c : .c's preceding .b is li.b, whose preceding .a is li.a → 1
        assert_eq!(tree.query_selector_all(".a ~ .b ~ .c").len(), 1);
        // adjacent vs general: `.a + .c` (immediate) does NOT match (q is between)
        assert_eq!(tree.query_selector_all(".a + .c").len(), 0);
        // but `.a ~ .c` does (general sibling, any distance)
        assert_eq!(tree.query_selector_all(".a ~ .c").len(), 1);
    }

    #[test]
    fn pseudo_is_selector_list() {
        let tree = Tree::parse(
            "<div class=a>1</div><div class=b>2</div><div class=c>3</div>",
        );
        // :is(.a, .b) → the .a and .b divs (2), not .c
        assert_eq!(tree.query_selector_all(":is(.a, .b)").len(), 2);
        // single-item list is the degenerate case
        assert_eq!(tree.query_selector_all(":is(.c)").len(), 1);
        // :is matching nothing
        assert_eq!(tree.query_selector_all(":is(.nope)").len(), 0);
    }

    #[test]
    fn pseudo_is_with_combinator_and_descendant_arg() {
        // div:is(.x) anchors the :is list at the element; combine with a descendant.
        let tree = Tree::parse(
            "<div class=x><span class=y>hit</span></div>             <div class=z><span class=y>miss</span></div>",
        );
        // div:is(.x) .y → only the .y inside the div.x
        let r = tree.query_selector_all("div:is(.x) .y");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.text_content(r[0]), "hit");
        // :is() argument may itself be a complex selector (descendant)
        let t2 = Tree::parse(
            "<section><p class=t>a</p></section><p class=t>b</p>",
        );
        // :is(section .t, .nope) → only the .t inside the section
        let r2 = t2.query_selector_all(":is(section .t, .nope)");
        assert_eq!(r2.len(), 1);
        assert_eq!(t2.text_content(r2[0]), "a");
    }

    #[test]
    fn pseudo_where_matches_like_is() {
        let tree = Tree::parse("<i class=a>1</i><i class=b>2</i><i>3</i>");
        // :where(.a,.b) matches identically to :is (specificity aside)
        assert_eq!(tree.query_selector_all(":where(.a,.b)").len(), 2);
        assert_eq!(tree.query_selector_all("i:where(.a)").len(), 1);
    }

    #[test]
    fn pseudo_not_selector_list() {
        let tree = Tree::parse(
            "<ul><li class=a>1</li><li class=b>2</li><li class=c>3</li></ul>",
        );
        // :not(.a, .b) → only the .c li survives
        let r = tree.query_selector_all("li:not(.a, .b)");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.text_content(r[0]), "3");
        // :not(div, span) on a tag list — none of these <li> are div or span → all 3
        assert_eq!(tree.query_selector_all("li:not(div, span)").len(), 3);
        // :not(li, .c) → li that is neither (none) → 0
        assert_eq!(tree.query_selector_all("li:not(li, .c)").len(), 0);
    }

    #[test]
    fn pseudo_has_descendant() {
        let tree = Tree::parse(
            "<div id=hit><span class=x>y</span></div><div id=miss><span>z</span></div>",
        );
        // div:has(.x) → only the div containing a .x descendant
        let r = tree.query_selector_all("div:has(.x)");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.get_attribute(r[0], "id"), Some("hit"));
        // negative: nothing has a .absent descendant
        assert_eq!(tree.query_selector_all("div:has(.absent)").len(), 0);
    }

    #[test]
    fn pseudo_has_child_combinator() {
        let tree = Tree::parse(
            "<ul id=a><li class=active>1</li></ul>             <ul id=b><li>2</li></ul>             <ul id=c><div><li class=active>deep</li></div></ul>",
        );
        // ul:has(> li.active) → only the ul with a DIRECT li.active child (a).
        // c has an li.active but it is a grandchild → must NOT match.
        let r = tree.query_selector_all("ul:has(> li.active)");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.get_attribute(r[0], "id"), Some("a"));
        // descendant form (no `>`) DOES reach the grandchild → a and c match
        assert_eq!(tree.query_selector_all("ul:has(li.active)").len(), 2);
    }

    #[test]
    fn pseudo_has_adjacent_sibling() {
        let tree = Tree::parse(
            "<section><h2 id=p>t</h2><p>para</p></section>             <section><h2 id=q>t2</h2><div>not-p</div></section>",
        );
        // h2:has(+ p) → only the h2 immediately followed by a <p> (the first one)
        let r = tree.query_selector_all("h2:has(+ p)");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.get_attribute(r[0], "id"), Some("p"));
    }

    #[test]
    fn pseudo_has_multi_compound_scoped() {
        // :has(.a .b) — a multi-compound relative complex. The match must be
        // CONFINED to the scope element's subtree: `.a` and `.b` both inside it.
        let tree = Tree::parse(
            "<section id=in><div class=a><span class=b>hit</span></div></section>             <section id=out><span class=b>noA</span></section>             <div class=a><section id=split><span class=b>aOutside</span></section></div>",
        );
        let r = tree.query_selector_all("section:has(.a .b)");
        // `in` matches (.a and .b both inside). `out` has no .a. `split` has a .b
        // but its only .a ancestor is OUTSIDE the section → scope cap rejects it.
        assert_eq!(r.len(), 1);
        assert_eq!(tree.get_attribute(r[0], "id"), Some("in"));
    }

    #[test]
    fn pseudo_has_general_sibling_and_list() {
        let tree = Tree::parse(
            "<div><h3 id=a>x</h3><span>s</span><p>after</p></div>             <div><h3 id=b>y</h3><span>only</span></div>",
        );
        // h3:has(~ p) → an h3 with SOME following sibling <p> (a, not b)
        let r = tree.query_selector_all("h3:has(~ p)");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.get_attribute(r[0], "id"), Some("a"));
        // a comma list of relative selectors: matches if ANY relative matches
        assert_eq!(tree.query_selector_all("h3:has(~ p, ~ .none)").len(), 1);
        // empty :has() never matches
        assert_eq!(tree.query_selector_all("div:has()").len(), 0);
    }

    #[test]
    fn pseudo_has_relative_complex_combinators() {
        // The relative complex inside :has() may itself contain combinators —
        // exercises the scoped matcher's Child/Adjacent/GeneralSibling arms.
        let tree = Tree::parse(
            "<section id=s1><div class=a><span class=b>x</span></div></section>             <section id=s2><div class=a></div><span class=b>y</span></section>             <section id=s3><div class=a></div><i>z</i><span class=b>w</span></section>",
        );
        // child combinator in the relative complex → s1 only
        let gt = tree.query_selector_all("section:has(.a > .b)");
        assert_eq!(gt.len(), 1);
        assert_eq!(tree.get_attribute(gt[0], "id"), Some("s1"));
        // adjacent → s2 only
        let plus = tree.query_selector_all("section:has(.a + .b)");
        assert_eq!(plus.len(), 1);
        assert_eq!(tree.get_attribute(plus[0], "id"), Some("s2"));
        // general sibling → s2 and s3
        assert_eq!(tree.query_selector_all("section:has(.a ~ .b)").len(), 2);
    }

    #[test]
    fn pseudo_has_ancestor_walk_iterates_within_scope() {
        // `.b` nested two levels under `.a` with a non-`.a` wrapper between → the
        // scoped descendant ancestor walk must skip the wrapper and keep ascending
        // (but never reach/pass the scope element).
        let tree = Tree::parse(
            "<section id=hit><div class=a><div class=wrap><span class=b>x</span></div></div></section>             <section id=miss><div class=wrap><span class=b>y</span></div></section>",
        );
        let r = tree.query_selector_all("section:has(.a .b)");
        assert_eq!(r.len(), 1);
        assert_eq!(tree.get_attribute(r[0], "id"), Some("hit"));
    }
}

//! The `selector-dual` gate's cascade mode (turbo-dom consolidation S3b; turbo-dom
//! `docs/selector-dual-gate.md`). Compiled only with the non-default `selector-dual`
//! feature.
//!
//! The LEGACY engine is the old html2pdf selector parser (`selector.rs`, unchanged since
//! S1) and its matcher (below, as it was in `cascade.rs` at S2) over match contexts
//! rebuilt from the scope's [`Navigator`] exactly as S2 built them while styling. At every
//! cascade decision both engines answer and turbo-dom's dual log records each
//! disagreement:
//!
//! - `cascade_validity`: the rule's selector list is valid in one engine only;
//! - `cascade_list`: both valid, different item counts;
//! - `cascade_specificity`: an item's specificity differs;
//! - `cascade_match`: an item matches an element in one engine only (`cascade_rule_match`
//!   compares the whole rule when the lists do not line up).
//!
//! `TURBO_DOM_SELECTOR_ENGINE=legacy|new` picks the answer the cascade uses. Records are
//! deduplicated per process (same API, selector and element shape). Temporary: deleted,
//! with `selector.rs`, in the last S3b commit.

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Write as _;

use turbo_dom::rtdom::query::{match_for_cascade, SelectorList, Specificity};
use turbo_dom::rtdom::selector_dual::cascade as hooks;
use turbo_dom::rtdom::tree::NodeType;
use turbo_dom::Handle;

use super::super::parser::RuleSelectors;
use super::super::selector::{
    parse_selector_list, AttrOp, AttrSel, Combinator, Compound, Pseudo, Selector,
};
use super::{Attrs, Navigator};

thread_local! {
    static SEEN: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// The navigator's cache of the last element's legacy match path (the cascade matches
/// every rule against one element before moving on).
pub(crate) type PathCache<'t> = RefCell<Option<(Handle, Vec<Ctx<'t>>)>>;

fn report(fields: String) {
    let fresh = SEEN.with(|seen| seen.borrow_mut().insert(fields.clone()));
    hooks::compared(fresh.then_some(fields.as_str()));
}

fn agree() {
    hooks::compared(None);
}

fn spec_json(s: Specificity) -> String {
    format!("[{},{},{}]", s.0, s.1, s.2)
}

/// Parse `prelude` with the legacy parser and compare it with the new parse.
pub(crate) fn parse(prelude: &str, new: Option<&SelectorList>) -> Vec<Selector> {
    let legacy = parse_selector_list(prelude);
    let q = hooks::json_str;
    match new {
        Some(list) if !legacy.is_empty() => {
            let items = list.selectors();
            if items.len() == legacy.len() {
                agree();
                for (old, new) in legacy.iter().zip(items) {
                    if old.specificity == new.specificity() {
                        agree();
                    } else {
                        report(format!(
                            "\"api\":\"cascade_specificity\",\"prelude\":{},\"selector\":{},\"legacy\":{},\"new\":{},\"pseudo_element\":{}",
                            q(prelude),
                            q(new.text()),
                            spec_json(old.specificity),
                            spec_json(new.specificity()),
                            new.pseudo_element().map_or_else(|| "null".into(), q),
                        ));
                    }
                }
            } else {
                report(format!(
                    "\"api\":\"cascade_list\",\"prelude\":{},\"legacy\":{},\"new\":{}",
                    q(prelude),
                    legacy.len(),
                    items.len()
                ));
            }
        }
        None if legacy.is_empty() => agree(),
        _ => report(format!(
            "\"api\":\"cascade_validity\",\"prelude\":{},\"legacy\":{},\"new\":{},\"error\":{}",
            q(prelude),
            !legacy.is_empty(),
            new.is_some(),
            q(&turbo_dom::rtdom::query::parse_for_cascade(prelude)
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default()),
        )),
    }
    legacy
}

/// Compare the rule's legacy and new matches on `h` and return the configured answer.
pub(crate) fn best_match(
    rule: &RuleSelectors,
    nav: &Navigator,
    h: Handle,
    new_best: Option<Specificity>,
) -> Option<Specificity> {
    let mut cache = nav.legacy_path.borrow_mut();
    if !matches!(&*cache, Some((e, _)) if *e == h) {
        *cache = Some((h, legacy_path(nav, h)));
    }
    let path = &cache.as_ref().expect("just built").1;
    let legacy_best = rule
        .legacy
        .iter()
        .filter(|s| matches(s, path))
        .map(|s| s.specificity)
        .max();
    if rule.selectors().len() == rule.legacy.len() {
        compare_items(rule, nav, h, path);
    } else {
        compare_rule(rule, nav, h, legacy_best.is_some(), new_best.is_some());
    }
    if hooks::legacy_mode() {
        legacy_best
    } else {
        new_best
    }
}

/// Per (selector, element): the lists line up item by item.
fn compare_items(rule: &RuleSelectors, nav: &Navigator, h: Handle, path: &[Ctx]) {
    for (old, new) in rule.legacy.iter().zip(rule.selectors()) {
        let (old_m, new_m) = (matches(old, path), match_for_cascade(nav, h, new));
        if old_m == new_m {
            agree();
            continue;
        }
        report(format!(
            "\"api\":\"cascade_match\",\"prelude\":{},\"selector\":{},\"legacy\":{old_m},\"new\":{new_m},\"element\":{}",
            hooks::json_str(rule.prelude()),
            hooks::json_str(new.text()),
            shape(nav, h),
        ));
    }
}

/// The whole rule, when the lists do not line up (one engine rejected the prelude).
fn compare_rule(rule: &RuleSelectors, nav: &Navigator, h: Handle, legacy: bool, new: bool) {
    if legacy == new {
        agree();
        return;
    }
    report(format!(
        "\"api\":\"cascade_rule_match\",\"prelude\":{},\"legacy\":{legacy},\"new\":{new},\"new_valid\":{},\"element\":{}",
        hooks::json_str(rule.prelude()),
        rule.is_valid(),
        shape(nav, h),
    ));
}

/// An element's shape for a divergence record: tree facts plus the scope facts the
/// legacy contexts were built from.
fn shape(nav: &Navigator, h: Handle) -> String {
    let tree = nav.view().tree();
    let q = hooks::json_str;
    let mut out = format!(
        "{{\"tag\":{},\"ns\":{}",
        tree.local_name(h).map_or_else(|| "null".into(), q),
        tree.namespace_uri(h).map_or_else(|| "null".into(), q)
    );
    out.push_str(",\"attrs\":[");
    for (i, (name, value)) in tree.attributes(h).iter().enumerate() {
        let _ = write!(
            out,
            "{}[{},{}]",
            if i > 0 { "," } else { "" },
            q(name),
            q(value)
        );
    }
    out.push_str("],\"local_attrs\":[");
    for (i, (name, value)) in tree.local_attributes(h).enumerate() {
        let _ = write!(
            out,
            "{}[{},{}]",
            if i > 0 { "," } else { "" },
            q(name),
            q(value)
        );
    }
    let position = nav.position(h).expect("an element");
    let mut ancestors = Vec::new();
    let mut node = nav.parent(h);
    while let Some(a) = node {
        ancestors.push(tree.local_name(a).map_or_else(|| "null".into(), q));
        node = nav.parent(a);
    }
    let kept_children: Vec<&str> = nav
        .children(h)
        .iter()
        .map(|&c| {
            if tree.node_type(c) == NodeType::Element {
                "element"
            } else {
                "text"
            }
        })
        .collect();
    let _ = write!(
        out,
        "],\"facts\":{{\"shell\":{},\"marker\":{},\"scope_root\":{},\"position\":[{},{},{},{}],\"kept_children\":[{}],\"ancestors\":[{}]}}}}",
        nav.is_shell(h),
        nav.view().marker(h).is_some(),
        nav.roots().contains(&h),
        position.index,
        position.siblings,
        position.of_type_index,
        position.of_type_total,
        kept_children.iter().map(|k| format!("\"{k}\"")).collect::<Vec<_>>().join(","),
        ancestors.join(","),
    );
    out
}

/// The legacy match path of `h`: the contexts of its scope ancestors (outermost first)
/// and its own, exactly as S2's styling walk built them — each element context carries its
/// preceding element siblings (with no siblings of their own), a shell sits at the root
/// position, empty and alone.
fn legacy_path<'t>(nav: &Navigator<'_, 't>, h: Handle) -> Vec<Ctx<'t>> {
    let mut chain = vec![h];
    while let Some(parent) = nav.parent(*chain.last().expect("non-empty")) {
        chain.push(parent);
    }
    chain
        .iter()
        .rev()
        .map(|&e| legacy_ctx(nav, e, true))
        .collect()
}

fn legacy_ctx<'t>(nav: &Navigator<'_, 't>, h: Handle, with_prev: bool) -> Ctx<'t> {
    let view = nav.view();
    let tree = view.tree();
    if nav.is_shell(h) {
        let pos = ElemPos {
            index: 0,
            of_type_index: 0,
            of_type_total: 1,
            siblings: 1,
        };
        return ctx_from(
            tree.local_name(h),
            Attrs::Tree(tree, h),
            pos,
            true,
            Vec::new(),
        );
    }
    let p = nav.position(h).expect("an element");
    let pos = ElemPos {
        index: p.index,
        of_type_index: p.of_type_index,
        of_type_total: p.of_type_total,
        siblings: p.siblings,
    };
    let prev = if with_prev {
        let siblings = turbo_dom::rtdom::query::Navigator::element_siblings(nav, h);
        siblings
            .iter()
            .take_while(|&&s| s != h)
            .map(|&s| legacy_ctx(nav, s, false))
            .collect()
    } else {
        Vec::new()
    };
    ctx_from(
        view.match_name(h),
        Attrs::Tree(tree, h),
        pos,
        nav.is_empty(h),
        prev,
    )
}

// ---------------------------------------------------------------- the legacy matcher (S2 cascade.rs)

#[derive(Debug, Clone, Copy)]
pub(crate) struct ElemPos {
    index: usize,
    of_type_index: usize,
    of_type_total: usize,
    siblings: usize,
}

#[derive(Clone)]
pub(crate) struct Ctx<'a> {
    tag: Option<&'a str>,
    id: Option<&'a str>,
    classes: Vec<&'a str>,
    attrs: Attrs<'a>,
    pos: ElemPos,
    /// Whether the element has no element/text children (`:empty`).
    empty: bool,
    /// Preceding element siblings in document order (for the `+`/`~` combinators).
    /// Each carries an empty `prev`; the matcher re-slices this list when it steps
    /// left across a sibling combinator.
    prev: Vec<Ctx<'a>>,
}

/// A match context over an element's name and attributes (shared by both inputs).
fn ctx_from<'a>(
    tag: Option<&'a str>,
    attrs: Attrs<'a>,
    pos: ElemPos,
    empty: bool,
    prev: Vec<Ctx<'a>>,
) -> Ctx<'a> {
    Ctx {
        tag,
        id: attrs.get("id"),
        classes: attrs
            .get("class")
            .map(str::split_whitespace)
            .map(Iterator::collect)
            .unwrap_or_default(),
        attrs,
        pos,
        empty,
        prev,
    }
}

fn dash_match(actual: &str, expected: &str) -> bool {
    actual == expected || actual.starts_with(&format!("{expected}-"))
}

fn op_matches(op: &AttrOp, actual: &str, expected: &str) -> bool {
    match op {
        AttrOp::Exists => true,
        AttrOp::Equals => actual == expected,
        AttrOp::Includes => actual.split_whitespace().any(|w| w == expected),
        AttrOp::DashMatch => dash_match(actual, expected),
        AttrOp::Prefix => actual.starts_with(expected),
        AttrOp::Suffix => actual.ends_with(expected),
        AttrOp::Substring => actual.contains(expected),
    }
}

fn attr_matches(sel: &AttrSel, attrs: Attrs) -> bool {
    match attrs.get(&sel.name) {
        Some(actual) => op_matches(&sel.op, actual, &sel.value),
        None => false,
    }
}

fn nth_match(a: i32, b: i32, index: usize) -> bool {
    let idx = index as i32;
    if a == 0 {
        return idx == b;
    }
    let n = idx - b;
    n % a == 0 && n / a >= 0
}

fn pseudo_matches(pseudo: &Pseudo, ctx: &Ctx) -> bool {
    let pos = ctx.pos;
    match pseudo {
        Pseudo::FirstChild => pos.index == 1,
        Pseudo::LastChild => pos.index == pos.siblings,
        Pseudo::OnlyChild => pos.siblings == 1,
        Pseudo::FirstOfType => pos.of_type_index == 1,
        Pseudo::LastOfType => pos.of_type_index == pos.of_type_total,
        Pseudo::OnlyOfType => pos.of_type_total == 1,
        Pseudo::NthChild(a, b) => nth_match(*a, *b, pos.index),
        Pseudo::NthOfType(a, b) => nth_match(*a, *b, pos.of_type_index),
        Pseudo::Root => ctx.tag == Some("html"),
        Pseudo::Empty => ctx.empty,
        Pseudo::Checked => has_attr(ctx.attrs, "checked"),
        Pseudo::Disabled => has_attr(ctx.attrs, "disabled"),
        Pseudo::Enabled => is_form_control(ctx.tag) && !has_attr(ctx.attrs, "disabled"),
        Pseudo::Link => {
            matches!(ctx.tag, Some("a" | "area" | "link")) && has_attr(ctx.attrs, "href")
        }
        Pseudo::Not(compounds) => !compounds.iter().any(|c| compound_matches(c, ctx)),
        Pseudo::Is(compounds) => compounds.iter().any(|c| compound_matches(c, ctx)),
        Pseudo::NeverMatch => false,
    }
}

fn has_attr(attrs: Attrs, name: &str) -> bool {
    attrs.get_ignore_case(name).is_some()
}

fn is_form_control(tag: Option<&str>) -> bool {
    matches!(
        tag,
        Some("input" | "button" | "select" | "textarea" | "option")
    )
}

fn tag_ok(compound: &Compound, ctx: &Ctx) -> bool {
    match &compound.tag {
        Some(t) => ctx.tag == Some(t.as_str()),
        None => true,
    }
}

fn id_ok(compound: &Compound, ctx: &Ctx) -> bool {
    match &compound.id {
        Some(i) => ctx.id == Some(i.as_str()),
        None => true,
    }
}

fn classes_ok(compound: &Compound, ctx: &Ctx) -> bool {
    compound
        .classes
        .iter()
        .all(|c| ctx.classes.contains(&c.as_str()))
}

fn compound_matches(compound: &Compound, ctx: &Ctx) -> bool {
    tag_ok(compound, ctx)
        && id_ok(compound, ctx)
        && classes_ok(compound, ctx)
        && compound.attrs.iter().all(|a| attr_matches(a, ctx.attrs))
        && compound.pseudos.iter().all(|p| pseudo_matches(p, ctx))
}

fn match_child(sel: &Selector, ci: usize, path: &[Ctx], pi: usize) -> bool {
    pi > 0
        && compound_matches(&sel.compounds[ci - 1], &path[pi - 1])
        && match_ancestors(sel, ci - 1, path, pi - 1)
}

fn match_descendant(sel: &Selector, ci: usize, path: &[Ctx], pi: usize) -> bool {
    (0..pi).rev().any(|parent| {
        compound_matches(&sel.compounds[ci - 1], &path[parent])
            && match_ancestors(sel, ci - 1, path, parent)
    })
}

/// Match compound `ci-1` against a preceding sibling of the element at `path[pi]`,
/// then continue the selector from that sibling. `immediate` restricts to the
/// directly-preceding sibling (`+`); otherwise any earlier sibling (`~`).
fn match_sibling(sel: &Selector, ci: usize, path: &[Ctx], pi: usize, immediate: bool) -> bool {
    let sibs = &path[pi].prev;
    let range = if immediate {
        sibs.len().saturating_sub(1)..sibs.len()
    } else {
        0..sibs.len()
    };
    range.rev().any(|k| {
        if !compound_matches(&sel.compounds[ci - 1], &sibs[k]) {
            return false;
        }
        // Continue the match from the sibling at the same depth: reuse the shared
        // ancestors, and give the sibling its own preceding siblings so chained
        // sibling combinators (`a ~ b + c`) keep working.
        let mut sp = path[..pi].to_vec();
        let mut sib = sibs[k].clone();
        sib.prev = sibs[..k].to_vec();
        sp.push(sib);
        match_ancestors(sel, ci - 1, &sp, pi)
    })
}

fn match_ancestors(sel: &Selector, ci: usize, path: &[Ctx], pi: usize) -> bool {
    if ci == 0 {
        return true;
    }
    match sel.combinators[ci - 1] {
        Combinator::Child => match_child(sel, ci, path, pi),
        Combinator::Descendant => match_descendant(sel, ci, path, pi),
        Combinator::NextSibling => match_sibling(sel, ci, path, pi, true),
        Combinator::SubsequentSibling => match_sibling(sel, ci, path, pi, false),
    }
}

fn matches(sel: &Selector, path: &[Ctx]) -> bool {
    let last = path.len() - 1;
    let subject = &sel.compounds[sel.compounds.len() - 1];
    compound_matches(subject, &path[last])
        && match_ancestors(sel, sel.compounds.len() - 1, path, last)
}

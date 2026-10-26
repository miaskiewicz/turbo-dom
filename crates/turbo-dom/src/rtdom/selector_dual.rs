//! The `selector-dual` gate (turbo-dom consolidation S3a; `docs/selector-dual-gate.md`).
//!
//! Built only with the non-default `selector-dual` cargo feature. Every selector-engine
//! output of [`Tree`] (`matches`, `matches_checked`, `query_selector[_all][_checked]`,
//! `get_element_by_id`, `get_elements_by_tag_name` and the UTF-16 `*_units` /
//! `*_from_units` / `get_element_by_id_units`) is computed by BOTH the current engine
//! (`query.rs` + `query_utf16.rs`) and the FROZEN B-main engine
//! (`query_frozen.rs` + `query_frozen_utf16.rs`, regenerated and checked by
//! `scripts/verify-query-frozen.sh`). Each engine keeps its own caches (the frozen ones
//! live in [`FrozenCaches`]), so a cached answer is compared again on every call.
//!
//! - `TURBO_DOM_SELECTOR_ENGINE=legacy|new` picks which answer callers get. A dual build
//!   aborts on the first selector call when it is unset or invalid (no silent default in
//!   an env-cleared child process).
//! - `TURBO_DOM_SELECTOR_DUAL_LOG_DIR=<dir>`: each process appends JSON lines to its own
//!   `<dir>/<pid>-<nonce>.jsonl` (also required): one `header` record (mode, frozen-engine
//!   SHA, build id from `TURBO_DOM_DUAL_BUILD_ID` at compile time, pid, argv0), `count`
//!   records at 1, 2, 4, 8, … compared calls (the vacuity guard reads the maximum), and one
//!   `divergence` record per disagreeing call: API, selector, both answers and the shape of
//!   every element the answers disagree on (tag, namespace, attributes, child node kinds,
//!   live form state) plus element-side `facts` over the nodes a selector can examine from
//!   it, which `scripts/selector-dual-classify.py` tests against the bug list.
//!
//! Temporary: deleted (with the frozen engine) in the last S3b commit.
use super::query::SelectorError;
use super::query_frozen;
use super::tree::{FormProp, Handle, NodeType, QueryCache, Tree};
use rustc_hash::FxHashMap;
use std::cell::{Cell, RefCell};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// The B-main commit the frozen engine was copied from (`scripts/verify-query-frozen.sh`).
pub const FROZEN_SHA: &str = "4d5a63df36f3f94fcfb22b184039b25afd485640";

/// The frozen engine's own caches (B-main kept them on `Tree` as `qcache` /
/// `parse_cache`). They hang off the cascade cache slot so `tree.rs` stays untouched in
/// S3a: that slot is reset at the same point `qcache` is (storage collection), and
/// `qcache` is version-keyed exactly like the original.
#[derive(Default)]
pub(crate) struct FrozenCaches {
    pub(crate) qcache: RefCell<QueryCache>,
    pub(crate) parse_cache: RefCell<FxHashMap<String, Rc<[query_frozen::Complex]>>>,
}

impl Tree {
    pub(crate) fn frozen_caches(&self) -> Rc<FrozenCaches> {
        self.css_cache.borrow().dual.clone()
    }
}

/// The frozen engine's error, as the current type (same variants).
#[allow(clippy::needless_pass_by_value)] // used as `map_err(thaw)`
pub(crate) fn thaw(error: query_frozen::SelectorError) -> SelectorError {
    match error {
        query_frozen::SelectorError::UnterminatedAttribute => SelectorError::UnterminatedAttribute,
        query_frozen::SelectorError::UnexpectedChar(c) => SelectorError::UnexpectedChar(c),
        query_frozen::SelectorError::EmptyCompound => SelectorError::EmptyCompound,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Legacy,
    New,
}

struct Log {
    mode: Mode,
    path: PathBuf,
    file: Mutex<std::fs::File>,
    compared: AtomicU64,
}

static LOG: OnceLock<Log> = OnceLock::new();

thread_local! {
    /// Set while a dual call runs both engines: nested public calls made by the current
    /// engine itself (e.g. `query_selector` -> `query_selector_all`) answer directly.
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

fn fatal(message: &str) -> ! {
    eprintln!("turbo-dom (selector-dual build): {message}");
    std::process::abort()
}

fn log() -> &'static Log {
    LOG.get_or_init(|| {
        let mode = match std::env::var("TURBO_DOM_SELECTOR_ENGINE").as_deref() {
            Ok("legacy") => Mode::Legacy,
            Ok("new") => Mode::New,
            other => fatal(&format!(
                "TURBO_DOM_SELECTOR_ENGINE must be `legacy` or `new` (got {other:?})"
            )),
        };
        let Some(dir) = std::env::var_os("TURBO_DOM_SELECTOR_DUAL_LOG_DIR") else {
            fatal("TURBO_DOM_SELECTOR_DUAL_LOG_DIR is not set")
        };
        let dir = PathBuf::from(dir);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            fatal(&format!("cannot create {}: {e}", dir.display()));
        }
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let local = 0u8;
        let nonce = (nanos as u64) ^ ((&raw const local) as u64).rotate_left(32);
        let path = dir.join(format!("{pid}-{nonce:016x}.jsonl"));
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)
            .unwrap_or_else(|e| fatal(&format!("cannot create {}: {e}", path.display())));
        let header = format!(
            "{{\"type\":\"header\",\"mode\":\"{}\",\"frozen_sha\":\"{FROZEN_SHA}\",\"build_id\":{},\"pid\":{pid},\"artifact\":{},\"turbo_dom\":\"{}\"}}\n",
            if mode == Mode::Legacy { "legacy" } else { "new" },
            json_str(option_env!("TURBO_DOM_DUAL_BUILD_ID").unwrap_or("unset")),
            json_str(&std::env::args().next().unwrap_or_default()),
            env!("CARGO_PKG_VERSION"),
        );
        if let Err(e) = file.write_all(header.as_bytes()) {
            fatal(&format!("cannot write {}: {e}", path.display()));
        }
        Log {
            mode,
            path,
            file: Mutex::new(file),
            compared: AtomicU64::new(0),
        }
    })
}

impl Log {
    fn write(&self, line: &str) {
        let mut file = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Err(e) = file.write_all(line.as_bytes()) {
            fatal(&format!("cannot write {}: {e}", self.path.display()));
        }
    }
}

/// Which selector an output was asked for.
#[derive(Clone, Copy)]
pub(crate) enum Sel<'a> {
    Str(&'a str),
    Units(&'a [u16]),
}

impl Sel<'_> {
    fn text(&self) -> String {
        match self {
            Sel::Str(s) => (*s).to_string(),
            Sel::Units(u) => String::from_utf16_lossy(u),
        }
    }
    fn json(&self) -> String {
        match self {
            Sel::Str(s) => format!("\"selector\":{}", json_str(s)),
            Sel::Units(u) => match String::from_utf16(u) {
                Ok(s) => format!("\"selector\":{},\"units\":true", json_str(&s)),
                Err(_) => format!(
                    "\"selector\":{},\"units\":{:?}",
                    json_str(&String::from_utf16_lossy(u)),
                    u
                ),
            },
        }
    }
}

/// An engine output the gate can compare and log.
pub(crate) trait Answer {
    fn same(&self, other: &Self) -> bool;
    fn json(&self) -> String;
    fn handles(&self) -> Vec<Handle>;
}

impl Answer for bool {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
    fn json(&self) -> String {
        self.to_string()
    }
    fn handles(&self) -> Vec<Handle> {
        Vec::new()
    }
}

impl Answer for Option<Handle> {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
    fn json(&self) -> String {
        self.map_or_else(|| "null".to_string(), |h| h.raw().to_string())
    }
    fn handles(&self) -> Vec<Handle> {
        self.iter().copied().collect()
    }
}

fn handles_json(handles: &[Handle]) -> String {
    let mut out = String::from("[");
    for (i, h) in handles.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(out, "{}", h.raw());
    }
    out.push(']');
    out
}

impl Answer for Rc<[Handle]> {
    fn same(&self, other: &Self) -> bool {
        self[..] == other[..]
    }
    fn json(&self) -> String {
        handles_json(self)
    }
    fn handles(&self) -> Vec<Handle> {
        self.to_vec()
    }
}

impl Answer for Vec<Handle> {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
    fn json(&self) -> String {
        handles_json(self)
    }
    fn handles(&self) -> Vec<Handle> {
        self.clone()
    }
}

impl<T: Answer> Answer for Result<T, SelectorError> {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Ok(a), Ok(b)) => a.same(b),
            (Err(a), Err(b)) => a == b,
            _ => false,
        }
    }
    fn json(&self) -> String {
        match self {
            Ok(v) => format!("{{\"ok\":{}}}", v.json()),
            Err(e) => format!("{{\"err\":{}}}", json_str(&e.to_string())),
        }
    }
    fn handles(&self) -> Vec<Handle> {
        self.as_ref().map(Answer::handles).unwrap_or_default()
    }
}

struct ActiveGuard;
impl ActiveGuard {
    fn enter() -> Self {
        ACTIVE.with(|a| a.set(true));
        ActiveGuard
    }
}
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        ACTIVE.with(|a| a.set(false));
    }
}

/// Run one engine output through both engines and return the configured one, or `None`
/// when already inside a dual call (the caller then answers with the current engine).
pub(crate) fn dual<T: Answer>(
    tree: &Tree,
    api: &'static str,
    selector: Sel<'_>,
    element: Option<Handle>,
    new: impl FnOnce() -> T,
    legacy: impl FnOnce() -> T,
) -> Option<T> {
    if ACTIVE.with(Cell::get) {
        return None;
    }
    let log = log();
    let guard = ActiveGuard::enter();
    let new = new();
    let legacy = legacy();
    let compared = log.compared.fetch_add(1, Ordering::Relaxed) + 1;
    if compared.is_power_of_two() {
        log.write(&format!(
            "{{\"type\":\"count\",\"pid\":{},\"compared\":{compared}}}\n",
            std::process::id()
        ));
    }
    if !new.same(&legacy) {
        let record = divergence(tree, api, selector, element, &legacy, &new);
        log.write(&record);
    }
    drop(guard);
    Some(if log.mode == Mode::Legacy {
        legacy
    } else {
        new
    })
}

/// Elements recorded per divergence (the answers' symmetric difference is capped).
const MAX_ELEMENTS: usize = 16;
/// Nodes examined for element-side facts / recorded tokens (caps keep huge pages bounded;
/// a capped record is marked `truncated` and the classifier may then reject it).
const MAX_REACH: usize = 4096;
const MAX_TOKENS: usize = 1024;

fn divergence<T: Answer>(
    tree: &Tree,
    api: &str,
    selector: Sel<'_>,
    element: Option<Handle>,
    legacy: &T,
    new: &T,
) -> String {
    let mut elements: Vec<Handle> = element.into_iter().collect();
    let (old_handles, new_handles) = (legacy.handles(), new.handles());
    for h in old_handles.iter().chain(&new_handles) {
        if old_handles.contains(h) != new_handles.contains(h) && !elements.contains(h) {
            elements.push(*h);
        }
    }
    let truncated_elements = elements.len() > MAX_ELEMENTS;
    elements.truncate(MAX_ELEMENTS);
    let relational = selector.text().contains(":has(");
    let shapes: Vec<String> = elements
        .iter()
        .map(|&h| shape(tree, h, relational))
        .collect();
    format!(
        "{{\"type\":\"divergence\",\"pid\":{},\"api\":\"{api}\",{},\"legacy\":{},\"new\":{},\"elements\":[{}],\"elements_truncated\":{truncated_elements}}}\n",
        std::process::id(),
        selector.json(),
        legacy.json(),
        new.json(),
        shapes.join(","),
    )
}

/// Every node a selector evaluated at `h` can examine: `h`, its descendants, its
/// ancestors and the siblings of `h` and of each ancestor. For a `:has()` selector, the
/// whole tree component of `h` (relative selectors reach siblings' subtrees).
fn reach(tree: &Tree, h: Handle, relational: bool) -> (Vec<Handle>, bool) {
    let mut out = Vec::new();
    let push_subtree = |root: Handle, out: &mut Vec<Handle>| {
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            if out.len() >= MAX_REACH {
                return;
            }
            out.push(n);
            tree.for_each_child(n, |c| stack.push(c));
        }
    };
    if relational {
        let mut top = h;
        while let Some(p) = tree.parent(top) {
            top = p;
        }
        push_subtree(top, &mut out);
    } else {
        push_subtree(h, &mut out);
        let mut node = h;
        while let Some(parent) = tree.parent(node) {
            tree.for_each_child(parent, |c| {
                if c != node && out.len() < MAX_REACH {
                    out.push(c);
                }
            });
            if out.len() < MAX_REACH {
                out.push(parent);
            }
            node = parent;
        }
    }
    let truncated = out.len() >= MAX_REACH;
    (out, truncated)
}

fn child_kinds(tree: &Tree, h: Handle) -> Vec<&'static str> {
    let mut kinds = Vec::new();
    tree.for_each_child(h, |c| {
        kinds.push(match tree.node_type(c) {
            NodeType::Element => "element",
            NodeType::Text if tree.node_value(c).is_none_or(|t| t.is_empty()) => "empty-text",
            NodeType::Text => "text",
            NodeType::Comment => "comment",
            NodeType::ProcessingInstruction => "pi",
            NodeType::Fragment => "fragment",
            NodeType::Doctype => "doctype",
            NodeType::Attribute => "attribute",
            NodeType::Document => "document",
        });
    });
    kinds
}

const HTML_NS: &str = "http://www.w3.org/1999/xhtml";
const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

fn shape(tree: &Tree, h: Handle, relational: bool) -> String {
    let mut out = format!("{{\"handle\":{}", h.raw());
    let ns = tree.namespace_uri(h);
    let _ = write!(
        out,
        ",\"node_type\":{},\"tag\":{},\"ns\":{}",
        tree.node_type(h) as u8,
        tree.local_name(h).map_or_else(|| "null".into(), json_str),
        ns.map_or_else(|| "null".into(), json_str)
    );
    out.push_str(",\"attrs\":[");
    for (i, (name, value)) in tree.attributes(h).iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(out, "[{},{}]", json_str(name), json_str(value));
    }
    out.push_str("],\"children\":[");
    out.push_str(
        &child_kinds(tree, h)
            .iter()
            .map(|k| format!("\"{k}\""))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("],\"form\":{");
    let props = [
        (FormProp::Checked, "checked"),
        (FormProp::Selected, "selected"),
        (FormProp::Disabled, "disabled"),
        (FormProp::Required, "required"),
        (FormProp::ReadOnly, "readonly"),
    ];
    let mut first = true;
    for (prop, name) in props {
        if let Some(v) = tree.form_property(h, prop) {
            if !first {
                out.push(',');
            }
            first = false;
            let _ = write!(out, "\"{name}\":{v}");
        }
    }
    out.push('}');
    // element-side facts over the reachable set
    let (nodes, truncated) = reach(tree, h, relational);
    let mut empty_ignorable = false;
    let mut link = false;
    let mut rw_differs = false;
    let mut tokens: Vec<String> = Vec::new();
    let add = |t: &str, tokens: &mut Vec<String>| {
        if t.len() <= 128 && tokens.len() < MAX_TOKENS && !tokens.iter().any(|x| x == t) {
            tokens.push(t.to_string());
        }
    };
    for &n in &nodes {
        if tree.node_type(n) != NodeType::Element {
            continue;
        }
        let kinds = child_kinds(tree, n);
        if !kinds.iter().any(|k| matches!(*k, "element" | "text"))
            && kinds
                .iter()
                .any(|k| matches!(*k, "comment" | "pi" | "fragment" | "empty-text"))
        {
            empty_ignorable = true;
        }
        let ns = tree.namespace_uri(n);
        let name = tree.local_name(n);
        if (ns == Some(HTML_NS)
            && matches!(name, Some("a" | "area"))
            && tree.has_attribute(n, "href"))
            || (ns == Some(SVG_NS)
                && name == Some("a")
                && (tree.get_attribute_ns_borrowed(n, None, "href").is_some()
                    || tree
                        .get_attribute_ns_borrowed(n, Some(XLINK_NS), "href")
                        .is_some()))
        {
            link = true;
        }
        // ACTIVE is set: `matches` answers with the current engine directly.
        if !rw_differs {
            for pseudo in [":read-write", ":read-only"] {
                if tree.frozen_matches(n, pseudo) != tree.matches(n, pseudo) {
                    rw_differs = true;
                }
            }
        }
        if let Some(name) = name {
            add(name, &mut tokens);
        }
        for (attr, value) in tree.attributes(n) {
            add(&attr, &mut tokens);
            add(&value, &mut tokens);
            for word in value.split_ascii_whitespace() {
                add(word, &mut tokens);
            }
        }
    }
    let _ = write!(
        out,
        ",\"facts\":{{\"has_parent\":{},\"empty_ignorable\":{empty_ignorable},\"link\":{link},\"rw_differs\":{rw_differs},\"reach\":{},\"truncated\":{truncated},\"tokens\":[{}]}}}}",
        tree.parent(h).is_some(),
        nodes.len(),
        tokens.iter().map(|t| json_str(t)).collect::<Vec<_>>().join(","),
    );
    out
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::super::tree::Tree;

    #[test]
    fn a_repeated_qsa_is_compared_again_from_each_engines_own_cache() {
        // Since S3b the frozen engine is the S3a tip, which the live Tree must still agree
        // with exactly (S3b changes only the cascade), so no selector diverges here. The
        // repeat must still be answered by BOTH engines from their OWN version-keyed caches.
        let tree = Tree::parse("<p class=dual-cache-probe>x</p>");
        let selector = "p.dual-cache-probe";
        let compared = || super::log().compared.load(super::Ordering::Relaxed);
        let before = compared();
        let first = tree.query_selector_all(selector);
        let second = tree.query_selector_all(selector);
        assert!(std::rc::Rc::ptr_eq(&first, &second));
        assert!(compared() >= before + 2, "both calls were compared");
        let frozen = tree.frozen_caches();
        let cache = frozen.qcache.borrow();
        let frozen_hit = cache
            .map
            .get(selector)
            .expect("the frozen engine cached it");
        assert_eq!(&frozen_hit[..], &first[..]);
        assert!(!std::rc::Rc::ptr_eq(frozen_hit, &first), "no cache sharing");
    }
}

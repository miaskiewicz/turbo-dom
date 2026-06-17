//! Phase-1 boundary spike (feature = "wasm-runtime"). NOT the final runtime.
//!
//! Goal: measure the JS<->WASM boundary cost of a Rust-core DOM accessed from JS
//! on the chattiest hot paths (querySelectorAll + getAttribute + parent-walk +
//! listener-less dispatch walk), vs the current pure-JS runtime. Read-only tree
//! over `core::Soa` (handle = node index) — no mutation/COW needed for the gate.
//!
//! Two access shapes are exported so the bench can A/B them:
//!   * MITIGATED (Option A) — qsa returns a packed Uint32Array in ONE crossing;
//!     structure reads return ints (handles/ids); strings resolved JS-side from
//!     bulk blobs pulled once. Boundary crossings ~ O(results), not O(nodes*fields).
//!   * NAIVE — per-node string returns (tag_name/get_attr as String) to show the
//!     cost the mitigations avoid.

use crate::core::{self, Soa};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

const ELEMENT_NODE: u8 = 1;

struct Doc {
    soa: Soa,
}

thread_local! {
    static DOCS: RefCell<Vec<Doc>> = RefCell::new(Vec::new());
}

fn with_doc<R>(doc: u32, f: impl FnOnce(&Doc) -> R) -> R {
    DOCS.with(|d| f(&d.borrow()[doc as usize]))
}

/// Parse + register a document. Returns the doc handle. ONE crossing per parse.
#[wasm_bindgen]
pub fn create(html: &str) -> u32 {
    let soa = core::parse_html_soa(html);
    DOCS.with(|d| {
        let mut v = d.borrow_mut();
        v.push(Doc { soa });
        (v.len() - 1) as u32
    })
}

/// Drop a doc (free reset path).
#[wasm_bindgen]
pub fn destroy(doc: u32) {
    DOCS.with(|d| {
        let mut v = d.borrow_mut();
        if (doc as usize) < v.len() {
            v[doc as usize] = Doc { soa: core::parse_html_soa("") };
        }
    });
}

// --- bulk string blobs: pulled ONCE per doc, split JS-side into a string table ---
// `\u{1f}` (unit separator) join — safe vs HTML text which never contains it.
fn join_blob(v: &[String]) -> String {
    v.join("\u{1f}")
}

#[wasm_bindgen]
pub fn tag_names_blob(doc: u32) -> String {
    with_doc(doc, |d| join_blob(&d.soa.tag_names))
}
#[wasm_bindgen]
pub fn attr_names_blob(doc: u32) -> String {
    with_doc(doc, |d| join_blob(&d.soa.attr_names))
}
#[wasm_bindgen]
pub fn attr_values_blob(doc: u32) -> String {
    with_doc(doc, |d| join_blob(&d.soa.attr_values))
}
#[wasm_bindgen]
pub fn strings_blob(doc: u32) -> String {
    with_doc(doc, |d| join_blob(&d.soa.strings))
}

#[wasm_bindgen]
pub fn node_count(doc: u32) -> u32 {
    with_doc(doc, |d| d.soa.node_type.len() as u32)
}

// --- MITIGATED per-node structure reads (ints only, no string marshaling) ---
#[wasm_bindgen]
pub fn node_type(doc: u32, h: u32) -> u8 {
    with_doc(doc, |d| d.soa.node_type[h as usize])
}
#[wasm_bindgen]
pub fn tag_id(doc: u32, h: u32) -> i32 {
    with_doc(doc, |d| {
        let i = h as usize;
        if d.soa.node_type[i] == ELEMENT_NODE { d.soa.tag_id[i] as i32 } else { -1 }
    })
}
#[wasm_bindgen]
pub fn parent(doc: u32, h: u32) -> i32 {
    with_doc(doc, |d| d.soa.parent[h as usize])
}
#[wasm_bindgen]
pub fn first_child(doc: u32, h: u32) -> i32 {
    with_doc(doc, |d| d.soa.first_child[h as usize])
}
#[wasm_bindgen]
pub fn next_sib(doc: u32, h: u32) -> i32 {
    with_doc(doc, |d| d.soa.next_sib[h as usize])
}
#[wasm_bindgen]
pub fn text_id(doc: u32, h: u32) -> i32 {
    with_doc(doc, |d| d.soa.text_id[h as usize])
}

/// MITIGATED getAttribute: returns the interned attr_value id (-1 if absent).
/// JS resolves the id against the attr_values string table (no per-call string).
/// `name` still crosses as &str (RTL passes a literal) — small inbound cost.
#[wasm_bindgen]
pub fn get_attr_id(doc: u32, h: u32, name: &str) -> i32 {
    with_doc(doc, |d| attr_value_id(&d.soa, h as usize, name).map_or(-1, |v| v as i32))
}

/// NAIVE getAttribute: returns the value String (per-call UTF-8 marshal).
#[wasm_bindgen]
pub fn get_attr_str(doc: u32, h: u32, name: &str) -> Option<String> {
    with_doc(doc, |d| {
        attr_value_id(&d.soa, h as usize, name).map(|v| d.soa.attr_values[v as usize].clone())
    })
}

/// NAIVE tagName: per-call String marshal.
#[wasm_bindgen]
pub fn tag_name_str(doc: u32, h: u32) -> Option<String> {
    with_doc(doc, |d| {
        let i = h as usize;
        if d.soa.node_type[i] == ELEMENT_NODE {
            Some(d.soa.tag_names[d.soa.tag_id[i] as usize].clone())
        } else {
            None
        }
    })
}

fn attr_value_id(soa: &Soa, i: usize, name: &str) -> Option<u32> {
    let start = soa.attr_start[i];
    if start < 0 {
        return None;
    }
    let start = start as usize;
    let count = soa.attr_count[i] as usize;
    for j in start..start + count {
        let nid = soa.attr_name_id[j] as usize;
        if soa.attr_names[nid] == name {
            return Some(soa.attr_value_id[j]);
        }
    }
    None
}

/// MITIGATED+: prefetch a node's whole record in ONE crossing. Layout:
/// [tag_id, parent, attr_count, (name_id, value_id)*]. JS caches it on the wrapper
/// and serves tagName/getAttribute/parentNode from the cache (0 further crossings
/// until the version bumps). This is the Option-A "cache scalars on the wrapper" path.
#[wasm_bindgen]
pub fn node_record(doc: u32, h: u32) -> Vec<i32> {
    with_doc(doc, |d| {
        let i = h as usize;
        let tag = if d.soa.node_type[i] == ELEMENT_NODE { d.soa.tag_id[i] as i32 } else { -1 };
        let start = d.soa.attr_start[i];
        let count = if start < 0 { 0 } else { d.soa.attr_count[i] as usize };
        let mut out = Vec::with_capacity(3 + count * 2);
        out.push(tag);
        out.push(d.soa.parent[i]);
        out.push(count as i32);
        if start >= 0 {
            let s = start as usize;
            for j in s..s + count {
                out.push(d.soa.attr_name_id[j] as i32);
                out.push(d.soa.attr_value_id[j] as i32);
            }
        }
        out
    })
}

/// Listener-less dispatch path: walk ancestors to the root, return path length.
/// Models the single ancestor-walk dispatch does (no JS callbacks crossed).
#[wasm_bindgen]
pub fn dispatch_walk(doc: u32, h: u32) -> u32 {
    with_doc(doc, |d| {
        let mut len = 1u32; // target
        let mut cur = d.soa.parent[h as usize];
        while cur >= 0 {
            len += 1;
            cur = d.soa.parent[cur as usize];
        }
        len
    })
}

// --- MITIGATED querySelectorAll: matched entirely Rust-side, ONE crossing out ---
// Minimal selector support for the spike: a compound `tag`, `.class`, `#id`, or any
// concatenation (`div.card`, `.a.b`, `#id.x`). No combinators (enough for `div.card`).
struct Compound<'a> {
    tag: Option<&'a str>,
    id: Option<&'a str>,
    classes: Vec<&'a str>,
}

fn parse_compound(sel: &str) -> Compound<'_> {
    let s = sel.trim();
    let mut c = Compound { tag: None, id: None, classes: Vec::new() };
    let mut i = 0;
    let bytes = s.as_bytes();
    // leading type selector
    if i < bytes.len() && bytes[i] != b'.' && bytes[i] != b'#' {
        let start = i;
        while i < bytes.len() && bytes[i] != b'.' && bytes[i] != b'#' {
            i += 1;
        }
        c.tag = Some(&s[start..i]);
    }
    while i < bytes.len() {
        let kind = bytes[i];
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i] != b'.' && bytes[i] != b'#' {
            i += 1;
        }
        let tok = &s[start..i];
        match kind {
            b'.' => c.classes.push(tok),
            b'#' => c.id = Some(tok),
            _ => {}
        }
    }
    c
}

/// whole-word class scan (alloc-free) — mirrors the JS `hasClass` discipline.
fn has_class(class_attr: &str, cls: &str) -> bool {
    if cls.is_empty() {
        return false;
    }
    let mut rest = class_attr;
    loop {
        match rest.find(cls) {
            None => return false,
            Some(pos) => {
                let before_ok = pos == 0 || rest.as_bytes()[pos - 1].is_ascii_whitespace();
                let end = pos + cls.len();
                let after_ok = end == rest.len() || rest.as_bytes()[end].is_ascii_whitespace();
                if before_ok && after_ok {
                    return true;
                }
                rest = &rest[end..];
            }
        }
    }
}

fn matches(soa: &Soa, i: usize, c: &Compound) -> bool {
    if soa.node_type[i] != ELEMENT_NODE {
        return false;
    }
    if let Some(tag) = c.tag {
        if soa.tag_names[soa.tag_id[i] as usize] != tag {
            return false;
        }
    }
    if let Some(id) = c.id {
        match attr_value_id(soa, i, "id") {
            Some(v) if soa.attr_values[v as usize] == id => {}
            _ => return false,
        }
    }
    if !c.classes.is_empty() {
        let cv = match attr_value_id(soa, i, "class") {
            Some(v) => &soa.attr_values[v as usize],
            None => return false,
        };
        for cls in &c.classes {
            if !has_class(cv, cls) {
                return false;
            }
        }
    }
    true
}

#[wasm_bindgen]
pub fn qsa(doc: u32, selector: &str) -> Vec<u32> {
    with_doc(doc, |d| {
        let c = parse_compound(selector);
        let n = d.soa.node_type.len();
        let mut out = Vec::new();
        for i in 0..n {
            if matches(&d.soa, i, &c) {
                out.push(i as u32);
            }
        }
        out
    })
}

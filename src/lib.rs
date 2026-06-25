//! turbo-dom parser — Layer 1.
//! One Rust core (`core`), two interchangeable front-ends selected by feature:
//!   * `napi-bind`  → native Node addon (default, fast path)
//!   * `wasm-bind`  → wasm32 fallback (StackBlitz / WebContainers / locked-down CI)
//! Both expose the same logical API: parse(html) -> tree, parseFragment(html) -> tree.

pub mod core;

// Phase-1 boundary spike (Rust DOM runtime exported to JS via wasm-bindgen).
#[cfg(feature = "wasm-runtime")]
pub mod spike;

// ---------------------------------------------------------------------------
// napi-rs front-end (native addon)
// ---------------------------------------------------------------------------
// `not(test)`: this is cdylib boundary glue, exercised by the JS integration
// tests, never Rust-unit-tested. napi's export registration doesn't emit under
// `cfg(test)`, so building it for `cargo test` only yields dead-code warnings.
#[cfg(all(feature = "napi-bind", not(test)))]
mod napi_front {
    use crate::core;
    use napi_derive::napi;

    /// Marshaled node returned to JS. Mirrors `core::Node` as a napi object so the
    /// whole tree crosses the boundary in one return value (full-marshaling mode).
    #[napi(object)]
    pub struct JsNode {
        pub node_type: u8,
        pub name: String,
        pub value: String,
        pub namespace: String,
        pub public_id: String,
        pub system_id: String,
        pub attrs: Vec<JsAttr>,
        pub children: Vec<JsNode>,
    }

    #[napi(object)]
    pub struct JsAttr {
        pub name: String,
        pub value: String,
        pub prefix: String,
    }

    impl From<core::Node> for JsNode {
        fn from(n: core::Node) -> Self {
            JsNode {
                node_type: n.node_type,
                name: n.name,
                value: n.value,
                namespace: n.namespace,
                public_id: n.public_id,
                system_id: n.system_id,
                attrs: n
                    .attrs
                    .into_iter()
                    .map(|a| JsAttr { name: a.name, value: a.value, prefix: a.prefix })
                    .collect(),
                children: n.children.into_iter().map(JsNode::from).collect(),
            }
        }
    }

    /// Parse a full HTML document. Boundary crossed exactly once.
    #[napi]
    pub fn parse(html: String) -> JsNode {
        core::parse_html_document(&html).into()
    }

    /// Parse-only: returns node count, builds no JS tree. For isolating raw parse
    /// cost from tree-build + boundary marshaling in benchmarks.
    #[napi(js_name = "parseRaw")]
    pub fn parse_raw(html: String) -> u32 {
        core::parse_html_document_count(&html)
    }

    use napi::bindgen_prelude::{Uint32Array, Uint8Array};

    /// SoA flat buffer: structure as typed arrays, crossed once. JS inflates node
    /// objects lazily from this — no eager full-tree allocation. The fast path.
    // All numeric columns packed into ONE little-endian byte blob (1 ArrayBuffer +
    // zero-copy views in JS, vs 13 separate addon buffers + finalizers per parse).
    // Layout — 4-byte block first (keeps every Int32/Uint32 view 4-aligned), then
    // u16, then u8.  Length-N: tag_id,parent,first_child,next_sib,text_id,pub_id,
    // sys_id,attr_start | Length-M: attr_name_id,attr_value_id,attr_prefix_id |
    // u16 N: attr_count | u8 N: node_type, ns.  JS unpack must mirror this order.
    #[napi(object)]
    pub struct JsSoa {
        pub packed: Uint8Array,
        pub n: u32,
        pub m: u32,
        // The five string tables (tag_names, attr_names, attr_prefixes, attr_values,
        // strings) are shipped as ONE raw UTF-8 byte blob + a meta array, NOT as
        // Vec<String>. Vec<String> forces napi to UTF-8→UTF-16 convert every string
        // at the boundary (~12% of parse, even for strings a test never reads). The
        // blob crosses with zero conversion; JS decodes once, lazily, in unpack().
        // str_meta = [count0..count4, then every string's byte length in table order].
        pub str_blob: Uint8Array,
        pub str_meta: Uint32Array,
    }

    impl From<core::Soa> for JsSoa {
        fn from(s: core::Soa) -> Self {
            let n = s.node_type.len();
            let m = s.attr_name_id.len();
            let mut buf: Vec<u8> = Vec::with_capacity(36 * n + 12 * m);
            for col in [&s.tag_id] { for v in col.iter() { buf.extend_from_slice(&v.to_le_bytes()); } }
            for col in [&s.parent, &s.first_child, &s.next_sib, &s.text_id, &s.pub_id, &s.sys_id, &s.attr_start] {
                for v in col.iter() { buf.extend_from_slice(&v.to_le_bytes()); }
            }
            for col in [&s.attr_name_id, &s.attr_value_id, &s.attr_prefix_id] {
                for v in col.iter() { buf.extend_from_slice(&v.to_le_bytes()); }
            }
            for v in s.attr_count.iter() { buf.extend_from_slice(&v.to_le_bytes()); }
            buf.extend_from_slice(&s.node_type);
            buf.extend_from_slice(&s.ns);
            // pack all five string tables into one byte blob + meta (5 counts, then
            // per-string byte lengths in table order). Zero napi string conversion.
            let tables: [&Vec<String>; 5] =
                [&s.tag_names, &s.attr_names, &s.attr_prefixes, &s.attr_values, &s.strings];
            let total_strs: usize = tables.iter().map(|t| t.len()).sum();
            let total_bytes: usize = tables.iter().flat_map(|t| t.iter()).map(|x| x.len()).sum();
            let mut str_blob: Vec<u8> = Vec::with_capacity(total_bytes);
            let mut str_meta: Vec<u32> = Vec::with_capacity(5 + total_strs);
            for t in &tables { str_meta.push(t.len() as u32); }
            for t in &tables {
                for x in t.iter() {
                    str_meta.push(x.len() as u32);
                    str_blob.extend_from_slice(x.as_bytes());
                }
            }
            JsSoa {
                packed: Uint8Array::new(buf),
                n: n as u32,
                m: m as u32,
                str_blob: Uint8Array::new(str_blob),
                str_meta: Uint32Array::new(str_meta),
            }
        }
    }

    /// Parse a document into the SoA flat buffer (the fast runtime path).
    #[napi(js_name = "parseBuffer")]
    pub fn parse_buffer(html: String) -> JsSoa {
        core::parse_html_soa(&html).into()
    }

    /// Parse an HTML fragment (innerHTML-style). `context` is the context element:
    /// e.g. "body" (default), "td", or namespaced "svg path" / "math ms".
    #[napi(js_name = "parseFragment")]
    pub fn parse_fragment(html: String, context: Option<String>) -> JsNode {
        core::parse_html_fragment_context(&html, context.as_deref().unwrap_or("")).into()
    }
}

// ---------------------------------------------------------------------------
// wasm-bindgen front-end (fallback)
// ---------------------------------------------------------------------------
#[cfg(feature = "wasm-bind")]
mod wasm_front {
    use crate::core;
    use wasm_bindgen::prelude::*;

    /// Same contract as the native addon, marshaled via serde → JS object.
    #[wasm_bindgen]
    pub fn parse(html: &str) -> Result<JsValue, JsValue> {
        let tree = core::parse_html_document(html);
        serde_wasm_bindgen::to_value(&tree).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "parseFragment")]
    pub fn parse_fragment(html: &str, context: Option<String>) -> Result<JsValue, JsValue> {
        let tree = core::parse_html_fragment_context(html, context.as_deref().unwrap_or(""));
        serde_wasm_bindgen::to_value(&tree).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "parseBuffer")]
    pub fn parse_buffer(html: &str) -> Result<JsValue, JsValue> {
        let soa = core::parse_html_soa(html);
        serde_wasm_bindgen::to_value(&soa).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

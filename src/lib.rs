//! fast-dom parser — Layer 1.
//! One Rust core (`core`), two interchangeable front-ends selected by feature:
//!   * `napi-bind`  → native Node addon (default, fast path)
//!   * `wasm-bind`  → wasm32 fallback (StackBlitz / WebContainers / locked-down CI)
//! Both expose the same logical API: parse(html) -> tree, parseFragment(html) -> tree.

pub mod core;

// ---------------------------------------------------------------------------
// napi-rs front-end (native addon)
// ---------------------------------------------------------------------------
#[cfg(feature = "napi-bind")]
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
}

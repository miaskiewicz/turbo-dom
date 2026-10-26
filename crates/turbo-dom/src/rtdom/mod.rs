//! Pure-Rust DOM runtime (native Rust API; no wasm-bindgen / napi boundary).
//! Phase-0 of the pivoted port (RUST_PORT_PLAN §7): a Rust consumer drives this
//! in-process — zero JS↔WASM boundary, every read/mutation a plain Rust call.
//!
//! Modules:
//!   * `tree`  — COW tree over the immutable parse buffer + version counter.
//!   * `query` — selector matching + version-cached querySelectorAll/getElementBy*.
//!     With the `selector-dual` feature, every output is also computed by the frozen
//!     B-main engine (`query_frozen`) and compared (`selector_dual`).
//!   * `color` — CSS color canonicalization (rgb()/rgba()), pure functions.
//!   * `css_values` — Chrome-compatible CSS value grammars (`zoom`), pure functions.

pub mod canvas;
pub mod cascade;
pub mod color;
pub mod css_values;
pub mod cssom;
pub mod custom_elements;
pub mod events;
pub mod file;
pub mod location;
pub mod mutations;
pub mod node_ref;
pub mod query;
// The `selector-dual` gate (consolidation S3a): the frozen B-main selector engine and the
// layer that runs it next to `query` at every engine output. Never in default builds.
#[cfg(feature = "selector-dual")]
#[allow(dead_code, clippy::all, clippy::pedantic)]
#[rustfmt::skip]
mod query_frozen;
#[cfg(feature = "selector-dual")]
#[doc(hidden)]
pub mod selector_dual;
pub mod serialize;
pub mod stubs;
pub mod svg;
pub mod tree;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod conformance;
#[cfg(test)]
mod dump;
#[cfg(test)]
mod gauntlet;

pub use events::{Dom, Event};
pub use node_ref::{DocumentExt, NodeRef};
pub use tree::{Handle, LocalAttributes, ParseOptions, SourcePosition, Tree};

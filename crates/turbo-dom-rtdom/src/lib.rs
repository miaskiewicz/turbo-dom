//! turbo-dom-rtdom — pure-Rust DOM runtime for in-process Rust consumers.
//!
//! A lazy, copy-on-write DOM tree built directly over an html5ever Structure-of-
//! Arrays parse buffer, with a native Rust API and ZERO JS boundary. Extracted
//! standalone from turbo-dom's `rtdom` module (no napi/wasm/JS — those exist only
//! for the JS-consumer path, where an in-process JS runtime is faster than a
//! WASM/napi boundary anyway).
//!
//! ```ignore
//! use turbo_dom_rtdom::{Dom, Tree};
//! use turbo_dom_rtdom::rtdom::cascade;
//!
//! let mut dom = Dom::parse("<main class=grid><div class=card id=hero>hi</div></main>");
//! let cards = dom.tree.query_selector_all("div.card");
//! let style = cascade::computed_style(&dom.tree, cards[0]);
//! ```
//!
//! Modules of interest live under [`rtdom`]: `tree` (COW tree + mutations),
//! `query` (selectors), `cascade` (partial getComputedStyle), `events` (`Dom`
//! dispatch), `serialize` (inner/outerHTML), plus `color`/`svg`/`cssom`/`file`/
//! `canvas`-less stubs/`custom_elements`/`location`/`mutations`/`node_ref`.

// `core.rs` is kept byte-identical to turbo-dom's parser core for easy re-sync; it
// carries `cfg(feature = "wasm-bind")` attrs for a feature this standalone crate
// doesn't define (the attrs are simply never active here).
#![allow(unexpected_cfgs)]

/// The html5ever-backed parser core (produces the read-only SoA the tree views).
pub mod core;

/// The pure-Rust DOM runtime.
pub mod rtdom;

pub use rtdom::{Dom, Event, Handle, Tree};
pub use rtdom::node_ref::{DocumentExt, NodeRef};

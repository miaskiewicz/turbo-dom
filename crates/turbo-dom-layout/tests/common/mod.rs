//! Shared helpers for the layout integration tests — thin re-exports of the crate's
//! `test-support` module (font fixtures from `assets/fonts/`, never a system path; a
//! `ComputedStyle` builder; tree literals that build a turbo-dom `Tree`), the same
//! helpers turbo-html2pdf-core's tests use. `dead_code`/`unused_imports` are allowed
//! because each test binary uses a subset.
#![allow(dead_code, unused_imports)]

pub use turbo_dom_layout::test_support::literal::{marker_name, marker_of, style_nodes};
pub use turbo_dom_layout::test_support::{
    evolventa, evolventa_bold, font_bytes, go, registry, style,
};

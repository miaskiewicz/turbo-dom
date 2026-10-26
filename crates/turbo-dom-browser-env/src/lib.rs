//! turbo-dom-browser-env: the native browser environment over turbo-dom.
//!
//! The binding lives in [`browser_env`]; see the crate README.

// The binding moved here from turbo-test, which compiled it without lint gates,
// and turbo-surf's copy carried the same two allows on its adapter. Keeping them
// crate-wide makes the move behaviour- and diff-neutral; tightening lints is a
// follow-up that should not ride along with the move. Page-facing callbacks
// keep deliberately unused parameters and test-only helpers, so `unused` stays
// allowed; clippy's default groups are allowed for the same reason.
#![allow(unused, clippy::all)]

pub mod browser_env;

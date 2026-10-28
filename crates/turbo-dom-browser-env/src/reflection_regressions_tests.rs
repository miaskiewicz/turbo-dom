//! Regression tests (crate-owned) for reflected attributes, selector admission and inert
//! style sheets, each pinned to the value Chrome 156.0.8078.47 returns on a localhost page.
use crate::browser_env::crate_surface::guard_edges_tests::page_json;

const PAGE: &str = "<!doctype html><html><head></head><body></body></html>";

/// turbo-dom#118: `HTMLBaseElement.prototype.target` reflects the `target` attribute.
#[test]
fn base_target_reflects() {
    assert_eq!(
        page_json(
            PAGE,
            "(() => { const b = document.createElement('base'); b.target = 'x'; \
             return [b.outerHTML, b.target, Object.hasOwn(HTMLBaseElement.prototype, 'target')]; })()",
        ),
        r#"["<base target=\"x\">","x",true]"#
    );
}

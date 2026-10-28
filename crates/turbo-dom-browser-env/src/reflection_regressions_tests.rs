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

/// turbo-dom#143: `Text.prototype.wholeText` joins the contiguous Text siblings and
/// brand-checks its receiver.
#[test]
fn text_whole_text() {
    assert_eq!(
        page_json(
            PAGE,
            "(() => { const d = document.createElement('div'); d.append('a', 'b', document.createElement('i'), 'c'); \
             const get = Object.getOwnPropertyDescriptor(Text.prototype, 'wholeText'); \
             let brand; try { get.get.call(document.createComment('z')); } catch (e) { brand = e.name; } \
             const e = document.createElement('div'); e.append('a', document.createComment('m'), 'z'); \
             e.append(document.createProcessingInstruction('x', 'y'), 'w'); \
             return [d.childNodes[1].wholeText, d.firstChild.wholeText, d.lastChild.wholeText, new Text('q').wholeText, \
               brand, get.get.name, typeof get.set, get.enumerable, e.firstChild.wholeText, e.lastChild.wholeText]; })()",
        ),
        r#"["ab","ab","c","q","TypeError","get wholeText","undefined",true,"a","w"]"#
    );
}

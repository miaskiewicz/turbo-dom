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

/// turbo-dom#148 / turbo-dom#168: after `::slotted()` Chrome 156 admits only its own short
/// list of pseudo-elements (no pseudo-classes, no pseudo-class after them).
#[test]
fn slotted_admits_chrome156_set() {
    let cases = [
        (":open", false),
        (":hover", false),
        (":first-child", false),
        ("::before", true),
        ("::after", true),
        ("::marker", true),
        ("::placeholder", true),
        ("::first-line", false),
        ("::first-letter", false),
        ("::selection", false),
        ("::part(x)", false),
        ("::backdrop", true),
        ("::file-selector-button", true),
        ("::highlight(x)", false),
        ("::slotted(p)", false),
        ("::details-content", true),
        ("::search-text", false),
        ("::view-transition", true),
        ("::target-text", false),
        ("::grammar-error", false),
        ("::spelling-error", false),
        ("::cue", false),
        ("::scroll-marker", false),
        ("::column", false),
        ("::checkmark", true),
        ("::picker-icon", true),
        ("::picker(select)", true),
        ("::-webkit-scrollbar", false),
        ("::before:hover", false),
        ("::marker:hover", false),
        (":not(p)", false),
        (":popover-open", false),
        (":modal", false),
        ("::before::marker", true),
        ("::after::marker", true),
        ("::scroll-button(left)", false),
        ("::interest-hint", false),
        ("::search-text:current", false),
    ];
    let list = cases
        .iter()
        .map(|(tail, _)| format!("'::slotted(div){tail}'"))
        .collect::<Vec<_>>()
        .join(",");
    let expected = cases
        .iter()
        .map(|(_, ok)| if *ok { "\"ok 1\"" } else { "\"SyntaxError 0\"" })
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        page_json(
            PAGE,
            &format!(
                "[{list}].map(s => {{ let a; try {{ document.querySelectorAll(s); a = 'ok'; }} catch (e) {{ a = e.name; }} \
                 const st = document.head.appendChild(document.createElement('style')); st.textContent = s + '{{color:red}}'; \
                 const n = st.sheet.cssRules.length; st.remove(); return a + ' ' + n; }})"
            ),
        ),
        format!("[{expected}]")
    );
}

/// turbo-dom#162: deferred `zoom` values (`var()` / `env()`) are kept, inline and in sheets.
#[test]
fn zoom_keeps_deferred_values() {
    assert_eq!(
        page_json(
            PAGE,
            "['zoom: var(--z)', 'zoom: var(--z) !important', 'zoom: env(x)', 'zoom: calc(var(--z) * 2)'].map(d => { \
             const e = document.createElement('div'); e.style.cssText = d; \
             const st = document.head.appendChild(document.createElement('style')); st.textContent = 'p{' + d + '}'; \
             const r = st.sheet.cssRules[0].style.cssText; st.remove(); \
             return [e.style.cssText, r, e.style.zoom, e.style.getPropertyPriority('zoom')]; })",
        ),
        r#"[["zoom: var(--z);","zoom: var(--z);","var(--z)",""],["zoom: var(--z) !important;","zoom: var(--z) !important;","var(--z)","important"],["zoom: env(x);","zoom: env(x);","env(x)",""],["zoom: calc(var(--z) * 2);","zoom: calc(var(--z) * 2);","calc(var(--z) * 2)",""]]"#
    );
}

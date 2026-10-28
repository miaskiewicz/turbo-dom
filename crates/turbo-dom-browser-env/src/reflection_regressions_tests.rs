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

/// turbo-dom#163: a `<style>` in a document without a browsing context still gets a sheet
/// (createHTMLDocument); a template's inert contents do not.
#[test]
fn inert_document_style_has_sheet() {
    assert_eq!(
        page_json(
            PAGE,
            "(() => { const doc = document.implementation.createHTMLDocument(); \
             const s = doc.head.appendChild(doc.createElement('style')); s.textContent = 'p { color: red }'; \
             const t = document.createElement('template'); t.innerHTML = '<style>i{color:red}</style>'; \
             return [String(s.sheet), s.sheet && s.sheet.cssRules.length, s.sheet && s.sheet.cssRules[0].cssText, \
               doc.styleSheets.length, \
               String(t.content.firstChild.sheet), s.sheet && s.sheet.ownerNode === s, s.sheet && String(s.sheet.href)]; })()",
        ),
        r#"["[object CSSStyleSheet]",1,"p { color: red; }",1,"null",true,"null"]"#
    );
}

/// turbo-dom#165: HTMLMediaElement's boolean attributes reflect (`muted` through
/// `defaultMuted`); a scripted `controls = true` gives an audio element a box.
#[test]
fn media_booleans_reflect() {
    assert_eq!(
        page_json(
            PAGE,
            "(() => { const a = document.body.appendChild(document.createElement('audio')); const out = []; \
             for (const n of ['controls', 'autoplay', 'loop', 'defaultMuted', 'playsInline', 'disableRemotePlayback']) { \
               const v = n === 'playsInline' ? document.createElement('video') : a; \
               v[n] = true; out.push(n + ' ' + v.outerHTML + ' ' + v[n]); v[n] = false; out.push(v.outerHTML); } \
             a.controls = true; out.push(getComputedStyle(a).display); \
             out.push(['controls', 'autoplay', 'loop', 'defaultMuted'].map(n => Object.hasOwn(HTMLMediaElement.prototype, n)).join()); \
             out.push(String('playsInline' in HTMLAudioElement.prototype)); \
             const v = document.createElement('video'); v.setAttribute('muted', ''); out.push(String(v.defaultMuted) + ' ' + v.muted); \
             return out; })()",
        ),
        r#"["controls <audio controls=\"\"></audio> true","<audio></audio>","autoplay <audio autoplay=\"\"></audio> true","<audio></audio>","loop <audio loop=\"\"></audio> true","<audio></audio>","defaultMuted <audio muted=\"\"></audio> true","<audio></audio>","playsInline <video playsinline=\"\"></video> true","<video></video>","disableRemotePlayback <audio disableremoteplayback=\"\"></audio> true","<audio></audio>","inline","true,true,true,true","false","true false"]"#
    );
}

/// turbo-dom#169: every `name` IDL attribute reflects the `name` content attribute.
#[test]
fn name_reflects_on_every_owner() {
    let tags = [
        "img", "embed", "object", "form", "iframe", "map", "meta", "slot", "param", "input",
        "button", "select", "textarea", "output", "fieldset", "a", "details",
    ];
    let list = tags
        .iter()
        .map(|t| format!("'{t}'"))
        .collect::<Vec<_>>()
        .join(",");
    let expected = tags
        .iter()
        .map(|t| {
            let close = if ["img", "embed", "meta", "param", "input"].contains(t) {
                String::new()
            } else {
                format!("</{t}>")
            };
            format!(r#""{t} x <{t} name=\"x\">{close}""#)
        })
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        page_json(
            PAGE,
            &format!(
                "[{list}].map(t => {{ const e = document.createElement(t); if (!('name' in e)) return t + ' none'; \
                 e.name = 'x'; return t + ' ' + e.getAttribute('name') + ' ' + e.outerHTML; }})"
            ),
        ),
        format!("[{expected}]")
    );
}

/// turbo-dom#174: `<param>` is an HTMLParamElement with Chrome 156's reflected members, in
/// Chrome's own-key order.
#[test]
fn param_element_interface() {
    assert_eq!(
        page_json(
            PAGE,
            "(() => { const e = document.createElement('param'); e.value = 'v'; e.type = 't'; e.valueType = 'ref'; \
             return [String(e), typeof HTMLParamElement, Reflect.ownKeys(HTMLParamElement.prototype).map(String), \
               e.outerHTML, e.valueType]; })()",
        ),
        r#"["[object HTMLParamElement]","function",["name","value","type","valueType","constructor","Symbol(Symbol.toStringTag)"],"<param value=\"v\" type=\"t\" valuetype=\"ref\">","ref"]"#
    );
}

/// turbo-dom#172: the engine's own JS uses intrinsics captured at install time, so a page
/// that replaces the builtins (and calls only originals it captured) can still insert
/// nodes, style sheets and dispatch events, like Chrome 156.
#[test]
#[ignore = "turbo-dom#172: fails until the fix"]
fn page_patched_intrinsics_keep_insertion_working() {
    assert_eq!(
        page_json(PAGE, include_str!("page_patched_intrinsics_probe.js")),
        r#"{"div":true,"text":true,"script":true,"iframe":true,"insert":true,"next":true,"style":2,"event":"lh","query":1,"patched":true}"#
    );
}

/// turbo-dom#173: an interface constructor called without `new` throws Blink's message
/// (Chrome 156): "Failed to construct" for interfaces with a constructor, "Illegal
/// constructor" for the others.
#[test]
fn constructors_called_without_new_throw_blink_messages() {
    const CONSTRUCTIBLE: &str = "UIEvent MouseEvent KeyboardEvent InputEvent FocusEvent PointerEvent WheelEvent CustomEvent PopStateEvent HashChangeEvent ProgressEvent CloseEvent Document DocumentFragment Text Comment EventTarget CSSStyleSheet Range StaticRange ToggleEvent XPathEvaluator MutationObserver SubmitEvent FormDataEvent CommandEvent CustomElementRegistry";
    const ILLEGAL: &str = "Node Element HTMLDocument ShadowRoot SVGElement SVGSVGElement MathMLElement CharacterData Attr DocumentType CDATASection ProcessingInstruction NodeList HTMLCollection DOMTokenList NamedNodeMap CSSStyleDeclaration CSSRule StyleSheet MediaList MutationRecord HTMLUnknownElement CSSRuleList StyleSheetList CSSStyleRule CSSImportRule TextTrackCueList TextTrack DOMImplementation XMLDocument AbstractRange NodeRange XPathExpression XPathResult ValidityState HTMLFormControlsCollection RadioNodeList HTMLOptionsCollection ElementInternals CustomStateSet FragmentDirective CSSPseudoElement";
    let got = page_json(
        PAGE,
        &format!(
            "(() => {{ const out = {{}}; for (const name of '{CONSTRUCTIBLE} {ILLEGAL}'.split(' ')) {{ \
             try {{ globalThis[name](); out[name] = 'no throw'; }} catch (e) {{ out[name] = e.constructor.name + ': ' + e.message; }} }} \
             return out; }})()"
        ),
    );
    let got: serde_json::Value = serde_json::from_str(&got).unwrap();
    for name in CONSTRUCTIBLE.split(' ') {
        assert_eq!(
            got[name],
            format!("TypeError: Failed to construct '{name}': Please use the 'new' operator, this DOM object constructor cannot be called as a function."),
            "{name}"
        );
    }
    for name in ILLEGAL.split(' ') {
        assert_eq!(got[name], "TypeError: Illegal constructor", "{name}");
    }
}

//! HTML helpers for the screenshot drive: the URLs a caller must fetch
//! ([`image_urls`], [`image_urls_in_css`], [`stylesheet_hrefs`] — the raster does no
//! I/O; `<link rel="stylesheet">` bodies come back as the drive's linked CSS) and
//! lazy-image filling ([`delazy_images`], [`delazy_tree`]).
//!
//! The `<img>` helpers work on the parsed document's HTML `<img>` ELEMENTS
//! (miaskiewicz/turbo-dom#27), never on source text: `<img` inside an attribute value, a
//! `<script>` string, a comment or `<template>` content is not an image (Chrome's
//! `document.images`), and attribute values come character-reference-decoded from the
//! parser.
//!
//! Nothing here touches the page's own `<style>`/`<script>`/`<noscript>`/`<template>`
//! any more: turbo-dom-layout's HTML drive styles the whole parsed document (head and
//! body `<style>`s in document order; miaskiewicz/turbo-html2pdf#29), parses with
//! scripting on (so `<noscript>` content, its `<style>`s included, is inert raw text)
//! and never lays out non-visual elements. The old raw-source `<style>` scraper and the
//! text-level script/noscript/template strips that kept scraped markup from painting
//! are gone (corpus-identical without them).

use turbo_dom_layout::parse_html_tree;
use turbo_dom_layout::turbo_dom::rtdom::serialize::serialize_outer;
use turbo_dom_layout::turbo_dom::rtdom::tree::NodeType;
use turbo_dom_layout::turbo_dom::{Handle, Tree};

use crate::image_paint::html_elements;

/// The `href`s of every `<link rel="stylesheet">` in `html`, in source order.
/// Values are returned verbatim (possibly relative) — the caller resolves them
/// against the page URL and fetches them (the raster itself does no I/O). `rel`
/// is matched case-insensitively and may carry other tokens (`stylesheet
/// preload`). Alternate stylesheets (`rel="alternate stylesheet"`) are skipped.
#[must_use]
pub fn stylesheet_hrefs(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut hrefs = Vec::new();
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find("<link") {
        let tag_start = cursor + rel;
        // A tag cut off by the end of the document is dropped, as the HTML tokenizer drops it.
        let Some(end) = lower[tag_start..].find('>').map(|g| tag_start + g) else {
            break;
        };
        let tag = &html[tag_start..end];
        let tag_lower = &lower[tag_start..end];
        let rel_val = attr_value(tag, tag_lower, "rel")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let is_sheet = rel_val.split_whitespace().any(|t| t == "stylesheet")
            && !rel_val.split_whitespace().any(|t| t == "alternate");
        if is_sheet {
            if let Some(href) = attr_value(tag, tag_lower, "href") {
                let href = html_unescape(href.trim());
                if !href.is_empty() {
                    hrefs.push(href);
                }
            }
        }
        cursor = end + 1;
    }
    hrefs
}

/// Decode the HTML entities that appear in URL attributes — chiefly `&amp;`,
/// which HTML *requires* in an `href`/`src` query string (`?a=1&amp;b=2`). Left
/// undecoded, the fetched URL carries a literal `&amp;` and the server sees a
/// bogus `amp;b` parameter (this is why Wikipedia's `load.php?...&amp;only=styles`
/// stylesheet returned a stub instead of the skin CSS). Covers the named entities
/// URLs use plus numeric (`&#38;` / `&#x26;`) forms; unknown entities pass through.
fn html_unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        if let Some(entity) = s[i..].find(';').map(|semi| &s[i + 1..i + semi]) {
            push_entity(entity, &mut out);
            i += entity.len() + 2; // '&' + entity + ';'
        } else {
            out.push('&');
            i += 1;
        }
    }
    out
}

/// Append the character for one entity body (between `&` and `;`) to `out`, or the
/// verbatim `&entity;` if it isn't one we decode.
fn push_entity(entity: &str, out: &mut String) {
    let decoded = match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        _ => numeric_entity(entity),
    };
    if let Some(c) = decoded {
        out.push(c);
    } else {
        out.push('&');
        out.push_str(entity);
        out.push(';');
    }
}

/// A numeric character reference: `#38` (decimal) or `#x26` (hex).
fn numeric_entity(entity: &str) -> Option<char> {
    let digits = entity.strip_prefix('#')?;
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };
    char::from_u32(code)
}

/// Every image reference the layout engine can paint, in source order and
/// de-duplicated: `<img src>` values and `background-image: url(...)` urls. Values
/// are returned **verbatim** — exactly the resolver key turbo-dom-layout uses (the
/// raw `src`, or the unquoted `url(...)` inner) — so the caller resolves each
/// against the page URL, fetches the bytes, and stores them under the same key.
/// `data:` URIs are skipped (self-contained, nothing to fetch: the screenshot decodes an
/// `<img src="data:…">` itself, miaskiewicz/turbo-dom#27).
#[must_use]
pub fn image_urls(html: &str) -> Vec<String> {
    let tree = parse_html_tree(html);
    let mut urls: Vec<String> = html_elements(&tree, "img")
        .into_iter()
        .filter_map(|img| best_img_url(&|name| tree.get_attribute(img, name)))
        .collect();
    background_image_urls(html, &mut urls);
    let mut seen = std::collections::HashSet::new();
    urls.retain(|u| !u.starts_with("data:") && seen.insert(u.clone()));
    urls
}

/// Like [`image_urls`] but over a raw CSS string — the `background-image: url(...)`
/// references in external `<link>` stylesheets, which a scan of the HTML alone
/// misses (the layout engine paints them, but the caller never fetches their bytes).
/// Returned verbatim (the resolver key), `data:` URIs skipped. Callers should scan
/// both `image_urls(html)` and `image_urls_in_css(external_css)`.
#[must_use]
pub fn image_urls_in_css(css: &str) -> Vec<String> {
    let mut urls = Vec::new();
    background_image_urls(css, &mut urls);
    let mut seen = std::collections::HashSet::new();
    urls.retain(|u| !u.starts_with("data:") && seen.insert(u.clone()));
    urls
}

/// Attributes carrying an image URL when `src` is absent/empty — lazy-loading and
/// responsive patterns modern sites use (Nike puts the URL in `data-landscape-url`,
/// no `src` at all). `srcset`/`data-srcset` hold a candidate *list*; the first URL
/// is taken. Order = preference.
const LAZY_IMG_ATTRS: &[&str] = &[
    "data-src",
    "data-original",
    "data-lazy-src",
    "data-lazy",
    "data-image-src",
    "data-landscape-url",
    "data-portrait-url",
    "srcset",
    "data-srcset",
];

/// The *largest* candidate URL in a `srcset` value — the sharpest image, matching
/// what a browser picks for a full-width hero. Each candidate is `url [descriptor]`
/// where the descriptor is a width (`1024w`) or pixel-density (`2x`); a bare URL
/// scores 1. `"a.png 480w, b.png 1024w"` → `b.png`. Falls back to the first URL when
/// no descriptors are present.
fn largest_srcset_url(v: &str) -> &str {
    let mut best = "";
    let mut best_score = -1.0f32;
    for cand in v.split(',') {
        let mut it = cand.split_whitespace();
        let Some(url) = it.next() else { continue };
        let score = match it.next() {
            Some(d) if d.ends_with('w') => d[..d.len() - 1].parse::<f32>().unwrap_or(1.0),
            Some(d) if d.ends_with('x') => d[..d.len() - 1].parse::<f32>().unwrap_or(1.0),
            _ => 1.0,
        };
        if score > best_score {
            best_score = score;
            best = url;
        }
    }
    best
}

/// Tokens that mark a `src` as a lazy-load *placeholder* rather than the real
/// image (a 1×1 spacer / blank pixel a site shows until JS swaps in the real URL
/// held in a `data-*` attribute). Nike (and most lazy-loaders) ship a non-empty
/// placeholder `src` + the real URL in `data-*`, so a non-empty `src` alone must
/// not win over the lazy attr.
const PLACEHOLDER_SRC_TOKENS: &[&str] = &[
    "spacer",
    "blank.",
    "pixel",
    "1x1",
    "transparent",
    "placeholder",
    "dot.gif",
    "grey.gif",
    "gray.gif",
];

/// Whether an `<img src>` is a lazy-load placeholder we should look past when a
/// real lazy/responsive URL is also present. Any inline `data:` URI counts (real
/// content images are served by URL; an inline `src` alongside a `data-*` lazy
/// attr is the placeholder), as does a spacer/blank/1×1 filename.
fn is_placeholder_src(src: &str) -> bool {
    let s = src.trim().to_ascii_lowercase();
    s.starts_with("data:") || PLACEHOLDER_SRC_TOKENS.iter().any(|t| s.contains(t))
}

/// An element's attribute value by name (character references decoded).
type Attrs<'a> = dyn Fn(&str) -> Option<&'a str> + 'a;

/// The first lazy/responsive attribute URL of an `<img>` (order = preference).
fn lazy_img_url(attrs: &Attrs) -> Option<String> {
    for attr in LAZY_IMG_ATTRS {
        if let Some(v) = attrs(attr) {
            let v = v.trim();
            let url = if attr.contains("srcset") {
                largest_srcset_url(v)
            } else {
                v
            };
            let url = url.trim();
            if !url.is_empty() {
                return Some(url.to_string());
            }
        }
    }
    None
}

/// The best image URL for an `<img>`: a real (non-empty, non-placeholder) `src`, else the
/// first lazy/responsive attribute that carries one. A placeholder `src` (1×1 spacer /
/// inline `data:` pixel) yields to the lazy URL so the real image is fetched + painted
/// instead of the spacer.
fn best_img_url(attrs: &Attrs) -> Option<String> {
    let src = attrs("src").map(str::trim).filter(|s| !s.is_empty());
    match src {
        // A real src wins outright.
        Some(s) if !is_placeholder_src(s) => Some(s.to_string()),
        // A placeholder src yields to a lazy URL when one exists, else stands.
        Some(s) => lazy_img_url(attrs).or_else(|| Some(s.to_string())),
        // No src at all → the lazy/responsive fallback.
        None => lazy_img_url(attrs),
    }
}

/// What filling `<img>` element `img` takes: its new `src`, and whether to force it visible
/// (it had no `src` and no inline `style`). `None`: it already has a real `src`, or has
/// nothing better.
fn lazy_fill(tree: &Tree, img: Handle) -> Option<(String, bool)> {
    let attrs = |name: &str| tree.get_attribute(img, name);
    let src = attrs("src").map(str::trim);
    if src.is_some_and(|s| !s.is_empty() && !is_placeholder_src(s)) {
        return None; // already points at a real image
    }
    // A present placeholder `src` yields only to a lazy URL (else it stands); a missing one
    // takes the lazy/responsive fallback.
    let placeholder = src.is_some_and(|s| !s.is_empty());
    let url = if placeholder {
        lazy_img_url(&attrs)
    } else {
        best_img_url(&attrs)
    }?;
    Some((url, !placeholder && attrs("style").is_none()))
}

/// Fill in a missing `<img src>` (or replace a lazy-load *placeholder* `src`) of every
/// `<img>` element of the parsed document from its lazy/responsive attribute, so layout
/// (which sizes/paints an image box from `src`) renders lazy-loaded images whose JS
/// `src`-swap didn't run headless. A lazy image is revealed by JS swapping an
/// opacity/visibility class on load; that can't run, so an image that had no `src` is
/// forced visible too (else the engine drops it as `opacity:0` — nike's hero `<img>`s start
/// `opacity:0` as a blank band) unless it has an inline `style` that would be clobbered.
/// `<img>`s with a real `src` are left as-is. The URL matches what [`image_urls`] returns,
/// so the caller's fetched bytes resolve. Returns whether anything changed.
pub fn delazy_tree(tree: &mut Tree) -> bool {
    let fills: Vec<(Handle, (String, bool))> = html_elements(tree, "img")
        .into_iter()
        .filter_map(|img| lazy_fill(tree, img).map(|fill| (img, fill)))
        .collect();
    for (img, (url, reveal)) in &fills {
        tree.set_attribute(*img, "src", url);
        if *reveal {
            tree.set_attribute(*img, "style", REVEAL);
        }
    }
    !fills.is_empty()
}

/// The inline style that forces a de-lazied image visible.
const REVEAL: &str = "opacity:1 !important;visibility:visible !important";

/// [`delazy_tree`] over an HTML string: `html` itself when no `<img>` changes, else the
/// filled document re-serialized (its doctype, public/system ids included, then the
/// document's nodes as `outerHTML`).
#[must_use]
pub fn delazy_images(html: &str) -> String {
    let mut tree = parse_html_tree(html);
    if !delazy_tree(&mut tree) {
        return html.to_string();
    }
    tree.children(tree.root())
        .into_iter()
        .map(|node| match tree.node_type(node) {
            NodeType::Doctype => doctype(&tree, node),
            _ => serialize_outer(&tree, node),
        })
        .collect()
}

/// A doctype node as source, with its public and system identifiers (they decide the
/// document's quirks mode on a re-parse).
fn doctype(tree: &Tree, node: Handle) -> String {
    let name = tree.doctype_name(node).unwrap_or("html");
    let public = tree.doctype_public_id(node).filter(|id| !id.is_empty());
    let system = tree.doctype_system_id(node).filter(|id| !id.is_empty());
    match (public, system) {
        (Some(p), Some(s)) => format!(r#"<!DOCTYPE {name} PUBLIC "{p}" "{s}">"#),
        (Some(p), None) => format!(r#"<!DOCTYPE {name} PUBLIC "{p}">"#),
        (None, Some(s)) => format!(r#"<!DOCTYPE {name} SYSTEM "{s}">"#),
        (None, None) => format!("<!DOCTYPE {name}>"),
    }
}

/// Push the url of every `background`/`background-image: url(...)` declaration
/// onto `out` (inline `style=` attrs and `<style>` blocks alike — a raw source
/// scan). Both the longhand and the `background:` shorthand carry image urls;
/// real stylesheets use the shorthand pervasively.
fn background_image_urls(html: &str, out: &mut Vec<String>) {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    // `background` also prefixes `background-image`, so this matches both.
    while let Some(rel) = lower[cursor..].find("background") {
        let decl = cursor + rel + "background".len();
        cursor = decl;
        // Only a `url(` inside THIS declaration (before its terminator) counts —
        // a later declaration's url must not be attributed to a `background`
        // property that had none (`background: #fff`).
        // `;`/`}` end a CSS declaration; `<`/newline bound an inline `style=`
        // attr's value. Quotes are NOT terminators — they may wrap the url itself
        // (`url("x.png")`).
        let end = lower[decl..]
            .find([';', '}', '<', '\n'])
            .map_or(html.len(), |e| decl + e);
        let Some(urel) = lower[decl..end].find("url(") else {
            continue;
        };
        let inner_start = decl + urel + "url(".len();
        let Some(close) = html[inner_start..end].find(')') else {
            continue;
        };
        let name = html[inner_start..inner_start + close]
            .trim()
            .trim_matches(['"', '\''])
            .trim();
        if !name.is_empty() {
            out.push(html_unescape(name));
        }
        cursor = inner_start + close + 1;
    }
}

/// Read `name="value"` (or `name='value'`) out of an opening-tag slice. `tag` is
/// the original-case text; `tag_lower` its lowercase twin (for case-insensitive
/// attribute-name matching while returning the original-case value).
fn attr_value(tag: &str, tag_lower: &str, name: &str) -> Option<String> {
    let mut from = 0;
    loop {
        let rel = tag_lower[from..].find(name)?;
        let at = from + rel;
        // Must be a whole attribute name: preceded by whitespace/`<`, followed by `=`.
        let before_ok = at == 0 || tag.as_bytes()[at - 1].is_ascii_whitespace();
        let after = tag_lower[at + name.len()..].trim_start();
        if before_ok && after.starts_with('=') {
            let rest = tag[at + name.len()..].trim_start();
            let rest = rest.strip_prefix('=')?.trim_start();
            let (quote, body) = match rest.chars().next()? {
                q @ ('"' | '\'') => (q, &rest[1..]),
                _ => return rest.split_whitespace().next().map(str::to_string),
            };
            return body.find(quote).map(|e| body[..e].to_string());
        }
        from = at + name.len();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn delazy_fills_src_and_reveals_lazy_image() {
        use super::delazy_images;
        // A `src`-less lazy `<img>` gets its `data-landscape-url` promoted to `src`
        // plus a reveal style (it starts `opacity:0`, revealed by JS on load).
        let out = delazy_images(r#"<img data-landscape-url="hero.jpg" alt="x">"#);
        assert!(out.contains(r#"src="hero.jpg""#), "src filled: {out}");
        assert!(out.contains("opacity:1 !important"), "revealed: {out}");
        assert!(out.contains("visibility:visible !important"));
    }

    #[test]
    fn delazy_takes_largest_srcset_and_skips_when_src_present() {
        use super::delazy_images;
        // No `src`: the LARGEST `srcset` candidate wins (sharpest hero) — `2x` over `1x`.
        let out = delazy_images(r#"<img srcset="a.jpg 1x, b.jpg 2x">"#);
        assert!(out.contains(r#"src="b.jpg""#), "{out}");
        // Width descriptors: the widest candidate wins.
        let w = delazy_images(r#"<img srcset="s.jpg 480w, l.jpg 1920w, m.jpg 1024w">"#);
        assert!(w.contains(r#"src="l.jpg""#), "{w}");
        // An `<img>` that already has a non-empty `src` is left untouched.
        let already = r#"<img src="real.jpg">"#;
        assert_eq!(delazy_images(already), already);
    }

    #[test]
    fn image_urls_survive_gt_inside_attribute_value() {
        use super::image_urls;
        // Nike's hero: `alt="PLAY MIND<br>GAMES"` puts a `>` inside a quoted attribute.
        // A naive tag-end scan truncates the tag at that `>` and never reaches
        // `data-landscape-url`, so the hero image is never fetched. The tag-end must
        // be quote-aware.
        let html =
            r#"<img loading="lazy" alt="PLAY MIND<br>GAMES" data-landscape-url="hero-1920.jpg">"#;
        let urls = image_urls(html);
        assert!(urls.contains(&"hero-1920.jpg".to_string()), "{urls:?}");
    }

    #[test]
    fn delazy_survives_gt_inside_attribute_value() {
        use super::delazy_images;
        // Same tag, through the delazy rewrite: the real hero URL must land in `src`.
        let html =
            r#"<img loading="lazy" alt="PLAY MIND<br>GAMES" data-landscape-url="hero-1920.jpg">"#;
        let out = delazy_images(html);
        assert!(out.contains(r#"src="hero-1920.jpg""#), "{out}");
    }

    #[test]
    fn delazy_replaces_a_placeholder_data_uri_src_with_the_lazy_url() {
        use super::delazy_images;
        // Nike-style: a non-empty 1×1 `data:` placeholder `src` + the real URL in a
        // lazy attr. The placeholder must be REPLACED (not left, not duplicated) so
        // the real image is sized/painted.
        let out = delazy_images(
            r#"<img src="data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==" data-src="real.jpg">"#,
        );
        assert!(
            out.contains(r#"src="real.jpg""#),
            "placeholder replaced: {out}"
        );
        assert!(!out.contains("data:image/gif"), "placeholder gone: {out}");
    }

    #[test]
    fn delazy_replaces_a_spacer_src_with_the_lazy_url() {
        use super::delazy_images;
        let out = delazy_images(r#"<img src="/img/spacer.gif" data-landscape-url="hero.jpg">"#);
        assert!(out.contains(r#"src="hero.jpg""#), "spacer replaced: {out}");
        assert!(!out.contains("spacer.gif"), "spacer gone: {out}");
    }

    #[test]
    fn delazy_keeps_a_real_data_uri_when_no_lazy_url() {
        use super::delazy_images;
        // A genuine inline `data:` image with NO lazy attr must be left as-is (we only
        // look past a placeholder when a real lazy URL exists to use instead).
        let real = r#"<img src="data:image/png;base64,iVBORw0KGgoAAAANSU">"#;
        assert_eq!(delazy_images(real), real);
    }

    #[test]
    fn delazy_leaves_a_data_uri_src_with_character_references_verbatim() {
        use super::delazy_images;
        // A raw-UTF-8 SVG `data:` URL carries its quotes as `&quot;`: with no lazy URL to
        // swap in, the source text must not be rewritten (writing the unescaped value back
        // would end the attribute at its first `"`; miaskiewicz/turbo-dom#25).
        let real = r#"<img src="data:image/svg+xml;utf8,<svg xmlns=&quot;http://www.w3.org/2000/svg&quot;></svg>">"#;
        assert_eq!(delazy_images(real), real);
    }

    #[test]
    fn delazy_keeps_the_doctype_and_its_identifiers() {
        use super::delazy_images;
        let body = r#"<img data-src="a.jpg">"#;
        for (doctype, kept) in [
            (
                r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd">"#,
                r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd">"#,
            ),
            (
                r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN">"#,
                r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN">"#,
            ),
            (
                r#"<!DOCTYPE html SYSTEM "about:legacy-compat">"#,
                r#"<!DOCTYPE html SYSTEM "about:legacy-compat">"#,
            ),
            ("<!doctype html>", "<!DOCTYPE html>"),
        ] {
            let out = delazy_images(&format!("{doctype}{body}"));
            assert!(out.starts_with(kept), "{out}");
            assert!(out.contains(r#"src="a.jpg""#), "{out}");
        }
        // A document without one gets none; root-level comments are kept.
        let out = delazy_images(&format!("<!-- c -->{body}"));
        assert!(out.starts_with("<!-- c --><html>"), "{out}");
    }

    #[test]
    fn delazy_and_image_urls_skip_template_content_and_img_text() {
        use super::{delazy_images, image_urls};
        // turbo-dom#27: only the document's <img> elements are images.
        let html = r#"<template><img data-src="t.jpg"></template><script>"<img data-src='s.jpg'>"</script><!-- <img data-src="c.jpg"> --><p title="<img data-src='a.jpg'>"></p>"#;
        assert_eq!(delazy_images(html), html);
        assert!(image_urls(html).is_empty());
    }

    #[test]
    fn delazy_escapes_the_lazy_url_it_writes() {
        use super::delazy_images;
        // The written URL round-trips: the output parses back to the decoded lazy URL.
        let out = delazy_images(r#"<img data-src="a.jpg?x=1&amp;q=&quot;'">"#);
        assert!(out.contains(r#" src="a.jpg?x=1&amp;q=&quot;'""#), "{out}");
        let tree = super::parse_html_tree(&out);
        let img = super::html_elements(&tree, "img")[0];
        assert_eq!(tree.get_attribute(img, "src"), Some(r#"a.jpg?x=1&q="'"#));
        let out = delazy_images(r#"<img src="data:," data-src="b.jpg?x=1&amp;y=2">"#);
        assert!(out.contains(r#"<img src="b.jpg?x=1&amp;y=2""#), "{out}");
    }

    #[test]
    fn image_urls_prefers_lazy_over_a_placeholder_src() {
        use super::image_urls;
        // The fetch side agrees with delazy: a placeholder `src` yields the lazy URL,
        // so the caller fetches the real image (not the 1×1 spacer).
        let urls = image_urls(r#"<img src="data:image/gif;base64,R0lGOD" data-src="real.jpg">"#);
        assert!(
            urls.iter().any(|u| u == "real.jpg"),
            "fetches real: {urls:?}"
        );
        assert!(
            !urls.iter().any(|u| u.starts_with("data:")),
            "not the placeholder: {urls:?}"
        );
    }

    #[test]
    fn delazy_does_not_clobber_an_existing_inline_style() {
        use super::delazy_images;
        // A lazy `<img>` that carries an inline `style` keeps it — no reveal override
        // is injected (it would need merging, not a second `style` attribute).
        let out = delazy_images(r#"<img data-src="x.jpg" style="border:0">"#);
        assert!(out.contains(r#"src="x.jpg""#), "{out}");
        assert!(!out.contains("opacity:1"), "no style override: {out}");
        assert!(out.contains(r#"style="border:0""#));
    }

    #[test]
    fn stylesheet_hrefs_extracts_rel_stylesheet() {
        use super::stylesheet_hrefs;
        let html = r#"<head>
            <link rel="stylesheet" href="/a.css">
            <link href='https://cdn.example/b.css' rel="stylesheet preload">
            <link rel="icon" href="/favicon.ico">
            <link rel="alternate stylesheet" href="/dark.css">
            <link rel=stylesheet href=bare.css>
          </head>"#;
        let hrefs = stylesheet_hrefs(html);
        assert_eq!(
            hrefs,
            vec!["/a.css", "https://cdn.example/b.css", "bare.css"]
        );
    }

    #[test]
    fn stylesheet_hrefs_decode_html_entities() {
        use super::stylesheet_hrefs;
        // HTML requires `&` in an href to be written `&amp;`; the returned href
        // must be the real URL (`&`), not the escaped source, or the fetch 404s /
        // drops query params (Wikipedia's load.php skin CSS bug).
        let html =
            r#"<link rel="stylesheet" href="/w/load.php?lang=en&amp;only=styles&amp;skin=vector">"#;
        assert_eq!(
            stylesheet_hrefs(html),
            vec!["/w/load.php?lang=en&only=styles&skin=vector"]
        );
    }

    #[test]
    fn hrefs_decode_named_and_numeric_references_and_keep_the_rest() {
        use super::stylesheet_hrefs;
        let html = concat!(
            r#"<link rel="stylesheet" href="/a?l=&lt;&gt;&quot;&apos;&#39;&#38;&#x26;&#X26;"#,
            r#"&bogus;&#xZZ;&#99999999;&#;&#65&end">"#,
        );
        // Unknown names, bad hex, code points past U+10FFFF and an empty `&#;` stay
        // verbatim; no `;` follows `&#65&end`, so its `&`s stay literal.
        assert_eq!(
            stylesheet_hrefs(html),
            [r#"/a?l=<>"''&&&&bogus;&#xZZ;&#99999999;&#;&#65&end"#]
        );
    }

    /// An unterminated `<link` at the end of the document is dropped (the HTML tokenizer
    /// discards a tag cut off by EOF, so Chrome loads nothing) — the scan must not panic.
    #[test]
    fn an_unterminated_link_at_the_end_is_not_a_stylesheet() {
        use super::stylesheet_hrefs;
        assert_eq!(
            stylesheet_hrefs(r#"<link rel="stylesheet" href="a.css""#),
            Vec::<String>::new()
        );
        assert_eq!(
            stylesheet_hrefs(r#"<link rel=stylesheet href=a.css><link rel=stylesheet href=b.css"#),
            ["a.css"]
        );
    }

    #[test]
    fn a_real_src_wins_over_lazy_attributes() {
        use super::{delazy_images, image_urls};
        let html = r#"<img src="real.jpg" data-src="lazy.jpg" srcset="big.jpg 2x">"#;
        assert_eq!(delazy_images(html), html, "left as is");
        assert_eq!(image_urls(html), ["real.jpg"]);
        // A spacer src is not real: the lazy URL replaces it, without a reveal style.
        let spacer = r#"<img src="/i/1x1.gif" data-src="lazy.jpg">"#;
        assert_eq!(image_urls(spacer), ["lazy.jpg"]);
        let out = delazy_images(spacer);
        assert!(
            out.contains(r#"<img src="lazy.jpg" data-src="lazy.jpg">"#),
            "{out}"
        );
    }

    #[test]
    fn css_image_urls_are_deduplicated_and_skip_data_urls() {
        use super::image_urls_in_css;
        let css = r#".a{background:url(a.png)}.b{background-image:url("data:image/svg+xml,x")}
            .c{background:url('a.png')}.d{background:url(b.png)}"#;
        assert_eq!(image_urls_in_css(css), ["a.png", "b.png"]);
    }

    #[test]
    fn a_long_url_is_read_once_and_the_scan_moves_past_it() {
        use super::image_urls_in_css;
        let css = ".a{background:url(a-very-long-image-file-name-0123456789-abcdefghij.png)}\
                   .b{background:url(b.png)}";
        assert_eq!(
            image_urls_in_css(css),
            [
                "a-very-long-image-file-name-0123456789-abcdefghij.png",
                "b.png"
            ]
        );
    }

    #[test]
    fn rel_is_matched_as_a_whole_attribute_name() {
        use super::stylesheet_hrefs;
        // `data-rel` is not `rel`; the real `rel` later in the tag is found.
        let html = r#"<link data-rel="icon" rel="stylesheet" href="a.css"><link href="b.css" data-rel="stylesheet">"#;
        assert_eq!(stylesheet_hrefs(html), ["a.css"]);
    }

    #[test]
    fn srcset_candidates_without_a_usable_descriptor_score_one() {
        use super::delazy_images;
        // A bare URL and a descriptor that is neither `w` nor `x` both score 1, so the
        // first such candidate stands unless a larger one follows; empty candidates skip.
        let first = delazy_images(r#"<img srcset="a.jpg, b.jpg 1q, ,">"#);
        assert!(first.contains(r#"src="a.jpg""#), "{first}");
        let larger = delazy_images(r#"<img srcset="a.jpg 3q, b.jpg 2x">"#);
        assert!(larger.contains(r#"src="b.jpg""#), "{larger}");
    }

    #[test]
    fn image_urls_decode_html_entities() {
        use super::image_urls;
        let html = r#"<img src="/img?w=100&amp;h=50&amp;fmt=webp">"#;
        assert_eq!(image_urls(html), vec!["/img?w=100&h=50&fmt=webp"]);
    }

    #[test]
    fn image_urls_extracts_img_and_background_skipping_data() {
        use super::image_urls;
        let html = r#"<img src="/a.png">
            <div style="background-image:url('b.jpg')"></div>
            <style>.x{ background-image: url(c.png) }</style>
            <div style="background:#fff url(d.webp) no-repeat"></div>
            <style>.y{ background: url("e.svg") center }</style>
            <div style="background:#000"></div>
            <img src="data:image/png;base64,AAAA">
            <img src="/a.png">"#;
        // `<img>` srcs first (source order), then background/-image urls (longhand
        // + shorthand, quoted or bare). A colour-only `background` and `data:` are
        // skipped; duplicates removed.
        assert_eq!(
            image_urls(html),
            vec!["/a.png", "b.jpg", "c.png", "d.webp", "e.svg"]
        );
    }

    #[test]
    fn image_urls_in_css_extracts_external_backgrounds() {
        use super::image_urls_in_css;
        // External `<link>` stylesheets carry `background-image` refs that a scan of
        // the HTML alone misses; longhand + shorthand, quoted or bare, `data:` skipped.
        let css = ".a{background-image:url(https://x/bg1.png)}\
                   .b{background:#fff url(\"bg2.jpg\") no-repeat}\
                   .c{background-image:url(data:image/png;base64,AAA)}";
        assert_eq!(image_urls_in_css(css), vec!["https://x/bg1.png", "bg2.jpg"]);
    }
}

//! Recover author CSS from a raw HTML snapshot.
//!
//! The layout engine's html5ever pass keeps only `<body>` children, so any
//! `<style>` in `<head>` is dropped before it can cascade. We scan the raw
//! source for `<style>…</style>` blocks and hand their concatenated text back as
//! author CSS. `<link rel="stylesheet">` is intentionally *not* followed — that
//! needs a network fetch, and a screenshot renders the snapshot as given.

/// Elements whose *text content* must never be painted: the layout engine would
/// otherwise flow their raw source (JS, CSS, fallbacks) as visible body text.
const NON_VISUAL_TAGS: [&str; 4] = ["script", "style", "noscript", "template"];

/// Strip every `<script>`/`<style>`/`<noscript>`/`<template>` element (tag +
/// body) from raw HTML so their source never renders as text. Call *after*
/// [`collect_style_blocks`] so `<style>` CSS is still cascaded. Case-insensitive;
/// tolerates attributes on the opening tag. Unclosed tags are left as-is.
pub fn strip_non_visual(html: &str) -> String {
    let mut out = html.to_string();
    for tag in NON_VISUAL_TAGS {
        out = strip_tag_blocks(&out, tag);
    }
    out
}

/// Strip every `<noscript>…</noscript>` element (tag + body) from raw HTML.
///
/// turbo-surf rasters the *hydrated* DOM — the page after its own JS ran — so
/// scripting is enabled and `<noscript>` content is inert (a browser never
/// applies it). Its inner `<style>`/`<img>` fallbacks must be dropped BEFORE
/// [`collect_style_blocks`] and [`delazy_images`], or a noscript rule like
/// Google's `<noscript><style>table,div,span,p{display:none}</style></noscript>`
/// cascades and hides every element lacking an explicit `display` (blanking the
/// centred search UI).
pub fn strip_noscript(html: &str) -> String {
    strip_tag_blocks(html, "noscript")
}

fn strip_tag_blocks(html: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}");
    let mut out = String::with_capacity(html.len());
    let lower = html.to_ascii_lowercase();
    let bytes = html.as_bytes();
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find(&open) {
        let tag_start = cursor + rel;
        // Guard against a longer tag name (`<style` vs `<styled`): the char after
        // the name must end the name (whitespace, `>`, or self-close `/`).
        let after = bytes.get(tag_start + open.len()).copied();
        if !matches!(after, Some(b) if b == b'>' || b == b'/' || b.is_ascii_whitespace()) {
            out.push_str(&html[cursor..tag_start + open.len()]);
            cursor = tag_start + open.len();
            continue;
        }
        out.push_str(&html[cursor..tag_start]);
        // Drop through the matching close tag's `>` (or to EOF if unclosed).
        match lower[tag_start..].find(&close) {
            Some(crel) => {
                let close_start = tag_start + crel;
                let end = lower[close_start..]
                    .find('>')
                    .map(|g| close_start + g + 1)
                    .unwrap_or(html.len());
                cursor = end;
            }
            None => {
                cursor = html.len();
                break;
            }
        }
    }
    out.push_str(&html[cursor..]);
    out
}

/// The `href`s of every `<link rel="stylesheet">` in `html`, in source order.
/// Values are returned verbatim (possibly relative) — the caller resolves them
/// against the page URL and fetches them (the raster itself does no I/O). `rel`
/// is matched case-insensitively and may carry other tokens (`stylesheet
/// preload`). Alternate stylesheets (`rel="alternate stylesheet"`) are skipped.
pub fn stylesheet_hrefs(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut hrefs = Vec::new();
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find("<link") {
        let tag_start = cursor + rel;
        let end = lower[tag_start..]
            .find('>')
            .map(|g| tag_start + g)
            .unwrap_or(html.len());
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
        match s[i..].find(';').map(|semi| &s[i + 1..i + semi]) {
            Some(entity) => {
                push_entity(entity, &mut out);
                i += entity.len() + 2; // '&' + entity + ';'
            }
            None => {
                out.push('&');
                i += 1;
            }
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
    match decoded {
        Some(c) => out.push(c),
        None => {
            out.push('&');
            out.push_str(entity);
            out.push(';');
        }
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
/// are returned **verbatim** — exactly the resolver key turbo-html2pdf uses (the
/// raw `src`, or the unquoted `url(...)` inner) — so the caller resolves each
/// against the page URL, fetches the bytes, and stores them under the same key.
/// `data:` URIs are skipped (self-contained, nothing to fetch).
pub fn image_urls(html: &str) -> Vec<String> {
    let mut urls = Vec::new();
    img_srcs(html, &mut urls);
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

/// The first lazy/responsive attribute URL on the tag (order = preference).
fn lazy_img_url(tag: &str, tag_lower: &str) -> Option<String> {
    for attr in LAZY_IMG_ATTRS {
        if let Some(v) = attr_value(tag, tag_lower, attr) {
            let v = v.trim();
            let url = if attr.contains("srcset") {
                largest_srcset_url(v)
            } else {
                v
            };
            let url = html_unescape(url.trim());
            if !url.is_empty() {
                return Some(url);
            }
        }
    }
    None
}

/// The best image URL for an `<img>` tag: a real (non-empty, non-placeholder)
/// `src`, else the first lazy/responsive attribute that carries one. A placeholder
/// `src` (1×1 spacer / inline `data:` pixel) yields to the lazy URL so the real
/// image is fetched + painted instead of the spacer.
fn best_img_url(tag: &str, tag_lower: &str) -> Option<String> {
    let src = attr_value(tag, tag_lower, "src")
        .map(|s| html_unescape(s.trim()))
        .filter(|s| !s.is_empty());
    match src {
        // A real src wins outright.
        Some(s) if !is_placeholder_src(&s) => Some(s),
        // A placeholder src yields to a lazy URL when one exists, else stands.
        Some(s) => lazy_img_url(tag, tag_lower).or(Some(s)),
        // No src at all → the lazy/responsive fallback.
        None => lazy_img_url(tag, tag_lower),
    }
}

/// Run `f` for each `<img>` tag with its (raw, lower) slice and its `[start,end)`.
fn for_each_img(html: &str, mut f: impl FnMut(&str, &str, usize, usize)) {
    let lower = html.to_ascii_lowercase();
    let bytes = html.as_bytes();
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find("<img") {
        let tag_start = cursor + rel;
        // Whole tag name: char after `<img` must end it (ws / `>` / `/`).
        let after = bytes.get(tag_start + 4).copied();
        if !matches!(after, Some(b) if b == b'>' || b == b'/' || b.is_ascii_whitespace()) {
            cursor = tag_start + 4;
            continue;
        }
        let end = tag_end(bytes, tag_start).unwrap_or(html.len());
        f(
            &html[tag_start..end],
            &lower[tag_start..end],
            tag_start,
            end,
        );
        cursor = end + 1;
    }
}

/// Byte offset of the `>` that closes the tag opening at `start`, skipping any `>`
/// inside a quoted attribute value (`alt="PLAY MIND<br>GAMES"` — a stray `>` in the
/// value must not end the tag). Returns `None` if unterminated.
fn tag_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'>' => return Some(i),
            None => {}
        }
    }
    None
}

/// Push the resolved image URL (`src`, or a lazy/responsive fallback) of every
/// `<img>` onto `out`.
fn img_srcs(html: &str, out: &mut Vec<String>) {
    for_each_img(html, |tag, tag_lower, _, _| {
        if let Some(url) = best_img_url(tag, tag_lower) {
            out.push(url);
        }
    });
}

/// The byte span `[start, end)` of attribute `name`'s (quoted) value within `tag`,
/// or `None` if absent/unquoted. Mirrors [`attr_value`]'s scan but returns the span
/// so a value can be rewritten in place.
fn attr_value_span(tag: &str, tag_lower: &str, name: &str) -> Option<(usize, usize)> {
    let mut from = 0;
    loop {
        let rel = tag_lower[from..].find(name)?;
        let at = from + rel;
        let before_ok = at == 0 || tag.as_bytes()[at - 1].is_ascii_whitespace();
        let after_name = at + name.len();
        if before_ok && tag_lower[after_name..].trim_start().starts_with('=') {
            // Advance past `=` and any whitespace to the opening quote.
            let eq = after_name + tag[after_name..].find('=')?;
            let vstart_rel = tag[eq + 1..].find(['"', '\''])?;
            let quote = tag.as_bytes()[eq + 1 + vstart_rel];
            let val_start = eq + 1 + vstart_rel + 1;
            let val_end = val_start + tag[val_start..].find(quote as char)?;
            return Some((val_start, val_end));
        }
        from = after_name;
    }
}

/// Fill in a missing `<img src>` (or replace a lazy-load *placeholder* `src`) from
/// its lazy/responsive attribute, so the layout (which sizes/paints an image box
/// from `src`) renders lazy-loaded images whose JS `src`-swap didn't run headless.
/// `<img>`s with a real `src` are left as-is. The resulting URL matches what
/// [`image_urls`] returns, so the caller's fetched bytes resolve.
pub fn delazy_images(html: &str) -> String {
    // Collect edits `[start, end) -> text` (an insert is a zero-width span), then
    // splice once, back-to-front so earlier offsets stay valid.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for_each_img(html, |tag, tag_lower, start, _end| {
        let src = attr_value(tag, tag_lower, "src").map(|s| s.trim().to_string());
        let real_src = src
            .as_deref()
            .is_some_and(|s| !s.is_empty() && !is_placeholder_src(s));
        if real_src {
            return; // already points at a real image
        }
        let Some(url) = best_img_url(tag, tag_lower) else {
            return;
        };
        match src {
            // A placeholder `src` is present: rewrite its value to the real URL in
            // place (a second `src` attr would be ignored — first wins in the parser).
            Some(s) if !s.is_empty() => {
                if let Some((vs, ve)) = attr_value_span(tag, tag_lower, "src") {
                    edits.push((start + vs, start + ve, url));
                }
            }
            // No `src` at all: insert one right after `<img` (offset start+4). A lazy
            // image is revealed by JS swapping an opacity/visibility class on load;
            // we can't run that, so force it visible too (else the engine drops it as
            // `opacity:0` — nike's hero `<img>`s start opacity:0 as a blank band).
            // Only when the tag has no inline `style` we would otherwise clobber.
            _ => {
                let reveal = if attr_value(tag, tag_lower, "style").is_none() {
                    r#" style="opacity:1 !important;visibility:visible !important""#
                } else {
                    ""
                };
                let at = start + 4;
                edits.push((at, at, format!(r#" src="{url}"{reveal}"#)));
            }
        }
    });
    if edits.is_empty() {
        return html.to_string();
    }
    edits.sort_by_key(|(s, _, _)| *s);
    let mut out = html.to_string();
    for (s, e, text) in edits.into_iter().rev() {
        out.replace_range(s..e, &text);
    }
    out
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
            .map(|e| decl + e)
            .unwrap_or(html.len());
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

/// Concatenate the text of every `<style>` element in `html`, in source order.
/// Attributes on the opening tag (e.g. `type`, `media`) are skipped; only the
/// element body is returned.
pub fn collect_style_blocks(html: &str) -> String {
    let mut out = String::new();
    let bytes = html.as_bytes();
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find("<style") {
        let tag_start = cursor + rel;
        // Confirm it's the `<style` element, not `<styled-x>` — next char must be
        // whitespace or the tag close.
        let after = bytes.get(tag_start + 6).copied();
        if !matches!(after, Some(b) if b == b'>' || b.is_ascii_whitespace()) {
            cursor = tag_start + 6;
            continue;
        }
        // Body starts after the opening tag's `>`.
        let Some(gt) = lower[tag_start..].find('>') else {
            break;
        };
        let body_start = tag_start + gt + 1;
        let Some(end_rel) = lower[body_start..].find("</style") else {
            break;
        };
        out.push_str(&html[body_start..body_start + end_rel]);
        out.push('\n');
        cursor = body_start + end_rel + "</style".len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::collect_style_blocks;

    #[test]
    fn pulls_head_and_body_styles_in_order() {
        let html = r#"<html><head>
            <style type="text/css">.a{color:red}</style>
          </head><body>
            <style>.b{color:blue}</style>
            <div>hi</div>
          </body></html>"#;
        let css = collect_style_blocks(html);
        assert!(css.contains(".a{color:red}"));
        assert!(css.contains(".b{color:blue}"));
        assert!(
            css.find(".a").unwrap() < css.find(".b").unwrap(),
            "source order"
        );
    }

    #[test]
    fn ignores_similar_tag_names() {
        // `<styled>` must not be mistaken for `<style>`.
        let css = collect_style_blocks("<styled>nope</styled><style>.x{}</style>");
        assert!(css.contains(".x{}"));
        assert!(!css.contains("nope"));
    }

    #[test]
    fn empty_when_no_styles() {
        assert_eq!(collect_style_blocks("<div>plain</div>"), "");
    }

    #[test]
    fn strip_noscript_removes_tag_and_body() {
        use super::strip_noscript;
        // The inert noscript block (styles + fallback markup) is dropped whole; the
        // surrounding real content survives.
        let html = r#"<body><noscript><style>div{display:none}</style><img src="fallback.png"></noscript><div>real</div></body>"#;
        let out = strip_noscript(html);
        assert!(!out.contains("noscript"));
        assert!(!out.contains("display:none"));
        assert!(!out.contains("fallback.png"));
        assert!(out.contains("<div>real</div>"));
    }

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
    fn strip_non_visual_removes_script_and_style_source() {
        use super::strip_non_visual;
        let html = r#"<div>hi</div>
            <script>var x = new Granim({a:1});</script>
            <style>.a{color:red}</style>
            <noscript>enable js</noscript>
            <p>bye</p>"#;
        let out = strip_non_visual(html);
        assert!(out.contains("<div>hi</div>"));
        assert!(out.contains("<p>bye</p>"));
        assert!(!out.contains("Granim"), "script source must be gone");
        assert!(!out.contains("color:red"), "style source must be gone");
        assert!(!out.contains("enable js"), "noscript must be gone");
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

    #[test]
    fn strip_non_visual_spares_similar_tags_and_unclosed() {
        // `<scripting>` is not `<script>`; an unclosed `<style` is left intact.
        let out = super::strip_non_visual("<scripting>keep</scripting><b>x</b>");
        assert!(out.contains("keep") && out.contains("<b>x</b>"));
    }
}

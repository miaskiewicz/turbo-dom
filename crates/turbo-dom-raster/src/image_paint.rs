//! Decode fetched image bytes (PNG/JPEG/GIF/WebP + SVG) into a single canonical
//! form — straight-alpha RGBA8 — that both painters consume, plus the intrinsic size
//! the layout engine sizes the image box with:
//!
//! - **Decode** runs through turbo-dom-layout's shared image layer where it is
//!   byte-identical: PNG via [`turbo_dom_layout::image::decode_png_rgba`] (the same
//!   decode the PDF emitter uses, reproducing `image::to_rgba8`), SVG via the one
//!   rasterizer [`turbo_dom_layout::svg::rasterize`] at 1x. JPEG (zune-jpeg), GIF
//!   (frame 0 composited) and WebP stay on the `image` crate.
//! - **Layout sizing**: [`DecodedImage::intrinsic`] — the shared header probe
//!   ([`turbo_dom_layout::image::probe`]) for raster formats, the rasterized 1x size
//!   for SVG — answered through [`ImageResolver::intrinsic`](turbo_dom_layout::ImageResolver::intrinsic),
//!   so nothing is re-encoded to PNG for sizing.
//! - **PNG paint**: scale the RGBA into the fragment box ([`DecodedImage::scaled_pixmap`]).
//! - **SVG paint**: embed a base64 PNG `data:` URI ([`data_uri`]) — universally
//!   renderable regardless of the source format.

use std::collections::HashMap;
use std::io::Cursor;

use tiny_skia::{Pixmap, PremultipliedColorU8};
use turbo_dom_layout::image::{self as layout_image, Format, Intrinsic, Orientation, Rgba8Image};
use turbo_dom_layout::svg::{self as layout_svg, Oversize};
use turbo_dom_layout::turbo_dom::rtdom::tree::{Namespace, NodeType};
use turbo_dom_layout::turbo_dom::{Handle, Tree};
use turbo_dom_layout::ImageResolver;

/// A decoded image in straight-alpha RGBA8 (row-major, 4 bytes/px), plus its
/// intrinsic pixel size. The canonical form every source format normalizes to.
#[derive(Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// The size + alpha flag layout sizes the image box with. `has_alpha` is always
    /// true: the painters draw every image as straight RGBA.
    pub intrinsic: Intrinsic,
}

/// Decoded images keyed by their reference (the same key [`crate::image_urls`]
/// returns), decoded once and shared by the layout-sizing and paint passes.
pub type DecodedAssets = HashMap<String, DecodedImage>;

/// Decode every fetched image, dropping any that don't decode. Done once so the
/// bytes are parsed a single time for both sizing and painting.
pub fn decode_all(raw: &HashMap<String, Vec<u8>>) -> DecodedAssets {
    raw.iter()
        .filter_map(|(k, v)| Some((k.clone(), decode(v)?)))
        .collect()
}

/// The layout engine's view of the decoded assets: sizing only (layout never needs
/// the bytes — the painters draw from the decode). Keys are looked up verbatim.
pub struct AssetResolver<'a>(pub &'a DecodedAssets);

impl ImageResolver for AssetResolver<'_> {
    /// Layout sizes through [`intrinsic`](Self::intrinsic); no encoded bytes are
    /// handed out.
    fn resolve(&self, _name: &str) -> Option<&[u8]> {
        None
    }

    fn intrinsic(&self, name: &str) -> Option<Intrinsic> {
        self.0.get(name).map(|img| img.intrinsic)
    }
}

/// Decode every `data:` image URI referenced by a `url(...)` (a CSS `mask-image`,
/// `background-image`, …) in `css`/`html` and add it to `decoded`, keyed by the
/// exact string turbo-dom-layout resolves against (the `url(...)` inner, surrounding
/// quotes stripped, inner `\"` retained). Caller-fetched assets skip `data:` URIs
/// (self-contained, nothing to fetch); this recovers them so an inline SVG mask or
/// background paints — Wikipedia ships its TOC-toggle chevron as an inline
/// `data:image/svg+xml` mask-image, invisible without this.
pub fn add_data_uri_images(decoded: &mut DecodedAssets, css: &str, html: &str) {
    for key in data_uri_url_tokens(css)
        .into_iter()
        .chain(data_uri_url_tokens(html))
    {
        if decoded.contains_key(&key) {
            continue;
        }
        if let Some(img) = decode_data_uri(&key).as_deref().and_then(decode) {
            decoded.insert(key, img);
        }
    }
}

/// Decode every `<img src="data:…">` ELEMENT of the parsed document (template content
/// excluded) into `decoded`, keyed by its `src` — the name layout sizes the image box with —
/// through turbo-dom-layout's [`decode_data_url`](layout_image::decode_data_url) (base64 or
/// percent-encoded; the scheme is case-insensitive). A `data:` image needs no fetch: the
/// screenshot paints it from its own URL, as Chrome does (miaskiewicz/turbo-dom#27).
/// Caller-supplied bytes under the same key win.
pub fn add_img_data_urls(decoded: &mut DecodedAssets, tree: &Tree) {
    for img in html_elements(tree, "img") {
        let Some(src) = tree.get_attribute(img, "src") else {
            continue;
        };
        if decoded.contains_key(src) {
            continue;
        }
        if let Some(image) = layout_image::decode_data_url(src).and_then(|d| decode(&d.bytes)) {
            decoded.insert(src.to_string(), image);
        }
    }
}

/// The document's HTML elements named `name`, in document order. Template content is a
/// separate document fragment, not the page, so it is not walked.
pub(crate) fn html_elements(tree: &Tree, name: &str) -> Vec<Handle> {
    let mut found = Vec::new();
    let mut stack = vec![tree.root()];
    while let Some(h) = stack.pop() {
        let element =
            tree.node_type(h) == NodeType::Element && tree.namespace(h) == Namespace::Html;
        if element && tree.local_name(h) == Some(name) {
            found.push(h);
        }
        let children = tree.children(h).into_iter().rev();
        stack.extend(children.filter(|&c| tree.node_type(c) != NodeType::Fragment));
    }
    found
}

/// Replace each inline `<svg>` ELEMENT of the parsed document (the outermost one of a
/// nested set) with an `<img>` carrying the SVG's `class`/`style`/`width`/`height` (so CSS
/// sizing still applies) and a synthetic `src` key, returning the `(key, svg-source)` pairs
/// for the caller to decode into the asset map (see [`add_inline_svg_assets`]). The layout
/// engine has no inline-SVG painter, so a page's inline `<svg>` logos/icons (google.com
/// ships ~21) would render blank; routing them through the existing `<img>` + resvg path
/// paints them.
///
/// It works on the turbo-dom tree, never on source text (miaskiewicz/turbo-dom#25): only
/// what the HTML parser made an SVG `<svg>` element is converted, so a `<svg` inside an
/// attribute value (a raw-UTF-8 SVG `data:` URL), a `<script>` string or `<noscript>` raw
/// text is left alone, and the parser's own recovery (implied end tags, HTML elements
/// breaking out of foreign content) applies. Each source is the element re-serialized as
/// namespace-well-formed XML ([`svg_markup`]).
pub fn inline_svg_to_img(tree: &mut Tree) -> Vec<(String, String)> {
    let roots = svg_roots(tree);
    let mut assets = Vec::with_capacity(roots.len());
    for svg in roots {
        let key = format!("turbo-inline-svg-{}", assets.len());
        let source = svg_markup(tree, svg);
        let img = tree.create_element("img");
        tree.set_attribute(img, "src", &key);
        for name in ["class", "style", "width", "height"] {
            if let Some(value) = tree.get_attribute(svg, name).map(str::to_owned) {
                tree.set_attribute(img, name, &value);
            }
        }
        tree.replace_with(svg, &[img]);
        assets.push((key, source));
    }
    assets
}

/// Whether `h` is an SVG-namespace `<svg>` element.
fn is_svg_element(tree: &Tree, h: Handle) -> bool {
    tree.node_type(h) == NodeType::Element
        && tree.namespace(h) == Namespace::Svg
        && tree.local_name(h) == Some("svg")
}

/// The outermost `<svg>` elements of the document, in document order (a nested `<svg>`
/// belongs to its ancestor's source). Document-fragment children — `<template>` content
/// (not rendered) and shadow roots (the layout's document view skips them) — are not
/// visited.
fn svg_roots(tree: &Tree) -> Vec<Handle> {
    let mut roots = Vec::new();
    let mut stack = vec![tree.root()];
    while let Some(h) = stack.pop() {
        if is_svg_element(tree, h) {
            roots.push(h);
            continue;
        }
        let children = tree.children(h);
        stack.extend(
            children
                .into_iter()
                .rev()
                .filter(|&c| tree.node_type(c) != NodeType::Fragment),
        );
    }
    roots
}

/// The `<svg>` element `root` as a standalone XML document for the rasterizer: the root
/// declares the SVG and `XLink` namespaces (inline SVG in HTML needs neither); SVG-namespace
/// descendants are written with the parser's (case-adjusted) names; attributes keep their
/// `xlink:`/`xml:` prefixes; text is XML-escaped (HTML character references are already decoded).
/// What XML cannot carry and resvg would not paint is dropped: comments, non-SVG elements
/// (`<foreignObject>` HTML content), namespace declarations, and names that are not plain
/// XML names (e.g. an undeclared `inkscape:label`).
fn svg_markup(tree: &Tree, root: Handle) -> String {
    enum Step {
        Open(Handle),
        Close(Handle),
    }
    let mut out = String::new();
    let mut stack = vec![Step::Open(root)];
    while let Some(step) = stack.pop() {
        match step {
            Step::Close(h) => {
                out.push_str("</");
                out.push_str(tree.local_name(h).unwrap_or_default());
                out.push('>');
            }
            Step::Open(h) => match tree.node_type(h) {
                NodeType::Text => xml_escape(&tree.node_value(h).unwrap_or_default(), &mut out),
                NodeType::Element if h == root || is_xml_svg_element(tree, h) => {
                    open_tag(tree, h, h == root, &mut out);
                    stack.push(Step::Close(h));
                    let children = tree.children(h);
                    stack.extend(children.into_iter().rev().map(Step::Open));
                }
                _ => {}
            },
        }
    }
    out
}

/// Write `h`'s start tag (with the namespace declarations on the document `root`).
fn open_tag(tree: &Tree, h: Handle, root: bool, out: &mut String) {
    out.push('<');
    out.push_str(tree.local_name(h).unwrap_or("svg"));
    if root {
        out.push_str(
            r#" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink""#,
        );
    }
    tree.for_each_attr_full(h, |prefix, local, value| {
        let Some(prefix) = xml_attr_prefix(prefix, local) else {
            return;
        };
        out.push(' ');
        out.push_str(prefix);
        out.push_str(local);
        out.push_str("=\"");
        xml_escape(value, out);
        out.push('"');
    });
    out.push('>');
}

/// An SVG-namespace element whose local name is a plain XML name.
fn is_xml_svg_element(tree: &Tree, h: Handle) -> bool {
    tree.namespace(h) == Namespace::Svg && tree.local_name(h).is_some_and(is_xml_name)
}

/// The XML prefix (`""`, `"xlink:"`, `"xml:"`) an attribute is written with, or `None` to
/// drop it: namespace declarations (the root declares what the output uses) and names that
/// are not plain XML names.
fn xml_attr_prefix(prefix: &str, local: &str) -> Option<&'static str> {
    let prefix = match prefix {
        "" if local == "xmlns" => return None,
        "" => "",
        "xlink" => "xlink:",
        "xml" => "xml:",
        _ => return None,
    };
    is_xml_name(local).then_some(prefix)
}

/// An ASCII XML name without a colon: a letter or `_`, then letters, digits, `-`, `_`, `.`.
fn is_xml_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Append `s` XML-escaped (text and double-quoted attribute values alike), dropping the
/// characters XML 1.0 cannot carry (C0 controls other than tab/LF/CR, U+FFFE/U+FFFF).
fn xml_escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if c < ' ' || c == '\u{fffe}' || c == '\u{ffff}' => {}
            c => out.push(c),
        }
    }
}

/// Decode the `(key, svg-source)` pairs from [`inline_svg_to_img`] into `decoded`
/// (SVG → RGBA via resvg), so the synthetic `<img src=key>` boxes resolve + paint.
pub fn add_inline_svg_assets(decoded: &mut DecodedAssets, assets: &[(String, String)]) {
    for (key, src) in assets {
        if decoded.contains_key(key) {
            continue;
        }
        if let Some(img) = decode(src.as_bytes()) {
            decoded.insert(key.clone(), img);
        }
    }
}

/// Every `data:` URI inside a `url(...)` in `s`, extracted the way
/// turbo-dom-layout's `url_token` does (strip `url(` … `)`, strip surrounding
/// quotes, trim) so the key matches its resolver lookup. Quote-aware: a `;`/`,`/
/// `)` inside the payload (an inline `<svg>` data URI has all three) doesn't end
/// the token — only the matching unescaped closing quote does.
fn data_uri_url_tokens(s: &str) -> Vec<String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = s[i..].find("url(") {
        let mut j = i + rel + "url(".len();
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        let inner = if j < b.len() && (b[j] == b'"' || b[j] == b'\'') {
            let quote = b[j];
            j += 1;
            let start = j;
            // Closing quote = the same quote NOT escaped by a preceding backslash.
            while j < b.len() && !(b[j] == quote && b[j - 1] != b'\\') {
                j += 1;
            }
            &s[start..j.min(b.len())]
        } else {
            let start = j;
            while j < b.len() && b[j] != b')' {
                j += 1;
            }
            &s[start..j]
        };
        let name = inner.trim();
        if name.starts_with("data:") {
            out.push(name.to_string());
        }
        i = (i + rel + "url(".len()).max(j);
    }
    out
}

/// The raw bytes of a `data:` URI: base64-decoded for `;base64,`, else the plain
/// payload with the stylesheet's CSS string escaping undone (`\"` → `"`) and
/// percent-encoding decoded (an inline SVG uses `%23` for `#`). `None` if `uri`
/// isn't a data URI or its base64 is malformed.
fn decode_data_uri(uri: &str) -> Option<Vec<u8>> {
    let rest = uri.strip_prefix("data:")?;
    let comma = rest.find(',')?;
    let (meta, payload) = (&rest[..comma], &rest[comma + 1..]);
    if meta.split(';').any(|t| t.eq_ignore_ascii_case("base64")) {
        base64_decode(payload.trim())
    } else {
        Some(percent_decode(&css_unescape(payload)))
    }
}

/// Undo CSS string escaping: a backslash quotes the next character verbatim
/// (`\"` → `"`). Data-URI masks embed the SVG's `"` this way inside the `url()`
/// string. (Hex escapes `\41` don't appear in these payloads.)
fn css_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            _ => out.push(c),
        }
    }
    out
}

/// Percent-decode `%XX` byte escapes; any stray `%` not followed by two hex
/// digits is passed through verbatim.
fn percent_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex_nibble(b[i + 1]), hex_nibble(b[i + 2])) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Decode standard base64 (RFC 4648), ignoring `=` padding and whitespace. `None`
/// on any non-alphabet byte. The inverse of [`base64`].
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn sextet(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a' + 26)),
            b'0'..=b'9' => Some(u32::from(c - b'0' + 52)),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let (mut buf, mut bits) = (0u32, 0u32);
    for &c in s.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        buf = buf << 6 | sextet(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// Decode PNG (turbo-dom-layout's shared decoder), JPEG/GIF/WebP (the `image` crate)
/// or SVG (the shared rasterizer, 1x) into straight-alpha RGBA. `None` on an
/// unrecognized or corrupt image.
pub fn decode(bytes: &[u8]) -> Option<DecodedImage> {
    if is_svg(bytes) {
        return decode_svg(bytes);
    }
    let format = layout_image::sniff(bytes);
    let pixels = match format {
        Some(Format::Png) => layout_image::decode_png_rgba(bytes),
        Some(Format::Avif) => crate::avif::decode(bytes),
        Some(Format::Jxl) => crate::jxl::decode(bytes),
        // The entry a browser shows: a PNG entry through the shared PNG decoder (the
        // `image` crate's ICO decoder rejects non-RGBA PNG entries, which Chrome shows),
        // else the `image` crate's ICO decoder — format forced, as a cursor (resource
        // type 2) has the same layout but not its magic.
        Some(Format::Ico) => match layout_image::ico_entry(bytes) {
            Some(entry) if entry.png => layout_image::decode_png_rgba(entry.data),
            _ => image_rgba(image::load_from_memory_with_format(
                bytes,
                image::ImageFormat::Ico,
            )),
        },
        _ => image_rgba(image::load_from_memory(bytes)),
    };
    let Some(rgba) = pixels else {
        // An AVIF without the `avif` feature or a JPEG XL without `jxl` (or either on a
        // bitstream / codestream error) still lays out at the image's real size (the
        // shared header probe) and paints as the placeholder; any other undecodable image
        // is dropped.
        return match format {
            Some(Format::Avif | Format::Jxl) => size_only(bytes),
            _ => None,
        };
    };
    let probed = layout_image::probe(bytes);
    let size = raster_intrinsic(probed, &rgba);
    let mut decoded = with_intrinsic(rgba, size);
    // The EXIF orientation (JPEG APP1 / PNG eXIf) the shared probe reads; layout turns
    // the box and the painters turn the pixels (`ImagePlacement::orientation`).
    decoded.intrinsic.orientation = probed.map_or(Orientation::IDENTITY, |i| i.orientation);
    Some(decoded)
}

/// A probe-only entry: the image's layout size with no pixels (the painters draw the
/// placeholder for it).
fn size_only(bytes: &[u8]) -> Option<DecodedImage> {
    let intrinsic = layout_image::probe(bytes)?;
    Some(DecodedImage {
        width: 0,
        height: 0,
        rgba: Vec::new(),
        intrinsic: Intrinsic {
            has_alpha: true,
            ..intrinsic
        },
    })
}

/// An `image`-crate decode as straight RGBA8.
fn image_rgba(decoded: image::ImageResult<image::DynamicImage>) -> Option<Rgba8Image> {
    let rgba = decoded.ok()?.to_rgba8();
    Some(Rgba8Image {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

/// The layout size of a decoded raster image: the shared header probe of its bytes.
/// Should the probe ever fail or disagree with what the decoder produced (a file the
/// `image` crate decodes but the header reader can't size), the decoded frame size
/// wins — that is what layout sized before the probe was shared.
fn raster_intrinsic(probed: Option<Intrinsic>, rgba: &Rgba8Image) -> (u32, u32) {
    probed
        .map(|i| (i.width, i.height))
        .filter(|&size| size == (rgba.width, rgba.height))
        .unwrap_or((rgba.width, rgba.height))
}

fn with_intrinsic(rgba: Rgba8Image, (width, height): (u32, u32)) -> DecodedImage {
    DecodedImage {
        width: rgba.width,
        height: rgba.height,
        rgba: rgba.rgba,
        intrinsic: Intrinsic::new(width, height, true),
    }
}

/// A light structural SVG sniff (SVG has no byte magic): the first non-space chunk
/// looks like an `<svg` or XML/doctype preamble leading to one.
fn is_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    let lower = head.to_ascii_lowercase();
    let contains = |needle: &[u8]| lower.windows(needle.len()).any(|w| w == needle);
    contains(b"<svg")
        && (lower.starts_with(b"<svg") || contains(b"<?xml") || contains(b"<!doctype"))
}

/// Rasterize SVG bytes at their intrinsic CSS-pixel size (1x, an axis above 8192 px
/// rejected) through the shared rasterizer; laid out at that pixel size.
fn decode_svg(bytes: &[u8]) -> Option<DecodedImage> {
    let rgba = layout_svg::rasterize(bytes, 1.0, Oversize::Reject)?;
    let size = (rgba.width, rgba.height);
    Some(with_intrinsic(rgba, size))
}

impl DecodedImage {
    /// Re-encode as a PNG (for SVG data-URIs). `None` on an encode failure or a
    /// degenerate size.
    pub fn to_png(&self) -> Option<Vec<u8>> {
        let img = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())?;
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).ok()?;
        Some(out.into_inner())
    }

    /// This image as displayed under `orientation` (the stored pixels re-laid; a copy
    /// only when the orientation is not the identity).
    #[must_use]
    pub fn oriented(&self, orientation: Orientation) -> std::borrow::Cow<'_, DecodedImage> {
        if orientation.is_identity() {
            return std::borrow::Cow::Borrowed(self);
        }
        let (width, height, rgba) = orientation.apply_rgba(self.width, self.height, &self.rgba);
        std::borrow::Cow::Owned(DecodedImage {
            width,
            height,
            rgba,
            intrinsic: Intrinsic::new(width, height, true),
        })
    }

    /// Resize into a `w × h` premultiplied pixmap ready to `draw_pixmap` into the
    /// fragment box. `None` on a degenerate target size.
    pub fn scaled_pixmap(&self, w: u32, h: u32) -> Option<Pixmap> {
        if w == 0 || h == 0 || self.width == 0 || self.height == 0 {
            return None;
        }
        let src = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())?;
        let scaled = image::imageops::resize(&src, w, h, image::imageops::FilterType::Triangle);
        let mut pm = Pixmap::new(w, h)?;
        for (src, dst) in scaled.pixels().zip(pm.pixels_mut()) {
            let [r, g, b, a] = src.0;
            // premul(c,a) ≤ a always, so the premultiplied invariant holds.
            *dst = PremultipliedColorU8::from_rgba(premul(r, a), premul(g, a), premul(b, a), a)?;
        }
        Some(pm)
    }
}

/// `round(c * a / 255)` — a straight channel scaled into premultiplied space.
fn premul(c: u8, a: u8) -> u8 {
    ((u16::from(c) * u16::from(a) + 127) / 255) as u8
}

/// A base64 PNG `data:` URI for embedding a decoded image in an SVG `<image href>`.
/// Always PNG (re-encoded from RGBA), so any SVG viewer renders it regardless of
/// the source format. `None` if the image won't re-encode.
pub fn data_uri(img: &DecodedImage) -> Option<String> {
    let png = img.to_png()?;
    Some(format!("data:image/png;base64,{}", base64(&png)))
}

/// Standard base64 (RFC 4648, `+/`, `=` padding). Hand-rolled to keep the crate
/// dependency-light; only the SVG data-URI path uses it.
fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = u32::from(chunk[0]) << 16
            | u32::from(*chunk.get(1).unwrap_or(&0)) << 8
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        base64, base64_decode, css_unescape, data_uri_url_tokens, decode_data_uri, percent_decode,
    };
    use super::{decode, inline_svg_to_img, DecodedAssets};

    fn converted(html: &str) -> (String, Vec<(String, String)>) {
        let mut tree = turbo_dom_layout::parse_html_tree(html);
        let assets = inline_svg_to_img(&mut tree);
        let body =
            turbo_dom_layout::turbo_dom::rtdom::serialize::serialize_inner(&tree, tree.root());
        (body, assets)
    }

    #[test]
    fn inline_svg_becomes_img_carrying_size_and_class() {
        let html = r#"<div><svg class="ic" width="60" height="30" viewBox="0 0 60 30"><rect width="60" height="30"/></svg> after</div>"#;
        let (out, assets) = converted(html);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].0, "turbo-inline-svg-0");
        assert_eq!(
            assets[0].1,
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" class="ic" width="60" height="30" viewBox="0 0 60 30"><rect width="60" height="30"></rect></svg>"#
        );
        assert!(
            out.contains(r#"<div><img src="turbo-inline-svg-0" class="ic" width="60" height="30"> after</div>"#),
            "converted: {out}"
        );
    }

    #[test]
    fn inline_svg_nested_is_one_source_and_an_unclosed_svg_is_closed_by_the_parser() {
        // Nested <svg> — the OUTER element is the source, one asset.
        let (_, nested) = converted("<svg><svg></svg></svg>");
        assert_eq!(nested.len(), 1);
        assert!(
            nested[0].1.ends_with("><svg></svg></svg>"),
            "{}",
            nested[0].1
        );
        // Unclosed — the parser closes it at EOF (as Chrome does), so it converts.
        let (out, one) = converted("<svg width=\"1\"><rect/>");
        assert_eq!(one.len(), 1);
        assert!(
            one[0].1.ends_with(r#" width="1"><rect></rect></svg>"#),
            "{}",
            one[0].1
        );
        assert!(
            out.contains(r#"<img src="turbo-inline-svg-0" width="1">"#),
            "{out}"
        );
    }

    #[test]
    fn svg_text_outside_elements_is_not_converted() {
        let html = concat!(
            r#"<img src="data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg'></svg>">"#,
            r#"<script>var a = '<svg>', b = '</svg>';</script><div title="<svg>"></div>"#,
            r#"<noscript><svg></svg></noscript><template><svg></svg></template>"#,
        );
        let (out, none) = converted(html);
        assert!(none.is_empty(), "{none:?}");
        assert!(
            out.contains(
                r#"src="data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg'></svg>""#
            ),
            "{out}"
        );
    }

    #[test]
    fn svg_markup_is_namespace_well_formed_xml() {
        let html = concat!(
            r#"<svg xmlns="urn:wrong" xmlns:xlink="urn:wrong" xml:space="preserve" viewbox="0 0 1 1" "#,
            r#"@click="x" inkscape:label="l" data-a="&quot;&lt;&amp;"><lineargradient id="g"/>"#,
            r##"<use xlink:href="#g"/><!-- c --><title>a&nbsp;&lt;b&gt;&#1;</title>"##,
            r#"<foreignObject><div>html</div></foreignObject><x:y/></svg>"#,
        );
        let (_, assets) = converted(html);
        assert_eq!(
            assets[0].1,
            concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" "#,
                r#"xml:space="preserve" viewBox="0 0 1 1" data-a="&quot;&lt;&amp;">"#,
                r##"<linearGradient id="g"></linearGradient><use xlink:href="#g"></use>"##,
                "<title>a\u{a0}&lt;b&gt;</title><foreignObject></foreignObject></svg>",
            )
        );
        assert!(
            resvg::usvg::Tree::from_str(&assets[0].1, &resvg::usvg::Options::default()).is_ok()
        );
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_decode_is_the_inverse_of_encode() {
        for v in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"foobar",
            &[0u8, 255, 1, 254, 128],
            // `+` and `/`, the two non-alphanumeric sextets.
            &[0xfb, 0xff],
        ] {
            assert_eq!(base64_decode(&base64(v)).unwrap(), v, "roundtrip {v:?}");
        }
        // Padding and embedded whitespace are ignored; a non-alphabet byte fails.
        assert_eq!(base64_decode("Zm9v\nYmFy").unwrap(), b"foobar");
        assert_eq!(base64_decode("*bad*"), None);
    }

    #[test]
    fn percent_decode_and_css_unescape() {
        assert_eq!(percent_decode("fill=%23fff"), b"fill=#fff");
        assert_eq!(percent_decode("100%"), b"100%"); // stray % passed through
                                                     // Hex digits in either case; a `%` before non-hex digits is passed through.
        assert_eq!(percent_decode("%2f%2F%7e%zz%4"), b"//~%zz%4");
        assert_eq!(css_unescape(r#"<svg a=\"b\">"#), r#"<svg a="b">"#);
    }

    #[test]
    fn url_token_scan_takes_data_uris_whole_ignoring_inner_delimiters() {
        // A `;`, `,` and `)` all appear inside the payload; only the closing quote
        // ends the token. A non-`data:` url and a bare colour are skipped.
        let css = r#"a{mask-image:url("data:image/svg+xml;utf8,<svg viewBox=\"0 0 1,1\"><path d=\"M0 0z)\"/></svg>")}
                     b{background:#fff url(pic.png) no-repeat}
                     c{mask:url('data:image/png;base64,iVBORw0=')}"#;
        let toks = data_uri_url_tokens(css);
        assert_eq!(toks.len(), 2, "two data: urls, the .png skipped: {toks:?}");
        assert!(toks[0].starts_with("data:image/svg+xml;utf8,<svg"));
        assert!(toks[0].ends_with("</svg>"), "kept whole: {}", toks[0]);
        assert_eq!(toks[1], "data:image/png;base64,iVBORw0=");
        // Whitespace between `url(` and the quote is skipped (the token is the same).
        assert_eq!(
            data_uri_url_tokens(r#"a{mask:url(  'data:x,1') url( "data:y,2")}"#),
            ["data:x,1", "data:y,2"]
        );
    }

    #[test]
    fn decode_data_uri_handles_base64_and_plain_svg() {
        // base64 payload → raw bytes.
        let uri = format!("data:image/png;base64,{}", base64(b"hello"));
        assert_eq!(decode_data_uri(&uri).unwrap(), b"hello");
        // Plain SVG payload: CSS-unescaped + percent-decoded into valid XML bytes.
        let svg =
            decode_data_uri(r#"data:image/svg+xml;utf8,<svg fill=\"%23f00\" width=\"2\">x</svg>"#)
                .unwrap();
        assert_eq!(svg, br##"<svg fill="#f00" width="2">x</svg>"##);
    }

    #[test]
    fn add_data_uri_images_recovers_an_inline_svg_mask() {
        // A minimal inline-SVG mask url() in the CSS is decoded and keyed by the
        // exact resolver string (the unquoted url inner), ready for the paint pass.
        let key = r#"data:image/svg+xml;utf8,<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"><rect width=\"4\" height=\"4\" fill=\"%23000\"/></svg>"#;
        let css = format!(".i{{mask-image:url(\"{key}\")}}");
        let mut decoded: DecodedAssets = DecodedAssets::new();
        super::add_data_uri_images(&mut decoded, &css, "");
        let img = decoded
            .get(key)
            .expect("data-uri mask decoded under its key");
        assert_eq!((img.width, img.height), (4, 4));
    }

    use super::{add_img_data_urls, add_inline_svg_assets, AssetResolver, DecodedImage};
    use turbo_dom_layout::image::Intrinsic;
    use turbo_dom_layout::ImageResolver;

    const SQUARE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="3" height="2"/>"#;

    #[test]
    fn layout_sizes_through_the_resolver_and_never_gets_bytes() {
        let mut decoded = DecodedAssets::new();
        decoded.insert("a.svg".into(), decode(SQUARE.as_bytes()).unwrap());
        let resolver = AssetResolver(&decoded);
        assert_eq!(resolver.resolve("a.svg"), None);
        assert_eq!(
            resolver.intrinsic("a.svg"),
            Some(Intrinsic::new(3, 2, true))
        );
        assert_eq!(resolver.intrinsic("b.svg"), None);
    }

    #[test]
    fn img_data_urls_decode_only_src_bearing_data_images_not_already_supplied() {
        let tree = turbo_dom_layout::parse_html_tree(concat!(
            r#"<img alt="no src"><img src="b.png">"#,
            r#"<img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='5' height='4'/%3E">"#,
            r#"<img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='7' height='7'/%3E">"#,
            r#"<img src="data:,not-an-image">"#,
        ));
        let kept =
            "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='7' height='7'/%3E";
        let mut decoded = DecodedAssets::new();
        decoded.insert(kept.into(), decode(SQUARE.as_bytes()).unwrap());
        add_img_data_urls(&mut decoded, &tree);
        assert_eq!(decoded.len(), 2, "{:?}", decoded.keys().collect::<Vec<_>>());
        let added = &decoded["data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='5' height='4'/%3E"];
        assert_eq!((added.width, added.height), (5, 4));
        assert_eq!(
            (decoded[kept].width, decoded[kept].height),
            (3, 2),
            "caller bytes win"
        );
    }

    #[test]
    fn inline_svg_assets_keep_caller_bytes_and_skip_undecodable_sources() {
        let caller: DecodedImage = decode(SQUARE.as_bytes()).unwrap();
        let mut decoded = DecodedAssets::new();
        decoded.insert("turbo-inline-svg-0".into(), caller);
        let svg =
            |w: u32| format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="1"/>"#);
        add_inline_svg_assets(
            &mut decoded,
            &[
                ("turbo-inline-svg-0".into(), svg(9)),
                ("turbo-inline-svg-1".into(), svg(6)),
                ("turbo-inline-svg-2".into(), "<svg".into()),
            ],
        );
        assert_eq!(decoded["turbo-inline-svg-0"].width, 3);
        assert_eq!(decoded["turbo-inline-svg-1"].width, 6);
        assert!(!decoded.contains_key("turbo-inline-svg-2"));
    }

    #[test]
    fn svg_markup_keeps_xml_whitespace_and_drops_other_controls() {
        let (_, assets) = converted("<svg><text>a\tb\nc\rd\u{1}e\u{fffe}\u{ffff}f</text></svg>");
        assert!(assets[0].1.contains("<text>a\tb\nc\nd"), "{}", assets[0].1);
        assert!(assets[0].1.contains("ef</text>"), "{}", assets[0].1);
    }

    #[test]
    fn undecodable_avif_and_jpeg_xl_lay_out_at_their_size_without_pixels() {
        let fixtures = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../turbo-dom-layout/tests/fixtures/images"
        );
        // A truncated JPEG XL keeps its size header; an AVIF whose AV1 stream starts with
        // an OBU header that has its forbidden bit set keeps its `ispe`.
        let jxl = std::fs::read(format!("{fixtures}/jxl/modular_rle_fast_path.jxl")).unwrap();
        let mut avif = std::fs::read(format!("{fixtures}/formats/q.avif")).unwrap();
        let stream = turbo_dom_layout::image::avif_payload(&avif).unwrap().color;
        let at = avif
            .windows(stream.len())
            .position(|w| w == &stream[..])
            .unwrap();
        avif[at] = 0xff;
        for (label, bytes) in [("jxl", &jxl[..12]), ("avif", &avif[..])] {
            let probed = turbo_dom_layout::image::probe(bytes).expect(label);
            let img = decode(bytes).expect(label);
            assert_eq!(
                (img.width, img.height, img.rgba.len()),
                (0, 0, 0),
                "{label}"
            );
            assert_eq!(
                (img.intrinsic.width, img.intrinsic.height),
                (probed.width, probed.height),
                "{label}"
            );
            assert!(img.intrinsic.has_alpha, "{label}");
        }
        // Other undecodable images are dropped.
        assert!(decode(b"\x89PNG\r\n\x1a\n broken").is_none());
        assert!(decode(b"GIF89a").is_none());
    }

    #[test]
    fn decodes_a_simple_svg_to_rgba() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="6"><rect width="10" height="6" fill="#f00"/></svg>"##;
        let img = decode(svg).expect("svg decodes");
        assert_eq!((img.width, img.height), (10, 6));
        assert_eq!(img.rgba.len(), 10 * 6 * 4);
        // Top-left pixel is red, fully opaque.
        assert_eq!(&img.rgba[0..4], &[255, 0, 0, 255]);
    }
}

/// The S5 differential gate for the shared image layer: every decode and size the
/// screenshot painter gets from turbo-dom-layout's image layer equals the
/// pre-consolidation path byte for byte — `image::load_from_memory(..).to_rgba8()`
/// (and the old 1x resvg decode for SVG) for the pixels, layout's probe of the old
/// PNG re-encode for the size.
#[cfg(test)]
mod shared_layer_tests {
    use super::{decode, is_svg, DecodedImage};
    use image::{ImageBuffer, Luma, LumaA, Rgb, Rgba};
    use std::path::{Path, PathBuf};
    use turbo_dom_layout::image::{probe, Intrinsic};

    /// The pre-S5 raster decode: the `image` crate for every raster format, resvg at
    /// 1x for SVG.
    fn old_decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
        if is_svg(bytes) {
            return old_decode_svg(bytes);
        }
        let rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
        Some((rgba.width(), rgba.height(), rgba.into_raw()))
    }

    /// The pre-S5 `decode_svg` (turbo-surf-raster 0.5.5), verbatim.
    fn old_decode_svg(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
        use resvg::{tiny_skia as rsk, usvg};
        let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()?;
        let size = tree.size();
        let (w, h) = (size.width().ceil() as u32, size.height().ceil() as u32);
        if w == 0 || h == 0 || w > 8192 || h > 8192 {
            return None;
        }
        let mut pm = rsk::Pixmap::new(w, h)?;
        resvg::render(&tree, rsk::Transform::identity(), &mut pm.as_mut());
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for px in pm.pixels() {
            let c = px.demultiply();
            rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
        Some((w, h, rgba))
    }

    /// What layout sized an image with before S5: its probe of the PNG re-encode.
    fn old_intrinsic(width: u32, height: u32, rgba: Vec<u8>) -> Option<Intrinsic> {
        let img = DecodedImage {
            width,
            height,
            rgba,
            intrinsic: Intrinsic::new(0, 0, false),
        };
        probe(&img.to_png()?)
    }

    /// Assert old == new for one input; returns whether it decoded.
    fn assert_same(label: &str, bytes: &[u8]) -> bool {
        let old = old_decode(bytes);
        let new = decode(bytes);
        match (old, new) {
            (None, None) => false,
            (Some((w, h, rgba)), Some(new)) => {
                assert_eq!((new.width, new.height), (w, h), "{label}: size");
                assert!(new.rgba == rgba, "{label}: RGBA bytes differ");
                let old_size = old_intrinsic(w, h, rgba).expect("re-encode probes");
                // The size + alpha layout used before S5 (the EXIF orientation, read since
                // P5b turbo-html2pdf#30, is not part of the old re-encode probe).
                assert_eq!(
                    Intrinsic {
                        orientation: old_size.orientation,
                        ..new.intrinsic
                    },
                    old_size,
                    "{label}: layout intrinsic"
                );
                assert_eq!(
                    probe(bytes)
                        .map(|i| (i.width, i.height))
                        .filter(|_| !is_svg(bytes)),
                    (!is_svg(bytes)).then_some((w, h)),
                    "{label}: the shared header probe sizes it directly"
                );
                true
            }
            // AVIF decodes since P5b (turbo-surf#30) under the `avif` feature; the old
            // decoder had no AVIF support.
            (None, Some(_))
                if turbo_dom_layout::image::sniff(bytes)
                    == Some(turbo_dom_layout::image::Format::Avif) =>
            {
                false
            }
            (old, new) => panic!(
                "{label}: decodes differ (old {}, new {})",
                old.is_some(),
                new.is_some()
            ),
        }
    }

    fn matrix_dirs() -> Vec<PathBuf> {
        let parity = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/parity/inputs");
        vec![
            parity.join("image-matrix/images"),
            parity.join("extra/images"),
        ]
    }

    #[test]
    fn image_matrix_decodes_and_sizes_byte_identically() {
        let dirs = matrix_dirs();
        if !dirs[0].is_dir() {
            eprintln!("SKIP: no tools/parity inputs (packaged crate)");
            return;
        }
        let (mut files, mut decoded) = (0, 0);
        for dir in dirs {
            let mut entries: Vec<_> = std::fs::read_dir(&dir)
                .expect("matrix dir")
                .map(|e| e.expect("entry").path())
                .collect();
            entries.sort();
            for path in entries {
                let bytes = std::fs::read(&path).expect("read image");
                files += 1;
                decoded += usize::from(assert_same(&path.display().to_string(), &bytes));
            }
        }
        // 19 matrix images + 4 extra (AVIF and the corrupt JPEG do not decode).
        assert_eq!((files, decoded), (23, 20));
    }

    fn png<P: image::PixelWithColorType>(img: &ImageBuffer<P, Vec<P::Subpixel>>) -> Vec<u8>
    where
        [P::Subpixel]: image::EncodableLayout,
    {
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png)
            .expect("encode test png");
        out.into_inner()
    }

    #[test]
    fn every_sample_value_converts_like_to_rgba8() {
        // 256x256 16-bit images carrying every u16 sample value in every channel
        // layout, and 16x16 8-bit ones carrying every u8: the RGBA derived from the
        // shared decode must equal `to_rgba8` for each (incl. the gray f32 path).
        let v16 = |x: u32, y: u32| (y * 256 + x) as u16;
        let v8 = |x: u32, y: u32| (y * 16 + x) as u8;
        let r16 = |x: u32, y: u32| v16(x, y).rotate_left(5);
        let cases: Vec<(&str, Vec<u8>)> = vec![
            (
                "L16",
                png(&ImageBuffer::from_fn(256, 256, |x, y| Luma([v16(x, y)]))),
            ),
            (
                "La16",
                png(&ImageBuffer::from_fn(256, 256, |x, y| {
                    LumaA([v16(x, y), r16(x, y)])
                })),
            ),
            (
                "Rgb16",
                png(&ImageBuffer::from_fn(256, 256, |x, y| {
                    Rgb([v16(x, y), r16(x, y), !v16(x, y)])
                })),
            ),
            (
                "Rgba16",
                png(&ImageBuffer::from_fn(256, 256, |x, y| {
                    Rgba([v16(x, y), r16(x, y), !v16(x, y), r16(y, x)])
                })),
            ),
            (
                "L8",
                png(&ImageBuffer::from_fn(16, 16, |x, y| Luma([v8(x, y)]))),
            ),
            (
                "La8",
                png(&ImageBuffer::from_fn(16, 16, |x, y| {
                    LumaA([v8(x, y), v8(y, x)])
                })),
            ),
            (
                "Rgb8",
                png(&ImageBuffer::from_fn(16, 16, |x, y| {
                    Rgb([v8(x, y), v8(y, x), !v8(x, y)])
                })),
            ),
            (
                "Rgba8",
                png(&ImageBuffer::from_fn(16, 16, |x, y| {
                    Rgba([v8(x, y), v8(y, x), !v8(x, y), v8(x, y) ^ 0x5a])
                })),
            ),
        ];
        for (label, bytes) in cases {
            assert!(assert_same(label, &bytes), "{label} decodes");
        }
    }
}

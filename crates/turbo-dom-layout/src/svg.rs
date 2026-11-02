//! Vector (SVG) image rasterization (Phase 15b, §7.4; consolidation S5): the ONE SVG
//! rasterizer of the shared image layer. Compiled under `svg-rasterize` (which the
//! `svg` feature implies). [`rasterize`] turns `image/svg+xml` bytes into a
//! deterministic straight-alpha RGBA buffer at a caller-chosen oversample:
//!
//! - the PDF pipeline ([`probe_svg`] / [`decode_svg`], routed from
//!   [`crate::image::probe`]/[`crate::image::decode`] under `svg`) rasterizes at
//!   [`PDF_SVG_SCALE`] (2x) and lays the box out at that 2x pixel size, clamping an
//!   axis above [`MAX_DIM`] ([`Oversize::Clamp`]); the RGBA is split into an RGB
//!   payload + alpha plane for the existing image `XObject`/`SMask` emitter;
//! - turbo-dom-raster rasterizes at 1x and lays out at `ceil(css size)`, rejecting an
//!   SVG above [`MAX_DIM`] ([`Oversize::Reject`]).
//!
//! The intrinsic CSS size ([`css_size`]) is separate from the oversampled pixel size
//! ([`pixel_size`]), so both behaviours are kept exactly.
//!
//! **Hostile input.** An SVG is always an SVG *image* here: its `<image>` hrefs load
//! `data:` URLs only, never a file or any other scheme (Chrome 156; turbo-dom#196),
//! element nesting plus `href`/`url()` reference chains are capped at [`MAX_DEPTH`]
//! before any recursive parser runs, and parse/render run on a fixed large stack
//! (turbo-dom#197); a [`Oversize::Clamp`]ed raster holds the whole image scaled down
//! (turbo-dom#198).
//!
//! **Determinism (§0.2).** `usvg::Options::default()` is used apart from the image
//! resolver above: the font
//! database is left **empty** — no system-font lookup, no filesystem probe, no
//! clock, no threads (`resvg`/`tiny-skia` rasterize single-threaded). Identical
//! SVG bytes therefore rasterize to byte-identical pixels on every host. Text in
//! the SVG needs no glyphs to stay deterministic: with an empty `fontdb` glyph
//! runs simply contribute nothing, the same way an unresolved `<img>` paints
//! nothing. Shapes, paths, gradients, clips and opacity all render fully.

use resvg::tiny_skia;
use resvg::usvg;

use crate::image::{ColorSpace, Intrinsic, Payload, RasterImage, Rgba8Image};

/// The PDF pipeline's oversampling factor: SVGs rasterize at this multiple of their
/// intrinsic CSS-pixel size so the embedded raster stays sharp under PDF zoom (the
/// layout size caps in `imgsize` scale the result to the box). A fixed constant
/// keeps output byte-deterministic.
pub const PDF_SVG_SCALE: f32 = 2.0;

/// Hard cap on either raster axis (px); a normal logo/icon is far below this.
pub const MAX_DIM: u32 = 8192;

/// What [`pixel_size`] does with an axis above [`MAX_DIM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Oversize {
    /// Clamp the axis to [`MAX_DIM`] (the PDF pipeline).
    Clamp,
    /// Reject the SVG (turbo-dom-raster).
    Reject,
}

/// Whether `bytes` look like an SVG document (XML with an `<svg` element) — the PDF
/// pipeline's sniff (turbo-dom-raster keeps its own decode dispatch).
///
/// SVG has no single magic-number signature, so this sniffs the leading,
/// possibly-BOM/whitespace-prefixed bytes for the XML/SVG markers the way
/// browsers do. Kept conservative: a false negative just means the bytes fall
/// through to the raster sniffer (and then paint nothing), never a panic.
#[must_use]
pub fn is_svg(bytes: &[u8]) -> bool {
    // Skip a UTF-8 BOM and leading ASCII whitespace.
    let mut rest = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    while let [first, tail @ ..] = rest {
        if first.is_ascii_whitespace() {
            rest = tail;
        } else {
            break;
        }
    }
    // Scan a bounded prefix for the `<svg` tag or an XML declaration that an SVG
    // document opens with. Bounded so a huge non-SVG blob is rejected cheaply.
    let head = &rest[..rest.len().min(512)];
    contains_ci(head, b"<svg")
        || (head.starts_with(b"<?xml") && contains_ci(rest, b"<svg"))
        // A prefixed root (`<s:svg xmlns:s="…/svg">`), which Chrome renders too.
        || (contains_ci(head, b":svg") && contains_ci(rest, SVG_NS.as_bytes()))
}

/// Case-insensitive substring search over a byte slice (ASCII), used for the SVG
/// tag sniff where the element name may be `<svg` or `<SVG`.
fn contains_ci(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

/// The SVG namespace an SVG image's root element must be in.
const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// Parse SVG bytes into a usvg tree with the deterministic default options
/// (empty font database: no system-font lookup, no I/O). `None` on malformed
/// input and on documents Blink does not render as an SVG image
/// (miaskiewicz/turbo-dom#59, Chrome 156 control
/// `scripts/image-truncation-chrome-control.mjs`): the root element must be an `<svg>`
/// in the SVG namespace (XML: case-sensitive, any prefix), and the bytes are XML text —
/// a gzip (`svgz`) stream is not inflated.
#[must_use]
pub fn parse(bytes: &[u8]) -> Option<usvg::Tree> {
    // `Options::default()` ships an empty `fontdb`; we deliberately never call
    // `load_system_fonts`, so the parse depends only on the input bytes. (The same
    // parse `usvg::Tree::from_data` runs on XML text, with the root checked first.)
    // roxmltree's tokenizer recurses per element too, so the XML parse runs on the
    // SVG stack as well.
    on_svg_stack(|| {
        let document = parse_xml(bytes)?;
        usvg::Tree::from_xmltree(&document, &image_options()).ok()
    })
}

/// The stack usvg's parse and resvg's render get (turbo-dom#197): both recurse once
/// per nesting level, which [`MAX_DEPTH`] bounds, so a fixed, generous stack makes
/// the depth limit safe whatever stack the caller runs on (a napi or Python worker
/// thread may have far less than 8 MiB). Only address space is reserved up front.
const SVG_STACK: usize = 256 << 20;

/// Run `f` on a scoped thread with [`SVG_STACK`] of stack; inline where threads
/// cannot be spawned (a `wasm32` build). A panic in `f` resumes on the caller.
fn on_svg_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    let job = std::sync::Mutex::new(Some(f));
    let spawned = std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .stack_size(SVG_STACK)
            .spawn_scoped(scope, || run(&job))
            .ok()?;
        Some(
            handle
                .join()
                .unwrap_or_else(|p| std::panic::resume_unwind(p)),
        )
    });
    match spawned.flatten() {
        Some(out) => out,
        None => run(&job).expect("the job runs inline when no thread could start"),
    }
}

/// Take the job out of its slot and run it (`None` if it already ran).
fn run<T, F: FnOnce() -> T>(job: &std::sync::Mutex<Option<F>>) -> Option<T> {
    let f = job.lock().map_or(None, |mut f| f.take());
    f.map(|f| f())
}

/// usvg options for an SVG used as an image: the defaults (empty `fontdb`), with an
/// image resolver that loads `data:` URLs only (turbo-dom#196). Chrome 156 never
/// fetches a subresource — file, http or any other scheme — for an SVG image, so a
/// non-`data:` `href` resolves to nothing by construction: usvg's default string
/// resolver (which reads the filesystem) is never installed.
fn image_options() -> usvg::Options<'static> {
    usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(resolve_data_image),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    }
}

/// A `data:` image inside an SVG image: rasters as usvg decodes them; a nested SVG
/// only if it passes the same root and depth checks as [`parse`] (usvg parses it
/// itself, past our guard, and with no images of its own).
fn resolve_data_image(
    mime: &str,
    data: std::sync::Arc<Vec<u8>>,
    options: &usvg::Options,
) -> Option<usvg::ImageKind> {
    // usvg treats image/svg+xml, and text/plain that is not a PNG or JPEG, as SVG.
    let raster = [&b"\x89PNG"[..], b"\xFF\xD8", b"GIF8", b"RIFF"]
        .iter()
        .any(|magic| data.starts_with(magic));
    if !raster || mime == "image/svg+xml" {
        parse_xml(&data)?;
    }
    usvg::ImageHrefResolver::default_data_resolver()(mime, data, options)
}

/// The XML document of an SVG image: UTF-8 XML (DTDs allowed, as usvg parses) whose
/// root element is an SVG-namespace `<svg>`.
fn parse_xml(bytes: &[u8]) -> Option<usvg::roxmltree::Document<'_>> {
    let text = std::str::from_utf8(bytes).ok()?;
    // roxmltree's tokenizer recurses once per open element, so the depth bound is
    // checked on the raw text before it runs (turbo-dom#197).
    if markup_depth(bytes) > MAX_DEPTH {
        return None;
    }
    let options = usvg::roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let document = usvg::roxmltree::Document::parse_with_options(text, options).ok()?;
    let root = document.root_element().tag_name();
    let svg_root = root.namespace() == Some(SVG_NS) && root.name() == "svg";
    (svg_root && within_depth(&document)).then_some(document)
}

/// The deepest chain of elements usvg/resvg may recurse through (turbo-dom#197).
/// usvg itself rejects a tree deeper than 1024 elements, but only after recursing
/// that far in its own parse; deeper input overflowed the stack. Past this the
/// image is broken (Chrome 156 draws a 5000-deep document as a broken image too,
/// though it still renders 3000 levels).
pub const MAX_DEPTH: usize = 1024;

/// An upper bound on the element nesting roxmltree reaches parsing `bytes`, found
/// by a flat scan: the deepest `<tag` nesting in the document, plus ten times the
/// number of element starts inside a DOCTYPE internal subset (an entity may expand to
/// markup, and roxmltree allows entity references ten deep).
fn markup_depth(bytes: &[u8]) -> usize {
    let (mut i, mut depth, mut deepest, mut dtd_opens) = (0, 0usize, 0, 0);
    while let Some(off) = bytes[i..].iter().position(|&b| b == b'<') {
        i += off;
        let rest = &bytes[i..];
        let skip_to = |end: &[u8]| find(rest, end).map_or(bytes.len(), |e| i + e + end.len());
        if rest.starts_with(b"<!--") {
            i = skip_to(b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            i = skip_to(b"]]>");
        } else if rest.starts_with(b"<?") {
            i = skip_to(b"?>");
        } else if rest.starts_with(b"<!") {
            let internal =
                rest.iter().position(|&b| b == b'[') < rest.iter().position(|&b| b == b'>');
            let end = if internal {
                skip_to(b"]>")
            } else {
                skip_to(b">")
            };
            dtd_opens += bytes[i + 1..end].windows(2).filter(|w| is_open(w)).count();
            i = end;
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            i = skip_to(b">");
        } else {
            let end = tag_end(rest);
            if rest[..end].ends_with(b"/") {
                deepest = deepest.max(depth + 1);
            } else {
                depth += 1;
                deepest = deepest.max(depth);
            }
            i += end + 1;
        }
        if i >= bytes.len() {
            break;
        }
    }
    deepest.saturating_add(dtd_opens.saturating_mul(10))
}

/// Whether a two-byte window starts an element tag (`<` then a name character).
fn is_open(w: &[u8]) -> bool {
    w[0] == b'<' && !matches!(w[1], b'/' | b'!' | b'?' | b' ' | b'\t' | b'\n' | b'\r')
}

/// The index of the `>` closing the start tag at the head of `tag` (quoted
/// attribute values may hold `>`), or the slice length if it never closes.
fn tag_end(tag: &[u8]) -> usize {
    let mut quote = None;
    for (k, &b) in tag.iter().enumerate() {
        match quote {
            Some(q) if b == q => quote = None,
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'>' => return k,
            _ => {}
        }
    }
    tag.len()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Whether every chain of element nesting **and** `href="#id"` / `url(#id)`
/// references (`<use>`, clip paths, masks, patterns, filters, markers, gradients —
/// each one a recursion in usvg) is at most [`MAX_DEPTH`] long. Iterative (an
/// explicit stack), so the check itself never recurses on input depth; a cyclic
/// reference adds nothing (usvg drops it).
fn within_depth(document: &usvg::roxmltree::Document<'_>) -> bool {
    use usvg::roxmltree::Node;
    const UNSEEN: usize = usize::MAX;
    const OPEN: usize = usize::MAX - 1;
    let ids: std::collections::HashMap<&str, Node<'_, '_>> = document
        .descendants()
        .filter_map(|n| Some((n.attribute("id")?, n)))
        .collect();
    let index = |n: Node<'_, '_>| n.id().get_usize();
    let mut depth = vec![UNSEEN; document.descendants().count() + 1];
    // (node, its successors, next successor, deepest successor so far)
    let mut stack: Vec<(Node<'_, '_>, Vec<Node<'_, '_>>, usize, usize)> = Vec::new();
    let root = document.root_element();
    depth[index(root)] = OPEN;
    stack.push((root, successors(root, &ids), 0, 0));
    while let Some(top) = stack.last_mut() {
        if let Some(&next) = top.1.get(top.2) {
            top.2 += 1;
            match depth[index(next)] {
                OPEN => {}
                UNSEEN => {
                    if stack.len() >= MAX_DEPTH {
                        return false;
                    }
                    depth[index(next)] = OPEN;
                    stack.push((next, successors(next, &ids), 0, 0));
                }
                d => top.3 = top.3.max(d),
            }
            continue;
        }
        let (node, _, _, deepest) = stack.pop().expect("non-empty");
        depth[index(node)] = deepest + 1;
        if let Some(parent) = stack.last_mut() {
            parent.3 = parent.3.max(deepest + 1);
        }
    }
    true
}

/// An element's child elements and the elements its attributes reference.
fn successors<'a, 'i>(
    node: usvg::roxmltree::Node<'a, 'i>,
    ids: &std::collections::HashMap<&str, usvg::roxmltree::Node<'a, 'i>>,
) -> Vec<usvg::roxmltree::Node<'a, 'i>> {
    let mut out: Vec<_> = node
        .children()
        .filter(usvg::roxmltree::Node::is_element)
        .collect();
    for attr in node.attributes() {
        let value = attr.value();
        if let Some(id) = value.trim().strip_prefix('#') {
            out.extend(ids.get(id.trim()));
        }
        for part in value.split("url(").skip(1) {
            let id = part.trim_start().trim_start_matches(['"', '\'']);
            if let Some(id) = id.strip_prefix('#') {
                let end = id.find([')', '"', '\'']).unwrap_or(id.len());
                out.extend(ids.get(id[..end].trim()));
            }
        }
    }
    out
}

/// The intrinsic CSS-pixel size of a parsed SVG (its root `width`/`height`, or the
/// `viewBox`), before any oversampling.
#[must_use]
pub fn css_size(tree: &usvg::Tree) -> (f32, f32) {
    let size = tree.size();
    (size.width(), size.height())
}

/// The raster dimensions of an SVG of CSS-pixel size `css` oversampled by `scale`,
/// or `None` if either axis is degenerate (non-finite, ≤ 0, or rounds to 0) or, with
/// [`Oversize::Reject`], above [`MAX_DIM`].
#[must_use]
pub fn pixel_size(css: (f32, f32), scale: f32, oversize: Oversize) -> Option<(u32, u32)> {
    let axis = |len: f32| match oversize {
        Oversize::Clamp => clamped_axis(len, scale),
        Oversize::Reject => bounded_axis(len, scale),
    };
    Some((axis(css.0)?, axis(css.1)?))
}

/// One clamped axis: a finite, positive CSS-pixel length scaled, ceiled and clamped
/// to [`MAX_DIM`]. `None` for a degenerate length.
fn clamped_axis(len: f32, scale: f32) -> Option<u32> {
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    let px = (len * scale).ceil().min(MAX_DIM as f32) as u32;
    (px > 0).then_some(px)
}

/// One bounded axis: scaled and ceiled; `None` if it rounds to 0 (degenerate) or
/// exceeds [`MAX_DIM`].
fn bounded_axis(len: f32, scale: f32) -> Option<u32> {
    let px = (len * scale).ceil() as u32;
    (px != 0 && px <= MAX_DIM).then_some(px)
}

/// Rasterize SVG bytes at `scale`× their CSS-pixel size into straight-alpha RGBA,
/// or `None` if the bytes are not a parseable SVG or the size is degenerate /
/// rejected by `oversize`.
#[must_use]
pub fn rasterize(bytes: &[u8], scale: f32, oversize: Oversize) -> Option<Rgba8Image> {
    let tree = parse(bytes)?;
    let (width, height) = pixel_size(css_size(&tree), scale, oversize)?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    // Scale the unit SVG up to the oversampled buffer, so the transform (and thus
    // the output) depends only on `scale` and the input. A clamped axis scales the
    // whole image down into the clamped raster instead (Chrome draws all of it into
    // the box; turbo-dom#198).
    let (css_w, css_h) = css_size(&tree);
    let fit = |px: u32, css: f32| {
        if (css * scale).ceil() > MAX_DIM as f32 {
            px as f32 / css
        } else {
            scale
        }
    };
    let transform = tiny_skia::Transform::from_scale(fit(width, css_w), fit(height, css_h));
    on_svg_stack(|| resvg::render(&tree, transform, &mut pixmap.as_mut()));
    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for px in pixmap.pixels() {
        // `demultiply` recovers straight (non-premultiplied) colour.
        let c = px.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Some(Rgba8Image {
        width,
        height,
        rgba,
    })
}

/// The natural size a browser reports for an SVG `<img>` (`naturalWidth`/`naturalHeight`
/// before rounding — Chrome 156 rounds to the nearest integer), from the root element's
/// `width`/`height` (absolute lengths: plain numbers or `px`) and `viewBox` ratio:
/// both lengths as given; one length, the other from the ratio (else the 300x150
/// default object size's axis); neither, the largest box of the ratio inside 300x150
/// (else 300x150). `None` if the bytes are not a parseable SVG. turbo-surf's live image
/// loads size SVGs with this (see [`crate::image::probe_natural`]); the screenshot
/// painter keeps laying SVGs out at [`css_size`].
#[must_use]
pub fn natural_size(bytes: &[u8]) -> Option<(f32, f32)> {
    parse(bytes)?;
    on_svg_stack(|| {
        let document = parse_xml(bytes)?;
        let root = document.root_element();
        let (width, height) = (svg_length(root, "width"), svg_length(root, "height"));
        let ratio = view_box_ratio(root).or_else(|| width.zip(height).map(|(w, h)| w / h));
        Some(natural_from(width, height, ratio))
    })
}

/// The natural size from the root's lengths and ratio (the 300x150 default object size
/// fills what is missing — "contain" when only the ratio is known).
fn natural_from(width: Option<f32>, height: Option<f32>, ratio: Option<f32>) -> (f32, f32) {
    match (width, height, ratio) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, r) => (w, r.map_or(150.0, |r| w / r)),
        (None, Some(h), r) => (r.map_or(300.0, |r| h * r), h),
        (None, None, Some(r)) => contain_default(r),
        (None, None, None) => (300.0, 150.0),
    }
}

/// The largest box of aspect ratio `r` inside the 300x150 default object size.
fn contain_default(r: f32) -> (f32, f32) {
    if 300.0 / r <= 150.0 {
        (300.0, 300.0 / r)
    } else {
        (150.0 * r, 150.0)
    }
}

/// An absolute length attribute of the root element (a plain number or `px`), if positive.
fn svg_length(root: usvg::roxmltree::Node<'_, '_>, name: &str) -> Option<f32> {
    let v = root.attribute(name)?.trim();
    let v = v.strip_suffix("px").unwrap_or(v).trim();
    v.parse::<f32>().ok().filter(|n| n.is_finite() && *n > 0.0)
}

/// The width/height ratio of the root's `viewBox`, if it is four numbers with a
/// positive size.
fn view_box_ratio(root: usvg::roxmltree::Node<'_, '_>) -> Option<f32> {
    let n: Vec<f32> = root
        .attribute("viewBox")?
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect();
    (n.len() == 4 && n[2] > 0.0 && n[3] > 0.0).then(|| n[2] / n[3])
}

/// The PDF pipeline's intrinsic size of an SVG: its [`PDF_SVG_SCALE`] pixel size
/// (the box is laid out at it). Reports `has_alpha = true`: a rasterized SVG always
/// carries an alpha plane (its background is transparent), so the emitter writes an
/// `SMask`.
#[must_use]
pub fn probe_svg(bytes: &[u8]) -> Option<Intrinsic> {
    let tree = parse(bytes)?;
    let (width, height) = pixel_size(css_size(&tree), PDF_SVG_SCALE, Oversize::Clamp)?;
    Some(Intrinsic::new(width, height, true))
}

/// Rasterize SVG bytes into an embeddable [`RasterImage`] (PDF pipeline, 2x): RGB
/// samples plus a straight-alpha plane (→ `SMask`), matching what the PNG path
/// produces for an RGBA source so the XObject/SMask emitter handles both
/// identically. `None` if the bytes are not a parseable SVG or its size is
/// degenerate.
#[must_use]
pub fn decode_svg(bytes: &[u8]) -> Option<RasterImage> {
    let image = rasterize(bytes, PDF_SVG_SCALE, Oversize::Clamp)?;
    let pixel_count = (image.width as usize) * (image.height as usize);
    let mut samples = Vec::with_capacity(pixel_count * 3);
    let mut alpha = Vec::with_capacity(pixel_count);
    for &[r, g, b, a] in image.rgba.as_chunks::<4>().0 {
        samples.extend_from_slice(&[r, g, b]);
        alpha.push(a);
    }
    Some(RasterImage {
        width: image.width,
        height: image.height,
        payload: Payload::Raw {
            samples,
            color: ColorSpace::Rgb,
        },
        alpha: Some(alpha),
    })
}

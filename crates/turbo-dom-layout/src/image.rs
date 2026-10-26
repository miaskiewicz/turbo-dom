//! The shared image layer (§7.4, Phase 9b; consolidation S5): the caller-supplied
//! [`ImageResolver`], the one header **size probe** every format goes through
//! ([`probe`]), the one **PNG decode** both painters share ([`PngFrame`]: native
//! samples for the PDF emitter, straight RGBA for turbo-dom-raster), and, behind
//! `svg-rasterize`, the one **SVG rasterizer** ([`crate::svg::rasterize`]).
//!
//! **No I/O (§0.2).** This module never touches the network or the filesystem.
//! Every image is named in the template (`<img src>` / `background-image`) and
//! the bytes for that name are produced by a caller-supplied resolver. A render
//! with no resolver simply paints no images (the [`NoImages`] default).
//!
//! **Layout vs. emit.** Layout needs only the *intrinsic* pixel size to size the
//! box and apply the overflow caps; the emitter needs the full pixel payload. The
//! same resolver answers both: [`ImageResolver::intrinsic`] (by default [`probe`] of
//! the resolved bytes) reads the header for `(w, h)` cheaply, [`decode`] produces
//! the embeddable [`RasterImage`]. PNG is decoded to 8-bit RGB(A) (alpha split off
//! as an `SMask`); JPEG is passed through verbatim as `DCTDecode` (§7.4: "JPEG passed
//! through where possible"). turbo-dom-raster decodes its images itself (the
//! `image` crate for JPEG/GIF/WebP, [`PngFrame::to_rgba8`] for PNG, the shared SVG
//! rasterizer) and answers [`ImageResolver::intrinsic`] from that decode, so it no
//! longer re-encodes every image to PNG just to be sized.
//!
//! SVG support rides behind the off-by-default `svg` feature (Phase 15b). When
//! enabled, the gated arms in [`probe`]/[`decode`] (and the [`is_svg`] sniff)
//! rasterize `image/svg+xml` via `resvg`/`usvg` into a straight-alpha RGBA buffer
//! and slot a [`RasterImage`] in alongside the raster ones — the `XObject` path
//! already embeds RGBA via the alpha `SMask`, so no new emit code is needed (see
//! [`crate::svg`]). With the feature off these arms are not compiled, so the
//! default decode path is byte-for-byte unchanged. (`svg-rasterize` alone compiles
//! the rasterizer without routing [`probe`]/[`decode`] through it — what
//! turbo-dom-raster uses.)
//!
//! [`is_svg`]: crate::svg::is_svg

use std::io::Cursor;

/// Caller-supplied image source (§0.2): maps a template image name (an `<img>`
/// `src` or a `background-image` `url(...)`) to its encoded bytes. The engine
/// never fetches; everything the document shows comes from here.
///
/// Implemented by the host (a `HashMap`, a CMS lookup, an embedded asset table).
/// A render that supplies no resolver uses [`NoImages`], which resolves nothing.
pub trait ImageResolver {
    /// The encoded bytes (PNG or JPEG) for `name`, or `None` if unknown. A
    /// `None` result lets the image lay out at zero intrinsic size and emit
    /// nothing, so an unresolved reference degrades gracefully.
    fn resolve(&self, name: &str) -> Option<&[u8]>;

    /// The intrinsic size + alpha flag layout sizes `name` with, or `None` if it
    /// does not resolve or decode. The default is [`probe`] of the
    /// [`resolve`](Self::resolve)d bytes; a host that has already decoded its images
    /// (turbo-dom-raster) answers from that decode instead.
    fn intrinsic(&self, name: &str) -> Option<Intrinsic> {
        self.resolve(name).and_then(probe)
    }
}

/// The default resolver: every lookup misses, so no images are embedded. Lets
/// the layout and emit entry points keep a zero-config signature.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoImages;

impl ImageResolver for NoImages {
    fn resolve(&self, _name: &str) -> Option<&[u8]> {
        None
    }
}

/// The container format of an encoded image, decided by its magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
    /// GIF87a/GIF89a. Intrinsic-size only (header probe); not decoded for paint.
    Gif,
    /// RIFF/WebP (lossy `VP8 `, lossless `VP8L`, extended `VP8X`). Intrinsic-size
    /// only (header probe); not decoded for paint.
    WebP,
    /// Windows bitmap (`BM`). Intrinsic-size only here (header probe); turbo-dom-raster
    /// decodes it for paint.
    Bmp,
    /// Windows icon (`ICO`, resource type 1) or cursor (`CUR`, type 2): a directory of
    /// BMP- or PNG-encoded entries, sized by the entry a browser shows (the largest).
    /// Intrinsic-size only here; turbo-dom-raster decodes it for paint.
    Ico,
    /// AVIF (an ISOBMFF `ftyp` with the `avif`/`avis` brand), sized from the primary
    /// item's `ispe`. Intrinsic-size only here; turbo-dom-raster decodes it for paint
    /// under its `avif` feature.
    Avif,
}

const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// GIF magic: either the `GIF87a` or `GIF89a` signature.
fn is_gif(bytes: &[u8]) -> bool {
    bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")
}

/// RIFF/WebP magic: `RIFF` … `WEBP` with the `FourCC` at bytes 8..12.
fn is_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
}

/// Sniff the encoded format from the leading magic bytes, or `None` if neither.
#[must_use]
pub fn sniff(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(PNG_MAGIC) {
        return Some(Format::Png);
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(Format::Jpeg);
    }
    if is_gif(bytes) {
        return Some(Format::Gif);
    }
    if is_webp(bytes) {
        return Some(Format::WebP);
    }
    if bytes.starts_with(b"BM") {
        return Some(Format::Bmp);
    }
    if is_ico(bytes) {
        return Some(Format::Ico);
    }
    if is_avif(bytes) {
        return Some(Format::Avif);
    }
    None
}

/// ICO/CUR magic: reserved 0, resource type 1 (icon) or 2 (cursor), at least one entry.
fn is_ico(bytes: &[u8]) -> bool {
    matches!(bytes.get(..4), Some([0, 0, 1 | 2, 0])) && bytes.get(4..6).is_some_and(|n| n != [0, 0])
}

/// AVIF magic: a leading `ftyp` box whose major or a compatible brand is `avif`/`avis`.
fn is_avif(bytes: &[u8]) -> bool {
    let Some((ftyp, _)) = bmff_box(bytes, 0).filter(|(b, _)| b.kind == *b"ftyp") else {
        return false;
    };
    let body = &bytes[ftyp.body.clone()];
    let brand = |b: &[u8]| b == b"avif" || b == b"avis";
    body.get(..4).is_some_and(brand)
        || body
            .get(8..)
            .is_some_and(|c| c.as_chunks::<4>().0.iter().any(|b| brand(b)))
}

/// The intrinsic pixel size of an encoded image plus whether it has an alpha
/// channel and its EXIF [`Orientation`], all read from the header without decoding
/// pixels. `width`/`height` are the STORED pixel size; [`Intrinsic::displayed`] turns
/// it into the size layout uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intrinsic {
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    /// How the stored pixels are displayed (`image-orientation: from-image`).
    pub orientation: Orientation,
}

impl Intrinsic {
    /// A stored size with no EXIF orientation.
    #[must_use]
    pub fn new(width: u32, height: u32, has_alpha: bool) -> Intrinsic {
        Intrinsic {
            width,
            height,
            has_alpha,
            orientation: Orientation::IDENTITY,
        }
    }

    /// The intrinsic an image box lays out and paints with: under
    /// `image-orientation: from-image` (`from_image`, the CSS default) the displayed size
    /// (width and height swapped for orientations 5–8) keeping the orientation to apply
    /// at paint; under `image-orientation: none` the stored size, painted as stored.
    #[must_use]
    pub fn displayed(self, from_image: bool) -> Intrinsic {
        if !from_image {
            return Intrinsic {
                orientation: Orientation::IDENTITY,
                ..self
            };
        }
        let (width, height) = self.orientation.displayed_size(self.width, self.height);
        Intrinsic {
            width,
            height,
            ..self
        }
    }
}

/// An image's EXIF orientation (TIFF tag 0x0112, values 1–8): how the stored pixels
/// map onto the displayed image (CSS Images 3 `image-orientation: from-image`). Read by
/// [`probe`] from a JPEG `APP1` Exif segment and a PNG `eXIf` chunk — the two sources
/// Chrome 156 applies (it ignores WebP `EXIF` chunks; see turbo-dom's
/// `scripts/image-orientation-chrome-control.mjs`). Anything else (absent, malformed,
/// out of range) is [`Orientation::IDENTITY`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Orientation(u8);

impl Default for Orientation {
    fn default() -> Self {
        Orientation::IDENTITY
    }
}

impl Orientation {
    /// Orientation 1: the stored pixels are the displayed image.
    pub const IDENTITY: Orientation = Orientation(1);

    /// The orientation for EXIF value `value` (`1..=8`; anything else is identity).
    #[must_use]
    pub fn from_exif(value: u16) -> Orientation {
        match u8::try_from(value) {
            Ok(v @ 1..=8) => Orientation(v),
            _ => Orientation::IDENTITY,
        }
    }

    /// The EXIF value (`1..=8`).
    #[must_use]
    pub fn exif(self) -> u8 {
        self.0
    }

    /// Whether this is orientation 1 (nothing to transform).
    #[must_use]
    pub fn is_identity(self) -> bool {
        self.exif() == 1
    }

    /// Whether the displayed image is the stored one transposed (orientations 5–8: a
    /// quarter turn), so its width and height swap.
    #[must_use]
    pub fn swaps_axes(self) -> bool {
        self.exif() >= 5
    }

    /// The displayed `(width, height)` of a `width × height` stored image.
    #[must_use]
    pub fn displayed_size(self, width: u32, height: u32) -> (u32, u32) {
        if self.swaps_axes() {
            (height, width)
        } else {
            (width, height)
        }
    }

    /// The stored pixel shown at displayed pixel `(x, y)` of a `width × height` STORED
    /// image (`x`/`y` index the displayed image, whose size is
    /// [`displayed_size`](Self::displayed_size)).
    #[must_use]
    pub fn source_pixel(self, x: u32, y: u32, width: u32, height: u32) -> (u32, u32) {
        let (r, b) = (width - 1, height - 1);
        match self.exif() {
            2 => (r - x, y),
            3 => (r - x, b - y),
            4 => (x, b - y),
            5 => (y, x),
            6 => (y, b - x),
            7 => (r - y, b - x),
            8 => (r - y, x),
            _ => (x, y),
        }
    }

    /// Re-lay a `width × height` straight RGBA8 image (4 bytes/px, row-major) as
    /// displayed: `(displayed width, displayed height, pixels)`.
    #[must_use]
    pub fn apply_rgba(self, width: u32, height: u32, rgba: &[u8]) -> (u32, u32, Vec<u8>) {
        let (dw, dh) = self.displayed_size(width, height);
        if self.is_identity() {
            return (dw, dh, rgba.to_vec());
        }
        let mut out = Vec::with_capacity(rgba.len());
        for y in 0..dh {
            for x in 0..dw {
                let (sx, sy) = self.source_pixel(x, y, width, height);
                let at = ((sy * width + sx) * 4) as usize;
                out.extend_from_slice(&rgba[at..at + 4]);
            }
        }
        (dw, dh, out)
    }
}

/// The orientation in a TIFF/Exif block (`II*\0` or `MM\0*` header): IFD0's
/// Orientation (0x0112, SHORT) entry, else identity.
fn tiff_orientation(tiff: &[u8]) -> Orientation {
    exif_orientation_value(tiff).map_or(Orientation::IDENTITY, Orientation::from_exif)
}

fn exif_orientation_value(tiff: &[u8]) -> Option<u16> {
    let little = match tiff.get(..4)? {
        b"II*\0" => true,
        b"MM\0*" => false,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<u16> {
        let b: [u8; 2] = tiff.get(at..at + 2)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(b)
        } else {
            u16::from_be_bytes(b)
        })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let b: [u8; 4] = tiff.get(at..at + 4)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        })
    };
    let ifd = usize::try_from(u32_at(4)?).ok()?;
    let count = usize::from(u16_at(ifd)?);
    (0..count).find_map(|i| {
        let entry = ifd + 2 + i * 12;
        // Tag 0x0112, type SHORT (3), the value in the entry's first two value bytes.
        (u16_at(entry)? == 0x0112 && u16_at(entry + 2)? == 3).then(|| u16_at(entry + 8))?
    })
}

/// A JPEG's orientation: the first `APP1` segment carrying `Exif\0\0`, scanning the
/// marker segments up to start-of-scan.
fn jpeg_orientation(bytes: &[u8]) -> Orientation {
    let mut at = 2; // past SOI
    while let Some(&[0xFF, marker]) = bytes.get(at..at + 2) {
        if marker == 0xDA || marker == 0xD9 {
            break; // start of scan / end of image: no more metadata segments
        }
        let Some(len) = bytes
            .get(at + 2..at + 4)
            .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
        else {
            break;
        };
        let body = bytes.get(at + 4..at + 2 + len).unwrap_or_default();
        if marker == 0xE1 {
            if let Some(tiff) = body.strip_prefix(b"Exif\0\0") {
                return tiff_orientation(tiff);
            }
        }
        at += 2 + len;
    }
    Orientation::IDENTITY
}

/// Read just the intrinsic size + alpha flag from encoded bytes, without
/// decoding pixels. Used at layout time to size the image box and apply the
/// overflow caps.
#[must_use]
pub fn probe(bytes: &[u8]) -> Option<Intrinsic> {
    // SVG has no fixed magic, so it is sniffed structurally (gated): an SVG box is
    // sized from its rasterized intrinsic dimensions before the raster sniff runs.
    #[cfg(feature = "svg")]
    if crate::svg::is_svg(bytes) {
        return crate::svg::probe_svg(bytes);
    }
    match sniff(bytes)? {
        Format::Png => probe_png(bytes),
        Format::Jpeg => probe_jpeg(bytes),
        Format::Gif => probe_gif(bytes),
        Format::WebP => probe_webp(bytes),
        Format::Bmp => probe_bmp(bytes),
        Format::Ico => probe_ico(bytes),
        Format::Avif => probe_avif(bytes),
    }
}

/// BMP intrinsic size from the DIB header after the 14-byte file header: a
/// `BITMAPCOREHEADER` (12 bytes: `u16` width/height) or a `BITMAPINFOHEADER`+ (`i32`
/// width/height; a negative height is a top-down bitmap). `has_alpha` for 32-bit
/// bitmaps (their fourth channel or alpha mask may carry transparency).
fn probe_bmp(bytes: &[u8]) -> Option<Intrinsic> {
    let word = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let dword = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let header = dword(14)?;
    let (w, h, bpp) = if header == 12 {
        (i32::from(word(18)?), i32::from(word(20)?), word(24)?)
    } else {
        (dword(18)?, dword(22)?, word(28)?)
    };
    let (w, h) = (w.unsigned_abs(), h.unsigned_abs());
    (w > 0 && h > 0).then_some(Intrinsic::new(w, h, bpp == 32))
}

/// The directory entry of an ICO/CUR file a browser shows: the largest by area, then by
/// bit depth (an entry's 0 width/height byte means 256; a PNG-encoded entry is sized
/// from its own header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IcoEntry<'a> {
    pub width: u32,
    pub height: u32,
    /// The entry's image: a PNG stream, or a headerless BMP (DIB + AND mask).
    pub data: &'a [u8],
    /// Whether `data` is a PNG stream.
    pub png: bool,
}

/// The [`IcoEntry`] a browser shows of an ICO/CUR file, or `None` if it is not one.
#[must_use]
pub fn ico_entry(bytes: &[u8]) -> Option<IcoEntry<'_>> {
    if sniff(bytes) != Some(Format::Ico) {
        return None;
    }
    let count = usize::from(u16::from_le_bytes([*bytes.get(4)?, *bytes.get(5)?]));
    let mut best: Option<(u64, u16, IcoEntry)> = None;
    for i in 0..count {
        let entry = bytes.get(6 + 16 * i..22 + 16 * i)?;
        let dim = |b: u8| if b == 0 { 256 } else { u32::from(b) };
        let bpp = u16::from_le_bytes([entry[6], entry[7]]);
        let size = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;
        let offset = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;
        let data = bytes
            .get(offset..offset.saturating_add(size))
            .unwrap_or_default();
        let png = data.starts_with(PNG_MAGIC);
        let (width, height) = if png {
            let i = probe_png(data)?;
            (i.width, i.height)
        } else {
            (dim(entry[0]), dim(entry[1]))
        };
        let area = u64::from(width) * u64::from(height);
        if best.is_none_or(|(a, b, _)| (area, bpp) > (a, b)) {
            best = Some((
                area,
                bpp,
                IcoEntry {
                    width,
                    height,
                    data,
                    png,
                },
            ));
        }
    }
    best.map(|(_, _, e)| e)
}

/// ICO/CUR intrinsic size: the [`ico_entry`] a browser shows (icons always carry alpha:
/// a 32-bit channel or the AND mask).
fn probe_ico(bytes: &[u8]) -> Option<Intrinsic> {
    let entry = ico_entry(bytes)?;
    Some(Intrinsic::new(entry.width, entry.height, true))
}

/// One ISOBMFF box: its four-character type and the byte range of its body.
pub(crate) struct BmffBox {
    pub(crate) kind: [u8; 4],
    pub(crate) body: std::ops::Range<usize>,
}

/// The box starting at `at` in `bytes` and the offset just past it (`size` 1 = a 64-bit
/// size follows, 0 = to the end). `None` when truncated or malformed.
pub(crate) fn bmff_box(bytes: &[u8], at: usize) -> Option<(BmffBox, usize)> {
    let head = bytes.get(at..at + 8)?;
    let size = u64::from(u32::from_be_bytes([head[0], head[1], head[2], head[3]]));
    let kind = [head[4], head[5], head[6], head[7]];
    let (start, end) = match size {
        0 => (at + 8, bytes.len()),
        1 => {
            let large = u64::from_be_bytes(bytes.get(at + 8..at + 16)?.try_into().ok()?);
            (at + 16, at.checked_add(usize::try_from(large).ok()?)?)
        }
        n => (at + 8, at.checked_add(usize::try_from(n).ok()?)?),
    };
    (start <= end && end <= bytes.len()).then_some((
        BmffBox {
            kind,
            body: start..end,
        },
        end,
    ))
}

/// The child boxes of `range` (a box body, or the whole file), skipping `skip` leading
/// bytes (a full box's version/flags).
pub(crate) fn bmff_children(
    bytes: &[u8],
    range: std::ops::Range<usize>,
    skip: usize,
) -> Vec<BmffBox> {
    let mut out = Vec::new();
    let mut at = range.start + skip;
    let slice_end = range.end;
    while at < slice_end {
        let Some((b, next)) = bmff_box(&bytes[..slice_end], at) else {
            break;
        };
        out.push(b);
        at = next;
    }
    out
}

/// The AVIF item properties: the primary item id, and for each item its associated
/// property boxes (from `meta/pitm`, `meta/iprp/ipco` + `ipma`).
pub(crate) struct AvifMeta {
    pub(crate) children: Vec<BmffBox>,
    pub(crate) primary: u32,
    pub(crate) properties: Vec<BmffBox>,
    /// `(item id, 1-based property indices)`.
    pub(crate) associations: Vec<(u32, Vec<usize>)>,
}

impl AvifMeta {
    /// Parse the top-level `meta` box of an AVIF file.
    pub(crate) fn parse(bytes: &[u8]) -> Option<AvifMeta> {
        let meta = bmff_children(bytes, 0..bytes.len(), 0)
            .into_iter()
            .find(|b| b.kind == *b"meta")?;
        let children = bmff_children(bytes, meta.body, 4);
        let pitm = children.iter().find(|b| b.kind == *b"pitm")?;
        let primary = read_versioned_id(bytes, pitm.body.start, bytes[pitm.body.start] == 0)?;
        let iprp = children.iter().find(|b| b.kind == *b"iprp")?;
        let iprp_children = bmff_children(bytes, iprp.body.clone(), 0);
        let ipco = iprp_children.iter().find(|b| b.kind == *b"ipco")?;
        let properties = bmff_children(bytes, ipco.body.clone(), 0);
        let associations = iprp_children
            .iter()
            .find(|b| b.kind == *b"ipma")
            .map(|ipma| parse_ipma(bytes, &ipma.body))
            .unwrap_or_default();
        Some(AvifMeta {
            children,
            primary,
            properties,
            associations,
        })
    }

    /// The property boxes associated with `item`.
    pub(crate) fn properties_of(&self, item: u32) -> impl Iterator<Item = &BmffBox> {
        self.associations
            .iter()
            .filter(move |(id, _)| *id == item)
            .flat_map(|(_, idx)| idx.iter())
            .filter_map(|&i| self.properties.get(i.checked_sub(1)?))
    }
}

/// The natural size a browser reports for an image (`naturalWidth`/`naturalHeight`): its
/// displayed size (EXIF orientation applied — Chrome reports it even under
/// `image-orientation: none`) for every raster format [`probe`] reads, and, under
/// `svg-rasterize`, an SVG's [`crate::svg::natural_size`] rounded to whole pixels.
/// `None` for bytes a browser would not decode. The probe a live `<img>` load
/// (turbo-surf) decides success and `natural*` with (miaskiewicz/turbo-surf#30).
#[must_use]
pub fn probe_natural(bytes: &[u8]) -> Option<Intrinsic> {
    #[cfg(feature = "svg-rasterize")]
    if crate::svg::is_svg(bytes) {
        let (w, h) = crate::svg::natural_size(bytes)?;
        return Some(Intrinsic::new(w.round() as u32, h.round() as u32, true));
    }
    // Raster formats only: SVG has no magic, so the sniff rejects it here (and `probe`'s
    // own SVG arm under `svg` would size it at the PDF's 2x).
    sniff(bytes)?;
    Some(probe(bytes)?.displayed(true))
}

/// A decoded `data:` URL (the Fetch standard's data: URL processor): its MIME type
/// (`text/plain;charset=US-ASCII` when omitted, as the standard) and payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataUrl {
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// Decode a `data:` URL: percent-decoded bytes, then (`;base64` parameter) forgiving
/// base64 (ASCII whitespace ignored, padding optional). Covers the forms pages use for
/// inline images — base64 rasters and percent-encoded or raw `image/svg+xml` — so a live
/// `<img>` load can resolve every `data:` source (miaskiewicz/turbo-surf#30). `None` if
/// `url` is not a `data:` URL or its base64 is invalid.
#[must_use]
pub fn decode_data_url(url: &str) -> Option<DataUrl> {
    let rest = url.trim_start();
    let scheme = rest.get(..5)?;
    if !scheme.eq_ignore_ascii_case("data:") {
        return None;
    }
    let (meta, payload) = rest[5..].split_once(',')?;
    let meta = meta.trim();
    let (mime, base64) = match meta.rsplit_once(';') {
        Some((head, last)) if last.trim().eq_ignore_ascii_case("base64") => (head.trim(), true),
        _ => (meta, false),
    };
    let mime = if mime.is_empty() || mime.starts_with(';') {
        format!("text/plain;charset=US-ASCII{mime}")
    } else {
        mime.to_string()
    };
    let decoded = percent_decode(payload.as_bytes());
    let bytes = if base64 {
        forgiving_base64(&decoded)?
    } else {
        decoded
    };
    Some(DataUrl { mime, bytes })
}

/// `%XX` escapes to bytes; a `%` not followed by two hex digits stays literal.
fn percent_decode(input: &[u8]) -> Vec<u8> {
    let hex = |b: u8| (b as char).to_digit(16);
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' {
            if let (Some(h), Some(l)) = (
                input.get(i + 1).copied().and_then(hex),
                input.get(i + 2).copied().and_then(hex),
            ) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(input[i]);
        i += 1;
    }
    out
}

/// The WHATWG forgiving-base64 decode: ASCII whitespace dropped, `=` padding optional
/// (but only at the end), any other non-alphabet byte or a length of 1 mod 4 fails.
fn forgiving_base64(input: &[u8]) -> Option<Vec<u8>> {
    let mut data: Vec<u8> = input
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if data.len().is_multiple_of(4) {
        for _ in 0..2 {
            if data.last() == Some(&b'=') {
                data.pop();
            }
        }
    }
    if data.len() % 4 == 1 {
        return None;
    }
    let sextet = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(data.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in data {
        acc = (acc << 6) | sextet(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// The AV1 payloads of an AVIF still image, demuxed from its container for a decoder:
/// the primary item's AV1 OBU stream, the alpha auxiliary item's (if any), and the
/// primary item's `nclx` colour description (`colr`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvifPayload {
    /// The primary (colour) image's AV1 bitstream (low-overhead OBU format).
    pub color: Vec<u8>,
    /// The alpha auxiliary image's AV1 bitstream (monochrome), when present.
    pub alpha: Option<Vec<u8>>,
    /// `nclx` colour primaries / transfer / matrix coefficients / full-range flag of the
    /// primary item, when it carries one (else the AV1 sequence header's apply).
    pub nclx: Option<Nclx>,
}

/// An ISOBMFF `colr` box of type `nclx` (ITU-T H.273 code points).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nclx {
    pub primaries: u16,
    pub transfer: u16,
    pub matrix: u16,
    pub full_range: bool,
}

/// Demux an AVIF still image (see [`AvifPayload`]): the `iloc` extents of the primary
/// item and of its alpha auxiliary item (construction method 0, file offsets, or 1, the
/// `idat` box). `None` when the file is not a parseable AVIF.
#[must_use]
pub fn avif_payload(bytes: &[u8]) -> Option<AvifPayload> {
    if sniff(bytes) != Some(Format::Avif) {
        return None;
    }
    let meta = AvifMeta::parse(bytes)?;
    let iloc = meta.children.iter().find(|b| b.kind == *b"iloc")?;
    let idat = meta.children.iter().find(|b| b.kind == *b"idat");
    let locations = parse_iloc(bytes, &iloc.body)?;
    let data_of = |item: u32| -> Option<Vec<u8>> {
        let (method, extents) = locations
            .iter()
            .find(|(id, ..)| *id == item)
            .map(|(_, m, e)| (*m, e))?;
        let base = match method {
            0 => 0,
            1 => idat?.body.start,
            _ => return None,
        };
        let mut out = Vec::new();
        for &(offset, length) in extents {
            let start = base.checked_add(offset)?;
            // A zero length runs to the end of the file / idat.
            let end = if length == 0 {
                if method == 1 {
                    idat?.body.end
                } else {
                    bytes.len()
                }
            } else {
                start.checked_add(length)?
            };
            out.extend_from_slice(bytes.get(start..end)?);
        }
        Some(out)
    };
    let alpha_item = meta.associations.iter().map(|(id, _)| *id).find(|&id| {
        id != meta.primary
            && meta.properties_of(id).any(|b| {
                b.kind == *b"auxC"
                    && bytes
                        .get(b.body.clone())
                        .is_some_and(|c| c.windows(5).any(|w| w == b"alpha"))
            })
    });
    let nclx = meta
        .properties_of(meta.primary)
        .filter(|b| b.kind == *b"colr")
        .find_map(|b| parse_nclx(bytes.get(b.body.clone())?));
    Some(AvifPayload {
        color: data_of(meta.primary)?,
        alpha: alpha_item.and_then(data_of),
        nclx,
    })
}

/// An `nclx` `colr` body: `nclx`, three `u16` code points, then the full-range bit.
fn parse_nclx(body: &[u8]) -> Option<Nclx> {
    if body.get(..4)? != b"nclx" {
        return None;
    }
    let u16_at = |at: usize| -> Option<u16> {
        Some(u16::from_be_bytes(body.get(at..at + 2)?.try_into().ok()?))
    };
    Some(Nclx {
        primaries: u16_at(4)?,
        transfer: u16_at(6)?,
        matrix: u16_at(8)?,
        full_range: body.get(10)? & 0x80 != 0,
    })
}

/// `iloc` items: `(item id, construction method, [(offset, length)])`, with each
/// extent's offset already including the item's base offset.
#[allow(clippy::type_complexity)]
fn parse_iloc(
    bytes: &[u8],
    body: &std::ops::Range<usize>,
) -> Option<Vec<(u32, u8, Vec<(usize, usize)>)>> {
    let data = bytes.get(body.clone())?;
    let version = *data.first()?;
    let mut at = 4;
    let mut take = |n: usize| -> Option<u64> {
        let v = data
            .get(at..at + n)?
            .iter()
            .fold(0u64, |acc, &b| acc << 8 | u64::from(b));
        at += n;
        Some(v)
    };
    let sizes = take(1)?;
    let (offset_size, length_size) = ((sizes >> 4) as usize, (sizes & 15) as usize);
    let sizes = take(1)?;
    let base_size = (sizes >> 4) as usize;
    let index_size = if version >= 1 {
        (sizes & 15) as usize
    } else {
        0
    };
    let count = take(if version < 2 { 2 } else { 4 })?;
    let mut out = Vec::new();
    for _ in 0..count {
        let item = u32::try_from(take(if version < 2 { 2 } else { 4 })?).ok()?;
        let method = if version >= 1 {
            (take(2)? & 15) as u8
        } else {
            0
        };
        let _data_reference = take(2)?;
        let base = usize::try_from(take(base_size)?).ok()?;
        let extents = take(2)?;
        let mut list = Vec::new();
        for _ in 0..extents {
            take(index_size)?;
            let offset = usize::try_from(take(offset_size)?).ok()?;
            let length = usize::try_from(take(length_size)?).ok()?;
            list.push((base.checked_add(offset)?, length));
        }
        out.push((item, method, list));
    }
    Some(out)
}

/// A full box's item id after its version/flags: `u16` (`short`) or `u32`.
fn read_versioned_id(bytes: &[u8], body: usize, short: bool) -> Option<u32> {
    if short {
        Some(u32::from(u16::from_be_bytes(
            bytes.get(body + 4..body + 6)?.try_into().ok()?,
        )))
    } else {
        Some(u32::from_be_bytes(
            bytes.get(body + 4..body + 8)?.try_into().ok()?,
        ))
    }
}

/// `ipma` entries: `(item id, property indices)` (version 0: 16-bit item ids; flag 1:
/// 15-bit property indices, else 7-bit; the essential bit is dropped).
fn parse_ipma(bytes: &[u8], body: &std::ops::Range<usize>) -> Vec<(u32, Vec<usize>)> {
    let data = &bytes[body.clone()];
    let (Some(&version), Some(&flags)) = (data.first(), data.get(3)) else {
        return Vec::new();
    };
    let mut at = 4;
    let mut take = |n: usize| -> Option<u32> {
        let v = data
            .get(at..at + n)?
            .iter()
            .fold(0u32, |acc, &b| acc << 8 | u32::from(b));
        at += n;
        Some(v)
    };
    let mut out = Vec::new();
    let Some(count) = take(4) else {
        return out;
    };
    for _ in 0..count {
        let Some(item) = take(if version < 1 { 2 } else { 4 }) else {
            break;
        };
        let Some(n) = take(1) else {
            break;
        };
        let mut indices = Vec::new();
        for _ in 0..n {
            let Some(raw) = take(if flags & 1 == 1 { 2 } else { 1 }) else {
                break;
            };
            let mask = if flags & 1 == 1 { 0x7FFF } else { 0x7F };
            indices.push((raw & mask) as usize);
        }
        out.push((item, indices));
    }
    out
}

/// AVIF intrinsic size: the primary item's `ispe` (image spatial extents); `has_alpha`
/// when the file carries an alpha auxiliary image (`auxC` of the alpha URN). Rotation
/// / mirroring properties (`irot`/`imir`) are not applied.
fn probe_avif(bytes: &[u8]) -> Option<Intrinsic> {
    let meta = AvifMeta::parse(bytes)?;
    let ispe = meta
        .properties_of(meta.primary)
        .find(|b| b.kind == *b"ispe")?;
    let body = bytes.get(ispe.body.clone())?;
    let w = u32::from_be_bytes(body.get(4..8)?.try_into().ok()?);
    let h = u32::from_be_bytes(body.get(8..12)?.try_into().ok()?);
    let alpha = meta.properties.iter().any(|b| {
        b.kind == *b"auxC"
            && bytes
                .get(b.body.clone())
                .is_some_and(|c| c.windows(5).any(|w| w == b"alpha"))
    });
    (w > 0 && h > 0).then_some(Intrinsic::new(w, h, alpha))
}

/// GIF intrinsic size from the logical-screen descriptor: width/height are the two
/// little-endian `u16`s at bytes 6..10 (right after the 6-byte signature). No pixel
/// decode. `has_alpha` is left false — GIF transparency is per-frame and only
/// matters at emit, which this format doesn't take.
fn probe_gif(bytes: &[u8]) -> Option<Intrinsic> {
    if bytes.len() < 10 {
        return None;
    }
    let w = u16::from_le_bytes([bytes[6], bytes[7]]);
    let h = u16::from_le_bytes([bytes[8], bytes[9]]);
    (w > 0 && h > 0).then_some(Intrinsic::new(u32::from(w), u32::from(h), false))
}

/// WebP intrinsic size across the three chunk layouts (lossy `VP8 `, lossless
/// `VP8L`, extended `VP8X`). Reads only the header dimensions — no pixel decode.
fn probe_webp(bytes: &[u8]) -> Option<Intrinsic> {
    // The chunk FourCC follows the 12-byte RIFF/WEBP header.
    let fourcc = bytes.get(12..16)?;
    let (w, h) = webp_size(fourcc, bytes)?;
    // lossless/extended may carry alpha; an `EXIF` chunk's orientation is not applied
    // (Chrome 156 ignores it).
    (w > 0 && h > 0).then_some(Intrinsic::new(w, h, fourcc != b"VP8 "))
}

/// Dispatch to the per-chunk size reader for a WebP `FourCC`; unknown chunks yield
/// `None`.
fn webp_size(fourcc: &[u8], bytes: &[u8]) -> Option<(u32, u32)> {
    match fourcc {
        b"VP8 " => webp_lossy_size(bytes),
        b"VP8L" => webp_lossless_size(bytes),
        b"VP8X" => webp_extended_size(bytes),
        _ => None,
    }
}

/// Lossy WebP: after the `VP8 ` chunk header (8 bytes) the VP8 key-frame carries a
/// 3-byte start code `9d 01 2a`, then 14-bit width and height (little-endian) at
/// bytes 26..30 of the file.
fn webp_lossy_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let d = bytes.get(20..30)?; // frame tag (3) + start code (3) + w/h (4)
    if d[3..6] != [0x9d, 0x01, 0x2a] {
        return None;
    }
    let w = u16::from_le_bytes([d[6], d[7]]) & 0x3fff;
    let h = u16::from_le_bytes([d[8], d[9]]) & 0x3fff;
    Some((u32::from(w), u32::from(h)))
}

/// Lossless WebP: after the `VP8L` chunk header the signature byte `0x2f` precedes
/// 14-bit `width-1` and `height-1` packed little-endian across the next 4 bytes.
fn webp_lossless_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let d = bytes.get(20..25)?; // signature (1) + 4 packed bytes
    if d[0] != 0x2f {
        return None;
    }
    let bits = u32::from_le_bytes([d[1], d[2], d[3], d[4]]);
    let w = (bits & 0x3fff) + 1;
    let h = ((bits >> 14) & 0x3fff) + 1;
    Some((w, h))
}

/// Extended WebP (`VP8X`): the canvas size is two 24-bit `value-1` little-endian
/// fields at bytes 24..30 (after the 4-byte flags of the VP8X chunk body).
fn webp_extended_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let d = bytes.get(24..30)?;
    let w = u32::from_le_bytes([d[0], d[1], d[2], 0]) + 1;
    let h = u32::from_le_bytes([d[3], d[4], d[5], 0]) + 1;
    Some((w, h))
}

fn probe_png(bytes: &[u8]) -> Option<Intrinsic> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().ok()?;
    let info = reader.info();
    Some(Intrinsic {
        width: info.width,
        height: info.height,
        has_alpha: png_has_alpha(info.color_type, info.trns.is_some()),
        // The `eXIf` chunk (read before the first IDAT) is a bare TIFF block.
        orientation: info
            .exif_metadata
            .as_deref()
            .map_or(Orientation::IDENTITY, tiff_orientation),
    })
}

/// Whether a PNG carries transparency: a color type with an alpha channel, or a
/// `tRNS` chunk (which `normalize_to_color8` expands into one on decode).
fn png_has_alpha(color: png::ColorType, has_trns: bool) -> bool {
    matches!(color, png::ColorType::GrayscaleAlpha | png::ColorType::Rgba) || has_trns
}

fn probe_jpeg(bytes: &[u8]) -> Option<Intrinsic> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    Some(Intrinsic {
        width: u32::from(info.width),
        height: u32::from(info.height),
        has_alpha: false,
        orientation: jpeg_orientation(bytes),
    })
}

/// The PDF color space an image's samples live in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// 1 sample/pixel grayscale (`DeviceGray`).
    Gray,
    /// 3 samples/pixel (`DeviceRGB`).
    Rgb,
}

impl ColorSpace {
    /// Samples per pixel in this color space.
    #[must_use]
    pub fn components(self) -> usize {
        match self {
            ColorSpace::Gray => 1,
            ColorSpace::Rgb => 3,
        }
    }
}

/// How an image's pixels reach the PDF: PNG is re-encoded as raw samples (the
/// emitter Flate-compresses the stream); JPEG rides through untouched as a
/// `DCTDecode` stream (§7.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    /// Raw 8-bit samples in `color`, row-major, no padding.
    Raw { samples: Vec<u8>, color: ColorSpace },
    /// The original JPEG bytes, embedded as a `DCTDecode` stream.
    Jpeg { bytes: Vec<u8>, color: ColorSpace },
}

/// A decoded image ready to embed: its pixel size, the color payload, and the
/// optional 8-bit alpha plane that becomes the `XObject`'s `SMask` (§7.4 AC-7.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    pub payload: Payload,
    /// One alpha byte per pixel, row-major, when the source had transparency.
    pub alpha: Option<Vec<u8>>,
}

impl RasterImage {
    /// The image's color space (RGB or gray), regardless of payload encoding.
    #[must_use]
    pub fn color(&self) -> ColorSpace {
        match &self.payload {
            Payload::Raw { color, .. } | Payload::Jpeg { color, .. } => *color,
        }
    }
}

/// Decode encoded bytes into an embeddable [`RasterImage`], or `None` if the
/// format is unrecognized or the data is malformed.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<RasterImage> {
    // SVG (gated): rasterize to RGBA before the raster sniff. The resulting
    // [`RasterImage`] feeds the same XObject/SMask path as a decoded RGBA PNG.
    #[cfg(feature = "svg")]
    if crate::svg::is_svg(bytes) {
        return crate::svg::decode_svg(bytes);
    }
    match sniff(bytes)? {
        Format::Png => decode_png(bytes),
        Format::Jpeg => decode_jpeg(bytes),
        // GIF/WebP/BMP/ICO/AVIF are probed for intrinsic size (layout) but not
        // pixel-decoded for the PDF — an unsupported decode emits nothing rather than a
        // wrong image (turbo-dom-raster decodes them for screenshots).
        Format::Gif | Format::WebP | Format::Bmp | Format::Ico | Format::Avif => None,
    }
}

fn decode_png(bytes: &[u8]) -> Option<RasterImage> {
    Some(PngFrame::decode(bytes, PngBudget::PngDefault)?.to_raster_image())
}

/// The channel layout of a decoded [`PngFrame`]: palette, low-bit and `tRNS` inputs
/// are already expanded, so only these four remain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngColor {
    Gray,
    GrayAlpha,
    Rgb,
    Rgba,
}

impl PngColor {
    /// Samples per pixel.
    #[must_use]
    pub fn channels(self) -> usize {
        match self {
            PngColor::Gray => 1,
            PngColor::GrayAlpha => 2,
            PngColor::Rgb => 3,
            PngColor::Rgba => 4,
        }
    }

    /// The expanded png-crate colour type. `EXPAND` turns `Indexed` into RGB(A), so
    /// any other value is RGBA.
    fn of(color: png::ColorType) -> PngColor {
        match color {
            png::ColorType::Grayscale => PngColor::Gray,
            png::ColorType::GrayscaleAlpha => PngColor::GrayAlpha,
            png::ColorType::Rgb => PngColor::Rgb,
            _ => PngColor::Rgba,
        }
    }

    /// The PDF colour space + alpha sample index of this layout.
    fn pdf_layout(self) -> (ColorSpace, Option<usize>) {
        match self {
            PngColor::Gray => (ColorSpace::Gray, None),
            PngColor::GrayAlpha => (ColorSpace::Gray, Some(1)),
            PngColor::Rgb => (ColorSpace::Rgb, None),
            PngColor::Rgba => (ColorSpace::Rgb, Some(3)),
        }
    }
}

/// The allocation budget of one [`PngFrame::decode`] — each consumer keeps the
/// limits its decoder had before the decode was shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngBudget {
    /// The png crate's own default limits (64 MiB of decoder buffers): the PDF path.
    PngDefault,
    /// The `image` crate's default `Limits` (512 MiB `max_alloc`, also checked against
    /// the decoded frame size) — what `image::load_from_memory` applied to the
    /// screenshot painter's PNGs.
    ImageCrate,
}

/// `image::Limits::default().max_alloc`.
const IMAGE_CRATE_MAX_ALLOC: usize = 512 * 1024 * 1024;

impl PngBudget {
    fn limits(self) -> png::Limits {
        match self {
            PngBudget::PngDefault => png::Limits::default(),
            PngBudget::ImageCrate => png::Limits {
                bytes: IMAGE_CRATE_MAX_ALLOC,
            },
        }
    }

    /// Whether a decoded frame of `bytes` fits (the `image` crate reserves the frame
    /// buffer against its budget; the png crate's own default does not).
    fn admits_frame(self, bytes: usize) -> bool {
        self == PngBudget::PngDefault || bytes <= IMAGE_CRATE_MAX_ALLOC
    }
}

/// ONE PNG decode shared by the PDF emitter and turbo-dom-raster: palette, low-bit
/// and `tRNS` are expanded (`png::Transformations::EXPAND`) and 16-bit samples are
/// KEPT, so each consumer derives its own 8-bit form from the same decode —
/// [`to_raster_image`](Self::to_raster_image) (native samples, gray stays gray,
/// 16-bit truncated to the high byte exactly as `STRIP_16` did) and
/// [`to_rgba8`](Self::to_rgba8) (straight RGBA, reproducing `image::to_rgba8`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PngFrame {
    pub width: u32,
    pub height: u32,
    pub color: PngColor,
    /// Whether each sample is 16-bit (big-endian, two bytes) rather than 8-bit.
    pub sixteen_bit: bool,
    /// Row-major samples, no padding.
    pub samples: Vec<u8>,
}

impl PngFrame {
    /// Decode the first frame of a PNG under `budget`, or `None` if it is malformed
    /// or over budget.
    #[must_use]
    pub fn decode(bytes: &[u8], budget: PngBudget) -> Option<PngFrame> {
        let mut decoder = png::Decoder::new_with_limits(Cursor::new(bytes), budget.limits());
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().ok()?;
        let size = reader.output_buffer_size()?;
        if !budget.admits_frame(size) {
            return None;
        }
        let mut samples = vec![0u8; size];
        let out = reader.next_frame(&mut samples).ok()?;
        samples.truncate(out.buffer_size());
        Some(PngFrame {
            width: out.width,
            height: out.height,
            color: PngColor::of(out.color_type),
            sixteen_bit: out.bit_depth == png::BitDepth::Sixteen,
            samples,
        })
    }

    /// The 8-bit samples: 16-bit ones keep their high byte (`STRIP_16`).
    fn samples8(&self) -> std::borrow::Cow<'_, [u8]> {
        if self.sixteen_bit {
            self.samples.iter().step_by(2).copied().collect()
        } else {
            std::borrow::Cow::Borrowed(&self.samples)
        }
    }

    /// The PDF form: an RGB/gray payload (gray stays `DeviceGray`) plus the alpha
    /// plane split off for the `SMask`.
    #[must_use]
    pub fn to_raster_image(&self) -> RasterImage {
        let (color, alpha_idx) = self.color.pdf_layout();
        let samples8 = self.samples8();
        let pixels = (self.width * self.height) as usize;
        let comps = color.components();
        let mut samples = Vec::with_capacity(pixels * comps);
        let mut alpha = alpha_idx.map(|_| Vec::with_capacity(pixels));
        for px in samples8.chunks_exact(self.color.channels()) {
            samples.extend_from_slice(&px[..comps]);
            if let (Some(a), Some(i)) = (alpha.as_mut(), alpha_idx) {
                a.push(px[i]);
            }
        }
        RasterImage {
            width: self.width,
            height: self.height,
            payload: Payload::Raw { samples, color },
            alpha,
        }
    }

    /// Straight RGBA8, byte-for-byte what `image::load_from_memory(png).to_rgba8()`
    /// (image 0.25) produces from the same file: RGB(A) channels convert numerically
    /// (8-bit as is, 16-bit `(c + 128) / 257`, missing alpha opaque), gray(+alpha)
    /// through image's f32 path (`round(c / MAX * 255)`, gray replicated).
    #[must_use]
    pub fn to_rgba8(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((self.width * self.height) as usize * 4);
        let width = if self.sixteen_bit { 2 } else { 1 };
        let channels = self.color.channels();
        for px in self.samples.chunks_exact(channels * width) {
            let sample = |i: usize| -> u16 {
                if self.sixteen_bit {
                    u16::from_be_bytes([px[2 * i], px[2 * i + 1]])
                } else {
                    u16::from(px[i])
                }
            };
            let pixel = match self.color {
                PngColor::Gray => gray_rgba(self.sixteen_bit, sample(0), None),
                PngColor::GrayAlpha => gray_rgba(self.sixteen_bit, sample(0), Some(sample(1))),
                PngColor::Rgb => rgb_rgba(
                    self.sixteen_bit,
                    [sample(0), sample(1), sample(2), u16::MAX],
                ),
                PngColor::Rgba => rgb_rgba(
                    self.sixteen_bit,
                    [sample(0), sample(1), sample(2), sample(3)],
                ),
            };
            rgba.extend_from_slice(&pixel);
        }
        rgba
    }
}

/// image's numeric channel cast to 8-bit: identity for 8-bit, `(c + 128) / 257`
/// for 16-bit. An absent alpha is passed as `u16::MAX` and maps to 255.
fn rgb_rgba(sixteen_bit: bool, px: [u16; 4]) -> [u8; 4] {
    let cast = |c: u16| -> u8 {
        if sixteen_bit {
            ((u32::from(c) + 128) / 257) as u8
        } else {
            c.min(255) as u8
        }
    };
    px.map(cast)
}

/// image's gray -> RGBA cast (through f32: `c * (1 / MAX)`, then `round(v * 255)`),
/// the luma replicated into R, G and B; no alpha is opaque.
fn gray_rgba(sixteen_bit: bool, luma: u16, alpha: Option<u16>) -> [u8; 4] {
    let expand = |c: u16| -> f32 {
        if sixteen_bit {
            f32::from(c) * (1.0 / f32::from(u16::MAX))
        } else {
            f32::from(c) * (1.0 / f32::from(u8::MAX))
        }
    };
    let clamp = |v: f32| -> u8 { (v * f32::from(u8::MAX)).round() as u8 };
    let l = clamp(expand(luma));
    let a = alpha.map_or(u8::MAX, |a| clamp(expand(a)));
    [l, l, l, a]
}

/// A straight-alpha RGBA8 image (row-major, 4 bytes per pixel) — the form
/// turbo-dom-raster paints and the shared SVG rasterizer produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Decode a PNG to straight RGBA under the `image` crate's limits (what
/// turbo-dom-raster's `image::load_from_memory(..).to_rgba8()` did), or `None`.
#[must_use]
pub fn decode_png_rgba(bytes: &[u8]) -> Option<Rgba8Image> {
    let frame = PngFrame::decode(bytes, PngBudget::ImageCrate)?;
    Some(Rgba8Image {
        width: frame.width,
        height: frame.height,
        rgba: frame.to_rgba8(),
    })
}

fn decode_jpeg(bytes: &[u8]) -> Option<RasterImage> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let color = jpeg_color(info.pixel_format)?;
    Some(RasterImage {
        width: u32::from(info.width),
        height: u32::from(info.height),
        payload: Payload::Jpeg {
            bytes: bytes.to_vec(),
            color,
        },
        alpha: None,
    })
}

/// Map a baseline-JPEG pixel format to a PDF color space. CMYK JPEGs are not
/// passed through in v1 (they need an `/Decode` inversion); they decode to
/// `None` and emit nothing.
fn jpeg_color(format: jpeg_decoder::PixelFormat) -> Option<ColorSpace> {
    match format {
        jpeg_decoder::PixelFormat::L8 | jpeg_decoder::PixelFormat::L16 => Some(ColorSpace::Gray),
        jpeg_decoder::PixelFormat::RGB24 => Some(ColorSpace::Rgb),
        jpeg_decoder::PixelFormat::CMYK32 => None,
    }
}

#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn png_budgets_keep_each_consumers_limits() {
        // The image-crate budget reserves the decoded frame against 512 MiB; the png
        // crate's default budget never checked the frame size.
        assert!(PngBudget::ImageCrate.admits_frame(IMAGE_CRATE_MAX_ALLOC));
        assert!(!PngBudget::ImageCrate.admits_frame(IMAGE_CRATE_MAX_ALLOC + 1));
        assert!(PngBudget::PngDefault.admits_frame(usize::MAX));
        assert_eq!(PngBudget::ImageCrate.limits().bytes, IMAGE_CRATE_MAX_ALLOC);
        assert_eq!(
            PngBudget::PngDefault.limits().bytes,
            png::Limits::default().bytes
        );
    }

    #[test]
    fn sniff_recognizes_gif_and_webp() {
        assert_eq!(sniff(b"GIF89a\x00\x00\x00\x00\x00\x00"), Some(Format::Gif));
        assert_eq!(sniff(b"GIF87a\x00\x00\x00\x00\x00\x00"), Some(Format::Gif));
        assert_eq!(sniff(b"RIFF\x00\x00\x00\x00WEBPVP8 "), Some(Format::WebP));
        assert_eq!(sniff(b"not an image"), None);
    }

    #[test]
    fn probe_gif_reads_logical_screen_size() {
        // GIF89a header: signature, then width=120 (0x0078) and height=60 (0x003C) LE.
        let gif = b"GIF89a\x78\x00\x3c\x00\xf0\x00\x00";
        let intrinsic = probe(gif).expect("gif intrinsic");
        assert_eq!((intrinsic.width, intrinsic.height), (120, 60));
    }

    #[test]
    fn probe_webp_lossy_reads_frame_size() {
        // RIFF/WEBP + `VP8 ` chunk; 14-bit width=100, height=50 after the start code.
        let mut b = Vec::from(*b"RIFF\x00\x00\x00\x00WEBPVP8 \x00\x00\x00\x00");
        b.extend_from_slice(&[0x00, 0x00, 0x00]); // frame tag
        b.extend_from_slice(&[0x9d, 0x01, 0x2a]); // start code
        b.extend_from_slice(&[0x64, 0x00, 0x32, 0x00]); // w=100, h=50 (LE, 14-bit)
        let intrinsic = probe(&b).expect("webp intrinsic");
        assert_eq!((intrinsic.width, intrinsic.height), (100, 50));
    }

    #[test]
    fn probe_webp_lossless_reads_packed_size() {
        // `VP8L`: signature 0x2f then packed (width-1)|((height-1)<<14). w=8, h=6.
        let bits: u32 = (8 - 1) | ((6 - 1) << 14);
        let mut b = Vec::from(*b"RIFF\x00\x00\x00\x00WEBPVP8L\x00\x00\x00\x00");
        b.push(0x2f);
        b.extend_from_slice(&bits.to_le_bytes());
        let intrinsic = probe(&b).expect("webp lossless intrinsic");
        assert_eq!((intrinsic.width, intrinsic.height), (8, 6));
    }

    #[test]
    fn gif_is_probed_but_not_decoded() {
        // Intrinsic sizing works (layout) but the pixel decode is intentionally
        // unsupported (paint) — an unsupported decode emits nothing.
        let gif = b"GIF89a\x08\x00\x06\x00\xf0\x00\x00";
        assert!(probe(gif).is_some());
        assert!(decode(gif).is_none());
    }

    #[test]
    fn probe_gif_too_short_for_screen_descriptor_is_none() {
        // A GIF whose signature matches but is truncated before the 10-byte logical
        // screen descriptor has no readable intrinsic size.
        assert!(probe(b"GIF89a\x00").is_none());
    }

    #[test]
    fn probe_webp_extended_reads_canvas_size() {
        // `VP8X`: 4-byte chunk size + 4-byte flags, then two 24-bit `value-1` LE
        // canvas fields. width-1=99 -> 100, height-1=49 -> 50.
        let mut b = Vec::from(*b"RIFF\x00\x00\x00\x00WEBPVP8X");
        b.extend_from_slice(&[0x0a, 0x00, 0x00, 0x00]); // chunk size
        b.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // flags
        b.extend_from_slice(&[0x63, 0x00, 0x00]); // width-1 = 99
        b.extend_from_slice(&[0x31, 0x00, 0x00]); // height-1 = 49
        let intrinsic = probe(&b).expect("vp8x intrinsic");
        assert_eq!((intrinsic.width, intrinsic.height), (100, 50));
        assert!(intrinsic.has_alpha, "extended WebP may carry alpha");
    }

    #[test]
    fn probe_webp_unknown_chunk_is_none() {
        // A RIFF/WEBP wrapper with an unrecognized FourCC has no known size reader.
        let b = Vec::from(*b"RIFF\x00\x00\x00\x00WEBPXXXX\x00\x00\x00\x00");
        assert!(probe(&b).is_none());
    }

    #[test]
    fn probe_webp_lossy_rejects_bad_start_code() {
        // `VP8 ` chunk without the `9d 01 2a` key-frame start code is not sized.
        let mut b = Vec::from(*b"RIFF\x00\x00\x00\x00WEBPVP8 \x00\x00\x00\x00");
        b.extend_from_slice(&[0x00; 6]); // frame tag + wrong start code
        assert!(probe(&b).is_none());
    }

    #[test]
    fn probe_webp_lossy_full_len_wrong_start_code_is_none() {
        // Long enough to pass the 30-byte length check but with a corrupted start
        // code, so the size read is rejected at the start-code guard (not the
        // length `?`) — the branch a too-short buffer can't reach.
        let mut b = Vec::from(*b"RIFF\x00\x00\x00\x00WEBPVP8 \x00\x00\x00\x00");
        b.extend_from_slice(&[0x00, 0x00, 0x00]); // frame tag
        b.extend_from_slice(&[0xde, 0xad, 0xbe]); // wrong start code (not 9d 01 2a)
        b.extend_from_slice(&[0x64, 0x00, 0x32, 0x00]); // w/h bytes (unreached)
        assert!(probe(&b).is_none());
    }

    #[test]
    fn probe_webp_lossless_rejects_bad_signature() {
        // `VP8L` chunk whose leading byte is not the `0x2f` signature is not sized.
        let mut b = Vec::from(*b"RIFF\x00\x00\x00\x00WEBPVP8L\x00\x00\x00\x00");
        b.extend_from_slice(&[0x00; 5]); // wrong signature + packed bytes
        assert!(probe(&b).is_none());
    }
}

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
    None
}

/// The intrinsic pixel size of an encoded image plus whether it has an alpha
/// channel, all read from the header without decoding pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intrinsic {
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
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
    }
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
    (w > 0 && h > 0).then_some(Intrinsic {
        width: u32::from(w),
        height: u32::from(h),
        has_alpha: false,
    })
}

/// WebP intrinsic size across the three chunk layouts (lossy `VP8 `, lossless
/// `VP8L`, extended `VP8X`). Reads only the header dimensions — no pixel decode.
fn probe_webp(bytes: &[u8]) -> Option<Intrinsic> {
    // The chunk FourCC follows the 12-byte RIFF/WEBP header.
    let fourcc = bytes.get(12..16)?;
    let (w, h) = webp_size(fourcc, bytes)?;
    (w > 0 && h > 0).then_some(Intrinsic {
        width: w,
        height: h,
        has_alpha: fourcc != b"VP8 ", // lossless/extended may carry alpha
    })
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
        // GIF/WebP are probed for intrinsic size (layout) but not yet pixel-decoded
        // for paint — an unsupported decode emits nothing rather than a wrong image.
        Format::Gif | Format::WebP => None,
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

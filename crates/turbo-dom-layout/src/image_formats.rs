//! The shared image layer's container readers beyond PNG/JPEG/GIF/WebP (P5b,
//! Chrome 156-controlled): EXIF orientation (JPEG `APP1`, PNG `eXIf` —
//! miaskiewicz/turbo-html2pdf#30), BMP, ICO/CUR and AVIF headers, the AVIF demux, and
//! the Fetch-standard `data:` URL decoder (miaskiewicz/turbo-surf#30). Header reads
//! only — no pixel decode (turbo-dom-raster decodes for screenshots).

use super::{probe_png, Format, Intrinsic, PNG_MAGIC};

/// `n` (≤ 8) bytes at `at` as a big-endian unsigned integer.
fn be_uint(bytes: &[u8], at: usize, n: usize) -> Option<u64> {
    let raw = bytes.get(at..at.checked_add(n)?)?;
    Some(raw.iter().fold(0, |acc, &b| acc << 8 | u64::from(b)))
}

/// `n` (≤ 8) bytes at `at` as a little-endian unsigned integer.
fn le_uint(bytes: &[u8], at: usize, n: usize) -> Option<u64> {
    let raw = bytes.get(at..at.checked_add(n)?)?;
    Some(raw.iter().rev().fold(0, |acc, &b| acc << 8 | u64::from(b)))
}

/// A byte-order-aware unsigned read (`little`: little-endian).
fn uint(bytes: &[u8], at: usize, n: usize, little: bool) -> Option<u64> {
    if little {
        le_uint(bytes, at, n)
    } else {
        be_uint(bytes, at, n)
    }
}

/// `n` bytes at `at` as a `usize` (`None` if absent or out of range).
fn be_usize(bytes: &[u8], at: usize, n: usize) -> Option<usize> {
    be_uint(bytes, at, n).and_then(|v| usize::try_from(v).ok())
}

// --------------------------------------------------------------------------
// EXIF orientation
// --------------------------------------------------------------------------

/// An image's EXIF orientation (TIFF tag 0x0112, values 1–8): how the stored pixels
/// map onto the displayed image (CSS Images 3 `image-orientation: from-image`). Read by
/// [`probe`](super::probe) from a JPEG `APP1` Exif segment and a PNG `eXIf` chunk — the
/// two sources Chrome 156 applies (it ignores WebP `EXIF` chunks; see turbo-dom's
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

    /// This orientation followed by `next` (applied to the image this one displays): the
    /// one orientation with the same effect (the eight form a group).
    #[must_use]
    pub fn then(self, next: Orientation) -> Orientation {
        // A 3x2 image tells the eight apart: compare where every displayed pixel comes from.
        let (w, h) = (3, 2);
        let (mw, mh) = self.displayed_size(w, h);
        let composed = |x, y| {
            let (u, v) = next.source_pixel(x, y, mw, mh);
            self.source_pixel(u, v, w, h)
        };
        let (dw, dh) = next.displayed_size(mw, mh);
        (1..=8)
            .map(Orientation)
            .find(|o| {
                o.displayed_size(w, h) == (dw, dh)
                    && (0..dh)
                        .all(|y| (0..dw).all(|x| o.source_pixel(x, y, w, h) == composed(x, y)))
            })
            .unwrap_or(Orientation::IDENTITY)
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
pub(super) fn tiff_orientation(tiff: &[u8]) -> Orientation {
    exif_orientation_value(tiff).map_or(Orientation::IDENTITY, Orientation::from_exif)
}

/// The TIFF byte order: `Some(true)` little-endian (`II*\0`), `Some(false)` big-endian.
fn tiff_byte_order(tiff: &[u8]) -> Option<bool> {
    match tiff.get(..4)? {
        b"II*\0" => Some(true),
        b"MM\0*" => Some(false),
        _ => None,
    }
}

fn exif_orientation_value(tiff: &[u8]) -> Option<u16> {
    let little = tiff_byte_order(tiff)?;
    let ifd = usize::try_from(uint(tiff, 4, 4, little)?).ok()?;
    let count = uint(tiff, ifd, 2, little)? as usize;
    (0..count).find_map(|i| orientation_entry(tiff, ifd + 2 + i * 12, little))
}

/// The value of a 12-byte IFD entry at `entry` if it is the Orientation tag (0x0112,
/// type SHORT 3; the value is the entry's first two value bytes).
fn orientation_entry(tiff: &[u8], entry: usize, little: bool) -> Option<u16> {
    let tag = uint(tiff, entry, 2, little)?;
    let kind = uint(tiff, entry + 2, 2, little)?;
    let value = uint(tiff, entry + 8, 2, little)?;
    (tag == 0x0112 && kind == 3).then_some(value as u16)
}

/// A JPEG's orientation: the first `APP1` segment carrying `Exif\0\0`, scanning the
/// marker segments up to start-of-scan.
pub(super) fn jpeg_orientation(bytes: &[u8]) -> Orientation {
    jpeg_exif(bytes).map_or(Orientation::IDENTITY, tiff_orientation)
}

/// The TIFF block of a JPEG's first Exif `APP1` segment.
fn jpeg_exif(bytes: &[u8]) -> Option<&[u8]> {
    let mut at = 2; // past SOI
    loop {
        let (marker, body, next) = jpeg_segment(bytes, at)?;
        if marker == 0xE1 && body.starts_with(b"Exif\0\0") {
            return Some(&body[6..]);
        }
        at = next;
    }
}

/// The marker segment at `at`: `(marker, body, next offset)`; `None` at start-of-scan,
/// end-of-image or on a malformed marker.
fn jpeg_segment(bytes: &[u8], at: usize) -> Option<(u8, &[u8], usize)> {
    let marker = *bytes.get(at + 1).filter(|_| bytes.get(at) == Some(&0xFF))?;
    if marker == 0xDA || marker == 0xD9 {
        return None; // start of scan / end of image: no more metadata segments
    }
    let len = be_usize(bytes, at + 2, 2)?;
    let body = bytes.get(at + 4..at + 2 + len).unwrap_or_default();
    Some((marker, body, at + 2 + len.max(2)))
}

// --------------------------------------------------------------------------
// Size availability (miaskiewicz/turbo-dom#59)
// --------------------------------------------------------------------------
//
// Blink reports an image's size — and so lets an `<img>` load (painting what it can
// decode of a truncated body) — only once its decoder has read a format's size point;
// before that the image is broken. Chrome 156 control:
// `scripts/image-truncation-chrome-control.mjs` (every byte prefix of
// `tests/fixtures/images/truncation`). PNG's point is its first `IDAT` chunk header,
// which the png crate's `read_info` already requires.

/// A JPEG's size point: the end of the first start-of-scan (`SOS`) header after a frame
/// header (libjpeg's `jpeg_read_header`). Every marker segment before it must be whole.
pub(super) fn jpeg_size_available(bytes: &[u8]) -> bool {
    let mut at = 2; // past SOI
    let mut frame = false;
    loop {
        // Fill bytes (`FF FF …`) may precede a marker.
        while bytes.get(at) == Some(&0xFF) && bytes.get(at + 1) == Some(&0xFF) {
            at += 1;
        }
        let Some(&marker) = bytes.get(at + 1).filter(|_| bytes.get(at) == Some(&0xFF)) else {
            return false;
        };
        // Standalone markers carry no length.
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            at += 2;
            continue;
        }
        if marker == 0xD8 || marker == 0xD9 {
            return false;
        }
        let Some(len) = be_usize(bytes, at + 2, 2).filter(|&len| len >= 2) else {
            return false;
        };
        let end = at + 2 + len;
        if end > bytes.len() {
            return false;
        }
        match marker {
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => frame = true,
            0xDA => return frame,
            _ => {}
        }
        at = end;
    }
}

/// A GIF's size point: the first image descriptor's position and size fields (separator
/// plus 8 bytes), reached past the global colour table and every whole extension block.
pub(super) fn gif_size_available(bytes: &[u8]) -> bool {
    let Some(&flags) = bytes.get(10) else {
        return false;
    };
    let mut at = 13;
    if flags & 0x80 != 0 {
        at += 3 << ((flags & 7) + 1);
    }
    loop {
        match bytes.get(at) {
            Some(0x2C) => return at + 9 <= bytes.len(),
            Some(0x21) => {
                // Label, then data sub-blocks up to the zero-length terminator.
                at += 2;
                loop {
                    match bytes.get(at) {
                        Some(0) => break,
                        Some(&n) => at += 1 + usize::from(n),
                        None => return false,
                    }
                }
                at += 1;
            }
            _ => return false,
        }
    }
}

/// A WebP's size point: once all data has arrived Blink demuxes the whole RIFF payload
/// (`WebPDemux`), so the file must hold the `RIFF` size it declares (trailing bytes
/// past it are ignored).
pub(super) fn webp_complete(bytes: &[u8]) -> bool {
    le_uint(bytes, 4, 4).is_some_and(|riff| bytes.len() as u64 >= riff + 8)
}

// --------------------------------------------------------------------------
// BMP, ICO/CUR
// --------------------------------------------------------------------------

/// BMP magic: `BM`.
pub(super) fn is_bmp(bytes: &[u8]) -> bool {
    bytes.starts_with(b"BM")
}

/// ICO/CUR magic: reserved 0, resource type 1 (icon) or 2 (cursor), at least one entry.
pub(super) fn is_ico(bytes: &[u8]) -> bool {
    matches!(bytes.get(..4), Some([0, 0, 1 | 2, 0])) && bytes.get(4..6).is_some_and(|n| n != [0, 0])
}

/// BMP intrinsic size from the DIB header after the 14-byte file header: a
/// `BITMAPCOREHEADER` (12 bytes: `u16` width/height) or a `BITMAPINFOHEADER`+ (`i32`
/// width/height; a negative height is a top-down bitmap). `has_alpha` for 32-bit
/// bitmaps (their fourth channel or alpha mask may carry transparency).
pub(super) fn probe_bmp(bytes: &[u8]) -> Option<Intrinsic> {
    let core = le_uint(bytes, 14, 4)? == 12;
    let (h_at, bpp_at, n) = if core { (20, 24, 2) } else { (22, 28, 4) };
    let ((w, h), bpp) = le_uint(bytes, 18, n)
        .zip(le_uint(bytes, h_at, n))
        .zip(le_uint(bytes, bpp_at, 2))?;
    // 32-bit fields are signed (`as u32 as i32` reinterprets; 16-bit ones are positive).
    let (w, h) = (
        (w as u32 as i32).unsigned_abs(),
        (h as u32 as i32).unsigned_abs(),
    );
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
    if !is_ico(bytes) {
        return None;
    }
    let count = le_uint(bytes, 4, 2)? as usize;
    // `max_by_key` keeps the LAST maximum, so walk backwards to keep the first.
    (0..count)
        .rev()
        .filter_map(|i| ico_directory_entry(bytes, i))
        .max_by_key(|&(score, _)| score)
        .map(|(_, entry)| entry)
}

/// Directory entry `i`: `((area, bit depth), entry)`.
fn ico_directory_entry(bytes: &[u8], i: usize) -> Option<((u64, u64), IcoEntry<'_>)> {
    let entry = bytes.get(6 + 16 * i..22 + 16 * i)?;
    let field = |at: usize, n: usize| le_uint(entry, at, n).unwrap_or(0) as usize;
    let data = bytes
        .get(field(12, 4)..)
        .and_then(|d| d.get(..field(8, 4)))
        .unwrap_or_default();
    let png = data.starts_with(PNG_MAGIC);
    let (width, height) = if png {
        probe_png(data).map(|i| (i.width, i.height))?
    } else {
        (ico_dim(entry[0]), ico_dim(entry[1]))
    };
    let score = (u64::from(width) * u64::from(height), field(6, 2) as u64);
    Some((
        score,
        IcoEntry {
            width,
            height,
            data,
            png,
        },
    ))
}

/// An ICO directory width/height byte (0 = 256).
fn ico_dim(b: u8) -> u32 {
    if b == 0 {
        256
    } else {
        u32::from(b)
    }
}

/// ICO/CUR intrinsic size: the [`ico_entry`] a browser shows (icons always carry alpha:
/// a 32-bit channel or the AND mask).
pub(super) fn probe_ico(bytes: &[u8]) -> Option<Intrinsic> {
    let entry = ico_entry(bytes)?;
    Some(Intrinsic::new(entry.width, entry.height, true))
}

// --------------------------------------------------------------------------
// ISOBMFF / AVIF
// --------------------------------------------------------------------------

/// One ISOBMFF box: its four-character type and the byte range of its body.
pub(crate) struct BmffBox {
    pub(crate) kind: [u8; 4],
    pub(crate) body: std::ops::Range<usize>,
}

/// The box starting at `at` in `bytes` and the offset just past it (`size` 1 = a 64-bit
/// size follows, 0 = to the end). `None` when truncated or malformed.
fn bmff_box(bytes: &[u8], at: usize) -> Option<(BmffBox, usize)> {
    let size = be_uint(bytes, at, 4)?;
    let kind: [u8; 4] = bytes.get(at + 4..at + 8)?.try_into().ok()?;
    let (start, end) = bmff_extent(bytes, at, size).filter(|&(s, e)| s <= e && e <= bytes.len())?;
    Some((
        BmffBox {
            kind,
            body: start..end,
        },
        end,
    ))
}

/// A box's body `(start, end)` from its 32-bit `size` field.
fn bmff_extent(bytes: &[u8], at: usize, size: u64) -> Option<(usize, usize)> {
    let end = |len: Option<usize>| len.and_then(|n| at.checked_add(n));
    match size {
        0 => Some((at + 8, bytes.len())),
        1 => end(be_usize(bytes, at + 8, 8)).map(|e| (at + 16, e)),
        n => end(usize::try_from(n).ok()).map(|e| (at + 8, e)),
    }
}

/// The child boxes of `range` (a box body, or the whole file), skipping `skip` leading
/// bytes (a full box's version/flags).
pub(super) fn bmff_children(
    bytes: &[u8],
    range: &std::ops::Range<usize>,
    skip: usize,
) -> Vec<BmffBox> {
    let scope = bytes.get(..range.end).unwrap_or_default();
    let mut out = Vec::new();
    let mut at = range.start + skip;
    while let Some((b, next)) = bmff_box(scope, at) {
        out.push(b);
        at = next;
    }
    out
}

/// The first child of `boxes` of type `kind`.
fn child(boxes: &[BmffBox], kind: [u8; 4]) -> Option<&BmffBox> {
    boxes.iter().find(|b| b.kind == kind)
}

/// AVIF magic: a leading `ftyp` box whose major or a compatible brand is `avif`/`avis`.
pub(super) fn is_avif(bytes: &[u8]) -> bool {
    let brand = |b: &[u8]| b == b"avif" || b == b"avis";
    bmff_box(bytes, 0)
        .filter(|(b, _)| b.kind == *b"ftyp")
        .and_then(|(b, _)| bytes.get(b.body))
        .is_some_and(|body| {
            body.get(..4).is_some_and(brand)
                || body
                    .get(8..)
                    .is_some_and(|c| c.as_chunks::<4>().0.iter().any(|b| brand(b)))
        })
}

/// The AVIF item properties: the `meta` children, the primary item id, and for each
/// item its associated property boxes (from `pitm`, `iprp/ipco` + `ipma`).
struct AvifMeta {
    children: Vec<BmffBox>,
    primary: u32,
    properties: Vec<BmffBox>,
    /// `(item id, 1-based property indices)`.
    associations: Vec<(u32, Vec<usize>)>,
}

impl AvifMeta {
    /// Parse the top-level `meta` box of an AVIF file.
    fn parse(bytes: &[u8]) -> Option<AvifMeta> {
        let top = bmff_children(bytes, &(0..bytes.len()), 0);
        let children = bmff_children(bytes, &child(&top, *b"meta")?.body, 4);
        let primary = child(&children, *b"pitm").and_then(|p| pitm_item(bytes, p))?;
        let iprp = bmff_children(bytes, &child(&children, *b"iprp")?.body, 0);
        let properties = bmff_children(bytes, &child(&iprp, *b"ipco")?.body, 0);
        let associations =
            child(&iprp, *b"ipma").map_or_else(Vec::new, |b| parse_ipma(bytes, &b.body));
        Some(AvifMeta {
            children,
            primary,
            properties,
            associations,
        })
    }

    /// The property boxes associated with `item`.
    fn properties_of(&self, item: u32) -> impl Iterator<Item = &BmffBox> {
        self.associations
            .iter()
            .filter(move |(id, _)| *id == item)
            .flat_map(|(_, idx)| idx.iter())
            .filter_map(|&i| self.properties.get(i.checked_sub(1)?))
    }

    /// Whether `item` carries an alpha auxiliary-type property (`auxC` of the
    /// `…:auxiliary:alpha` / `…:auxid:1` URN).
    fn is_alpha(&self, bytes: &[u8], item: u32) -> bool {
        self.properties_of(item).any(|b| is_alpha_aux(bytes, b))
    }
}

/// Whether a property box is an alpha `auxC`.
fn is_alpha_aux(bytes: &[u8], b: &BmffBox) -> bool {
    b.kind == *b"auxC"
        && bytes.get(b.body.clone()).is_some_and(|c| {
            c.windows(5).any(|w| w == b"alpha") || c.windows(7).any(|w| w == b"auxid:1")
        })
}

/// The primary item id of a `pitm` full box (version 0: 16-bit, else 32-bit).
fn pitm_item(bytes: &[u8], pitm: &BmffBox) -> Option<u32> {
    let width = if *bytes.get(pitm.body.start)? == 0 {
        2
    } else {
        4
    };
    be_uint(bytes, pitm.body.start + 4, width).and_then(|v| u32::try_from(v).ok())
}

/// A big-endian cursor over a box body.
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    /// The next `n`-byte big-endian unsigned field (`n` 0 reads 0).
    fn take(&mut self, n: usize) -> Option<u64> {
        let v = be_uint(self.data, self.at, n)?;
        self.at += n;
        Some(v)
    }

    /// [`take`](Self::take) as a `usize`.
    fn size(&mut self, n: usize) -> Option<usize> {
        self.take(n).and_then(|v| usize::try_from(v).ok())
    }
}

/// `ipma` entries: `(item id, property indices)` (version 0: 16-bit item ids; flag 1:
/// 15-bit property indices, else 7-bit; the essential bit is dropped).
fn parse_ipma(bytes: &[u8], body: &std::ops::Range<usize>) -> Vec<(u32, Vec<usize>)> {
    let data = bytes.get(body.clone()).unwrap_or_default();
    let (version, flags) = (
        data.first().copied().unwrap_or(0),
        data.get(3).copied().unwrap_or(0),
    );
    let mut r = Reader { data, at: 4 };
    let count = r.take(4).unwrap_or(0);
    let id_width = if version < 1 { 2 } else { 4 };
    (0..count)
        .map_while(|_| ipma_entry(&mut r, id_width, flags & 1 == 1))
        .collect()
}

/// One `ipma` entry: its item id and property indices.
fn ipma_entry(r: &mut Reader, id_width: usize, wide: bool) -> Option<(u32, Vec<usize>)> {
    let item = u32::try_from(r.take(id_width)?).ok()?;
    let n = r.take(1)?;
    let (width, mask) = if wide { (2, 0x7FFF) } else { (1, 0x7F) };
    let indices = (0..n)
        .map_while(|_| r.size(width).map(|v| v & mask))
        .collect();
    Some((item, indices))
}

/// AVIF intrinsic size: the primary item's `ispe` (image spatial extents); `has_alpha`
/// when the file carries an alpha auxiliary image; the orientation of its HEIF transform
/// properties ([`avif_orientation`], miaskiewicz/turbo-dom#45).
pub(super) fn probe_avif(bytes: &[u8]) -> Option<Intrinsic> {
    let meta = AvifMeta::parse(bytes)?;
    let ispe = meta
        .properties_of(meta.primary)
        .find(|b| b.kind == *b"ispe")?;
    let (w, h) = ispe_size(bytes, ispe)?;
    let alpha = meta.properties.iter().any(|b| is_alpha_aux(bytes, b));
    Some(Intrinsic {
        orientation: avif_orientation(bytes, &meta),
        ..Intrinsic::new(w, h, alpha)
    })
}

/// The orientation the primary item's `irot` (rotation, angle x 90 degrees
/// anti-clockwise) and `imir` (mirror: axis 0 exchanges top and bottom, 1 left and right)
/// properties display it with, applied in association order (HEIF, ISO/IEC 23008-12
/// §6.5.10–6.5.12) — as Chrome 156 displays them under `image-orientation: from-image`.
fn avif_orientation(bytes: &[u8], meta: &AvifMeta) -> Orientation {
    meta.properties_of(meta.primary)
        .filter_map(|b| transform_orientation(b.kind, *bytes.get(b.body.start)?))
        .fold(Orientation::IDENTITY, Orientation::then)
}

/// The orientation of one transform property box (`irot` / `imir`, its first body byte).
fn transform_orientation(kind: [u8; 4], value: u8) -> Option<Orientation> {
    match &kind {
        b"irot" => Some(Orientation([1, 8, 3, 6][usize::from(value & 3)])),
        b"imir" if value & 1 == 0 => Some(Orientation(4)),
        b"imir" => Some(Orientation(2)),
        _ => None,
    }
}

/// An `ispe` property's (non-zero) width and height.
fn ispe_size(bytes: &[u8], ispe: &BmffBox) -> Option<(u32, u32)> {
    let dim =
        |at: usize| be_uint(bytes, ispe.body.start + at, 4).and_then(|v| u32::try_from(v).ok());
    dim(4).zip(dim(8)).filter(|&(w, h)| w > 0 && h > 0)
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

/// One `iloc` item: its id, construction method and `(offset, length)` extents (offsets
/// include the item's base offset).
struct Location {
    item: u32,
    method: u64,
    extents: Vec<(usize, usize)>,
}

/// Demux an AVIF still image (see [`AvifPayload`]): the `iloc` extents of the primary
/// item and of its alpha auxiliary item (construction method 0, file offsets, or 1, the
/// `idat` box). `None` when the file is not a parseable AVIF.
#[must_use]
pub fn avif_payload(bytes: &[u8]) -> Option<AvifPayload> {
    if !is_avif(bytes) {
        return None;
    }
    let meta = AvifMeta::parse(bytes)?;
    let locations = child(&meta.children, *b"iloc").and_then(|b| parse_iloc(bytes, &b.body))?;
    let idat = child(&meta.children, *b"idat").map(|b| b.body.clone());
    let data = |item: u32| item_data(bytes, &locations, idat.as_ref(), item);
    let alpha = meta
        .associations
        .iter()
        .map(|(id, _)| *id)
        .find(|&id| id != meta.primary && meta.is_alpha(bytes, id));
    let nclx = meta
        .properties_of(meta.primary)
        .filter(|b| b.kind == *b"colr")
        .find_map(|b| parse_nclx(bytes.get(b.body.clone())?));
    Some(AvifPayload {
        color: data(meta.primary)?,
        alpha: alpha.and_then(data),
        nclx,
    })
}

/// The concatenated extents of `item` (a zero length runs to the end of the file or
/// `idat`).
fn item_data(
    bytes: &[u8],
    locations: &[Location],
    idat: Option<&std::ops::Range<usize>>,
    item: u32,
) -> Option<Vec<u8>> {
    let location = locations.iter().find(|l| l.item == item)?;
    let scope = item_scope(bytes, location.method, idat)?;
    let mut out = Vec::new();
    for &(offset, length) in &location.extents {
        out.extend_from_slice(extent(bytes, &scope, offset, length)?);
    }
    Some(out)
}

/// The bytes an item's extents index: the file (construction method 0) or the `idat`
/// body (1).
fn item_scope(
    bytes: &[u8],
    method: u64,
    idat: Option<&std::ops::Range<usize>>,
) -> Option<std::ops::Range<usize>> {
    match method {
        0 => Some(0..bytes.len()),
        1 => idat.cloned(),
        _ => None,
    }
}

/// One extent of `scope` (`length` 0: to the end of the scope).
fn extent<'b>(
    bytes: &'b [u8],
    scope: &std::ops::Range<usize>,
    offset: usize,
    length: usize,
) -> Option<&'b [u8]> {
    let start = scope.start.checked_add(offset)?;
    let end = if length == 0 {
        scope.end
    } else {
        start.checked_add(length)?
    };
    bytes.get(start..end)
}

/// An `nclx` `colr` body: `nclx`, three `u16` code points, then the full-range bit.
fn parse_nclx(body: &[u8]) -> Option<Nclx> {
    let code = |at: usize| be_uint(body, at, 2).map(|v| v as u16);
    let ((primaries, transfer), matrix) = code(4).zip(code(6)).zip(code(8))?;
    let flags = *body.get(10).filter(|_| body.starts_with(b"nclx"))?;
    Some(Nclx {
        primaries,
        transfer,
        matrix,
        full_range: flags & 0x80 != 0,
    })
}

/// The `iloc` field widths: `(offset, length, base offset, extent index)` bytes.
struct IlocSizes {
    offset: usize,
    length: usize,
    base: usize,
    index: usize,
    version: u8,
}

/// `iloc` items (versions 0–2).
fn parse_iloc(bytes: &[u8], body: &std::ops::Range<usize>) -> Option<Vec<Location>> {
    let data = bytes.get(body.clone())?;
    let mut r = Reader { data, at: 4 };
    let sizes = iloc_sizes(&mut r, *data.first()?)?;
    let count = r.take(id_width(sizes.version))?;
    (0..count).map(|_| iloc_item(&mut r, &sizes)).collect()
}

/// The `iloc` field widths after its version/flags.
fn iloc_sizes(r: &mut Reader, version: u8) -> Option<IlocSizes> {
    let a = r.size(1)?;
    let b = r.size(1)?;
    Some(IlocSizes {
        offset: a >> 4,
        length: a & 15,
        base: b >> 4,
        index: (b & 15) * usize::from(version >= 1),
        version,
    })
}

/// The width of an `iloc` item id / count field: 16-bit before version 2.
fn id_width(version: u8) -> usize {
    if version < 2 {
        2
    } else {
        4
    }
}

/// One `iloc` item.
fn iloc_item(r: &mut Reader, sizes: &IlocSizes) -> Option<Location> {
    let item = r
        .take(id_width(sizes.version))
        .and_then(|v| u32::try_from(v).ok())?;
    let method = iloc_method(r, sizes.version)?;
    // The data reference index, then the base offset.
    let base = r.take(2).and_then(|_| r.size(sizes.base))?;
    let extents = r
        .take(2)
        .and_then(|count| (0..count).map(|_| iloc_extent(r, sizes, base)).collect())?;
    Some(Location {
        item,
        method,
        extents,
    })
}

/// An item's construction method (version 1+; 0 = file offsets before that).
fn iloc_method(r: &mut Reader, version: u8) -> Option<u64> {
    if version >= 1 {
        r.take(2).map(|m| m & 15)
    } else {
        Some(0)
    }
}

/// One `iloc` extent `(base + offset, length)`.
fn iloc_extent(r: &mut Reader, sizes: &IlocSizes, base: usize) -> Option<(usize, usize)> {
    r.take(sizes.index)?;
    let offset = r.size(sizes.offset)?;
    let length = r.size(sizes.length)?;
    Some((base.checked_add(offset)?, length))
}

// --------------------------------------------------------------------------
// data: URLs
// --------------------------------------------------------------------------

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
    let (mime, base64, payload) = split_data_url(url)?;
    let decoded = percent_decode(payload.as_bytes());
    let bytes = if base64 {
        forgiving_base64(&decoded)?
    } else {
        decoded
    };
    Some(DataUrl { mime, bytes })
}

/// A `data:` URL's `(MIME type, base64?, payload)`.
fn split_data_url(url: &str) -> Option<(String, bool, &str)> {
    let rest = url.trim_start();
    let body = rest
        .get(..5)
        .filter(|s| s.eq_ignore_ascii_case("data:"))
        .map(|_| &rest[5..])?;
    let (meta, payload) = body.split_once(',')?;
    let meta = meta.trim();
    let (mime, base64) = match meta.rsplit_once(';') {
        Some((head, last)) if last.trim().eq_ignore_ascii_case("base64") => (head.trim(), true),
        _ => (meta, false),
    };
    Some((default_mime(mime), base64, payload))
}

/// The MIME type, defaulting to `text/plain;charset=US-ASCII` (keeping any parameters).
fn default_mime(mime: &str) -> String {
    if mime.is_empty() || mime.starts_with(';') {
        format!("text/plain;charset=US-ASCII{mime}")
    } else {
        mime.to_string()
    }
}

/// `%XX` escapes to bytes; a `%` not followed by two hex digits stays literal.
fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if let Some(byte) = percent_escape(input, i) {
            out.push(byte);
            i += 3;
        } else {
            out.push(input[i]);
            i += 1;
        }
    }
    out
}

/// The byte of a `%XX` escape at `i`.
fn percent_escape(input: &[u8], i: usize) -> Option<u8> {
    let hex = |at: usize| input.get(at).and_then(|&b| (b as char).to_digit(16));
    let (h, l) = hex(i + 1).zip(hex(i + 2)).filter(|_| input[i] == b'%')?;
    Some((h * 16 + l) as u8)
}

/// The WHATWG forgiving-base64 decode: ASCII whitespace dropped, `=` padding optional
/// (but only at the end), any other non-alphabet byte or a length of 1 mod 4 fails.
fn forgiving_base64(input: &[u8]) -> Option<Vec<u8>> {
    let data = strip_padding(
        input
            .iter()
            .copied()
            .filter(|b| !b.is_ascii_whitespace())
            .collect(),
    );
    if data.len() % 4 == 1 {
        return None;
    }
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

/// Drop one or two trailing `=` from a whole number of base64 quanta.
fn strip_padding(mut data: Vec<u8>) -> Vec<u8> {
    if data.len().is_multiple_of(4) {
        for _ in 0..2 {
            data.pop_if(|b| *b == b'=');
        }
    }
    data
}

/// A standard-alphabet base64 character's value.
fn sextet(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some(u32::from(c - b'A')),
        b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// The natural size a browser reports for an image (`naturalWidth`/`naturalHeight`): its
/// displayed size (EXIF orientation applied — Chrome reports it even under
/// `image-orientation: none`) for every raster format [`probe`](super::probe) reads,
/// and, under `svg-rasterize`, an SVG's [`crate::svg::natural_size`] rounded to whole
/// pixels. `None` for bytes a browser would not decode. The probe a live `<img>` load
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
    super::sniff(bytes)
        .and_then(|_| super::probe(bytes))
        .map(|i| i.displayed(true))
}

/// Which non-legacy container `bytes` are (for [`super::sniff`]).
pub(super) fn sniff_extra(bytes: &[u8]) -> Option<Format> {
    [
        (is_bmp as fn(&[u8]) -> bool, Format::Bmp),
        (is_ico, Format::Ico),
        (is_avif, Format::Avif),
        (super::jxl::is_jxl, Format::Jxl),
    ]
    .into_iter()
    .find(|(is, _)| is(bytes))
    .map(|(_, format)| format)
}

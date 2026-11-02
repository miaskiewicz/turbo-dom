//! JPEG XL (ISO/IEC 18181) header probe (miaskiewicz/turbo-dom#45 item 2): Chrome 156
//! decodes JPEG XL, so an `<img>` of one lays out at its size like any other image. The
//! size and orientation come from the codestream's `SizeHeader` and the start of its
//! `ImageMetadata` — no pixel decode (turbo-dom-raster decodes it for paint under its `jxl`
//! feature). Both forms are read: the bare codestream (`FF 0A`) and the ISOBMFF container
//! (`JXL ` signature box), whose codestream is the `jxlc` box or the `jxlp` boxes.

use super::formats::{bmff_children, Orientation};
use super::Intrinsic;

/// The bare codestream signature.
const CODESTREAM: [u8; 2] = [0xFF, 0x0A];

/// The container's leading signature box (`size 12`, type `JXL `, `0D 0A 87 0A`).
const CONTAINER: [u8; 12] = [
    0, 0, 0, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A,
];

/// JPEG XL magic: a bare codestream or the container signature.
pub(super) fn is_jxl(bytes: &[u8]) -> bool {
    bytes.starts_with(&CODESTREAM) || bytes.starts_with(&CONTAINER)
}

/// JPEG XL intrinsic size and orientation (alpha is not read: only the PDF needs it, and
/// the PDF does not embed JPEG XL).
pub(super) fn probe_jxl(bytes: &[u8]) -> Option<Intrinsic> {
    let codestream = codestream(bytes)?;
    let mut r = Bits::new(codestream.strip_prefix(&CODESTREAM)?);
    let (width, height) = size_header(&mut r)?;
    let orientation = orientation(&mut r)?;
    Some(Intrinsic {
        orientation,
        ..Intrinsic::new(width, height, false)
    })
}

/// The codestream: the file itself, or the container's `jxlc` box (else its first
/// `jxlp` box, after the 4-byte part index).
fn codestream(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.starts_with(&CODESTREAM) {
        return Some(bytes);
    }
    let boxes = bmff_children(bytes, &(0..bytes.len()), 0);
    if let Some(b) = boxes.iter().find(|b| b.kind == *b"jxlc") {
        return bytes.get(b.body.clone());
    }
    let part = boxes.iter().find(|b| b.kind == *b"jxlp")?;
    bytes.get(part.body.start + 4..part.body.end)
}

/// `SizeHeader`: `(xsize, ysize)`.
fn size_header(r: &mut Bits) -> Option<(u32, u32)> {
    let small = r.bit()?;
    let height = dimension(r, small)?;
    let ratio = r.take(3)?;
    let width = if ratio == 0 {
        dimension(r, small)?
    } else {
        fixed_ratio(height, ratio)
    };
    Some((width, height))
}

/// One dimension: `(Bits(5) + 1) * 8` when small, else `U32(BitsOffset(9, 1),
/// BitsOffset(13, 1), BitsOffset(18, 1), BitsOffset(30, 1))`.
fn dimension(r: &mut Bits, small: bool) -> Option<u32> {
    if small {
        return Some((r.take(5)? + 1) * 8);
    }
    let bits = [9, 13, 18, 30][r.take(2)? as usize];
    Some(r.take(bits)? + 1)
}

/// The width of a `ratio` (1..=7) aspect of `height`: 1:1, 12:10, 4:3, 3:2, 16:9, 5:4, 2:1.
fn fixed_ratio(height: u32, ratio: u32) -> u32 {
    let (num, den) =
        [(1, 1), (12, 10), (4, 3), (3, 2), (16, 9), (5, 4), (2, 1)][ratio as usize - 1];
    (u64::from(height) * num / den) as u32
}

/// `ImageMetadata`'s orientation: identity when `all_default` or without `extra_fields`,
/// else `1 + Bits(3)` (the EXIF values).
fn orientation(r: &mut Bits) -> Option<Orientation> {
    if r.bit()? || !r.bit()? {
        return Some(Orientation::IDENTITY);
    }
    Some(Orientation::from_exif((r.take(3)? + 1) as u16))
}

/// A JPEG XL bit reader: least significant bit first.
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Bits<'a> {
        Bits { data, at: 0 }
    }

    fn bit(&mut self) -> Option<bool> {
        let byte = self.data.get(self.at / 8)?;
        let bit = byte >> (self.at % 8) & 1 == 1;
        self.at += 1;
        Some(bit)
    }

    /// `Bits(n)` (n <= 32): the first bit read is the least significant.
    fn take(&mut self, n: u32) -> Option<u32> {
        (0..n).try_fold(0u32, |v, i| Some(v | u32::from(self.bit()?) << i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A codestream header from `(value, bit count)` fields, least significant bit first.
    fn header(fields: &[(u32, u32)]) -> Vec<u8> {
        let mut out = CODESTREAM.to_vec();
        let mut at = 0usize;
        for &(value, n) in fields {
            for i in 0..n {
                if at.is_multiple_of(8) {
                    out.push(0);
                }
                *out.last_mut().unwrap() |= u8::from(value >> i & 1 == 1) << (at % 8);
                at += 1;
            }
        }
        out
    }

    fn probe(fields: &[(u32, u32)]) -> Option<(u32, u32, u8)> {
        probe_jxl(&header(fields)).map(|i| (i.width, i.height, i.orientation.exif()))
    }

    #[test]
    fn size_header_forms() {
        // small: (Bits(5) + 1) * 8 each; ratio 0 reads the width.
        assert_eq!(
            probe(&[(1, 1), (1, 5), (0, 3), (3, 5), (1, 1)]),
            Some((32, 16, 1))
        );
        // large: U32 selector + bits + 1; every fixed ratio.
        let large = |sel: u32, bits: u32, v: u32, ratio: u32| {
            probe(&[(0, 1), (sel, 2), (v, bits), (ratio, 3), (1, 1)]).map(|(w, h, _)| (w, h))
        };
        assert_eq!(large(0, 9, 99, 1), Some((100, 100)));
        assert_eq!(large(1, 13, 99, 2), Some((120, 100)));
        assert_eq!(large(2, 18, 89, 3), Some((120, 90)));
        assert_eq!(large(3, 30, 99, 4), Some((150, 100)));
        assert_eq!(large(0, 9, 89, 5), Some((160, 90)));
        assert_eq!(large(0, 9, 99, 6), Some((125, 100)));
        assert_eq!(large(0, 9, 99, 7), Some((200, 100)));
        // A large width after ratio 0.
        assert_eq!(
            probe(&[(0, 1), (0, 2), (9, 9), (0, 3), (1, 2), (19, 13), (1, 1)]),
            Some((20, 10, 1))
        );
    }

    #[test]
    fn orientation_from_the_metadata_extra_fields() {
        let small = [(1, 1), (0, 5), (1, 3)];
        let with = |more: &[(u32, u32)]| probe(&[&small[..], more].concat()).map(|p| p.2);
        assert_eq!(with(&[(1, 1)]), Some(1), "all_default");
        assert_eq!(with(&[(0, 1), (0, 1)]), Some(1), "no extra fields");
        assert_eq!(with(&[(0, 1), (1, 1), (5, 3)]), Some(6));
        assert_eq!(with(&[(0, 1), (1, 1), (7, 3)]), Some(8));
    }

    #[test]
    fn truncated_or_foreign_input_is_none() {
        assert!(probe_jxl(&CODESTREAM).is_none());
        assert!(probe(&[(1, 1), (0, 5)]).is_none(), "no ratio");
        assert!(
            probe(&[(0, 1), (0, 2), (9, 9), (1, 3)]).is_none(),
            "15 bits: all_default false, no extra_fields bit"
        );
        assert!(probe_jxl(b"\x89PNG").is_none());
        // A container without a codestream box, and one with a truncated jxlp.
        let mut container = CONTAINER.to_vec();
        assert!(probe_jxl(&container).is_none());
        container.extend_from_slice(&[0, 0, 0, 10, b'j', b'x', b'l', b'p', 0, 0]);
        assert!(probe_jxl(&container).is_none());
        assert!(is_jxl(&container) && !is_jxl(b"JXL"));
    }

    #[test]
    fn a_jxlp_part_holds_the_codestream() {
        let stream = header(&[(1, 1), (0, 5), (1, 3), (1, 1)]);
        let mut container = CONTAINER.to_vec();
        let size = 8 + 4 + stream.len() as u32;
        container.extend_from_slice(&size.to_be_bytes());
        container.extend_from_slice(b"jxlp");
        container.extend_from_slice(&0u32.to_be_bytes());
        container.extend_from_slice(&stream);
        assert_eq!(
            probe_jxl(&container).map(|i| (i.width, i.height)),
            Some((8, 8))
        );
    }
}

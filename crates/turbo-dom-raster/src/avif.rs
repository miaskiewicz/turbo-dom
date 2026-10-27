//! AVIF pixel decode for screenshots (miaskiewicz/turbo-surf#30), behind the off-by-default
//! `avif` feature.
//!
//! Sizing never needs this: turbo-dom-layout's shared probe reads an AVIF's `ispe` (and
//! [`turbo_dom_layout::image::avif_payload`] demuxes its AV1 streams) in pure Rust, so an
//! AVIF image box always lays out at its real size. Painting its pixels needs an AV1
//! decoder. The options assessed (2026-10): the `image` crate's AVIF decoding is
//! `avif-native` only — libdav1d, a native C library found through pkg-config; `ravif`
//! encodes only; `zenavif` / `rav1d-safe` are AGPL; `rav1d` (1.1, BSD-2-Clause, the ISRG
//! Prossimo Rust port of dav1d) is pure Rust with its assembly off
//! (`default-features = false`), at the cost of its C-ABI (`unsafe extern "C"`) entry
//! points and a large dependency tree. This module uses `rav1d` (no assembly, one
//! thread) and converts the decoded YUV planes to RGBA itself; without the feature an
//! AVIF paints as the placeholder of any undecodable image.

use turbo_dom_layout::image::Rgba8Image;

/// Decode an AVIF still image to straight RGBA8 (`None` without the `avif` feature, or
/// on any container / bitstream error).
#[cfg(not(feature = "avif"))]
pub fn decode(_bytes: &[u8]) -> Option<Rgba8Image> {
    None
}

/// Decode an AVIF still image to straight RGBA8, or `None` on any container or
/// bitstream error.
#[cfg(feature = "avif")]
pub fn decode(bytes: &[u8]) -> Option<Rgba8Image> {
    let payload = turbo_dom_layout::image::avif_payload(bytes)?;
    let color = av1::decode(&payload.color)?;
    // An alpha plane that fails to decode or does not match the colour frame is dropped
    // (the image paints opaque) rather than failing the whole image.
    let alpha = payload
        .alpha
        .as_deref()
        .and_then(av1::decode)
        .filter(|a| (a.width, a.height) == (color.width, color.height));
    let (matrix, full_range) = match payload.nclx {
        Some(n) => (u32::from(n.matrix), n.full_range),
        None => (color.matrix, color.full_range),
    };
    Some(yuv::to_rgba(&color, alpha.as_ref(), matrix, full_range))
}

/// One decoded AV1 frame: planes widened to `u16`, chroma subsampling, bit depth and the
/// sequence header's colour description.
#[cfg(feature = "avif")]
pub(crate) struct Frame {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) bits: u32,
    /// Chroma subsampling shifts `(x, y)`; `None` for a monochrome (4:0:0) frame.
    pub(crate) chroma: Option<(usize, usize)>,
    pub(crate) planes: [Vec<u16>; 3],
    pub(crate) matrix: u32,
    pub(crate) full_range: bool,
}

#[cfg(feature = "avif")]
mod av1 {
    //! The `rav1d` C-ABI calls: open a one-thread decoder, feed one temporal unit,
    //! take the picture, copy its planes out, release everything.
    use std::ptr::NonNull;

    use rav1d::include::dav1d::data::Dav1dData;
    use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
    use rav1d::include::dav1d::headers::{
        DAV1D_PIXEL_LAYOUT_I400, DAV1D_PIXEL_LAYOUT_I420, DAV1D_PIXEL_LAYOUT_I422,
    };
    use rav1d::include::dav1d::picture::Dav1dPicture;
    use rav1d::src::lib::{
        dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings,
        dav1d_get_picture, dav1d_open, dav1d_picture_unref, dav1d_send_data,
    };

    use super::Frame;

    const AGAIN: i32 = -libc::EAGAIN;

    /// Decode one AV1 still frame (an AVIF item's OBU stream).
    pub(crate) fn decode(stream: &[u8]) -> Option<Frame> {
        if stream.is_empty() {
            return None;
        }
        let mut settings = std::mem::MaybeUninit::<Dav1dSettings>::uninit();
        // SAFETY: `settings` is valid to write the default settings into.
        unsafe { dav1d_default_settings(NonNull::new(settings.as_mut_ptr())?) };
        // SAFETY: `dav1d_default_settings` initialized every field.
        let mut settings = unsafe { settings.assume_init() };
        settings.n_threads = 1;
        settings.max_frame_delay = 1;
        let mut ctx: Option<Dav1dContext> = None;
        // SAFETY: `ctx` is valid to write to and `settings` to read from.
        let opened =
            unsafe { dav1d_open(NonNull::new(&raw mut ctx), NonNull::new(&raw mut settings)) };
        if opened.0 != 0 {
            return None;
        }
        let frame = decode_with(ctx, stream);
        // SAFETY: `ctx` came from `dav1d_open` and is closed exactly once.
        unsafe { dav1d_close(NonNull::new(&raw mut ctx)) };
        frame
    }

    fn decode_with(ctx: Option<Dav1dContext>, stream: &[u8]) -> Option<Frame> {
        let mut data = Dav1dData::default();
        // SAFETY: `data` is valid to write the new buffer into.
        let buffer = unsafe { dav1d_data_create(NonNull::new(&raw mut data), stream.len()) };
        if buffer.is_null() {
            return None;
        }
        // SAFETY: `dav1d_data_create` returned a writable buffer of `stream.len()` bytes.
        unsafe { std::ptr::copy_nonoverlapping(stream.as_ptr(), buffer, stream.len()) };
        let mut picture = Dav1dPicture::default();
        let mut got = false;
        for _ in 0..16 {
            if data.sz > 0 {
                // SAFETY: `ctx` is open; `data` is a valid, owned data buffer.
                let sent = unsafe { dav1d_send_data(ctx, NonNull::new(&raw mut data)) };
                if sent.0 != 0 && sent.0 != AGAIN {
                    break;
                }
            }
            // SAFETY: `ctx` is open; `picture` is valid to write to.
            let res = unsafe { dav1d_get_picture(ctx, NonNull::new(&raw mut picture)) };
            if res.0 == 0 {
                got = true;
                break;
            }
            if res.0 != AGAIN {
                break;
            }
        }
        if data.sz > 0 {
            // SAFETY: `data` still owns its (unconsumed) buffer.
            unsafe { dav1d_data_unref(NonNull::new(&raw mut data)) };
        }
        if !got {
            return None;
        }
        let frame = copy_frame(&picture);
        // SAFETY: `picture` was filled by `dav1d_get_picture` and is released once.
        unsafe { dav1d_picture_unref(NonNull::new(&raw mut picture)) };
        frame
    }

    /// Copy a picture's planes out as `u16` samples.
    fn copy_frame(picture: &Dav1dPicture) -> Option<Frame> {
        let width = usize::try_from(picture.p.w).ok()?;
        let height = usize::try_from(picture.p.h).ok()?;
        let bits = u32::try_from(picture.p.bpc).ok()?;
        let chroma = match picture.p.layout {
            DAV1D_PIXEL_LAYOUT_I400 => None,
            DAV1D_PIXEL_LAYOUT_I420 => Some((1, 1)),
            DAV1D_PIXEL_LAYOUT_I422 => Some((1, 0)),
            _ => Some((0, 0)),
        };
        // SAFETY: a decoded picture carries its sequence header.
        let seq = unsafe { picture.seq_hdr?.as_ref() };
        let wide = bits > 8;
        let plane = |i: usize, w: usize, h: usize| -> Option<Vec<u16>> {
            let base = picture.data[i]?.as_ptr().cast::<u8>();
            let stride = picture.stride[usize::from(i > 0)];
            let row_bytes = w * if wide { 2 } else { 1 };
            let mut out = Vec::with_capacity(w * h);
            for y in 0..h {
                // SAFETY: rows `0..h` of `row_bytes` bytes lie inside the plane at `stride`.
                let row = unsafe {
                    std::slice::from_raw_parts(base.offset(stride * y as isize), row_bytes)
                };
                if wide {
                    out.extend(
                        row.as_chunks::<2>()
                            .0
                            .iter()
                            .map(|&b| u16::from_ne_bytes(b)),
                    );
                } else {
                    out.extend(row.iter().map(|&b| u16::from(b)));
                }
            }
            Some(out)
        };
        let luma = plane(0, width, height)?;
        let (u, v) = match chroma {
            Some((sx, sy)) => {
                let (cw, ch) = ((width + sx) >> sx, (height + sy) >> sy);
                (plane(1, cw, ch)?, plane(2, cw, ch)?)
            }
            None => (Vec::new(), Vec::new()),
        };
        Some(Frame {
            width,
            height,
            bits,
            chroma,
            planes: [luma, u, v],
            matrix: seq.mtrx,
            full_range: seq.color_range != 0,
        })
    }
}

#[cfg(feature = "avif")]
pub(crate) mod yuv {
    //! YUV (ITU-T H.273 matrix coefficients) to straight RGBA8.
    use turbo_dom_layout::image::Rgba8Image;

    use super::Frame;

    /// The `(Kr, Kb)` luma weights of a matrix-coefficients code point (BT.709 = 1,
    /// BT.2020 = 9/10, FCC = 4, SMPTE 240M = 7; BT.601 for 5/6 and, as libavif and
    /// Chrome do, for unspecified 2 and anything else). `None` = identity (GBR, 0).
    pub(crate) fn weights(matrix: u32) -> Option<(f32, f32)> {
        match matrix {
            0 => None,
            1 => Some((0.2126, 0.0722)),
            4 => Some((0.30, 0.11)),
            7 => Some((0.212, 0.087)),
            9 | 10 => Some((0.2627, 0.0593)),
            _ => Some((0.299, 0.114)),
        }
    }

    /// Convert a decoded colour frame (plus an optional alpha frame of the same size) to
    /// straight RGBA8 (nearest-neighbour chroma upsampling).
    pub(crate) fn to_rgba(
        color: &Frame,
        alpha: Option<&Frame>,
        matrix: u32,
        full: bool,
    ) -> Rgba8Image {
        let max = ((1u32 << color.bits) - 1) as f32;
        let scale = max / 255.0;
        let (luma_off, luma_range, chroma_range) = if full {
            (0.0, max, max)
        } else {
            (16.0 * scale, 219.0 * scale, 224.0 * scale)
        };
        let mid = (1u32 << (color.bits - 1)) as f32;
        let weights = weights(matrix);
        let clamp = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
        let alpha_max = alpha.map_or(1.0, |a| ((1u32 << a.bits) - 1) as f32);
        let mut rgba = Vec::with_capacity(color.width * color.height * 4);
        for y in 0..color.height {
            for x in 0..color.width {
                let luma = f32::from(color.planes[0][y * color.width + x]);
                let (cb, cr) = color.chroma.map_or((mid, mid), |(sx, sy)| {
                    let cw = (color.width + sx) >> sx;
                    let at = (y >> sy) * cw + (x >> sx);
                    (
                        f32::from(color.planes[1][at]),
                        f32::from(color.planes[2][at]),
                    )
                });
                let yn = (luma - luma_off) / luma_range;
                let (u, v) = ((cb - mid) / chroma_range, (cr - mid) / chroma_range);
                let (r, g, b) = match weights {
                    None if color.chroma.is_some() => (v + 0.5, yn, u + 0.5),
                    None => (yn, yn, yn),
                    Some(_) if color.chroma.is_none() => (yn, yn, yn),
                    Some((kr, kb)) => {
                        let kg = 1.0 - kr - kb;
                        let r = yn + 2.0 * (1.0 - kr) * v;
                        let b = yn + 2.0 * (1.0 - kb) * u;
                        (r, (yn - kr * r - kb * b) / kg, b)
                    }
                };
                let a = alpha.map_or(255, |a| {
                    let s = f32::from(a.planes[0][y * a.width + x]);
                    clamp(s / alpha_max)
                });
                rgba.extend_from_slice(&[clamp(r), clamp(g), clamp(b), a]);
            }
        }
        Rgba8Image {
            width: color.width as u32,
            height: color.height as u32,
            rgba,
        }
    }
}

#[cfg(all(test, feature = "avif"))]
mod tests {
    use super::decode;

    fn q_avif() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../turbo-dom-layout/tests/fixtures/images/formats/q.avif"
        ))
        .expect("q.avif")
    }

    /// `[start, end)` of q.avif's `mdat` payload: the colour item's AV1 stream alone.
    fn mdat(bytes: &[u8]) -> (usize, usize) {
        let mut at = 0;
        while at + 8 <= bytes.len() {
            let size = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            if &bytes[at + 4..at + 8] == b"mdat" {
                return (at + 8, at + size);
            }
            at += size;
        }
        panic!("no mdat");
    }

    #[test]
    #[ignore = "turbo-dom#109: fails (aborts) until the fix"]
    fn corrupt_av1_data_is_an_error_not_an_abort() {
        let intact = q_avif();
        let decoded = decode(&intact).expect("intact q.avif decodes");
        assert_eq!((decoded.width, decoded.height), (64, 40));
        let (start, end) = mdat(&intact);
        // The colour stream overwritten from every offset on with three fills: rav1d 1.1.0
        // aborted the process on most (its single-frame-context error path unwrapped a
        // taken frame header) instead of failing the decode.
        let mut undecodable = 0;
        for fill in [0xFF, 0x00, 0x55] {
            for from in start..end {
                let mut bytes = intact.clone();
                bytes[from..end].fill(fill);
                if decode(&bytes).is_none() {
                    undecodable += 1;
                }
            }
        }
        assert!(undecodable > 0);
        // The turbo-dom#107 frame-corrupt variant: the second half filled with 0xFF.
        let mut corrupt = intact.clone();
        corrupt[start + (end - start) / 2..end].fill(0xFF);
        assert!(decode(&corrupt).is_none());
    }
}

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
//! points and a large dependency tree. This module uses `rav1d` (no assembly, two frame
//! contexts — see [`av1::decode`] for why not one) and converts the decoded YUV planes to
//! RGBA itself; without the feature, or for a corrupt AVIF, an AVIF paints as the
//! placeholder of any undecodable image.

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
    //! The `rav1d` C-ABI calls: open a two-frame-context decoder, feed one temporal unit,
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
        // Two frame contexts (frame delay 2, two worker threads): with exactly one, rav1d
        // 1.1.0 tears a frame down twice on a decode error and panics inside its
        // `extern "C"` API, which aborts the process (miaskiewicz/turbo-dom#94, upstream
        // memorysafety/rav1d#1496, fixed after 1.1.0; 1.1.1 is unpublished). With two, a
        // decode error comes back as an error result. AV1 decoding is bit-exact, so the
        // pixels do not depend on the thread count.
        settings.n_threads = 2;
        settings.max_frame_delay = 2;
        let mut ctx: Option<Dav1dContext> = None;
        // SAFETY: `ctx` is valid to write to and `settings` to read from.
        let opened =
            unsafe { dav1d_open(NonNull::new(&raw mut ctx), NonNull::new(&raw mut settings)) };
        // Opening fails only when rav1d cannot allocate the context (these settings pass its
        // validation); nothing decodes then, and closing the unopened (`None`) context is a
        // no-op.
        let frame = (opened.0 == 0).then(|| decode_with(ctx, stream)).flatten();
        // SAFETY: `ctx` came from `dav1d_open` (or stayed `None`) and is closed exactly once.
        unsafe { dav1d_close(NonNull::new(&raw mut ctx)) };
        frame
    }

    fn decode_with(ctx: Option<Dav1dContext>, stream: &[u8]) -> Option<Frame> {
        let mut data = Dav1dData::default();
        // SAFETY: `data` is valid to write the new buffer into. Null only when rav1d cannot
        // allocate it (a slice's length is always within its `usize::MAX / 2` bound).
        let buffer =
            NonNull::new(unsafe { dav1d_data_create(NonNull::new(&raw mut data), stream.len()) })?;
        // SAFETY: `dav1d_data_create` returned a writable buffer of `stream.len()` bytes.
        unsafe { std::ptr::copy_nonoverlapping(stream.as_ptr(), buffer.as_ptr(), stream.len()) };
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
            // A picture (0) or a decode error ends the loop; `EAGAIN` asks for more input.
            if res.0 != AGAIN {
                got = res.0 == 0;
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
    use super::yuv::{to_rgba, weights};
    use super::{av1, Frame};

    /// The colour stream of a turbo-dom-layout format fixture (`q.avif`: 64x40, 8-bit).
    fn stream() -> Vec<u8> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../turbo-dom-layout/tests/fixtures/images/formats/q.avif"
        );
        let avif = std::fs::read(path).unwrap();
        turbo_dom_layout::image::avif_payload(&avif).unwrap().color
    }

    /// Every AVIF fixture of turbo-dom-layout and this crate.
    fn fixtures() -> Vec<(String, Vec<u8>)> {
        let root = env!("CARGO_MANIFEST_DIR");
        let mut out = Vec::new();
        for dir in [
            format!("{root}/../turbo-dom-layout/tests/fixtures/images/formats"),
            format!("{root}/tests/fixtures/images/avif"),
        ] {
            let mut paths: Vec<_> = std::fs::read_dir(&dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|e| e == "avif"))
                .collect();
            paths.sort();
            for p in paths {
                out.push((p.display().to_string(), std::fs::read(&p).unwrap()));
            }
        }
        out
    }

    /// The fuzz body [`corrupt_avifs_are_broken_images_never_a_process_abort`] runs in a
    /// child process (an abort inside the decoder takes the whole process down, which a
    /// test in this process could not observe): every AVIF fixture truncated at each
    /// sixteenth and with single bytes flipped across the file, decoded end to end.
    #[test]
    #[ignore = "run in a child process by corrupt_avifs_are_broken_images_never_a_process_abort"]
    fn corrupt_avif_fuzz_child() {
        let (mut decoded, mut rejected) = (0, 0);
        for (label, avif) in fixtures() {
            let mut variants: Vec<Vec<u8>> = (1..16)
                .map(|i| avif[..avif.len() * i / 16].to_vec())
                .collect();
            for at in (0..avif.len()).step_by(3) {
                for mask in [0x01, 0x80, 0xff] {
                    let mut v = avif.clone();
                    v[at] ^= mask;
                    variants.push(v);
                }
            }
            for v in variants {
                match super::decode(&v) {
                    Some(img) => {
                        assert_eq!(
                            img.rgba.len(),
                            img.width as usize * img.height as usize * 4,
                            "{label}"
                        );
                        decoded += 1;
                    }
                    None => rejected += 1,
                }
            }
        }
        println!("FUZZ-DONE decoded={decoded} rejected={rejected}");
    }

    /// A corrupt AVIF decodes to `None` (the box paints as a broken image) — it never
    /// aborts the process (miaskiewicz/turbo-dom#94: rav1d 1.1.0 panicked inside its
    /// `extern "C"` API on a decode error with one frame context, aborting).
    #[test]
    fn corrupt_avifs_are_broken_images_never_a_process_abort() {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "avif::tests::corrupt_avif_fuzz_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success() && stdout.contains("FUZZ-DONE"),
            "child {:?}\n{stdout}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .take(12)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn a_colour_stream_decodes_to_its_frame() {
        let frame = av1::decode(&stream()).expect("decodes");
        assert_eq!((frame.width, frame.height, frame.bits), (64, 40, 8));
        assert_eq!(frame.planes[0].len(), 64 * 40);
    }

    #[test]
    fn malformed_and_incomplete_streams_decode_to_nothing() {
        let good = stream();
        assert!(av1::decode(&[]).is_none(), "empty");
        // OBUs rav1d cannot parse: all-ones bytes (a header whose size overruns the data),
        // zeros (an unknown OBU type), the stream cut in half, and its middle third
        // scrambled.
        let n = good.len();
        let mut scrambled = good.clone();
        scrambled[n / 3..n / 2].iter_mut().for_each(|b| *b ^= 0x5a);
        for (label, bad) in [
            ("ones", vec![0xff; 32]),
            ("zeros", vec![0; 32]),
            ("half", good[..n / 2].to_vec()),
            ("scrambled", scrambled),
        ] {
            assert!(av1::decode(&bad).is_none(), "{label}");
        }
        // Only the temporal delimiter and sequence header (no frame): no picture comes.
        assert!(av1::decode(&good[..n / 8]).is_none(), "header only");
    }

    /// A one-row frame of `bits` depth with the given planes.
    fn frame(bits: u32, chroma: Option<(usize, usize)>, planes: [Vec<u16>; 3]) -> Frame {
        Frame {
            width: planes[0].len(),
            height: 1,
            bits,
            chroma,
            planes,
            matrix: 2,
            full_range: true,
        }
    }

    /// `rgb` (0..1) through the H.273 forward transform of `(kr, kb)` to full-range
    /// 8-bit `Y'CbCr`, then back through [`to_rgba`] for `matrix`.
    fn round_trip(matrix: u32, (kr, kb): (f32, f32), rgb: [f32; 3]) -> [u8; 4] {
        let [r, g, b] = rgb;
        let y = kr * r + (1.0 - kr - kb) * g + kb * b;
        let cb = (b - y) / (2.0 * (1.0 - kb));
        let cr = (r - y) / (2.0 * (1.0 - kr));
        let q = |v: f32| (v * 255.0).round() as u16;
        let f = frame(
            8,
            Some((0, 0)),
            [
                vec![q(y)],
                vec![q(cb + 128.0 / 255.0)],
                vec![q(cr + 128.0 / 255.0)],
            ],
        );
        let out = to_rgba(&f, None, matrix, true).rgba;
        [out[0], out[1], out[2], out[3]]
    }

    #[test]
    fn each_matrix_inverts_its_own_forward_transform() {
        // BT.709, FCC, SMPTE 240M, BT.2020 (NCL and CL weights), BT.601 (5, 6) and the
        // BT.601 default for unspecified (2) and any other code point.
        let cases = [
            (1, (0.2126, 0.0722)),
            (4, (0.30, 0.11)),
            (7, (0.212, 0.087)),
            (9, (0.2627, 0.0593)),
            (10, (0.2627, 0.0593)),
            (5, (0.299, 0.114)),
            (6, (0.299, 0.114)),
            (2, (0.299, 0.114)),
            (99, (0.299, 0.114)),
        ];
        for (matrix, k) in cases {
            assert_eq!(weights(matrix), Some(k), "{matrix}");
            for rgb in [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 1.0, 0.0],
                [0.5, 0.5, 0.5],
            ] {
                let got = round_trip(matrix, k, rgb);
                let want = rgb.map(|v| (v * 255.0).round() as u8);
                assert!(
                    got[..3].iter().zip(&want).all(|(g, w)| g.abs_diff(*w) <= 2) && got[3] == 255,
                    "matrix {matrix} {rgb:?}: {got:?}"
                );
            }
        }
        assert_eq!(weights(0), None, "identity");
    }

    #[test]
    fn monochrome_frames_are_grey_under_any_matrix() {
        // A monochrome frame under identity or any weighted matrix is grey from luma alone.
        let mono = frame(8, None, [vec![0, 128, 255], vec![], vec![]]);
        let grey = [0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255];
        assert_eq!(to_rgba(&mono, None, 0, true).rgba, grey);
        assert_eq!(to_rgba(&mono, None, 1, true).rgba, grey);
    }

    /// Identity (GBR, matrix 0; H.273 equations 41-43): G = Y, B = Cb, R = Cr, every plane
    /// quantized like luma — as libavif (Chrome's AVIF decoder) converts it.
    #[test]
    #[ignore = "turbo-dom#95: fails until the fix"]
    fn identity_gbr_maps_every_sample_exactly() {
        let all: Vec<u16> = (0..=255).collect();
        let rev: Vec<u16> = (0..=255).rev().collect();
        let mid: Vec<u16> = (0..=255).map(|v| (v + 77) % 256).collect();
        let full = frame(8, Some((0, 0)), [all.clone(), rev.clone(), mid.clone()]);
        let px = to_rgba(&full, None, 0, true).rgba;
        for i in 0..256 {
            let want = [mid[i] as u8, all[i] as u8, rev[i] as u8, 255];
            assert_eq!(px[i * 4..i * 4 + 4], want, "full range sample {i}");
        }
        // Limited range: 16..=235 on every plane is 0..=255.
        let limited = frame(
            8,
            Some((0, 0)),
            [vec![126, 16, 235], vec![16, 235, 126], vec![235, 126, 16]],
        );
        assert_eq!(
            to_rgba(&limited, None, 0, false).rgba,
            [255, 128, 0, 255, 128, 0, 255, 255, 0, 255, 128, 255]
        );
        // 10-bit full range: 1023 is 255, 0 is 0.
        let deep = frame(10, Some((0, 0)), [vec![1023], vec![0], vec![1023]]);
        assert_eq!(to_rgba(&deep, None, 0, true).rgba, [255, 255, 0, 255]);
    }

    #[test]
    fn limited_range_and_deep_samples_scale_to_eight_bits() {
        // Limited 8-bit: luma 16..235 is black..white; 10-bit full range: 1023 is white.
        let limited = frame(8, None, [vec![16, 235], vec![], vec![]]);
        assert_eq!(
            to_rgba(&limited, None, 1, false).rgba,
            [0, 0, 0, 255, 255, 255, 255, 255]
        );
        let deep = frame(10, None, [vec![1023, 512], vec![], vec![]]);
        assert_eq!(
            to_rgba(&deep, None, 1, true).rgba,
            [255, 255, 255, 255, 128, 128, 128, 255]
        );
        // A 10-bit alpha frame scales by its own depth.
        let alpha = frame(10, None, [vec![1023, 0], vec![], vec![]]);
        let out = to_rgba(&limited, Some(&alpha), 1, false).rgba;
        assert_eq!((out[3], out[7]), (255, 0));
    }
}

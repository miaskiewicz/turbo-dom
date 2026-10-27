//! JPEG XL pixel decode for screenshots (miaskiewicz/turbo-dom#45 item 2: Chrome 156
//! decodes JPEG XL), behind the off-by-default `jxl` feature.
//!
//! Sizing never needs this: turbo-dom-layout's header probe reads a JPEG XL's
//! `SizeHeader` and orientation, so its image box always lays out at its real size.
//! Painting its pixels needs a decoder. The options assessed (2026-10): `jxl-oxide`
//! (MIT OR Apache-2.0, pure Rust, the most complete Rust decoder; used here without its
//! `rayon` pool: one thread, deterministic); `jxl` (jxl-rs, BSD-3-Clause — the decoder
//! Chromium itself vendors — younger API); `jpegxl-rs` (bindings to the C++ libjxl: a
//! native toolchain); `zune-jpegxl` encodes only. Without the feature a JPEG XL paints
//! as the placeholder; a corrupt one (no pixels) paints nothing (turbo-dom#107).

use turbo_dom_layout::image::Rgba8Image;

/// Decode a JPEG XL image (its first keyframe) to straight RGBA8 in its STORED
/// orientation (the painters apply `Intrinsic::orientation`), or `None` without the
/// `jxl` feature.
#[cfg(not(feature = "jxl"))]
pub fn decode(_bytes: &[u8]) -> Option<Rgba8Image> {
    None
}

/// Decode a JPEG XL image (its first keyframe) to straight RGBA8 in its STORED
/// orientation (the painters apply `Intrinsic::orientation`, read by the shared probe),
/// or `None` on any container / codestream error.
#[cfg(feature = "jxl")]
pub fn decode(bytes: &[u8]) -> Option<Rgba8Image> {
    use jxl_oxide::JxlImage;
    use turbo_dom_layout::image::Orientation;

    let image = JxlImage::builder().read(bytes).ok()?;
    let render = image.render_frame(0).ok()?;
    let mut stream = render.stream();
    let (width, height, channels) = (stream.width(), stream.height(), stream.channels());
    let mut samples = vec![0u8; width as usize * height as usize * channels as usize];
    stream.write_to_buffer(&mut samples);
    let rgba = to_rgba(&samples, channels)?;
    // The stream is oriented; undo it so the painters' own orientation step applies once.
    let orientation = Orientation::from_exif(u16::try_from(render.orientation()).ok()?);
    let (sw, sh, stored) = inverse(orientation).apply_rgba(width, height, &rgba);
    Some(Rgba8Image {
        width: sw,
        height: sh,
        rgba: stored,
    })
}

/// Interleaved 8-bit samples of `channels` (grey, grey + alpha, RGB, RGB + alpha) as RGBA.
#[cfg(feature = "jxl")]
fn to_rgba(samples: &[u8], channels: u32) -> Option<Vec<u8>> {
    let px = |p: &[u8]| -> [u8; 4] {
        match *p {
            [g] => [g, g, g, 255],
            [g, a] => [g, g, g, a],
            [r, g, b] => [r, g, b, 255],
            // Four or more (`chunks_exact` of a non-zero count never yields an empty
            // chunk): RGB + alpha, any further channel (a CMYK black) dropped.
            _ => [p[0], p[1], p[2], p[3]],
        }
    };
    let n = usize::try_from(channels).ok().filter(|&n| n > 0)?;
    Some(samples.chunks_exact(n).flat_map(px).collect())
}

/// The orientation that undoes `o` (a quarter turn one way undoes the other; every other
/// orientation is its own inverse).
#[cfg(feature = "jxl")]
fn inverse(o: turbo_dom_layout::image::Orientation) -> turbo_dom_layout::image::Orientation {
    use turbo_dom_layout::image::Orientation;
    match o.exif() {
        6 => Orientation::from_exif(8),
        8 => Orientation::from_exif(6),
        _ => o,
    }
}

#[cfg(all(test, feature = "jxl"))]
mod tests {
    use super::*;
    use turbo_dom_layout::image::Orientation;

    #[test]
    fn sample_layouts_widen_to_rgba() {
        assert_eq!(to_rgba(&[7, 9], 1), Some(vec![7, 7, 7, 255, 9, 9, 9, 255]));
        assert_eq!(to_rgba(&[7, 50], 2), Some(vec![7, 7, 7, 50]));
        assert_eq!(to_rgba(&[1, 2, 3], 3), Some(vec![1, 2, 3, 255]));
        assert_eq!(to_rgba(&[1, 2, 3, 4], 4), Some(vec![1, 2, 3, 4]));
        assert_eq!(
            to_rgba(&[1, 2, 3, 4, 5], 5),
            Some(vec![1, 2, 3, 4]),
            "black channel dropped"
        );
        assert_eq!(to_rgba(&[1], 0), None);
    }

    #[test]
    fn inverse_undoes_every_orientation() {
        for v in 1..=8 {
            let o = Orientation::from_exif(v);
            assert!(o.then(inverse(o)).is_identity(), "{v}");
        }
    }

    #[test]
    fn corrupt_input_is_none() {
        assert!(decode(b"\xff\x0a\x00").is_none());
        assert!(decode(b"not jxl").is_none());
    }
}

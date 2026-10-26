//! The S5 differential gate for the shared image layer on the PDF side: the PNG decode
//! (`PngFrame` → `to_raster_image`) and, under `svg`, the shared SVG rasterizer
//! (`probe_svg` / `decode_svg` at 2x) equal the pre-consolidation code byte for byte.
//! The old code is kept here verbatim as the oracle. (The screenshot painter's side —
//! `image::to_rgba8` and the 1x SVG decode — is gated in turbo-dom-raster's
//! `image_paint::shared_layer_tests`.)

use std::io::Cursor;
use std::path::{Path, PathBuf};

use turbo_dom_layout::image::{
    decode, decode_png_rgba, probe, ColorSpace, Payload, PngBudget, PngColor, PngFrame, RasterImage,
};

/// The pre-S5 PDF PNG decode (turbo-html2pdf-core 0.3.9 `image::decode_png`),
/// verbatim: `normalize_to_color8` (EXPAND | `STRIP_16`), then split off alpha.
fn old_decode_png(bytes: &[u8]) -> Option<RasterImage> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let out = reader.next_frame(&mut buf).ok()?;
    buf.truncate(out.buffer_size());
    let (color_space, channels, alpha_idx) = match out.color_type {
        png::ColorType::Grayscale => (ColorSpace::Gray, 1, None),
        png::ColorType::GrayscaleAlpha => (ColorSpace::Gray, 2, Some(1)),
        png::ColorType::Rgb => (ColorSpace::Rgb, 3, None),
        _ => (ColorSpace::Rgb, 4, Some(3)),
    };
    let pixels = (out.width * out.height) as usize;
    let comps = color_space.components();
    let mut samples = Vec::with_capacity(pixels * comps);
    let mut alpha = alpha_idx.map(|_| Vec::with_capacity(pixels));
    for px in buf.chunks_exact(channels) {
        samples.extend_from_slice(&px[..comps]);
        if let (Some(a), Some(i)) = (alpha.as_mut(), alpha_idx) {
            a.push(px[i]);
        }
    }
    Some(RasterImage {
        width: out.width,
        height: out.height,
        payload: Payload::Raw {
            samples,
            color: color_space,
        },
        alpha,
    })
}

/// Encode a test PNG with the png crate.
fn encode(
    (w, h): (u32, u32),
    color: png::ColorType,
    depth: png::BitDepth,
    extra: &dyn Fn(&mut png::Encoder<'_, &mut Vec<u8>>),
    data: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(color);
        enc.set_depth(depth);
        extra(&mut enc);
        let mut writer = enc.write_header().expect("png header");
        writer.write_image_data(data).expect("png data");
    }
    out
}

/// PNGs over every colour type / bit depth the decoder expands: every 16-bit sample
/// value (256x256), every 8-bit one, low-bit gray, palette with and without `tRNS`,
/// and gray/RGB with a `tRNS` colour key at 8 and 16 bits.
fn generated() -> Vec<(&'static str, Vec<u8>)> {
    use png::{BitDepth as D, ColorType as C};
    let none = |_: &mut png::Encoder<'_, &mut Vec<u8>>| {};
    let ramp16 = |channels: usize| -> Vec<u8> {
        (0..65536u32 * channels as u32)
            .flat_map(|i| ((i.wrapping_mul(40503) >> (i as usize % channels)) as u16).to_be_bytes())
            .collect()
    };
    let ramp8 = |n: usize| -> Vec<u8> { (0..n).map(|i| (i * 7 + i / 256) as u8).collect() };
    let trns_gray16 = |e: &mut png::Encoder<'_, &mut Vec<u8>>| e.set_trns(vec![0x12, 0x34]);
    let trns_rgb16 = |e: &mut png::Encoder<'_, &mut Vec<u8>>| e.set_trns(vec![0, 1, 0, 2, 0, 3]);
    let trns_gray8 = |e: &mut png::Encoder<'_, &mut Vec<u8>>| e.set_trns(vec![0, 7]);
    let palette = |e: &mut png::Encoder<'_, &mut Vec<u8>>| {
        e.set_palette((0..=255u8).flat_map(|i| [i, !i, i / 2]).collect::<Vec<_>>());
    };
    let palette_trns = |e: &mut png::Encoder<'_, &mut Vec<u8>>| {
        palette(e);
        e.set_trns((0..128u8).map(|i| i * 2).collect::<Vec<_>>());
    };
    let gray16_keyed: Vec<u8> = (0..=u16::MAX)
        .flat_map(|i| if i % 3 == 0 { 0x1234 } else { i }.to_be_bytes())
        .collect();
    let rgb16_keyed: Vec<u8> = (0..=u16::MAX)
        .flat_map(|i| {
            let px: [u16; 3] = if i % 5 == 0 {
                [1, 2, 3]
            } else {
                [i, !i, i ^ 0xa5a5]
            };
            px.into_iter().flat_map(u16::to_be_bytes)
        })
        .collect();
    vec![
        (
            "gray16",
            encode((256, 256), C::Grayscale, D::Sixteen, &none, &ramp16(1)),
        ),
        (
            "grayalpha16",
            encode((256, 256), C::GrayscaleAlpha, D::Sixteen, &none, &ramp16(2)),
        ),
        (
            "rgb16",
            encode((256, 256), C::Rgb, D::Sixteen, &none, &ramp16(3)),
        ),
        (
            "rgba16",
            encode((256, 256), C::Rgba, D::Sixteen, &none, &ramp16(4)),
        ),
        (
            "gray16-trns",
            encode(
                (256, 256),
                C::Grayscale,
                D::Sixteen,
                &trns_gray16,
                &gray16_keyed,
            ),
        ),
        (
            "rgb16-trns",
            encode((256, 256), C::Rgb, D::Sixteen, &trns_rgb16, &rgb16_keyed),
        ),
        (
            "gray8",
            encode((16, 16), C::Grayscale, D::Eight, &none, &ramp8(256)),
        ),
        (
            "gray8-trns",
            encode((16, 16), C::Grayscale, D::Eight, &trns_gray8, &ramp8(256)),
        ),
        (
            "grayalpha8",
            encode((16, 16), C::GrayscaleAlpha, D::Eight, &none, &ramp8(512)),
        ),
        (
            "rgb8",
            encode((16, 16), C::Rgb, D::Eight, &none, &ramp8(768)),
        ),
        (
            "rgba8",
            encode((16, 16), C::Rgba, D::Eight, &none, &ramp8(1024)),
        ),
        (
            "gray1",
            encode((16, 16), C::Grayscale, D::One, &none, &ramp8(32)),
        ),
        (
            "gray2",
            encode((16, 16), C::Grayscale, D::Two, &none, &ramp8(64)),
        ),
        (
            "gray4-trns",
            encode((16, 16), C::Grayscale, D::Four, &trns_gray8, &ramp8(128)),
        ),
        (
            "palette8",
            encode((16, 16), C::Indexed, D::Eight, &palette, &ramp8(256)),
        ),
        (
            "palette8-trns",
            encode((16, 16), C::Indexed, D::Eight, &palette_trns, &ramp8(256)),
        ),
        (
            "palette4-trns",
            encode((16, 16), C::Indexed, D::Four, &palette_trns, &ramp8(128)),
        ),
    ]
}

fn parity_images() -> Vec<(String, Vec<u8>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/parity/inputs");
    let mut out = Vec::new();
    for dir in [root.join("image-matrix/images"), root.join("extra/images")] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries.map(|e| e.expect("entry").path()).collect();
        paths.sort();
        for path in paths {
            let bytes = std::fs::read(&path).expect("read image");
            out.push((path.display().to_string(), bytes));
        }
    }
    out
}

#[test]
fn shared_png_decode_matches_the_old_pdf_decode() {
    let mut cases: Vec<(String, Vec<u8>)> = generated()
        .into_iter()
        .map(|(label, bytes)| (label.to_string(), bytes))
        .collect();
    let matrix = parity_images();
    if matrix.is_empty() {
        eprintln!("SKIP matrix part: no tools/parity inputs (packaged crate)");
    }
    cases.extend(matrix.into_iter().filter(|(name, _)| {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    }));
    let mut decoded = 0;
    for (label, bytes) in &cases {
        let old = old_decode_png(bytes);
        assert_eq!(decode(bytes), old, "{label}");
        decoded += usize::from(old.is_some());
    }
    // 17 generated + the 6 matrix PNGs; the truncated extra PNG decodes to None.
    let has_matrix = cases.len() > 17;
    assert_eq!(decoded, if has_matrix { 23 } else { 17 });
}

#[test]
fn png_frame_keeps_16_bit_samples_and_gray() {
    let gray16 = encode(
        (2, 1),
        png::ColorType::Grayscale,
        png::BitDepth::Sixteen,
        &|_| {},
        &[0x12, 0x34, 0xff, 0x00],
    );
    let frame = PngFrame::decode(&gray16, PngBudget::PngDefault).expect("gray16");
    assert_eq!((frame.color, frame.sixteen_bit), (PngColor::Gray, true));
    assert_eq!(frame.samples, vec![0x12, 0x34, 0xff, 0x00]);
    // PDF: gray stays gray, the high byte is kept.
    let pdf = frame.to_raster_image();
    assert_eq!(
        pdf.payload,
        Payload::Raw {
            samples: vec![0x12, 0xff],
            color: ColorSpace::Gray
        }
    );
    assert_eq!(pdf.alpha, None);
    // RGBA: gray replicated, rounded (not truncated), opaque.
    assert_eq!(
        frame.to_rgba8(),
        vec![0x12, 0x12, 0x12, 255, 0xfe, 0xfe, 0xfe, 255]
    );
    assert_eq!(PngColor::GrayAlpha.channels(), 2);
    let rgba = decode_png_rgba(&gray16).expect("rgba");
    assert_eq!((rgba.width, rgba.height, rgba.rgba.len()), (2, 1, 8));
}

#[test]
fn png_budget_rejects_frames_above_the_image_crate_limit() {
    // A valid header announcing 16384 x 16384 RGBA16 (2 GiB decoded): the image-crate
    // budget refuses the frame before allocating (as image::load_from_memory did).
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = b"IHDR".to_vec();
    ihdr.extend_from_slice(&16384u32.to_be_bytes());
    ihdr.extend_from_slice(&16384u32.to_be_bytes());
    ihdr.extend_from_slice(&[16, 6, 0, 0, 0]); // 16-bit RGBA, no interlace
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(&ihdr);
    png.extend_from_slice(&crc32(&ihdr).to_be_bytes());
    // An empty IDAT, so the header parse completes (the pixel data never arrives).
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&crc32(b"IDAT").to_be_bytes());
    assert!(probe(&png).is_some(), "the header still probes");
    assert!(PngFrame::decode(&png, PngBudget::ImageCrate).is_none());
    assert!(decode_png_rgba(&png).is_none());
}

/// CRC-32 (ISO-HDLC, the PNG chunk checksum).
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[test]
fn malformed_png_decodes_to_none() {
    assert!(PngFrame::decode(b"\x89PNG\r\n\x1a\nnope", PngBudget::PngDefault).is_none());
}

#[cfg(feature = "svg")]
mod svg_oracle {
    use super::parity_images;
    use resvg::{tiny_skia, usvg};
    use turbo_dom_layout::image::{ColorSpace, Intrinsic, Payload, RasterImage};
    use turbo_dom_layout::svg::{
        css_size, decode_svg, parse, pixel_size, probe_svg, rasterize, Oversize, MAX_DIM,
    };

    /// The pre-S5 PDF SVG probe + decode (turbo-html2pdf-core 0.3.9 `svg.rs`),
    /// verbatim.
    fn old_axis(len: f32) -> Option<u32> {
        if !len.is_finite() || len <= 0.0 {
            return None;
        }
        let px = (len * 2.0).ceil().min(8192.0) as u32;
        (px > 0).then_some(px)
    }

    fn old_probe(bytes: &[u8]) -> Option<Intrinsic> {
        let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()?;
        let size = tree.size();
        Some(Intrinsic {
            width: old_axis(size.width())?,
            height: old_axis(size.height())?,
            has_alpha: true,
        })
    }

    fn old_decode(bytes: &[u8]) -> Option<RasterImage> {
        let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()?;
        let size = tree.size();
        let (width, height) = (old_axis(size.width())?, old_axis(size.height())?);
        let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
        let transform = tiny_skia::Transform::from_scale(2.0, 2.0);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        let mut samples = Vec::new();
        let mut alpha = Vec::new();
        for px in pixmap.pixels() {
            let c = px.demultiply();
            samples.extend_from_slice(&[c.red(), c.green(), c.blue()]);
            alpha.push(c.alpha());
        }
        Some(RasterImage {
            width,
            height,
            payload: Payload::Raw {
                samples,
                color: ColorSpace::Rgb,
            },
            alpha: Some(alpha),
        })
    }

    #[test]
    fn shared_svg_rasterizer_matches_the_old_pdf_svg_path() {
        let mut svgs: Vec<(String, Vec<u8>)> = parity_images()
            .into_iter()
            .filter(|(name, _)| {
                std::path::Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
            })
            .collect();
        svgs.push((
            "inline".into(),
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="3.25" height="2"><rect width="3" height="2" fill="#0f0" fill-opacity=".5"/></svg>"##.to_vec(),
        ));
        for (label, bytes) in &svgs {
            assert_eq!(probe_svg(bytes), old_probe(bytes), "{label} probe");
            assert_eq!(decode_svg(bytes), old_decode(bytes), "{label} decode");
        }
        assert!(svgs.len() == 1 || svgs.len() == 5, "4 matrix SVGs + inline");
    }

    #[test]
    fn pixel_size_separates_css_size_from_oversample() {
        let tree =
            parse(br#"<svg xmlns="http://www.w3.org/2000/svg" width="10.5" height="9000"/>"#)
                .expect("parse");
        assert_eq!(css_size(&tree), (10.5, 9000.0));
        // PDF: 2x, the tall axis clamped; raster: 1x, the tall axis rejected.
        assert_eq!(
            pixel_size(css_size(&tree), 2.0, Oversize::Clamp),
            Some((21, MAX_DIM))
        );
        assert_eq!(pixel_size(css_size(&tree), 1.0, Oversize::Reject), None);
        assert_eq!(
            pixel_size((10.5, 4.0), 1.0, Oversize::Reject),
            Some((11, 4))
        );
        assert_eq!(pixel_size((f32::NAN, 4.0), 1.0, Oversize::Reject), None);
        assert_eq!(pixel_size((0.0, 4.0), 2.0, Oversize::Clamp), None);
        let img = rasterize(
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="2" height="1" fill="red"/></svg>"#,
            1.0,
            Oversize::Reject,
        )
        .expect("raster");
        assert_eq!((img.width, img.height), (2, 1));
        assert_eq!(img.rgba, vec![255, 0, 0, 255, 255, 0, 0, 255]);
    }
}

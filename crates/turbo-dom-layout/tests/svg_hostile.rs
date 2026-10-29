//! Hostile SVG images (turbo-dom#196/#197/#198, mirrored from turbo-html2pdf's
//! `hostile_svg.rs`). Every case runs on a worker thread under a watchdog, so a
//! hang fails the test instead of wedging CI and a stack overflow aborts only
//! with a clear failure. Expectations are Chrome 156 (oracle 8078.47) rendering
//! the same bytes as an `<img>`: an SVG image never loads anything but `data:`
//! URLs, a too-deep document is a broken image, and an oversized image is scaled
//! into its raster, not cropped.

#![cfg(feature = "svg-rasterize")]

use std::fmt::Write as _;
use std::sync::mpsc;
use std::time::Duration;

use turbo_dom_layout::svg::{rasterize, Oversize, MAX_DIM, PDF_SVG_SCALE};

/// Run `f` on a thread with `stack` bytes of stack, failing after 60 s.
fn guarded<T: Send + 'static>(stack: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            tx.send(f()).ok();
        })
        .expect("spawn");
    rx.recv_timeout(Duration::from_secs(60))
        .expect("rasterize did not finish within 60 s")
}

/// Rasterize at the PDF scale (clamped) on a 2 MiB stack (the Rust default for
/// spawned threads, smaller than a main thread).
fn raster(svg: String) -> Option<(u32, u32, Vec<u8>)> {
    guarded(2 << 20, move || {
        rasterize(svg.as_bytes(), PDF_SVG_SCALE, Oversize::Clamp)
            .map(|i| (i.width, i.height, i.rgba))
    })
}

fn red_pixels(rgba: &[u8]) -> usize {
    rgba.chunks(4)
        .filter(|p| p[0] > 200 && p[1] < 50 && p[3] > 200)
        .count()
}

const RED_4X4: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="#ff0000"/></svg>"##;

fn image_svg(href: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="4" height="4"><image width="4" height="4" xlink:href="{href}"/><image width="4" height="4" href="{href}"/></svg>"#
    )
}

/// A `data:` URL of `svg`, percent-encoded so it can sit in an XML attribute.
fn data_svg(svg: &str) -> String {
    let mut out = String::from("data:image/svg+xml,");
    for b in svg.bytes() {
        if b.is_ascii_alphanumeric() || b"=/:. -".contains(&b) {
            out.push(b as char);
        } else {
            write!(out, "%{b:02X}").unwrap();
        }
    }
    out
}

// ---- #196: no external loads from an SVG image ----

#[test]
fn external_file_image_is_not_loaded() {
    let dir = std::env::temp_dir().join(format!("td-hostile-svg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let secret = dir.join("secret.svg");
    std::fs::write(&secret, RED_4X4).expect("write");
    let abs = secret.display().to_string();
    let hrefs = [abs.clone(), format!("file://{abs}")];
    let results: Vec<_> = hrefs
        .iter()
        .map(|h| raster(image_svg(h)).expect("a parseable SVG"))
        .collect();
    std::fs::remove_dir_all(&dir).ok();
    for (href, (_, _, rgba)) in hrefs.iter().zip(results) {
        // Chrome 156: an SVG image never fetches subresources -> transparent.
        assert_eq!(red_pixels(&rgba), 0, "a hostile SVG read {href}");
    }
}

#[test]
fn relative_file_image_is_not_loaded() {
    // A relative href resolves against the process cwd under usvg's default resolver.
    let name = format!("td-hostile-rel-{}.svg", std::process::id());
    std::fs::write(&name, RED_4X4).expect("write");
    let r = raster(image_svg(&name));
    std::fs::remove_file(&name).ok();
    let (_, _, rgba) = r.expect("a parseable SVG");
    assert_eq!(red_pixels(&rgba), 0, "a hostile SVG read a relative file");
}

#[test]
fn data_url_image_still_renders() {
    // Chrome renders `data:` images inside an SVG image; that path must survive.
    let href = data_svg(RED_4X4);
    let (_, _, rgba) = raster(image_svg(&href)).expect("a parseable SVG");
    assert_eq!(
        red_pixels(&rgba),
        4 * 4 * 4,
        "the data: image painted every pixel"
    );
}

// ---- #197: input-driven depth never overflows the stack ----

fn nested(depth: usize) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\">{}<rect width=\"4\" height=\"4\" fill=\"#ff0000\"/>{}</svg>",
        "<g>".repeat(depth),
        "</g>".repeat(depth)
    )
}

#[test]
fn deep_nesting_terminates_without_overflow() {
    // Chrome 156: 5000- and 100000-deep documents are broken images.
    assert!(raster(nested(100_000)).is_none());
    assert!(raster(nested(5_000)).is_none());
}

#[test]
fn moderate_nesting_still_renders() {
    let (_, _, rgba) = raster(nested(256)).expect("a 256-deep SVG renders");
    assert_eq!(red_pixels(&rgba), 64);
}

#[test]
fn nesting_at_the_limit_renders() {
    // 1000 <g> under the root stays inside usvg's own 1024-element limit.
    let (_, _, rgba) = raster(nested(1000)).expect("a 1000-deep SVG renders");
    assert_eq!(red_pixels(&rgba), 64);
}

#[test]
fn nesting_past_the_limit_is_broken() {
    assert!(raster(nested(1100)).is_none());
}

#[test]
fn deep_nesting_in_a_dtd_entity_is_bounded() {
    // An entity expanding to deep markup is counted before roxmltree recurses into it.
    let svg = format!(
        "<?xml version=\"1.0\"?><!DOCTYPE svg [<!ENTITY d \"{}{}\">]><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\">&d;</svg>",
        "<g>".repeat(100_000),
        "</g>".repeat(100_000)
    );
    assert!(raster(svg).is_none());
}

#[test]
fn deep_nesting_in_a_data_image_terminates() {
    // A nested `data:` SVG is parsed by the resolver, not by `parse`'s own check.
    let svg = image_svg(&data_svg(&nested(100_000)));
    let (_, _, rgba) = raster(svg).expect("the outer SVG parses");
    assert_eq!(red_pixels(&rgba), 0);
}

#[test]
fn long_use_chain_terminates_without_overflow() {
    let n = 100_000;
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="4" height="4"><defs><rect id="u0" width="4" height="4" fill="#ff0000"/>"##,
    );
    for i in 1..n {
        write!(svg, "<use id=\"u{i}\" xlink:href=\"#u{}\"/>", i - 1).unwrap();
    }
    write!(svg, "</defs><use xlink:href=\"#u{}\"/></svg>", n - 1).unwrap();
    let _ = raster(svg);
}

#[test]
fn long_reference_chain_terminates_without_overflow() {
    // clip-path -> clipPath whose child carries the next clip-path, 100k long.
    let n = 100_000;
    let mut svg =
        String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><defs>"#);
    for i in 0..n {
        let next = if i + 1 < n {
            format!(" clip-path=\"url(#c{})\"", i + 1)
        } else {
            String::new()
        };
        write!(
            svg,
            "<clipPath id=\"c{i}\"><rect width=\"4\" height=\"4\"{next}/></clipPath>"
        )
        .unwrap();
    }
    svg.push_str(
        r##"</defs><rect width="4" height="4" fill="#ff0000" clip-path="url(#c0)"/></svg>"##,
    );
    let _ = raster(svg);
}

// ---- #198: an oversize SVG is scaled down, not cropped ----

#[test]
fn oversized_svg_is_scaled_down_not_cropped() {
    // 10000x10000 CSS px with the only paint in the bottom-right quarter. Chrome
    // draws the whole image into its box; a clamped raster must still show it.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10000" height="10000"><rect x="5000" y="5000" width="5000" height="5000" fill="#ff0000"/></svg>"##;
    let (w, h, rgba) = raster(svg.to_string()).expect("a parseable SVG");
    assert_eq!((w, h), (MAX_DIM, MAX_DIM));
    let red = red_pixels(&rgba);
    let quarter = (MAX_DIM as usize / 2).pow(2);
    assert!(
        red.abs_diff(quarter) <= 2 * MAX_DIM as usize,
        "red {red} vs {quarter}"
    );
    let at = |x: u32, y: u32| ((y * w + x) * 4) as usize;
    assert_eq!(rgba[at(w - 1, h - 1)], 255, "bottom-right corner painted");
    assert_eq!(rgba[at(w / 4, h / 4) + 3], 0, "top-left quarter empty");
}

#[test]
fn oversized_axis_scales_independently() {
    // Only the width overflows: x is squeezed to fit, y keeps the PDF scale.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="8000" height="10"><rect x="7990" width="10" height="10" fill="#ff0000"/></svg>"##;
    let (w, h, rgba) = raster(svg.to_string()).expect("a parseable SVG");
    assert_eq!((w, h), (MAX_DIM, 20));
    let at = |x: u32, y: u32| ((y * w + x) * 4) as usize;
    assert_eq!(rgba[at(w - 1, h - 1)], 255, "right edge painted");
    assert_eq!(rgba[at(w - 1, h - 1) + 3], 255);
    assert_eq!(
        rgba[at(w - 12, 0) + 3],
        0,
        "the 10-px rect squeezed to ~10 px"
    );
}

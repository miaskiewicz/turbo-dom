//! Embedded raster images use the shared SVG-as-image renderer without external I/O.
#![cfg(feature = "svg-rasterize")]

use turbo_dom_layout::svg::{rasterize, Oversize};

const PNG: &[u8] = include_bytes!("fixtures/svg-raster/pixel.png");
const CANONICAL: &str = include_str!("fixtures/svg-raster/canonical-href.svg");

fn rgba(svg: &str) -> Vec<u8> {
    rasterize(svg.as_bytes(), 1.0, Oversize::Reject)
        .expect("valid SVG image")
        .rgba
}

fn assert_red(svg: &str) {
    let pixels = rgba(svg);
    assert_eq!(pixels.len(), 80 * 80 * 4);
    assert!(pixels
        .as_chunks::<4>()
        .0
        .iter()
        .all(|p| *p == [238, 34, 68, 255]));
}

fn image(href: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="80"><image width="80" height="80" href="{href}"/></svg>"#
    )
}

#[test]
fn embedded_png_paints_through_href_pattern_and_use_mask() {
    for svg in [
        CANONICAL,
        include_str!("fixtures/svg-raster/canonical-xlink-href.svg"),
        include_str!("fixtures/svg-raster/pattern.svg"),
        include_str!("fixtures/svg-raster/use-mask.svg"),
    ] {
        assert_red(svg);
    }
}

#[test]
fn valid_local_png_paths_and_http_are_not_loaded() {
    let name = format!("svg-raster-permission-{}.png", std::process::id());
    let path = std::env::current_dir().unwrap().join(&name);
    std::fs::write(&path, PNG).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), PNG);
    let absolute = path.to_str().unwrap();
    let hrefs = [
        name.clone(),
        absolute.to_owned(),
        format!("file://{absolute}"),
        "http://127.0.0.1:9/image.png".to_owned(),
        "https://example.invalid/image.png".to_owned(),
    ];
    let results: Vec<_> = hrefs.iter().map(|href| rgba(&image(href))).collect();
    std::fs::remove_file(path).unwrap();
    for (href, pixels) in hrefs.iter().zip(results) {
        assert!(pixels.iter().all(|&b| b == 0), "loaded external {href}");
    }
    assert_red(CANONICAL);
}

#[test]
fn raster_signature_resolves_mime_aliases_without_rewriting_svg() {
    for svg in [
        include_str!("fixtures/svg-raster/actual-href.svg"),
        include_str!("fixtures/svg-raster/actual-xlink-href.svg"),
    ] {
        assert_red(svg);
    }
    assert_red(&CANONICAL.replace("image/png", "application/octet-stream"));
    let wrong_svg_type = CANONICAL.replace("image/png", "image/svg+xml");
    assert!(rgba(&wrong_svg_type).iter().all(|&b| b == 0));
}

#[test]
fn malformed_riff_and_non_raster_xml_do_not_bypass_svg_guards() {
    for href in [
        "data:text/plain,RIFFnot-a-webp",
        "data:img/png,RIFFnot-a-webp",
        "data:text/plain,%3Chtml%3E%3C/html%3E",
        "data:image/png,%89PNG",
    ] {
        assert!(rgba(&image(href)).iter().all(|&b| b == 0));
    }
    let nested = format!("{}{}", "<g>".repeat(1025), "</g>".repeat(1025));
    let xml = format!("<svg xmlns='http://www.w3.org/2000/svg'>{nested}</svg>");
    let encoded = xml.replace('<', "%3C").replace('>', "%3E");
    assert!(rgba(&image(&format!("data:text/plain,{encoded}")))
        .iter()
        .all(|&b| b == 0));
}

fn header_png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        // Header-only fixture: resource rejection must happen without pixel decode.
        writer.write_chunk(png::chunk::IDAT, &[]).unwrap();
    }
    bytes
}

fn data_uri(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut uri = String::from("data:image/png,");
    for byte in bytes {
        write!(&mut uri, "%{byte:02X}").unwrap();
    }
    uri
}

#[test]
fn checked_svg_budget_errors_do_not_allocate_oversized_frames() {
    use turbo_dom_layout::svg::{
        parse, parse_checked, rasterize_checked, SvgResourceError, MAX_DIM,
    };
    let svg = image(&data_uri(&header_png(MAX_DIM + 1, 1)));
    assert!(matches!(
        parse_checked(svg.as_bytes()),
        Err(SvgResourceError::RasterDimensions)
    ));
    assert_eq!(
        rasterize_checked(svg.as_bytes(), 1.0, Oversize::Reject),
        Err(SvgResourceError::RasterDimensions)
    );
    assert!(parse(svg.as_bytes()).is_none());
    assert!(rasterize(svg.as_bytes(), 1.0, Oversize::Reject).is_none());
}

#[test]
fn cumulative_budget_counts_each_use_and_paint_server_occurrence() {
    use turbo_dom_layout::svg::{parse_checked, SvgResourceError, MAX_DIM};
    let href = data_uri(&header_png(MAX_DIM, MAX_DIM));
    let source = format!(r#"<image id="raster" width="80" height="80" href="{href}"/>"#);
    let bodies = [
        format!("{source}<use href='#raster'/>") ,
        format!("<defs><pattern id='p' width='1' height='1'>{source}</pattern></defs><rect width='80' height='80' fill='url(#p)'/><rect width='80' height='80' fill='url(#p)'/>"),
        format!("<defs><mask id='m'>{source}</mask></defs><rect width='80' height='80' mask='url(#m)'/><rect width='80' height='80' mask='url(#m)'/>"),
        format!("<defs><filter id='f'><feImage href='#raster'/></filter>{source}</defs><rect width='80' height='80' filter='url(#f)'/><rect width='80' height='80' filter='url(#f)'/>"),
    ];
    for body in bodies {
        let svg =
            format!("<svg xmlns='http://www.w3.org/2000/svg' width='80' height='80'>{body}</svg>");
        assert!(
            matches!(
                parse_checked(svg.as_bytes()),
                Err(SvgResourceError::RasterBytes)
            ),
            "unaccounted raster occurrence: {body}"
        );
    }
}

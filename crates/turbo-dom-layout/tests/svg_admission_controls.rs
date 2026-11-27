//! Public SVG image admission, metadata and static-cascade boundary controls.
#![cfg(feature = "svg-rasterize")]
use turbo_dom_layout::svg::{
    is_svg, natural_size, parse_checked, rasterize_checked, Oversize, SvgResourceError,
};

fn svg(body: &str) -> String {
    format!("<svg xmlns='http://www.w3.org/2000/svg' width='20' height='10'>{body}</svg>")
}

#[test]
fn xml_comments_processing_instructions_and_natural_size_preserve_metadata() {
    let source = format!(
        " \n\t<?xml version='1.0'?><!-- <not-an-element> -->{}",
        svg("<!-- inner --><rect width='20' height='10'/>")
    );
    // XML declarations cannot follow whitespace, even though sniffing recognizes SVG.
    assert!(is_svg(source.as_bytes()));
    assert_eq!(natural_size(source.as_bytes()), None);
    let valid = source.trim_start();
    assert_eq!(natural_size(valid.as_bytes()), Some((20.0, 10.0)));
    assert!(parse_checked(valid.as_bytes()).unwrap().is_some());
}

#[test]
fn comments_and_xml_attribute_escapes_do_not_change_frozen_selector_inputs() {
    let source = svg("<!-- source comment --><?source test?><style>rect{font-family:'a&lt;b&gt;c';fill:var(--paint,#ee2244)}</style><rect width='20' height='10'/>");
    let image = rasterize_checked(source.as_bytes(), 1.0, Oversize::Reject)
        .unwrap()
        .unwrap();
    assert!(image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| *pixel == [238, 34, 68, 255]));
}

#[test]
fn descriptor_statements_and_deep_groups_refuse_variable_rewriting() {
    for rule in [
        "@import 'forbidden.css';",
        "@font-face{font-family:x;src:url(forbidden.woff)}",
        "@layer named;",
    ] {
        let source = svg(&format!(
            "<style>{rule}</style><rect fill='var(--paint,red)' width='20' height='10'/>"
        ));
        assert_eq!(
            parse_checked(source.as_bytes()).unwrap_err(),
            SvgResourceError::UnsupportedCustomProperties
        );
    }
    let groups = format!(
        "{}rect{{fill:var(--paint,red)}}{}",
        "@media all{".repeat(17),
        "}".repeat(17)
    );
    assert_eq!(
        parse_checked(svg(&format!("<style>{groups}</style>")).as_bytes()).unwrap_err(),
        SvgResourceError::CustomPropertyNesting
    );
}

#[test]
fn malformed_unknown_rule_recovery_retains_independent_paint() {
    let source = svg("<style>@unknown anything; rect{fill:var(--paint,#ee2244)}</style><rect width='20' height='10'/>");
    let image = rasterize_checked(source.as_bytes(), 1.0, Oversize::Reject)
        .unwrap()
        .unwrap();
    assert_eq!(&image.rgba[..4], &[238, 34, 68, 255]);
}

#[test]
fn noninherited_paint_initial_and_explicit_color_inheritance_are_distinct() {
    for declaration in [
        "stop-color:var(--missing)",
        "color:inherit",
        "color:currentColor",
    ] {
        let source = svg(&format!("<g style='color:#22cc44'><rect width='20' height='10' style='{declaration};fill:var(--paint,currentColor)'/></g>"));
        let image = rasterize_checked(source.as_bytes(), 1.0, Oversize::Reject)
            .unwrap()
            .unwrap();
        assert_eq!(&image.rgba[..4], &[34, 204, 68, 255], "{declaration}");
    }
}

#[test]
fn resource_errors_describe_their_failed_admission_contract() {
    for (error, expected) in [
        (SvgResourceError::RasterDimensions, "dimension"),
        (SvgResourceError::RasterBytes, "decoded-byte"),
        (SvgResourceError::ResourceDepth, "depth"),
        (SvgResourceError::CustomPropertyNesting, "nesting"),
        (SvgResourceError::CustomPropertyBytes, "source-derived"),
    ] {
        assert!(error.to_string().contains(expected));
    }
}

fn data_uri(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut uri = String::from("data:image/jpeg,");
    for byte in bytes {
        write!(uri, "%{byte:02X}").unwrap();
    }
    uri
}

#[test]
fn jpeg_data_images_remain_admitted_inside_nested_masks_and_stroke_patterns() {
    let href = data_uri(include_bytes!("fixtures/images/orientation/jpeg-o1.jpg"));
    let body = format!("<defs><mask id='outer'><rect width='20' height='10' fill='white'/></mask><mask id='inner' mask='url(#outer)'><rect width='20' height='10' fill='white'/></mask><pattern id='p' width='1' height='1' patternContentUnits='objectBoundingBox'><image href='{href}' width='1' height='1' preserveAspectRatio='none'/></pattern></defs><rect x='2' y='2' width='16' height='6' fill='none' stroke='url(#p)' stroke-width='4' mask='url(#inner)'/>");
    let tree = parse_checked(svg(&body).as_bytes()).unwrap().unwrap();
    assert!(!tree.root().children().is_empty());
    let image = rasterize_checked(svg(&body).as_bytes(), 1.0, Oversize::Reject)
        .unwrap()
        .unwrap();
    assert!(image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .any(|pixel| pixel[3] != 0));
}

#[test]
fn cyclic_local_references_do_not_recurse_or_load_external_data() {
    let source = svg("<defs><g id='a'><use href='#a'/><rect width='20' height='10' fill='red'/></g></defs><use href='#a'/>");
    assert!(parse_checked(source.as_bytes()).unwrap().is_some());
    assert!(rasterize_checked(source.as_bytes(), 1.0, Oversize::Reject)
        .unwrap()
        .is_some());
}

#[test]
fn raster_signature_without_dimensions_retains_missing_image_behavior() {
    let source = svg("<image width='20' height='10' href='data:image/png,%89PNG%0D%0A%1A%0A'/>");
    let image = rasterize_checked(source.as_bytes(), 1.0, Oversize::Reject)
        .unwrap()
        .unwrap();
    assert!(image.rgba.iter().all(|byte| *byte == 0));
}

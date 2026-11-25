//! SVG-as-image paint against source-hashed independent Chromium observations.
#![cfg(feature = "svg-rasterize")]
use sha2::{Digest, Sha256};
use std::fmt::Write;
use turbo_dom_layout::svg::{parse_checked, rasterize, rasterize_checked, Oversize};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/svg_custom_paint_chrome159.json")).unwrap()
}

#[test]
fn svg_custom_paint_matches_independent_chrome() {
    let fixture = fixture();
    assert_eq!(
        fixture["probeSHA256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/svg-custom-paint-chrome-control.mjs"
            ))
        )
    );
    assert_eq!(
        fixture["casesSHA256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/svg_custom_paint_cases.json"))
        )
    );
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        if case["unsupported"] == true {
            continue;
        }
        let image = rasterize_checked(
            case["svg"].as_str().unwrap().as_bytes(),
            1.0,
            Oversize::Reject,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", case["id"]))
        .unwrap_or_else(|| panic!("{}: invalid SVG", case["id"]));
        let expected: Vec<u8> = serde_json::from_value(case["rgba"].clone()).unwrap();
        let index = (40 * image.width as usize + 40) * 4;
        if image.rgba[index..index + 4] != expected {
            failures.push(format!(
                "{}: {:?} != {expected:?}",
                case["id"],
                &image.rgba[index..index + 4]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn uncertified_variable_instance_and_conditional_cascade_are_explicit() {
    for case in fixture()["cases"].as_array().unwrap() {
        if case["unsupported"] != true {
            continue;
        }
        assert!(
            parse_checked(case["svg"].as_str().unwrap().as_bytes()).is_err(),
            "{}",
            case["id"]
        );
    }
}

fn svg_with_style(style: &str, body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="80" style="{style}">{body}</svg>"#
    )
}

#[test]
fn computation_limits_never_select_a_css_fallback() {
    use turbo_dom_layout::svg::SvgResourceError;
    let mut nested = String::new();
    for n in 0..20 {
        write!(nested, "--a{n:02}:var(--a{:02});", n + 1).unwrap();
    }
    nested.push_str("--a20:#22cc44");
    let body = "<rect width='80' height='80' fill='var(--a00,#ee2244)'/>";
    let source = svg_with_style(&nested, body);
    assert!(matches!(
        parse_checked(source.as_bytes()),
        Err(SvgResourceError::CustomPropertyNesting)
    ));
    assert!(rasterize(source.as_bytes(), 1.0, Oversize::Reject).is_none());
    let mut expanded = String::new();
    for n in 0..10 {
        write!(
            expanded,
            "--a{n:02}:var(--a{:02})var(--a{:02});",
            n + 1,
            n + 1
        )
        .unwrap();
    }
    expanded.push_str("--a10:#22cc44");
    assert!(matches!(
        parse_checked(svg_with_style(&expanded, body).as_bytes()),
        Err(SvgResourceError::CustomPropertyBytes)
    ));
}

#[test]
fn retained_tokens_and_xml_growth_are_source_bounded() {
    use turbo_dom_layout::svg::SvgResourceError;
    let nested = format!(
        "{}<rect width='80' height='80' fill='var(--paint)'/>{}",
        "<g style='--other:1'>".repeat(10),
        "</g>".repeat(10)
    );
    let source = svg_with_style(
        &format!("--big:{};--paint:#22cc44", "x".repeat(1000)),
        &nested,
    );
    assert!(matches!(
        parse_checked(source.as_bytes()),
        Err(SvgResourceError::CustomPropertyBytes)
    ));
    let body = format!(
        "<style>rect{{font-family:'{}';fill:var(--missing,#ee2244)}}</style>{}",
        "x".repeat(1000),
        "<rect width='80' height='80'/>".repeat(10)
    );
    let source = svg_with_style("", &body);
    assert!(matches!(
        parse_checked(source.as_bytes()),
        Err(SvgResourceError::CustomPropertyBytes)
    ));
}

#[test]
fn uncertified_xml_contexts_are_not_guessed() {
    use turbo_dom_layout::svg::SvgResourceError;
    let paint = "<rect width='80' height='80' fill='var(--missing,#ee2244)'/>";
    for body in [
        format!("{paint}<animate attributeName='fill' dur='1s'/>"),
        format!(
            "{paint}<foreignObject><div xmlns='http://www.w3.org/1999/xhtml'/></foreignObject>"
        ),
        format!("{paint}<style><g/></style>"),
    ] {
        let error = parse_checked(svg_with_style("", &body).as_bytes()).unwrap_err();
        assert_eq!(error, SvgResourceError::UnsupportedCustomProperties);
        assert!(error.to_string().contains("not certified"));
    }
}

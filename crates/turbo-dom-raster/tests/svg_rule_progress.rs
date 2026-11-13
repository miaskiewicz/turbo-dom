//! Public SVG screenshots must finish for finite offscreen column-rule geometry.
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use turbo_dom_layout::{
    layout_html_tree_in_viewport, parse_html_tree, Diagnostics, FontRegistry, Fragment,
    FragmentContent, HtmlOptions, ImageCtx, NoImages,
};
use turbo_dom_raster::{screenshot_svg_with_opts, ImageAssets, Viewport};

const CHILD: &str = "TURBO_DOM_LARGE_SVG_COLUMN_RULE_CHILD";
const TEST: &str = "large_finite_svg_column_rule_finishes";
const PAGE: &str = "<!DOCTYPE html><html><head><style>body{margin:0;background:white}</style></head><body><div style='position:absolute;left:0;top:1000000000px;width:60px;height:128px;columns:2;column-gap:8px;column-rule:1px dashed red'><div style='height:64px;background:#eef'></div><div style='height:64px;background:#eef'></div><div style='height:64px;background:#eef'></div><div style='height:64px;background:#eef'></div></div></body></html>";

fn column_rule(fragment: &Fragment) -> Option<&Fragment> {
    if matches!(fragment.content, FragmentContent::ColumnRule { .. }) {
        return Some(fragment);
    }
    fragment.children.iter().find_map(column_rule)
}

fn screenshot_child() {
    let rule_css = match std::env::var(CHILD).unwrap().as_str() {
        "dashed" => "column-rule:1px dashed red",
        "dotted" => "column-rule:1px dotted red",
        "subpixel" => "column-rule:0.5px dotted red",
        other => panic!("unknown child case {other}"),
    };
    let page = PAGE.replace("column-rule:1px dashed red", rule_css);
    let options = HtmlOptions {
        uncapped_images: true,
        ..HtmlOptions::default()
    };
    let layout = layout_html_tree_in_viewport(
        &parse_html_tree(&page),
        "",
        (64.0, 64.0),
        &FontRegistry::new(),
        &ImageCtx {
            resolver: &NoImages,
            body_height: None,
        },
        &options,
        &mut Diagnostics::default(),
    );
    let rule = column_rule(&layout).expect("fixture generates an actual column-rule fragment");
    assert_eq!(rule.y, 1_000_000_000.0);
    assert!(rule.height >= 128.0, "rule height: {}", rule.height);
    assert!(rule.y + rule.height > rule.y);
    assert_eq!(rule.y + 3.0 * rule.width, rule.y);
    println!(
        "SVG_PREFLIGHT x={} y={} width={} height={} opacity={}",
        rule.x, rule.y, rule.width, rule.height, rule.opacity
    );
    let viewport = Viewport {
        width: 64,
        height: 64,
    };
    let svg = screenshot_svg_with_opts(&page, "", viewport, &ImageAssets::new(), false, false)
        .expect("public SVG screenshot");
    println!("SVG_SCREENSHOT_RETURNED bytes={}", svg.len());
    let pixels = rasterize(&svg);
    assert!(pixels.pixels().iter().all(|pixel| pixel.red() == 255
        && pixel.green() == 255
        && pixel.blue() == 255
        && pixel.alpha() == 255));
    let visible = page.replace(
        "top:1000000000px",
        "top:1000000000px;transform:translateY(-1000000000px)",
    );
    let ordinary = page.replace("top:1000000000px", "top:0");
    let render = |html: &str| {
        let svg = screenshot_svg_with_opts(html, "", viewport, &ImageAssets::new(), false, false)
            .expect("transformed SVG screenshot");
        rasterize(&svg)
    };
    let expected = render(&ordinary);
    assert!(
        expected
            .pixels()
            .iter()
            .filter(|pixel| pixel.red().saturating_sub(pixel.green()) > 24)
            .count()
            > 8,
        "the ordinary control contains multiple visible red marks"
    );
    assert_eq!(
        render(&visible).data(),
        expected.data(),
        "a transform bringing the large-origin rule onscreen retains its full pattern"
    );
}

fn rasterize(svg: &str) -> tiny_skia::Pixmap {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .expect("well-formed finite SVG output");
    let mut pixels = tiny_skia::Pixmap::new(64, 64).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    pixels
}

fn run_child(case: &str) {
    // A broken progress loop stays inside a disposable process, not the test runner.
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture"])
        .env(CHILD, case)
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "SVG screenshot child failed: {status}");
            return;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("finite SVG screenshot did not finish within three seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn large_finite_svg_column_rule_finishes() {
    if std::env::var_os(CHILD).is_some() {
        screenshot_child();
        return;
    }
    for case in ["dashed", "dotted", "subpixel"] {
        run_child(case);
    }
}

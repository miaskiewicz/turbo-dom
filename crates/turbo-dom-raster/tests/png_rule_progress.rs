//! Public PNG screenshots must finish for finite offscreen column-rule geometry.
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use turbo_dom_layout::{
    layout_html_tree_in_viewport, parse_html_tree, Diagnostics, FontRegistry, Fragment,
    FragmentContent, HtmlOptions, ImageCtx, NoImages,
};
use turbo_dom_raster::{screenshot_png_with_opts, ImageAssets, Viewport};

const CHILD: &str = "TURBO_DOM_LARGE_PNG_COLUMN_RULE_CHILD";
const TEST: &str = "large_finite_png_column_rule_finishes";
const PAGE: &str = "<!DOCTYPE html><html><head><style>body{margin:0;background:white}</style></head><body><div style='position:absolute;left:0;top:1000000000px;width:60px;height:128px;columns:2;column-gap:8px;column-rule:1px dashed red'><div style='height:64px;background:#eef'></div><div style='height:64px;background:#eef'></div><div style='height:64px;background:#eef'></div><div style='height:64px;background:#eef'></div></div></body></html>";

fn column_rule(fragment: &Fragment) -> Option<&Fragment> {
    if matches!(fragment.content, FragmentContent::ColumnRule { .. }) {
        return Some(fragment);
    }
    fragment.children.iter().find_map(column_rule)
}

fn public_layout(page: &str) -> Fragment {
    let options = HtmlOptions {
        uncapped_images: true,
        ..HtmlOptions::default()
    };
    layout_html_tree_in_viewport(
        &parse_html_tree(page),
        "",
        (64.0, 64.0),
        &FontRegistry::new(),
        &ImageCtx {
            resolver: &NoImages,
            body_height: None,
        },
        &options,
        &mut Diagnostics::default(),
    )
}

fn screenshot_child() {
    let layout = public_layout(PAGE);
    let rule = column_rule(&layout).expect("fixture generates an actual column-rule fragment");
    assert_eq!(rule.y, 1_000_000_000.0);
    assert!(rule.height >= 128.0, "rule height: {}", rule.height);
    assert!(rule.y + rule.height > rule.y);
    assert_eq!(rule.y + 3.0 * rule.width, rule.y);
    println!(
        "PNG_PREFLIGHT x={} y={} width={} height={} opacity={}",
        rule.x, rule.y, rule.width, rule.height, rule.opacity
    );
    let viewport = Viewport {
        width: 64,
        height: 64,
    };
    let png = screenshot_png_with_opts(PAGE, "", viewport, &ImageAssets::new(), false, false)
        .expect("public PNG screenshot");
    println!("PNG_SCREENSHOT_RETURNED bytes={}", png.len());
    let pixels = image::load_from_memory(&png).expect("valid PNG").to_rgb8();
    assert!(pixels.pixels().all(|pixel| pixel.0 == [255; 3]));
    later_cursor_boundaries_finish(viewport);
    for style in ["1px dashed red", "1px dotted red", "0.5px dotted red"] {
        let page = PAGE.replace("1px dashed red", style);
        let normal = page.replace("top:1000000000px", "top:0px");
        let translated = page.replace(
            "top:1000000000px",
            "top:1000000000px;transform:translateY(-1000000000px)",
        );
        let ordinary =
            screenshot_png_with_opts(&normal, "", viewport, &ImageAssets::new(), false, false)
                .unwrap();
        let returned =
            screenshot_png_with_opts(&translated, "", viewport, &ImageAssets::new(), false, false)
                .unwrap();
        let ordinary = image::load_from_memory(&ordinary).unwrap().to_rgb8();
        let returned = image::load_from_memory(&returned).unwrap().to_rgb8();
        assert!(
            ordinary
                .pixels()
                .any(|pixel| pixel.0[0] > 200 && pixel.0[1] < 200),
            "{style}: actual visible red column rule"
        );
        assert!(
            ordinary.as_raw() == returned.as_raw(),
            "{style}: translated large-origin pattern matches ordinary native pixels"
        );
    }
}

fn later_cursor_boundaries_finish(viewport: Viewport) {
    for (style, top, period) in [
        ("1px dashed red", 67_108_860.0_f32, 3.0),
        ("1px dotted red", 33_554_430.0_f32, 2.0),
        ("0.5px dotted red", 33_554_430.0_f32, 2.0),
    ] {
        let page = PAGE
            .replace("1px dashed red", style)
            .replace("top:1000000000px", &format!("top:{top}px"));
        let layout = public_layout(&page);
        let rule = column_rule(&layout).expect("actual boundary column rule");
        assert_eq!(rule.y, top);
        let first = rule.y + period;
        assert!(first > rule.y, "first cursor step advances");
        assert_eq!(first + period, first, "the following step loses progress");
        let png = screenshot_png_with_opts(&page, "", viewport, &ImageAssets::new(), false, false)
            .unwrap();
        let image = image::load_from_memory(&png).unwrap().to_rgb8();
        assert!(
            image.pixels().all(|pixel| pixel.0 == [255; 3]),
            "offscreen {style}"
        );
    }
}

#[test]
fn large_finite_png_column_rule_finishes() {
    if std::env::var_os(CHILD).is_some() {
        screenshot_child();
        return;
    }
    // A broken progress loop stays inside a disposable process, not the test runner.
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture"])
        .env(CHILD, "1")
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "PNG screenshot child failed: {status}");
            return;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("finite PNG screenshot did not finish within three seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

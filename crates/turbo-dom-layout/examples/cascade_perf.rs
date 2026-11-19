//! Paired release-build workloads for cascade allocation experiments.
use std::fmt::Write;
use std::hint::black_box;
use std::time::Instant;

use sha2::{Digest, Sha256};
use turbo_dom_layout::style::{
    parse_declarations, resolve_matched_style, CascadeOrigin, ComputedStyle, MatchedDeclaration,
};
use turbo_dom_layout::{
    layout_html_tree_in_viewport, parse_html_tree, style_html_tree, Diagnostics, FontRegistry,
    HtmlOptions, ImageCtx, ImageResolver,
};

struct NoImages;
impl ImageResolver for NoImages {
    fn resolve(&self, _: &str) -> Option<&[u8]> {
        None
    }
}

fn report(rows: usize) -> String {
    let mut html = String::from("<!doctype html><style>body{font:12px Inter;color:#123456}table{width:700px;border-collapse:collapse}td{padding:3px 6px;border:1px solid #ddd}td.amount{text-align:right;font-weight:bold}</style><table>");
    for row in 0..rows {
        write!(html, "<tr><td>Item {row}</td><td>Product description {row}</td><td class=amount>{row}.50</td></tr>").unwrap();
    }
    html.push_str("</table>");
    html
}

fn cards() -> String {
    let mut html = String::from("<!doctype html><style>body{font:12px Inter}.card{padding:4px;margin:2px;border:1px solid #ddd}strong{color:#345678}</style>");
    for row in 0..100 {
        write!(html, "<div class=card style='color:#123456;background-color:white;line-height:16px;font-size:12px'><strong>Card {row}</strong><p>Details for card {row}</p></div>").unwrap();
    }
    html
}

fn article() -> String {
    format!("<!doctype html><style>body{{font:14px Inter;width:700px}}p{{margin:4px 0;color:#234567;line-height:20px}}</style>{}", "<p>A paragraph with repeated words, punctuation and readable text.</p>".repeat(100))
}

fn definite_items(display: &str) -> String {
    let tracks = if display == "grid" {
        "grid-template-columns:repeat(10,64px);grid-template-rows:repeat(10,24px)"
    } else {
        "flex-wrap:wrap;align-content:start;align-items:start"
    };
    let mut html = format!(
        "<!doctype html><style>body{{margin:0;font:12px Inter}}.items{{display:{display};width:712px;height:312px;gap:8px;{tracks}}}.item{{box-sizing:border-box;flex:none;width:64px;height:24px;line-height:16px;background:#def}}</style><div class=items>"
    );
    for item in 0..100 {
        write!(html, "<div class=item>Item {item}</div>").unwrap();
    }
    html.push_str("</div>");
    html
}

fn measure<T: std::fmt::Debug>(
    name: &str,
    iterations: usize,
    only: Option<&str>,
    mut run: impl FnMut() -> T,
) -> usize {
    if only.is_some_and(|selected| selected != name) {
        return 0;
    }
    let digest = format!("{:x}", Sha256::digest(format!("{:?}", run())));
    for _ in 0..3 {
        black_box(run());
    }
    let started = Instant::now();
    for _ in 0..iterations {
        black_box(run());
    }
    println!(
        "{{\"name\":\"{name}\",\"ns\":{},\"iterations\":{iterations},\"digest\":\"{digest}\"}}",
        started.elapsed().as_nanos()
    );
    1
}

fn matched_overrides(iterations: usize, only: Option<&str>) -> usize {
    let mut measured = 0;
    for (name, count, size) in [
        ("matched_override_small", 1, 64),
        ("matched_override_payload", 1, 16 * 1024),
        ("matched_override_theme", 8, 64),
    ] {
        let parent = ComputedStyle::from_pairs(
            (0..count).map(|index| (format!("--theme-{index}"), "x".repeat(size))),
        );
        let mut css = String::new();
        for index in 0..count {
            write!(css, "--theme-{index}:blue;").unwrap();
        }
        let declarations: Vec<_> = parse_declarations(&css)
            .into_iter()
            .enumerate()
            .map(|(order, declaration)| MatchedDeclaration {
                declaration,
                origin: CascadeOrigin::Author,
                scope_depth: 0,
                inline: false,
                specificity: (0, 0, 1),
                order,
            })
            .collect();
        measured += measure(name, iterations, only, || {
            let mut result = ComputedStyle::default();
            for _ in 0..100 {
                result = black_box(resolve_matched_style(&parent, &declarations));
            }
            result
        });
    }
    measured
}

fn main() {
    let mut args = std::env::args().skip(1);
    let iterations = args.next().map_or(20, |v| v.parse::<usize>().unwrap());
    let only = args.next().map(|flag| {
        assert_eq!(flag, "--only", "expected optional --only WORKLOAD");
        args.next().expect("--only requires a workload name")
    });
    assert!(args.next().is_none(), "unexpected extra arguments");
    let only = only.as_deref();
    let mut measured = 0;
    assert!(iterations > 0);
    let fonts = FontRegistry::new();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let options = HtmlOptions::default();
    for (name, html) in [
        ("report100", report(100)),
        ("report10", report(10)),
        ("cards100", cards()),
        ("article100", article()),
        ("definite_flex100", definite_items("flex")),
        ("definite_grid100", definite_items("grid")),
    ] {
        let tree = parse_html_tree(&html);
        measured += measure(&format!("style_{name}"), iterations, only, || {
            style_html_tree(&tree, "", 800.0, &options)
        });
        measured += measure(&format!("layout_{name}"), iterations, only, || {
            layout_html_tree_in_viewport(
                &tree,
                "",
                (800.0, 600.0),
                &fonts,
                &images,
                &options,
                &mut Diagnostics::default(),
            )
        });
    }
    measured += matched_overrides(iterations, only);
    assert!(measured > 0, "unknown workload selection");
}

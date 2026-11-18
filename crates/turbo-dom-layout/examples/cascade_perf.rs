//! Paired release-build workloads for cascade allocation experiments.
use std::fmt::Write;
use std::hint::black_box;
use std::time::Instant;

use sha2::{Digest, Sha256};
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

fn measure<T: std::fmt::Debug>(name: &str, iterations: usize, mut run: impl FnMut() -> T) {
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
}

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .map_or(20, |v| v.parse::<usize>().unwrap());
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
    ] {
        let tree = parse_html_tree(&html);
        measure(&format!("style_{name}"), iterations, || {
            style_html_tree(&tree, "", 800.0, &options)
        });
        measure(&format!("layout_{name}"), iterations, || {
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
}

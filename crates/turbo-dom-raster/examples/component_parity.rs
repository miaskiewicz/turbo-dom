//! Dev-only component screenshot/geometry driver. See tools/component-parity/README.md.
use serde_json::{json, Value};
use std::error::Error;
use std::path::Path;
use turbo_dom_raster::{
    screenshot_element_rects, screenshot_png_with_options, ImageAssets, ScreenshotOptions, Viewport,
};

fn assets(directory: &Path) -> Result<ImageAssets, Box<dyn Error>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let path = entry?.path();
            let name = path.file_name().ok_or("asset filename")?.to_string_lossy();
            Ok((format!("/assets/{name}"), std::fs::read(&path)?))
        })
        .collect()
}

fn selections(
    html: &str,
    selectors: &[Value],
) -> Result<(Vec<usize>, Vec<String>), Box<dyn Error>> {
    let tree = turbo_dom_layout::parse_html_tree(html);
    let all = tree.query_selector_all("*").to_vec();
    let mut indices = Vec::new();
    let mut keys = Vec::new();
    for selector in selectors {
        let selector = selector.as_str().ok_or("selector must be a string")?;
        for (index, node) in tree.query_selector_all(selector).iter().enumerate() {
            indices.push(
                all.iter()
                    .position(|other| other == node)
                    .ok_or("element index")?,
            );
            keys.push(format!("{selector}:{index}"));
        }
    }
    Ok((indices, keys))
}

fn main() -> Result<(), Box<dyn Error>> {
    let request = std::env::args()
        .nth(1)
        .ok_or("usage: component_parity REQUEST.json")?;
    let request: Value = serde_json::from_slice(&std::fs::read(request)?)?;
    let html = std::fs::read_to_string(request["fixture"].as_str().ok_or("fixture path")?)?;
    let output = request["output"].as_str().ok_or("output prefix")?;
    let viewport = Viewport {
        width: u32::try_from(request["width"].as_u64().ok_or("width")?)?,
        height: u32::try_from(request["height"].as_u64().ok_or("height")?)?,
    };
    let images = assets(Path::new(request["assets"].as_str().ok_or("assets path")?))?;
    let mut options = ScreenshotOptions::default();
    options.system_fonts = true;
    let selectors = request["selectors"].as_array().ok_or("selectors array")?;
    let (indices, keys) = selections(&html, selectors)?;
    let rects = screenshot_element_rects(&html, "", viewport, &images, &options, &indices)?;
    let boxes: Vec<_> = keys
        .into_iter()
        .zip(rects)
        .map(|(key, rect)| json!({"key": key, "rect": rect.map(|r| [r.x, r.y, r.width, r.height])}))
        .collect();
    std::fs::write(
        format!("{output}.png"),
        screenshot_png_with_options(&html, "", viewport, &images, &options)?,
    )?;
    std::fs::write(format!("{output}.json"), serde_json::to_vec_pretty(&boxes)?)?;
    Ok(())
}

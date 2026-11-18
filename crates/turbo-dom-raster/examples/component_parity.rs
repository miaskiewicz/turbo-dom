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

fn viewport(request: &Value) -> Result<Viewport, Box<dyn Error>> {
    Ok(Viewport {
        width: u32::try_from(request["width"].as_u64().ok_or("width")?)?,
        height: u32::try_from(request["height"].as_u64().ok_or("height")?)?,
    })
}

fn field<'a>(request: &'a Value, name: &str) -> Result<&'a str, Box<dyn Error>> {
    request[name]
        .as_str()
        .ok_or_else(|| format!("{name} must be a string").into())
}

fn render_request(request: &Value) -> Result<(Vec<Value>, Vec<u8>), Box<dyn Error>> {
    let html = std::fs::read_to_string(field(request, "fixture")?)?;
    let viewport = viewport(request)?;
    let images = assets(Path::new(field(request, "assets")?))?;
    let mut options = ScreenshotOptions::default();
    options.system_fonts = true;
    let selectors = request["selectors"].as_array().ok_or("selectors array")?;
    render(&html, selectors, viewport, &images, &options)
}

fn render(
    html: &str,
    selectors: &[Value],
    viewport: Viewport,
    images: &ImageAssets,
    options: &ScreenshotOptions,
) -> Result<(Vec<Value>, Vec<u8>), Box<dyn Error>> {
    let (indices, keys) = selections(html, selectors)?;
    let rects = screenshot_element_rects(html, "", viewport, images, options, &indices)?;
    let boxes = keys
        .into_iter()
        .zip(rects)
        .map(|(key, rect)| json!({"key": key, "rect": rect.map(|r| [r.x, r.y, r.width, r.height])}))
        .collect();
    let png = screenshot_png_with_options(html, "", viewport, images, options)?;
    Ok((boxes, png))
}

fn write_capture(output: &str, boxes: &[Value], png: &[u8]) -> Result<(), Box<dyn Error>> {
    std::fs::write(format!("{output}.png"), png)?;
    std::fs::write(format!("{output}.json"), serde_json::to_vec_pretty(boxes)?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let request = std::env::args()
        .nth(1)
        .ok_or("usage: component_parity REQUEST.json")?;
    let request: Value = serde_json::from_slice(&std::fs::read(request)?)?;
    let (boxes, png) = render_request(&request)?;
    write_capture(field(&request, "output")?, &boxes, &png)
}

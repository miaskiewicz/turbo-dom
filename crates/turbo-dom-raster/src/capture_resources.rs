//! Capture resource planning without fetching or decoding pixel bodies.
use crate::{image_paint, ScreenshotOptions, Viewport};
use std::collections::{BTreeSet, HashMap};
use turbo_dom_layout::{image::Intrinsic, Fragment, FragmentContent};

/// Intrinsic image headers, keyed like `ImageAssets`, retained without pixel bodies.
pub type ImageMetadata = HashMap<String, Intrinsic>;

/// Required pixel bodies and deterministic planning work counters.
#[derive(Debug, Default)]
pub struct CaptureResources {
    /// Resolver keys whose pixels may contribute to this capture, sorted and unique.
    pub pixel_urls: Vec<String>,
    /// Laid-out fragments examined, including offscreen descendants.
    pub fragments_examined: usize,
}

/// Plan using the screenshot's own layout, without fetching pixel bodies. Supply intrinsic
/// headers independently of bytes; missing headers conservatively keep a resource selected
/// because its eventual intrinsic size can change layout. Keep the same metadata in
/// `ScreenshotOptions::image_metadata` when painting with only the selected pixel bodies.
///
/// Descendants are always visited, including positioned overflow. Transformed or shadowed
/// subtrees and filter-bearing input keep all their resources: over-selection is preferable
/// to dropping extending ink. Clipping/occlusion are deliberately not used to reject URLs.
#[must_use]
pub fn capture_resources(
    html: &str,
    css: &str,
    viewport: Viewport,
    metadata: &ImageMetadata,
    options: &ScreenshotOptions,
) -> CaptureResources {
    let mut decoded = image_paint::DecodedAssets::new();
    add_metadata(&mut decoded, Some(metadata));
    image_paint::add_data_uri_images(&mut decoded, css, html);
    let tree = crate::parse_document(html, &mut decoded, options);
    let galley =
        options.with_fonts(|fonts| crate::lay_out(&tree, css, viewport, &decoded, fonts, options));
    let height = crate::paint_height(&galley, viewport, options.full_page);
    let all = crate::image_urls(html)
        .into_iter()
        .chain(crate::image_urls_in_css(css))
        .collect::<BTreeSet<_>>();
    let mut selected: BTreeSet<_> = all
        .iter()
        .filter(|url| !metadata.contains_key(*url))
        .cloned()
        .collect();
    let mut examined = 0;
    let conservative =
        html.to_ascii_lowercase().contains("filter") || css.to_ascii_lowercase().contains("filter");
    collect(
        &galley,
        viewport.width as f32,
        height as f32,
        conservative,
        &mut selected,
        &mut examined,
    );
    CaptureResources {
        pixel_urls: selected.intersection(&all).cloned().collect(),
        fragments_examined: examined,
    }
}

fn collect(
    fragment: &Fragment,
    width: f32,
    height: f32,
    inherited: bool,
    selected: &mut BTreeSet<String>,
    examined: &mut usize,
) {
    *examined += 1;
    let extending = inherited
        || matches!(
            &fragment.content,
            FragmentContent::Box {
                transform: Some(_),
                ..
            } | FragmentContent::Box {
                shadow: Some(_),
                ..
            }
        );
    if extending || intersects(fragment, width, height) {
        collect_content(&fragment.content, selected);
    }
    for child in &fragment.children {
        collect(child, width, height, extending, selected, examined);
    }
}

fn intersects(fragment: &Fragment, width: f32, height: f32) -> bool {
    fragment.x < width
        && fragment.y < height
        && fragment.x + fragment.width > 0.0
        && fragment.y + fragment.height > 0.0
}

fn collect_content(content: &FragmentContent, selected: &mut BTreeSet<String>) {
    match content {
        FragmentContent::Image(image) => {
            if !image.name.starts_with("data:") {
                selected.insert(image.name.clone());
            }
        }
        FragmentContent::Tiles(pattern) => collect_content(&pattern.tile, selected),
        _ => {}
    }
}

pub(crate) fn add_metadata(
    decoded: &mut image_paint::DecodedAssets,
    metadata: Option<&ImageMetadata>,
) {
    for (key, intrinsic) in metadata.into_iter().flatten() {
        decoded
            .entry(key.clone())
            .or_insert_with(|| image_paint::DecodedImage {
                // Intrinsic dimensions describe layout, not an available pixel buffer.
                width: 0,
                height: 0,
                rgba: Vec::new(),
                intrinsic: *intrinsic,
                placeholder: true,
            });
    }
}

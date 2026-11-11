use crate::{
    image_paint, lay_out, paint_height, paint_png, paint_svg, parse_document,
    screenshot_canvas_background, Fragment, ImageAssets, Rgba, ScreenshotOptions, Viewport,
};

/// An immutable, owned snapshot of a screenshot's layout and decoded resources.
///
/// Prepare once per document revision, then call [`Self::png`] or [`Self::svg`] any
/// number of times without parsing, decoding or laying out again. All inputs are
/// consumed into the prepared display list and owned image pixels; no caller data
/// is borrowed. Mutating HTML, CSS, viewport, fonts, image metadata/bytes, canvas
/// contents or IDL state requires constructing a replacement snapshot. Keeping
/// both snapshots alive is safe: neither can change the other's output.
///
/// This explicit lifetime avoids a partial cache key or a caller-maintained
/// revision number silently reusing stale font/layout state. No global scene cache
/// exists, and dropping the snapshot releases its retained resources.
pub struct PreparedScreenshot {
    galley: Fragment,
    decoded: image_paint::DecodedAssets,
    width: u32,
    height: u32,
    background: Rgba,
    scale: f32,
}

impl PreparedScreenshot {
    /// Snapshot all inputs, decode resources and perform one layout pass.
    #[must_use]
    pub fn new(
        html: &str,
        external_css: &str,
        viewport: Viewport,
        images: &ImageAssets,
        options: &ScreenshotOptions,
    ) -> Self {
        let mut decoded = image_paint::decode_all(images);
        crate::capture_resources::add_metadata(&mut decoded, options.image_metadata);
        image_paint::add_data_uri_images(&mut decoded, external_css, html);
        let tree = parse_document(html, &mut decoded, options);
        let galley = options
            .with_fonts(|fonts| lay_out(&tree, external_css, viewport, &decoded, fonts, options));
        let background = screenshot_canvas_background(&galley, options.omit_background);
        let height = paint_height(&galley, viewport, options.full_page);
        Self {
            galley,
            decoded,
            width: viewport.width,
            height,
            background,
            scale: options.device_scale,
        }
    }

    /// Paint the retained layout to PNG without re-reading any preparation input.
    pub fn png(&self) -> Result<Vec<u8>, String> {
        paint_png::paint_scaled(
            &self.galley,
            self.width,
            self.height,
            self.background,
            &self.decoded,
            self.scale,
        )
    }

    /// Paint the same retained layout to a standalone SVG.
    pub fn svg(&self) -> Result<String, String> {
        let pixels = crate::screenshot_pixel_size(self.width, self.height, self.scale)?;
        Ok(paint_svg::paint_at_size(
            &self.galley,
            (self.width, self.height),
            pixels,
            self.background,
            &self.decoded,
            self.scale,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_paints_do_not_build_a_layout() {
        let before = crate::LAYOUT_PASSES.with(std::cell::Cell::get);
        let scene = PreparedScreenshot::new(
            "<body>snapshot",
            "",
            Viewport {
                width: 20,
                height: 20,
            },
            &ImageAssets::new(),
            &ScreenshotOptions::default(),
        );
        assert_eq!(crate::LAYOUT_PASSES.with(std::cell::Cell::get), before + 1);
        for _ in 0..3 {
            scene.png().unwrap();
            let _ = scene.svg();
        }
        assert_eq!(crate::LAYOUT_PASSES.with(std::cell::Cell::get), before + 1);
        // The stateless public path is the prepare-per-capture control, using
        // precisely the same inputs and real layout instrumentation.
        for _ in 0..3 {
            crate::screenshot_png_with_options(
                "<body>snapshot",
                "",
                Viewport {
                    width: 20,
                    height: 20,
                },
                &ImageAssets::new(),
                &ScreenshotOptions::default(),
            )
            .unwrap();
        }
        assert_eq!(crate::LAYOUT_PASSES.with(std::cell::Cell::get), before + 4);
    }
}

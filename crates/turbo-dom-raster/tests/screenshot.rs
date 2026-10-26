//! Offline end-to-end: an HTML string renders into a valid PNG and a valid SVG.

use std::collections::HashMap;

use turbo_surf_raster::{
    image_urls, screenshot_png, screenshot_png_with_assets, screenshot_svg,
    screenshot_svg_with_assets, Format, ImageAssets, Viewport,
};

/// A `w × h` solid-colour RGBA PNG fixture (built with tiny-skia so the test
/// needs no committed binary).
fn solid_png(w: u32, h: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
    pm.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    pm.encode_png().unwrap()
}

const PAGE: &str = r#"<html><head>
    <style>.card{background-color:#3366cc;padding:16px} p{color:#ffffff;font-size:24px}</style>
  </head><body>
    <div class="card"><p>Hello Screenshot</p></div>
  </body></html>"#;

#[test]
fn renders_png_with_magic_and_size() {
    let vp = Viewport {
        width: 640,
        height: 200,
    };
    let png = screenshot_png(PAGE, vp).expect("png");
    // PNG signature.
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    // IHDR carries width/height big-endian at bytes 16..24.
    let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert_eq!((w, h), (640, 200));
}

#[test]
fn renders_svg_with_boxes_and_glyph_paths() {
    let svg = screenshot_svg(PAGE, Viewport::DEFAULT).expect("svg");
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("</svg>"));
    // The blue card background.
    assert!(svg.contains("#3366cc"), "expected card background rect");
    // Text rendered as outline paths (self-contained, no viewer font).
    assert!(svg.contains("<path d=\"M"), "expected glyph outline paths");
}

#[test]
fn z_index_controls_stacking_paint_order() {
    // Two overlapping absolutely-positioned boxes: the box that appears LATER in
    // the DOM has the LOWER z-index, so it must paint UNDERNEATH. SVG paints in
    // document order (later markup = on top), so in the output the low-z green
    // rect must come before the high-z red rect.
    let page = r#"<html><body style="margin:0">
        <div style="position:absolute;top:0;left:0;width:80px;height:80px;background-color:#ff0000;z-index:2"></div>
        <div style="position:absolute;top:0;left:0;width:80px;height:80px;background-color:#00aa00;z-index:1"></div>
      </body></html>"#;
    let svg = screenshot_svg(page, Viewport::DEFAULT).expect("svg");
    let green = svg.find("#00aa00").expect("green rect present");
    let red = svg.find("#ff0000").expect("red rect present");
    assert!(green < red, "z:1 (green) must paint before z:2 (red)");
}

#[test]
fn format_dispatch_matches_direct() {
    let vp = Viewport {
        width: 320,
        height: 240,
    };
    let via_enum = turbo_surf_raster::screenshot(PAGE, vp, Format::Png).expect("png");
    assert_eq!(&via_enum[..4], &[0x89, b'P', b'N', b'G']);
}

#[test]
fn image_urls_lists_img_and_background_refs() {
    let html = r#"<img src="/logo.png"><div style="background-image:url(bg.jpg)"></div>"#;
    assert_eq!(image_urls(html), vec!["/logo.png", "bg.jpg"]);
}

#[test]
fn css_in_js_inline_style_blocks_apply() {
    // CSS-in-JS (Emotion/styled-components, e.g. nike.com's 258 inline <style>
    // blocks with `.css-xxxxx` hashed classes) must cascade: the page's own inline
    // <style> sheets style the elements that reference them — including a `>`
    // combinator and an `@media` query — not just an external stylesheet.
    let page = r#"<html><head>
        <style>.css-a{background-color:rgb(20,120,220);width:100px;height:100px}</style>
        <style>.css-b{display:flex}.css-b>span{background-color:rgb(220,40,40);width:40px;height:40px}</style>
        <style>@media (min-width:100px){.css-c{background-color:rgb(30,180,60);width:60px;height:60px}}</style>
      </head><body style="margin:0">
        <div class="css-a"></div>
        <div class="css-b"><span>x</span></div>
        <div class="css-c"></div>
      </body></html>"#;
    let png = screenshot_png_with_assets(
        page,
        "",
        Viewport {
            width: 400,
            height: 400,
        },
        &ImageAssets::new(),
        false,
    )
    .expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let near = |x: u32, y: u32, c: (u8, u8, u8)| {
        let p = pm.pixel(x, y).unwrap();
        (p.red() as i32 - c.0 as i32).abs() < 30
            && (p.green() as i32 - c.1 as i32).abs() < 30
            && (p.blue() as i32 - c.2 as i32).abs() < 30
    };
    assert!(near(50, 50, (20, 120, 220)), "hashed-class bg (.css-a)");
    assert!(
        near(20, 118, (220, 40, 40)),
        "flex child via `>` (.css-b>span)"
    );
    assert!(near(30, 165, (30, 180, 60)), "@media hashed class (.css-c)");
}

#[test]
fn png_paints_supplied_image_bytes_not_a_placeholder() {
    // A 40×40 red `<img>` whose bytes the caller supplies must render as red
    // pixels (the fixture colour), not the grey placeholder or the white canvas.
    let mut assets: ImageAssets = HashMap::new();
    assets.insert("r.png".to_string(), solid_png(4, 4, 255, 0, 0));
    let page = r#"<html><body style="margin:0">
        <img src="r.png" style="width:40px;height:40px">
      </body></html>"#;
    let vp = Viewport {
        width: 100,
        height: 100,
    };
    let png = screenshot_png_with_assets(page, "", vp, &assets, false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode output");
    // Sample the middle of the image box (~20,20).
    let px = pm.pixel(20, 20).expect("pixel");
    assert!(
        px.red() > 200 && px.green() < 60 && px.blue() < 60,
        "image box should be red, got ({},{},{})",
        px.red(),
        px.green(),
        px.blue()
    );
}

#[test]
fn mask_image_icon_paints_tint_through_svg_alpha() {
    // Real Wikipedia TOC-toggle caret markup: a `.vector-icon` glyph rendered with
    // `-webkit-mask-image` (an SVG) tinted by `background-color`. The mask's alpha —
    // the caret shape — selects where the tint paints. Before mask support turbo
    // dropped the `::mask` and painted a solid coloured square (or nothing); this
    // asserts (a) the caret's tint DOES paint inside the glyph, and (b) an in-box
    // point OUTSIDE the glyph stays blank, i.e. it is a stencilled mask, not a rect.
    let caret_svg =
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><polygon points="2,3 18,3 10,14" fill="#000"/></svg>"##
            .to_vec();
    let mut assets: ImageAssets = HashMap::new();
    assets.insert("caret.svg".to_string(), caret_svg);
    // Real markup (cloned from en.wikipedia.org/wiki/Cat): the TOC-toggle button and
    // its `mw-ui-icon-wikimedia-expand` icon span.
    let page = r#"<html><body style="margin:0">
        <button class="cdx-button cdx-button--icon-only vector-toc-toggle">
          <span class="vector-icon mw-ui-icon-wikimedia-expand"></span>
        </button>
      </body></html>"#;
    // The mask technique, mirroring Vector's `.vector-icon` rule (mask-image glyph +
    // background-color tint). Distinct blue tint so we can find it in the output.
    let css = ".vector-toc-toggle{display:block;padding:0;border:0;background:transparent}\
               .vector-icon{-webkit-mask-image:url(caret.svg);background-color:#2244dd;display:block;width:20px;height:20px}";
    let vp = Viewport {
        width: 60,
        height: 60,
    };
    let png = screenshot_png_with_assets(page, css, vp, &assets, false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    // Inside the caret (near its top-center, y<14): tinted blue.
    let on = pm.pixel(10, 6).expect("pixel");
    assert!(
        on.blue() > 140 && on.red() < 130 && on.green() < 140,
        "caret glyph must paint the blue tint, got ({},{},{})",
        on.red(),
        on.green(),
        on.blue()
    );
    // Inside the 20×20 box but BELOW the caret apex (y=18 > polygon bottom 14): blank
    // canvas — proves a masked glyph, not a solid filled rectangle.
    let off = pm.pixel(3, 18).expect("pixel");
    assert!(
        off.blue() > 200 && off.red() > 200 && off.green() > 200,
        "outside the caret shape must stay blank (mask, not a solid box), got ({},{},{})",
        off.red(),
        off.green(),
        off.blue()
    );
}

#[test]
fn datauri_mask_image_icon_paints_without_a_fetched_asset() {
    // The Wikipedia TOC-toggle caret exactly as it ships: an EMPTY icon span whose
    // `mask-image` is an inline `data:image/svg+xml` URI (not a fetched asset), with
    // the SVG's own `"` CSS-escaped (`\"`) and `#` percent-encoded (`%23`) inside the
    // `url()` string. The fetch step skips `data:` URIs, so before the raster decoded
    // them itself the mask resolved to nothing and the caret vanished from the box
    // tree. This asserts the chevron tint paints straight from the data URI — no
    // `ImageAssets` entry supplied.
    let mask = "url(\"data:image/svg+xml;utf8,<svg xmlns=\\\"http://www.w3.org/2000/svg\\\" \
        width=\\\"20\\\" height=\\\"20\\\" viewBox=\\\"0 0 20 20\\\" fill=\\\"%232244dd\\\">\
        <path d=\\\"M2 3 H18 L10 14 Z\\\"/></svg>\")";
    let page = r#"<html><body style="margin:0">
        <button class="cdx-button cdx-button--icon-only vector-toc-toggle">
          <span class="vector-icon mw-ui-icon-wikimedia-expand"></span>
        </button>
      </body></html>"#;
    // Vector's real rule shape: mask-image glyph + background-color tint, on an
    // inline-block empty span sized by the icon rule (not the caller).
    let css = format!(
        ".vector-toc-toggle{{display:block;padding:0;border:0;background:transparent}}\
         .mw-ui-icon-wikimedia-expand{{-webkit-mask-image:{mask};mask-image:{mask};\
         background-color:#2244dd;display:inline-block;width:20px;height:20px}}"
    );
    let vp = Viewport {
        width: 60,
        height: 60,
    };
    // No supplied assets — the mask must come from the inline data URI alone.
    let assets: ImageAssets = HashMap::new();
    let png = screenshot_png_with_assets(page, &css, vp, &assets, false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    // Inside the caret (top-center, y<12): the blue tint painted through the mask.
    let on = pm.pixel(10, 5).expect("pixel");
    assert!(
        on.blue() > 140 && on.red() < 130 && on.green() < 140,
        "data-URI caret must paint the tint, got ({},{},{})",
        on.red(),
        on.green(),
        on.blue()
    );
    // Below the caret apex (y=18): blank — a stencilled mask, not a filled box.
    let off = pm.pixel(3, 18).expect("pixel");
    assert!(
        off.blue() > 200 && off.red() > 200 && off.green() > 200,
        "outside the caret shape must stay blank (mask, not a box), got ({},{},{})",
        off.red(),
        off.green(),
        off.blue()
    );
}

#[test]
fn box_shadow_paints_a_soft_offset_shadow_behind_a_card() {
    // A white card on a white canvas: only its `box-shadow` is visible, and only
    // OUTSIDE the card footprint (the white fill paints over the shadow where they
    // overlap). The shadow is offset down (`0 8px`) and blurred (`24px`), so a point
    // just below the card is a soft grey, and a far corner stays white.
    let page = r#"<html><body style="margin:0">
        <div style="margin:40px 0 0 40px;width:80px;height:60px;background:#fff;
                    box-shadow:0 8px 24px rgba(0,0,0,0.6)"></div>
      </body></html>"#;
    let vp = Viewport {
        width: 200,
        height: 200,
    };
    let png = screenshot_png_with_assets(page, "", vp, &ImageAssets::new(), false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    // Below the card bottom (card spans x40..120, y40..100): the shadow shows grey.
    let shadow = pm.pixel(80, 112).expect("pixel");
    assert!(
        shadow.red() < 235 && shadow.red() > 20 && shadow.green() == shadow.red(),
        "shadow below the card must be a neutral grey, got ({},{},{})",
        shadow.red(),
        shadow.green(),
        shadow.blue()
    );
    // Blur = soft: the shadow fades with distance, so a point deeper below the card
    // is lighter than one right under its edge (not a hard-edged filled rect).
    let near = pm.pixel(80, 104).expect("pixel").red();
    let far = pm.pixel(80, 150).expect("pixel").red();
    assert!(
        far > near,
        "shadow must fade with distance (blur), near={near} far={far}"
    );
    // A corner far from the card is untouched white canvas.
    let corner = pm.pixel(195, 5).expect("pixel");
    assert!(
        corner.red() > 250 && corner.green() > 250 && corner.blue() > 250,
        "far corner must stay white, got ({},{},{})",
        corner.red(),
        corner.green(),
        corner.blue()
    );
}

#[test]
fn linear_gradient_background_paints_left_to_right() {
    // `to right` (90°): the left edge is the first stop (red), the right edge the
    // last (blue), and the middle a blend of both.
    let page = r#"<html><body style="margin:0">
        <div style="width:200px;height:60px;
                    background:linear-gradient(to right, #ff0000, #0000ff)"></div>
      </body></html>"#;
    let vp = Viewport {
        width: 200,
        height: 60,
    };
    let png = screenshot_png_with_assets(page, "", vp, &ImageAssets::new(), false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let left = pm.pixel(4, 30).expect("pixel");
    let right = pm.pixel(196, 30).expect("pixel");
    let mid = pm.pixel(100, 30).expect("pixel");
    assert!(
        left.red() > 200 && left.blue() < 60,
        "left edge should be red, got ({},{},{})",
        left.red(),
        left.green(),
        left.blue()
    );
    assert!(
        right.blue() > 200 && right.red() < 60,
        "right edge should be blue, got ({},{},{})",
        right.red(),
        right.green(),
        right.blue()
    );
    // Middle blends: both channels present, neither saturated like the ends.
    assert!(
        mid.red() > 40 && mid.red() < 220 && mid.blue() > 40 && mid.blue() < 220,
        "middle should blend red↔blue, got ({},{},{})",
        mid.red(),
        mid.green(),
        mid.blue()
    );
}

#[test]
fn css_transform_rotates_a_box_about_its_centre() {
    // A 40×40 box rotated 45° about its centre becomes a diamond: the axis-aligned
    // corners rotate outside it (blank), the centre stays filled, and the diamond
    // apex reaches ABOVE the box's original top edge — proving the rotation paints.
    let page = r#"<html><body style="margin:0">
        <div style="position:absolute;left:60px;top:60px;width:40px;height:40px;
                    background:#ff0000;transform:rotate(45deg)"></div>
      </body></html>"#;
    let vp = Viewport {
        width: 160,
        height: 160,
    };
    let png = screenshot_png_with_assets(page, "", vp, &ImageAssets::new(), false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    // Centre (80,80) stays red.
    let c = pm.pixel(80, 80).expect("pixel");
    assert!(
        c.red() > 200 && c.green() < 80,
        "centre should stay red, got ({},{},{})",
        c.red(),
        c.green(),
        c.blue()
    );
    // The original top-left corner (60,60) is outside the rotated diamond → blank.
    let corner = pm.pixel(62, 62).expect("pixel");
    assert!(
        corner.red() > 200 && corner.green() > 200 && corner.blue() > 200,
        "rotated-away corner should be blank, got ({},{},{})",
        corner.red(),
        corner.green(),
        corner.blue()
    );
    // The diamond apex reaches above the original top edge (y=60): (80,55) is red.
    let apex = pm.pixel(80, 55).expect("pixel");
    assert!(
        apex.red() > 200 && apex.green() < 80,
        "diamond apex above the original edge should be red, got ({},{},{})",
        apex.red(),
        apex.green(),
        apex.blue()
    );
}

#[test]
fn png_falls_back_to_placeholder_without_bytes() {
    // Same page, no supplied bytes: the `<img>` box lays out but paints nothing
    // (no Image fragment), so the middle stays the white canvas — definitely not
    // red.
    let page = r#"<html><body style="margin:0">
        <img src="r.png" style="width:40px;height:40px">
      </body></html>"#;
    let vp = Viewport {
        width: 100,
        height: 100,
    };
    let png = screenshot_png_with_assets(page, "", vp, &ImageAssets::new(), false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let px = pm.pixel(20, 20).expect("pixel");
    assert!(
        px.red() > 200 && px.green() > 200 && px.blue() > 200,
        "no image → canvas stays light"
    );
}

#[test]
fn full_page_grows_height_to_content() {
    // Content taller than the viewport: a viewport-clipped shot is exactly the
    // viewport height, while `full_page` grows to fit the whole content.
    let page = r#"<html><body style="margin:0">
        <div style="height:2000px;background:#eee"></div>
      </body></html>"#;
    let vp = Viewport {
        width: 400,
        height: 300,
    };
    let clipped =
        screenshot_png_with_assets(page, "", vp, &ImageAssets::new(), false).expect("png");
    let full = screenshot_png_with_assets(page, "", vp, &ImageAssets::new(), true).expect("png");
    let hc = u32::from_be_bytes([clipped[20], clipped[21], clipped[22], clipped[23]]);
    let hf = u32::from_be_bytes([full[20], full[21], full[22], full[23]]);
    let wf = u32::from_be_bytes([full[16], full[17], full[18], full[19]]);
    assert_eq!(hc, 300, "clipped keeps the viewport height");
    assert_eq!(wf, 400, "full page keeps the viewport width");
    assert!(
        hf >= 2000,
        "full page grows to the content height, got {hf}"
    );
}

#[test]
fn svg_source_image_is_rasterized_and_painted() {
    // An `<img>` whose bytes are an SVG must be rasterized (resvg) and painted —
    // proving the normalize-any-format-to-RGBA path sizes + paints non-PNG/JPEG
    // sources end-to-end (turbo-html2pdf only ever sees the re-encoded PNG).
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="#0000ff"/></svg>"##;
    let mut assets: ImageAssets = HashMap::new();
    assets.insert("logo.svg".to_string(), svg.to_vec());
    let page = r#"<html><body style="margin:0">
        <img src="logo.svg" style="width:40px;height:40px">
      </body></html>"#;
    let vp = Viewport {
        width: 100,
        height: 100,
    };
    let png = screenshot_png_with_assets(page, "", vp, &assets, false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let px = pm.pixel(20, 20).expect("pixel");
    assert!(
        px.blue() > 200 && px.red() < 60 && px.green() < 60,
        "SVG box should be blue, got ({},{},{})",
        px.red(),
        px.green(),
        px.blue()
    );
}

#[test]
fn svg_embeds_supplied_image_as_data_uri() {
    let mut assets: ImageAssets = HashMap::new();
    assets.insert("r.png".to_string(), solid_png(2, 2, 0, 128, 0));
    let page = r#"<body style="margin:0"><img src="r.png" style="width:30px;height:30px"></body>"#;
    let svg = screenshot_svg_with_assets(page, "", Viewport::DEFAULT, &assets, false).expect("svg");
    assert!(svg.contains("<image "), "expected an <image> element");
    assert!(
        svg.contains("href=\"data:image/png;base64,"),
        "expected a base64 PNG data URI"
    );
}

#[test]
fn default_viewport_is_1280x800() {
    assert_eq!(
        Viewport::default(),
        Viewport {
            width: 1280,
            height: 800
        }
    );
}

#[test]
fn background_image_div_paints_supplied_bytes() {
    let mut assets: ImageAssets = HashMap::new();
    assets.insert("r.png".to_string(), solid_png(4, 4, 255, 0, 0));
    let page = r#"<body style="margin:0"><div style="width:60px;height:60px;background-image:url(r.png)"></div></body>"#;
    let svg = screenshot_svg_with_assets(page, "", Viewport::DEFAULT, &assets, false).expect("svg");
    assert!(
        svg.contains("<image "),
        "background-image should paint an <image>, svg=\n{svg}"
    );
}

#[test]
fn system_fonts_opt_in_renders() {
    // The opt-in system-font path produces a valid PNG (fonts resolve against the
    // machine's installed set; falls back to bundled when absent).
    use turbo_surf_raster::screenshot_png_with_opts;
    let vp = Viewport {
        width: 200,
        height: 100,
    };
    let png = screenshot_png_with_opts(
        "<body style='font-family:Arial'>hi</body>",
        "",
        vp,
        &ImageAssets::new(),
        false,
        true,
    )
    .expect("png");
    assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
}

#[test]
fn image_urls_extracts_lazy_and_responsive_attrs() {
    // Modern sites omit `src` and put the real URL in a lazy/responsive attribute
    // (Nike uses `data-landscape-url`). `image_urls` must recover all of them.
    use turbo_surf_raster::image_urls;
    let html = concat!(
        r#"<img src="a.png">"#,
        r#"<img data-src="b.png">"#,
        // `srcset` picks the LARGEST candidate (the `2x` here) — the sharpest hero.
        r#"<img srcset="c.png 1x, c2.png 2x">"#,
        r#"<img data-landscape-url="d.png">"#,
        r#"<img src="" data-original="e.png">"#,
    );
    let urls = image_urls(html);
    for want in ["a.png", "b.png", "c2.png", "d.png", "e.png"] {
        assert!(urls.iter().any(|u| u == want), "missing {want} in {urls:?}");
    }
}

#[test]
fn delazy_populates_missing_img_src_for_layout() {
    // The layout reads `<img src>`; a `src`-less lazy image must get its `src`
    // filled from the lazy attr so an image box (and its pixels) render.
    use turbo_surf_raster::delazy_images;
    let out = delazy_images(r#"<img data-landscape-url="hero.jpg" alt="x">"#);
    assert!(
        out.contains(r#"src="hero.jpg""#),
        "src injected, got: {out}"
    );
}

#[test]
fn border_radius_renders_rounded_rect() {
    // A `border-radius:50%` box (the Codex radio/checkbox circle) must paint round,
    // not square. In SVG that surfaces as an `rx`/`ry` on the background `<rect>`;
    // the PNG path draws the same rounded corners via tiny-skia.
    let page = r#"<html><body>
        <div style="width:18px;height:18px;border-radius:50%;background-color:#3366cc"></div>
      </body></html>"#;
    let svg = screenshot_svg(page, Viewport::DEFAULT).expect("svg");
    assert!(svg.contains("#3366cc"), "circle background present");
    assert!(
        svg.contains("rx="),
        "border-radius emits a rounded rect (rx)"
    );
    // PNG still renders (rounded fill path is valid).
    let png = screenshot_png(page, Viewport::DEFAULT).expect("png");
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
}

#[test]
fn noscript_styles_are_inert() {
    // With JS enabled (turbo-surf hydrates before rastering), `<noscript>` content
    // is inert — a browser never applies it. Google ships
    // `<noscript><style>table,div,span,p{display:none}</style></noscript>`; if that
    // style cascades it hides every div lacking an explicit `display` rule, blanking
    // the centred search UI. The noscript `<style>` must NOT be collected as author CSS.
    let page = r#"<html><head></head><body style="margin:0">
        <noscript><style>div{display:none}</style></noscript>
        <div style="background-color:#abcdef;height:40px">visible</div>
      </body></html>"#;
    let svg = screenshot_svg(page, Viewport::DEFAULT).expect("svg");
    assert!(
        svg.contains("#abcdef"),
        "noscript <style> must not cascade and hide the div"
    );
}

#[test]
fn screenshot_renders_fractionally_positioned_one_pixel_border() {
    let png = screenshot_png(
        r#"<html><body style="margin:0;background:white"><div style="position:absolute;left:0.5px;top:0;width:0;height:4px;border-left:1px solid black"></div></body></html>"#,
        Viewport { width: 4, height: 4 },
    ).expect("fractional border PNG");
    let image = image::load_from_memory(&png)
        .expect("decode PNG")
        .to_rgba8();
    assert_eq!(image.dimensions(), (4, 4));
    for y in 0..4 {
        for x in [0, 1] {
            let pixel = image.get_pixel(x, y).0;
            assert!(
                (120..=135).contains(&pixel[0]),
                "partial black border at {x},{y}: {pixel:?}"
            );
            assert_eq!(pixel[3], 255);
        }
        assert_eq!(image.get_pixel(2, y).0, [255, 255, 255, 255]);
    }
}

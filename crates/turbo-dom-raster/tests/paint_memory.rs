//! Bounded scratch memory for paint work whose layout size is far beyond the canvas
//! (round-1 review of 0.7.0, findings 4 and 5): this binary's allocator records the
//! largest single allocation, so a painter that allocates at the layout size (a 40000px
//! image tile, a canvas-sized layer per inset shadow on a tall page) fails here.
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use turbo_dom_layout::{layout_html, Diagnostics, FontRegistry, Fragment, FragmentContent};
use turbo_dom_raster::{
    screenshot_png_with_opts, ImageAssets, PreparedScreenshot, ScreenshotOptions, Viewport,
};

struct Peak;

static LARGEST: AtomicUsize = AtomicUsize::new(0);

/// Allocations of 1 MiB or more.
static BIG: AtomicUsize = AtomicUsize::new(0);

/// Every allocation (and reallocation).
static COUNT: AtomicUsize = AtomicUsize::new(0);

fn record(size: usize) {
    LARGEST.fetch_max(size, Ordering::Relaxed);
    COUNT.fetch_add(1, Ordering::Relaxed);
    if size >= 1 << 20 {
        BIG.fetch_add(1, Ordering::Relaxed);
    }
}

// SAFETY: forwards every call to the system allocator unchanged.
unsafe impl GlobalAlloc for Peak {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Peak = Peak;

/// One measurement at a time: the peak is process-wide.
static SERIAL: Mutex<()> = Mutex::new(());

/// The largest single allocation `run` makes and how many of 1 MiB or more it makes.
fn allocations(run: impl FnOnce()) -> (usize, usize) {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    LARGEST.store(0, Ordering::Relaxed);
    BIG.store(0, Ordering::Relaxed);
    run();
    (LARGEST.load(Ordering::Relaxed), BIG.load(Ordering::Relaxed))
}

/// How many allocations `run` makes.
fn allocation_count(run: impl FnOnce()) -> usize {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    COUNT.store(0, Ordering::Relaxed);
    run();
    COUNT.load(Ordering::Relaxed)
}

fn largest_allocation(run: impl FnOnce()) -> usize {
    allocations(run).0
}

const VIEWPORT: Viewport = Viewport {
    width: 200,
    height: 200,
};

/// A 2x2 PNG: red left column, blue right column.
fn two_by_two() -> Vec<u8> {
    let img = image::RgbaImage::from_fn(2, 2, |x, _| {
        if x == 0 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 255, 255])
        }
    });
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

fn pixel(png: &[u8], x: u32, y: u32) -> [u8; 3] {
    let img = image::load_from_memory(png).unwrap().to_rgb8();
    img.get_pixel(x, y).0
}

#[test]
fn a_huge_background_size_paints_with_bounded_scratch() {
    let mut assets: ImageAssets = HashMap::new();
    assets.insert("a.png".into(), two_by_two());
    // The 40000px tile's left half (red) covers the whole 100px box.
    let html = r#"<body style="margin:0;background:#fff"><div style="width:100px;height:100px;background:url(a.png) 0 0/40000px 40000px no-repeat"></div>"#;
    let mut png = Vec::new();
    let peak = largest_allocation(|| {
        png = screenshot_png_with_opts(html, "", VIEWPORT, &assets, false, false).unwrap();
    });
    assert_eq!(pixel(&png, 50, 50), [255, 0, 0]);
    assert_eq!(pixel(&png, 150, 50), [255, 255, 255], "clipped to the box");
    // A 40000 x 40000 RGBA tile would be 6.4 GB; the canvas is 160 KB.
    assert!(peak < 16 << 20, "largest allocation {peak} bytes");
}

#[test]
fn a_huge_img_paints_with_bounded_scratch() {
    let mut assets: ImageAssets = HashMap::new();
    assets.insert("a.png".into(), two_by_two());
    let html = r#"<body style="margin:0"><img src="a.png" style="display:block;width:40000px;height:40000px">"#;
    let mut png = Vec::new();
    let peak = largest_allocation(|| {
        png = screenshot_png_with_opts(html, "", VIEWPORT, &assets, false, false).unwrap();
    });
    // Chrome 156 lays the box out 40000px wide (no cap to its column): the red left
    // half covers the whole 200px canvas (0.7.0 round-2 review, finding 2).
    assert_eq!(pixel(&png, 10, 10), [255, 0, 0]);
    assert_eq!(pixel(&png, 190, 190), [255, 0, 0]);
    assert_eq!(pixel(&png, 150, 10), [255, 0, 0]);
    assert!(peak < 16 << 20, "largest allocation {peak} bytes");
}

#[test]
fn inset_shadows_on_a_tall_page_paint_with_bounded_scratch() {
    let mut html = String::from(r#"<body style="margin:0;background:#fff">"#);
    for _ in 0..200 {
        html.push_str(
            r#"<div style="width:40px;height:40px;margin:10px;box-shadow:inset 0 0 6px 4px #00f"></div>"#,
        );
    }
    let mut png = Vec::new();
    let (_, big) = allocations(|| {
        png = screenshot_png_with_opts(&html, "", VIEWPORT, &HashMap::new(), true, false).unwrap();
    });
    // The page is 200 x ~10000 px (8 MB); each shadow needs only its own box's pixels.
    let [r, g, b] = pixel(&png, 10, 10);
    assert!(b > 200 && r < 120 && g < 120, "inset edge {:?}", [r, g, b]);
    assert_eq!(pixel(&png, 30, 30), [255, 255, 255], "the hole");
    // The canvas and its PNG encode take a handful; two canvas-sized layers per shadow
    // would take 400.
    assert!(big < 20, "{big} allocations of 1 MiB or more");
}

/// 0.7.0 round-3 review, finding 4: a box of about 7.3 million 1.5px tiles (no
/// whole-pixel period) shown under `scale(0.25)` fills the whole canvas, down to its
/// bottom-right corner, in bounded work: one pattern of a few tiles, not one fragment per
/// tile (walking them cut off past 4.2 million tiles, after millions of allocations).
/// Chrome 156: `bg-tiles-shrunk-fractional` in `paint_chrome156.json`.
#[test]
fn millions_of_fractional_tiles_paint_the_whole_box_in_bounded_work() {
    let html = r#"<body style="margin:0;background:#0ff"><div style="position:absolute;left:0;top:0;width:800px;height:800px;transform:scale(0.25);transform-origin:0 0;background:#0f0 linear-gradient(#f00,#f00) 0 0/1.5px 1.5px"></div>"#;
    let mut png = Vec::new();
    let count = allocation_count(|| {
        png = screenshot_png_with_opts(html, "", VIEWPORT, &HashMap::new(), false, false).unwrap();
    });
    assert_eq!(pixel(&png, 0, 0), [255, 0, 0]);
    assert_eq!(
        pixel(&png, 199, 199),
        [255, 0, 0],
        "the bottom-right corner"
    );
    assert_eq!(pixel(&png, 100, 199), [255, 0, 0]);
    // 800 x 800 / 1.5^2 = 284444 tiles; walking them allocates at least once per tile.
    assert!(count < 20_000, "{count} allocations");
}

fn full_opacity_boxes(fragment: &Fragment, opacity: f32) -> usize {
    let selected = fragment.width == 1024.0
        && fragment.height == 1024.0
        && fragment.opacity == opacity
        && fragment.is_positioned;
    usize::from(selected)
        + fragment
            .children
            .iter()
            .map(|child| full_opacity_boxes(child, opacity))
            .sum::<usize>()
}

#[test]
fn opaque_and_zero_opacity_boxes_do_not_allocate_transparency_layers() {
    const BOXES: usize = 20;
    let viewport = Viewport {
        width: 1024,
        height: 1024,
    };
    for opacity in [0.0, 1.0] {
        // Preparation allocates too: keep it out of other tests' measurements.
        let preparation = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut html = String::from("<body style='margin:0;background:#fff'>");
        for _ in 0..BOXES {
            write!(html, "<div style='position:absolute;left:0;top:0;width:1024px;height:1024px;background:#f00;opacity:{opacity}'></div>").unwrap();
        }
        let galley = layout_html(
            &html,
            "",
            1024.0,
            &FontRegistry::new(),
            &mut Diagnostics::default(),
        )
        .unwrap();
        assert_eq!(
            full_opacity_boxes(&galley, opacity),
            BOXES,
            "the public layout must retain all target boxes"
        );
        let prepared = PreparedScreenshot::new(
            &html,
            "",
            viewport,
            &HashMap::new(),
            &ScreenshotOptions::default(),
        );
        drop(preparation);
        let (_, big) = allocations(|| {
            let png = prepared.png().unwrap();
            let image = image::load_from_memory(&png).unwrap().to_rgb8();
            let expected = if opacity == 0.0 {
                [255; 3]
            } else {
                [255, 0, 0]
            };
            assert_eq!(image.get_pixel(512, 512).0, expected);
            assert_eq!(image.get_pixel(1023, 1023).0, expected);
        });
        // The canvas, PNG encoder and verification decoder need only a handful
        // of large buffers. One 4-MiB group per target box would exceed this
        // budget even though the redundant groups leave every output pixel equal.
        assert!(
            big < BOXES,
            "opacity {opacity}: {big} allocations of at least 1 MiB"
        );
    }
}

#[test]
fn empty_opacity_boxes_do_not_allocate_scratch_layers() {
    let mut counts = [0; 2];
    for (i, opacity) in [1.0, 0.5].into_iter().enumerate() {
        let preparation = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut html = String::from("<body style='margin:0;background:white'>");
        for _ in 0..200 {
            write!(html, "<div style='position:absolute;left:10px;top:10px;width:0;height:0;opacity:{opacity}'></div>").unwrap();
        }
        let prepared = PreparedScreenshot::new(
            &html,
            "",
            VIEWPORT,
            &HashMap::new(),
            &ScreenshotOptions::default(),
        );
        drop(preparation);
        counts[i] = allocation_count(|| {
            let png = prepared.png().unwrap();
            assert_eq!(pixel(&png, 10, 10), [255; 3]);
        });
    }
    assert_eq!(
        counts[0], counts[1],
        "empty boxes must not create opacity scratch layers: {counts:?}"
    );
}

fn transparent_outer_shadows(fragment: &Fragment) -> usize {
    let selected = matches!(
        &fragment.content,
        FragmentContent::Box { shadow: Some(shadow), .. }
            if !shadow.inset && shadow.color.a == 0 && shadow.blur > 0
    );
    usize::from(selected)
        + fragment
            .children
            .iter()
            .map(transparent_outer_shadows)
            .sum::<usize>()
}

#[test]
fn transparent_outer_shadows_do_not_allocate_blur_layers() {
    const BOXES: usize = 20;
    let viewport = Viewport {
        width: 1024,
        height: 1024,
    };
    let preparation = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut html = String::from("<body style='margin:0;background:#fff'>");
    for _ in 0..BOXES {
        html.push_str("<div style='position:absolute;left:0;top:0;width:1024px;height:1024px;background:#f00;box-shadow:0 0 10px rgba(0,0,0,0)'></div>");
    }
    let galley = layout_html(
        &html,
        "",
        1024.0,
        &FontRegistry::new(),
        &mut Diagnostics::default(),
    )
    .unwrap();
    assert_eq!(
        transparent_outer_shadows(&galley),
        BOXES,
        "public layout must retain every transparent shadow"
    );
    let prepared = PreparedScreenshot::new(
        &html,
        "",
        viewport,
        &HashMap::new(),
        &ScreenshotOptions::default(),
    );
    drop(preparation);
    let (_, big) = allocations(|| {
        let png = prepared.png().unwrap();
        let image = image::load_from_memory(&png).unwrap().to_rgb8();
        assert_eq!(image.get_pixel(512, 512).0, [255, 0, 0]);
        assert_eq!(image.get_pixel(1023, 1023).0, [255, 0, 0]);
    });
    // Invisible shadows need no blur scratch. Raster/encoder/decoder buffers
    // fit below one large allocation per retained shadow.
    assert!(big < BOXES, "{big} allocations of at least 1 MiB");
}

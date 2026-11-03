//! Bounded scratch memory for paint work whose layout size is far beyond the canvas
//! (round-1 review of 0.7.0, findings 4 and 5): this binary's allocator records the
//! largest single allocation, so a painter that allocates at the layout size (a 40000px
//! image tile, a canvas-sized layer per inset shadow on a tall page) fails here.
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use turbo_dom_raster::{screenshot_png_with_opts, ImageAssets, Viewport};

struct Peak;

static LARGEST: AtomicUsize = AtomicUsize::new(0);

/// Allocations of 1 MiB or more.
static BIG: AtomicUsize = AtomicUsize::new(0);

fn record(size: usize) {
    LARGEST.fetch_max(size, Ordering::Relaxed);
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

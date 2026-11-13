//! Frame resolution needs no allocated property names, even for absent longhands.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use turbo_dom_layout::layout::value::{resolve_box_flow_style, ResolveCtx};
use turbo_dom_layout::style::ComputedStyle;

struct CountingAllocator;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn record_allocation() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}

// SAFETY: all operations delegate their original pointer/layout arguments to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn absent_frame_properties_do_not_allocate_lookup_names() {
    let style = ComputedStyle::default();
    let context = ResolveCtx {
        parent_font_size: 16.0,
        cb_width: 400.0,
    };
    // Warm the optional perf instrumentation before measuring the production path.
    std::hint::black_box(resolve_box_flow_style(&style, context));
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|counting| counting.set(true));
    for _ in 0..128 {
        std::hint::black_box(resolve_box_flow_style(&style, context));
    }
    COUNTING.with(|counting| counting.set(false));
    let allocations = ALLOCATIONS.with(Cell::get);
    assert_eq!(allocations, 0, "frame lookup allocated property names");
}

#[test]
fn frame_lookup_preserves_logical_overrides_and_context() {
    use turbo_dom_layout::layout::value::{resolve_box_style, Edges};
    let style = ComputedStyle::from_pairs([
        ("font-size", "125%"),
        ("margin", "1px 2px 3px 4px"),
        ("margin-top", "-2em"),
        ("margin-block-start", "10%"),
        ("margin-inline-end", "-5%"),
        ("padding-left", "invalid"),
        ("-internal-ua-padding", "9px 11px"),
        ("border", "1px solid currentcolor"),
        ("border-color", "#123456"),
        ("border-width", "3px"),
        ("border-left-width", "0.5em"),
        ("border-top", "5px inset red"),
        ("writing-mode", "vertical-rl"),
    ]);
    for (parent_font_size, cb_width, left_border) in [(16.0, 400.0, 10.0), (20.0, 200.0, 13.0)] {
        let context = ResolveCtx {
            parent_font_size,
            cb_width,
        };
        let full = resolve_box_style(&style, context);
        let flow = resolve_box_flow_style(&style, context);
        assert_eq!(
            flow.margin,
            Edges {
                top: cb_width * 0.1,
                right: -cb_width * 0.05,
                bottom: 3.0,
                left: 4.0
            }
        );
        // Even an invalid author longhand suppresses the UA padding fallback.
        assert_eq!(flow.padding, Edges::default());
        assert_eq!(
            flow.border_widths,
            Edges {
                top: 3.0,
                right: 3.0,
                bottom: 3.0,
                left: left_border
            }
        );
        assert_eq!(flow.margin, full.margin);
        assert_eq!(flow.padding, full.padding);
        assert_eq!(flow.border_widths, full.border.widths());
    }
}

#[test]
fn every_border_lookup_keeps_its_own_side() {
    use turbo_dom_layout::layout::value::{resolve_box_style, BorderEdges, BorderSide, Rgba};
    let style = ComputedStyle::from_pairs([
        ("border-top", "1px solid red"),
        ("border-right", "2px solid blue"),
        ("border-bottom", "3px solid green"),
        ("border-left", "4px solid black"),
        ("border-top-width", "5px"),
        ("border-right-color", "#123456"),
        ("border-bottom-style", "inset"),
        ("border-left-color", "currentcolor"),
        ("color", "#abcdef"),
    ]);
    let context = ResolveCtx {
        parent_font_size: 16.0,
        cb_width: 400.0,
    };
    let border = resolve_box_style(&style, context).border;
    assert_eq!(
        border,
        BorderEdges {
            top: BorderSide {
                width: 5,
                color: Some(Rgba::new(255, 0, 0, 255))
            },
            right: BorderSide {
                width: 2,
                color: Some(Rgba::new(0x12, 0x34, 0x56, 255))
            },
            bottom: BorderSide {
                width: 3,
                color: Some(turbo_dom_layout::layout::value::border_style_colour(
                    Rgba::new(0, 128, 0, 255),
                    false
                ))
            },
            left: BorderSide {
                width: 4,
                color: Some(Rgba::new(0xab, 0xcd, 0xef, 255))
            },
        }
    );
}

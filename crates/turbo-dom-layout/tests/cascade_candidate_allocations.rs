//! Cascade resource use and ownership must preserve computed-style semantics.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

fn record_allocation(bytes: usize) {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        let _ = BYTES.try_with(|total| total.set(total.get() + bytes));
    }
}

// SAFETY: all operations delegate their original pointer/layout arguments to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation(size);
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

use turbo_dom_layout::style::{
    parse_declarations, resolve_matched_style, CascadeOrigin, ComputedStyle, MatchedDeclaration,
    TokenSet,
};
use turbo_dom_layout::{
    build_cascade, parse_html_tree, style_input, Roots, StyleInput, StyleOptions, TreeView,
};

fn measured<T>(f: impl FnOnce() -> T) -> (T, usize) {
    ALLOCATIONS.with(|count| count.set(0));
    BYTES.with(|total| total.set(0));
    COUNTING.with(|counting| counting.set(true));
    let result = f();
    COUNTING.with(|counting| counting.set(false));
    (result, ALLOCATIONS.with(Cell::get))
}

fn inline_allocations(count: usize) -> (ComputedStyle, usize) {
    let tree = parse_html_tree(&format!(
        "<p style='{}'>text</p>",
        "color:#123456;".repeat(count)
    ));
    let element = tree
        .handles()
        .find(|&h| tree.local_name(h) == Some("p"))
        .unwrap();
    let roots = [element];
    let input = StyleInput {
        view: TreeView::new(&tree),
        roots: Roots::Detached(&roots),
    };
    let cascade = build_cascade("", "", TokenSet::default());
    let options = StyleOptions::default();
    let (nodes, allocations) = measured(|| style_input(&input, &cascade, &options));
    (nodes[0].as_element().unwrap().style.clone(), allocations)
}

#[test]
fn inline_declarations_move_parsed_strings_into_candidates() {
    let (expected, small) = inline_allocations(1);
    let (actual, large) = inline_allocations(129);
    assert_eq!(actual, expected);
    eprintln!("inline declarations: one={small}, duplicate129={large}");
    // Parsing owns two strings per declaration; the cascade must not clone them again.
    assert!(
        large <= small + 2 * 128 + 40,
        "one={small}, duplicate129={large}"
    );
}

#[test]
fn inherited_font_normalization_keeps_programmatic_parent_values() {
    for (font_size, expected) in [("1.25em", "20px"), ("12.00px", "12px"), ("medium", "16px")] {
        let parent = ComputedStyle::from_pairs([("font-size", font_size)]);
        let actual = resolve_matched_style(&parent, &[]);
        assert_eq!(actual.get("font-size"), Some(expected), "{font_size}");
    }
}

#[test]
fn owned_results_preserve_rank_variables_and_generated_longhands() {
    let parent = ComputedStyle::from_pairs([("--space", "2px 4px"), ("color", "green")]);
    let style = {
        let mut declarations = Vec::new();
        for (css, origin, scope_depth, inline, order) in [
            ("color: red !important", CascadeOrigin::Author, 0, true, 20),
            (
                "color: blue !important",
                CascadeOrigin::UserAgent,
                0,
                false,
                0,
            ),
            ("padding: var(--space)", CascadeOrigin::Author, 0, false, 1),
            ("padding-left: 9px", CascadeOrigin::Author, 0, true, 0),
            ("width: 8px", CascadeOrigin::Author, 1, true, 100),
            ("width: 6px", CascadeOrigin::Author, 0, false, 1),
            (
                "font: italic bold 12px serif",
                CascadeOrigin::Author,
                0,
                false,
                2,
            ),
            ("background: red", CascadeOrigin::Author, 0, false, 3),
        ] {
            declarations.push(MatchedDeclaration {
                declaration: parse_declarations(css).remove(0),
                origin,
                scope_depth,
                inline,
                specificity: (0, 0, 1),
                order,
            });
        }
        resolve_matched_style(&parent, &declarations)
    };
    for (property, expected) in [
        ("color", "blue"),
        ("padding-top", "2px"),
        ("padding-right", "4px"),
        ("padding-bottom", "2px"),
        ("padding-left", "9px"),
        ("width", "6px"),
        ("font-style", "italic"),
        ("font-weight", "bold"),
        ("font-size", "12px"),
        ("font-family", "serif"),
        ("background-color", "red"),
    ] {
        assert_eq!(style.get(property), Some(expected), "{property}");
    }
}

#[test]
fn overridden_custom_property_does_not_copy_the_parents_large_value() {
    let parent = ComputedStyle::from_pairs([("--theme", "x".repeat(1024 * 1024))]);
    let declarations = vec![MatchedDeclaration {
        declaration: parse_declarations("--theme: blue").remove(0),
        origin: CascadeOrigin::Author,
        scope_depth: 0,
        inline: false,
        specificity: (0, 0, 1),
        order: 0,
    }];
    let (actual, _) = measured(|| resolve_matched_style(&parent, &declarations));
    let bytes = BYTES.with(Cell::get);
    assert_eq!(actual.get("--theme"), Some("blue"));
    assert_eq!(parent.get("--theme").unwrap().len(), 1024 * 1024);
    assert!(
        bytes < 64 * 1024,
        "overriding one value allocated {bytes} bytes"
    );
}

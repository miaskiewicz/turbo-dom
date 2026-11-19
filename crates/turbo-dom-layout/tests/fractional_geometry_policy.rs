use turbo_dom_layout::{layout_boxes, with_fractional_css_geometry, Diagnostics, FontRegistry};

fn width() -> f32 {
    let boxes = layout_boxes("<style>body{margin:0}main{display:flex;width:100px}i{width:33.3px;height:10px;flex-shrink:0}</style><main><i data-cid='item'></i></main>", "", 800.0, 600.0, &FontRegistry::new(), &mut Diagnostics::default()).unwrap();
    boxes.iter().find(|item| item.cid == "item").unwrap().width
}

#[test]
fn css_render_geometry_is_explicit_and_does_not_change_paged_layout() {
    assert_eq!(width(), 33.0);
    with_fractional_css_geometry(|| {
        assert!((width() - 33.3).abs() < 0.001);
        with_fractional_css_geometry(|| assert!((width() - 33.3).abs() < 0.001));
        assert!((width() - 33.3).abs() < 0.001);
    });
    assert_eq!(width(), 33.0);
    let result = std::panic::catch_unwind(|| with_fractional_css_geometry(|| panic!("unwind")));
    assert!(result.is_err());
    assert_eq!(width(), 33.0);
}

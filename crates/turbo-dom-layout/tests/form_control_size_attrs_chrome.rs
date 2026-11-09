//! Form-control size attributes (R2 review #10): `textarea rows/cols` and `input size`
//! are valid positive integers that must fit a signed 32-bit int, else the default
//! (Chrome's `rows > 0x7fffffff` check), while `select size` keeps the unsigned range
//! (2147483648..=4294967295 is a list box there). Expected boxes:
//! `tests/fixtures/form_control_size_attrs_chrome156.json`
//! (`scripts/form-control-size-attrs-chrome-control.mjs`, frozen Chrome 156.0.8078.47).
use serde_json::Value;
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::{layout_boxes_tree, parse_html_tree, Diagnostics, HtmlOptions};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/form_control_size_attrs_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the Chrome fixture")).unwrap()
}

#[test]
fn size_attributes_past_i32_are_the_default_like_chrome_156() {
    let fixture = fixture();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    let fonts = fixture_fonts();
    let tree = parse_html_tree(fixture["html"].as_str().unwrap());
    let mut diags = Diagnostics::default();
    let got = layout_boxes_tree(
        &tree,
        "",
        1280.0,
        800.0,
        &fonts,
        &HtmlOptions::default(),
        &mut diags,
    );
    // Each box is compared with its own control at the default (`"0"`, the last value), in
    // our layout and in Chrome's: "is the default" must agree.
    let ours = |cid: &str| {
        let b = got
            .iter()
            .find(|b| b.cid == cid)
            .expect("a box per data-cid");
        (b.width, b.height)
    };
    let chrome = |cid: &str| {
        let b = fixture["boxes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["cid"] == cid)
            .expect("Chrome's box");
        let f = |k: &str| b[k].as_f64().unwrap() as f32;
        (f("width"), f("height"))
    };
    let last = fixture["values"].as_array().unwrap().len() - 1;
    let mut failures = Vec::new();
    let compared = check_default_agreement(last, &ours, &chrome, &mut failures);
    assert_eq!(compared, 15);
    // And every textarea / text input box is Chrome's (#263). The 187x38 vs 182x36 default
    // textarea was the control's: Chrome sized it in the host's UA monospace (the fixture
    // face now set on the controls), and boxes after the saturating list box were garbage.
    let mut absolute = check_absolute_boxes(last, &ours, &chrome, &mut failures);
    assert_eq!(absolute, 18);
    check_mono_boxes(&ours, &chrome, &mut absolute, &mut failures);
    assert_eq!(absolute, 22);
    let menu_lists = check_select_menu_classification(last, &ours, &chrome, &mut failures);
    assert_eq!(menu_lists, 6);
    assert!(
        failures.is_empty(),
        "{} differences from Chrome 156:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Each in-range value matches the default's "is the default" classification
/// (textarea rows/cols past i32 or input size past i32 falls back to defaults).
fn check_default_agreement(
    last: usize,
    ours: &dyn Fn(&str) -> (f32, f32),
    chrome: &dyn Fn(&str) -> (f32, f32),
    failures: &mut Vec<String>,
) -> usize {
    let mut compared = 0;
    for kind in ["rows", "cols", "size"] {
        let default = format!("{kind}-{last}");
        let axis = |(w, h): (f32, f32)| if kind == "size" { (w, 0.0) } else { (w, h) };
        // The in-range value is not the default in Chrome.
        assert_ne!(axis(chrome(&format!("{kind}-0"))), axis(chrome(&default)));
        for i in 0..last {
            let cid = format!("{kind}-{i}");
            let ours_default = axis(ours(&cid)) == axis(ours(&default));
            let chrome_default = axis(chrome(&cid)) == axis(chrome(&default));
            compared += 1;
            if ours_default != chrome_default {
                failures.push(format!(
                    "{cid}: ours {:?} (default {ours_default}), Chrome {:?} (default {chrome_default})",
                    ours(&cid),
                    chrome(&cid)
                ));
            }
        }
    }
    compared
}

/// Every textarea / text input box's absolute dimensions equal Chrome's.
fn check_absolute_boxes(
    last: usize,
    ours: &dyn Fn(&str) -> (f32, f32),
    chrome: &dyn Fn(&str) -> (f32, f32),
    failures: &mut Vec<String>,
) -> usize {
    let mut absolute = 0;
    for kind in ["rows", "cols", "size"] {
        for i in 0..=last {
            let cid = format!("{kind}-{i}");
            let axis = |(w, h): (f32, f32)| if kind == "size" { (w, 0.0) } else { (w, h) };
            let (o, c) = (axis(ours(&cid)), axis(chrome(&cid)));
            absolute += 1;
            if (o.0 - c.0).abs() > 0.01 || (o.1 - c.1).abs() > 0.01 {
                failures.push(format!("{cid}: box {o:?}, Chrome {c:?}"));
            }
        }
    }
    absolute
}

/// A default (monospace, 13.333px) textarea in the fixture Fira Code: default
/// rows and cols and a few sizes, exact boxes.
fn check_mono_boxes(
    ours: &dyn Fn(&str) -> (f32, f32),
    chrome: &dyn Fn(&str) -> (f32, f32),
    absolute: &mut usize,
    failures: &mut Vec<String>,
) {
    for cid in [
        "mono-default",
        "mono-rows-5",
        "mono-cols-40",
        "mono-rows-3-cols-10",
    ] {
        let (o, c) = (ours(cid), chrome(cid));
        *absolute += 1;
        if (o.0 - c.0).abs() > 0.01 || (o.1 - c.1).abs() > 0.01 {
            failures.push(format!("{cid}: box {o:?}, Chrome {c:?}"));
        }
    }
}

/// A `<select>`'s menu-list vs list-box classification (by height) agrees
/// with Chrome's (`select size` keeps the unsigned range).
fn check_select_menu_classification(
    last: usize,
    ours: &dyn Fn(&str) -> (f32, f32),
    chrome: &dyn Fn(&str) -> (f32, f32),
    failures: &mut Vec<String>,
) -> usize {
    let mut menu_lists = 0;
    for i in 0..=last {
        let cid = format!("select-{i}");
        let (ours_menu, chrome_menu) = (ours(&cid).1 < 30.0, chrome(&cid).1 < 30.0);
        menu_lists += 1;
        if ours_menu != chrome_menu {
            failures.push(format!(
                "{cid}: menu list {ours_menu}, Chrome {chrome_menu}"
            ));
        }
    }
    menu_lists
}

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
    let mut compared = 0;
    for kind in ["rows", "cols", "size"] {
        let default = format!("{kind}-{last}");
        // Text inputs: only the width carries `size`.
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
    assert_eq!(compared, 15);
    // And every textarea / text input box is Chrome's (#263). The 187x38 vs 182x36 default
    // textarea was the control's: Chrome sized it in the host's UA monospace (the fixture
    // face now set on the controls), and boxes after the saturating list box were garbage.
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
    assert_eq!(absolute, 18);
    let mut menu_lists = 0;
    for i in 0..=last {
        let cid = format!("select-{i}");
        // A menu list is one line; a list box is taller (Chrome caps its huge height), and
        // `select size` keeps the unsigned range in Chrome.
        let (ours_menu, chrome_menu) = (ours(&cid).1 < 30.0, chrome(&cid).1 < 30.0);
        menu_lists += 1;
        if ours_menu != chrome_menu {
            failures.push(format!(
                "{cid}: menu list {ours_menu}, Chrome {chrome_menu}"
            ));
        }
    }
    assert_eq!(menu_lists, 6);
    assert!(
        failures.is_empty(),
        "{} differences from Chrome 156:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

use sha2::{Digest, Sha256};
use turbo_dom_layout::{layout_html, Diagnostics, FontRegistry, Fragment, FragmentContent};

fn find(fragment: &Fragment, color: [u8; 3]) -> Option<[f32; 4]> {
    if let FragmentContent::Box {
        background: Some(bg),
        ..
    } = &fragment.content
    {
        if [bg.r, bg.g, bg.b] == color && bg.a == 255 {
            return Some([fragment.x, fragment.y, fragment.width, fragment.height]);
        }
    }
    fragment
        .children
        .iter()
        .find_map(|child| find(child, color))
}

#[test]
fn aspect_ratio_descendants_match_chromium156() {
    let source = include_str!("aspect_absolute_cases.json");
    let cases: serde_json::Value = serde_json::from_str(source).unwrap();
    let control: serde_json::Value =
        serde_json::from_str(include_str!("aspect_absolute_chrome156.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(source.as_bytes())),
        control["sourceSha256"].as_str().unwrap()
    );
    for (case, expected) in cases
        .as_array()
        .unwrap()
        .iter()
        .zip(control["cases"].as_array().unwrap())
    {
        assert_eq!(case["name"], expected["name"]);
        let fragment = layout_html(
            case["html"].as_str().unwrap(),
            "",
            1000.0,
            &FontRegistry::new(),
            &mut Diagnostics::default(),
        )
        .unwrap();
        for (id, color) in [
            ("parent", [0, 255, 0]),
            ("child", [255, 0, 0]),
            ("grand", [0, 0, 255]),
        ] {
            let Some(rect) = expected["rects"].get(id) else {
                continue;
            };
            let actual = find(&fragment, color).unwrap();
            for (axis, value) in actual.iter().enumerate() {
                let wanted = rect[axis].as_f64().unwrap() as f32;
                assert!(
                    (value - wanted).abs() < 0.05,
                    "{} {id} axis{axis}: {actual:?} != {rect}",
                    case["name"]
                );
            }
        }
    }
}

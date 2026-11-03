//! A tiled background keeps its phase when its box is moved after the layer is built
//! (miaskiewicz/turbo-dom#274: by `Fragment::translate` for flex items and floats, and by
//! an orthogonal flow's writing-mode mapping), and resolves its percentage and keyword
//! positions and percentage sizes against the box's physical size in an orthogonal flow
//! (miaskiewicz/turbo-dom#277). Expected tiles are Chrome 156's:
//! `tests/fixtures/background_tiles_chrome156.json`, from
//! `scripts/background-tiles-chrome-control.mjs` (a red 2px stripe at each tile's left or
//! top edge; `stripes` = the starts of the stripes wholly inside the box).
use serde_json::Value;
use turbo_dom_layout::{layout_html, Diagnostics, Fragment, FragmentContent};

fn fixture() -> Vec<Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/background_tiles_chrome156.json"
    );
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    fixture["cases"].as_array().unwrap().clone()
}

fn numbers(v: &Value) -> Vec<f32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_f64().unwrap() as f32)
        .collect()
}

/// The page's one tile layer: its box and the starts along `axis` of the tiles whose
/// 2px stripe lies wholly inside it. The layer's container clips to exactly that box
/// (in px from its top-left), mapped out of a vertical space with it.
fn layout_stripes(html: &str, axis: &str) -> (Vec<f32>, Vec<f32>) {
    fn find(f: &Fragment) -> Option<(&Fragment, &Fragment)> {
        if let Some(tiles) = f
            .children
            .iter()
            .find(|c| matches!(c.content, FragmentContent::Tiles(_)))
        {
            return Some((f, tiles));
        }
        f.children.iter().find_map(find)
    }
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    let root = layout_html(html, "", 800.0, &fonts, &mut Diagnostics::default()).unwrap();
    let (container, f) = find(&root).expect("a tile layer");
    let clip = container.clip.expect("a clipped layer");
    assert_eq!(
        (
            container.x,
            container.y,
            clip.x,
            clip.y,
            clip.width,
            clip.height
        ),
        (f.x, f.y, 0.0, 0.0, f.width, f.height),
        "{html}"
    );
    let FragmentContent::Tiles(p) = &f.content else {
        unreachable!()
    };
    let (from, len) = if axis == "x" {
        (f.x, f.width)
    } else {
        (f.y, f.height)
    };
    let mut starts: Vec<f32> = p
        .tiles((f.x, f.y, f.width, f.height))
        .map(|t| if axis == "x" { t.0 } else { t.1 })
        .filter(|&s| s >= from && s + 2.0 <= from + len)
        .collect();
    starts.sort_by(f32::total_cmp);
    starts.dedup();
    (vec![f.x, f.y, f.width, f.height], starts)
}

/// Every control case whose id starts with `prefix` matches Chrome.
fn check(prefix: &str) {
    let cases: Vec<Value> = fixture()
        .into_iter()
        .filter(|c| c["id"].as_str().unwrap().starts_with(prefix))
        .collect();
    assert!(!cases.is_empty(), "{prefix}");
    let mut failures = Vec::new();
    for case in &cases {
        let id = case["id"].as_str().unwrap();
        let got = layout_stripes(
            case["html"].as_str().unwrap(),
            case["axis"].as_str().unwrap(),
        );
        let chrome = (numbers(&case["box"]), numbers(&case["stripes"]));
        if got != chrome {
            failures.push(format!("{id}: got {got:?}, Chrome {chrome:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn translated_boxes_keep_their_tile_phase() {
    // A flex item and a float, laid out at their container's origin and moved into place.
    check("moved-f");
}

#[test]
fn orthogonal_flows_keep_their_tile_phase() {
    // A horizontal box in a vertical-rl flow lands at x = 213; its pixel offsets hold.
    check("moved-ortho-");
}

#[test]
fn orthogonal_flows_resolve_backgrounds_against_the_physical_box() {
    // Percentage and keyword positions and percentage sizes (miaskiewicz/turbo-dom#277).
    check("ortho-");
}

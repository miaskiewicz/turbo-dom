//! A tiled background keeps its phase when its box is moved after the layer is built
//! (miaskiewicz/turbo-dom#274): by `Fragment::translate` (flex items, floats, an
//! embedder's pagination) and by an orthogonal flow's writing-mode mapping. Expected tile
//! columns are Chrome 156's (headless shell screenshots of the same pages; a red 2px
//! stripe at each tile's left edge, `body { margin: 0 }`).
use turbo_dom_layout::{layout_html, Diagnostics, Fragment, FragmentContent};

/// The left edges of the tiles in the first row of every tile layer of `body`, in
/// tree order.
fn tile_columns(body: &str) -> Vec<Vec<f32>> {
    fn walk(f: &Fragment, out: &mut Vec<Vec<f32>>) {
        if let FragmentContent::Tiles(p) = &f.content {
            let tiles: Vec<_> = p.tiles((f.x, f.y, f.width, f.height)).collect();
            let top = tiles[0].1;
            out.push(tiles.iter().filter(|t| t.1 == top).map(|t| t.0).collect());
        }
        for c in &f.children {
            walk(c, out);
        }
    }
    let html = format!("<!DOCTYPE html><style>body{{margin:0}}</style><body>{body}</body>");
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    let root = layout_html(&html, "", 1280.0, &fonts, &mut Diagnostics::default()).unwrap();
    let mut out = Vec::new();
    walk(&root, &mut out);
    out
}

const STRIPE: &str = "linear-gradient(to right,red 0 2px,transparent 2px)";

#[test]
fn translated_boxes_keep_their_tile_phase() {
    // A flex item and a float, laid out at their container's origin and moved into
    // place: the 25px tiles start at the box's left edge (7px), 4 of them.
    for display in ["display:flex", "display:block"] {
        let item = if display == "display:flex" {
            ""
        } else {
            "float:left;"
        };
        let got = tile_columns(&format!(
            "<div style='{display};padding:13px 7px'><div style='{item}width:100px;height:50px;\
             background:{STRIPE} 0 0/25px 25px'></div></div>"
        ));
        assert_eq!(got, vec![vec![7.0, 32.0, 57.0, 82.0]], "{display}");
    }
}

#[test]
fn orthogonal_flows_keep_their_tile_phase() {
    // A horizontal box in a vertical-rl flow lands at x = 213 (the 313px container's
    // right edge): its 10px tiles stay 5px off its left edge (Chrome's stripes at 218,
    // 228, ...: the tiles from 208 meet the box).
    let got = tile_columns(&format!(
        "<div style='writing-mode:vertical-rl;width:313px;height:200px'><div \
         style='width:100px;height:50px;background:{STRIPE} 5px 3px/10px 4px'></div></div>"
    ));
    let chrome: Vec<f32> = (0..11u8).map(|i| 208.0 + 10.0 * f32::from(i)).collect();
    assert_eq!(got, vec![chrome]);
}

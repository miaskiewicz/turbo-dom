use super::{metrics, scroll};
use crate::live::testing;

const HTML: &str = "<!doctype html><body><svg id=root width=200 height=160><circle id=circle cx=50 cy=50 r=30 /><g id=group><rect width=12 height=9 /></g><circle id=styled cx=90 cy=50 r=10 style='width:40px;height:30px;padding:7px;border:3px solid;overflow:auto' /><svg id=nested x=100 y=20 width=40 height=30><rect width=10 height=10 /></svg><foreignObject id=foreign x=20 y=100 width=35 height=20><div xmlns='http://www.w3.org/1999/xhtml' id=html style='width:18px;height:9px'>x</div></foreignObject></svg></body>";

#[test]
fn svg_cssom_client_and_scroll_metrics_match_chrome_159() {
    let (tree, snapshot, styles) = testing::layout(HTML, "", (400.0, 300.0), Some(1.0));
    for selector in ["#circle", "#group", "#styled", "#nested"] {
        let id = testing::id(&tree, selector);
        let client = metrics::client_metrics(&snapshot, id).unwrap();
        assert_eq!(
            [client.left, client.top, client.width, client.height],
            [0; 4],
            "{selector}"
        );
        let scroll = scroll::scroll_metrics(&snapshot, &styles, id).unwrap();
        assert_eq!([scroll.width, scroll.height], [0, 0], "{selector}");
        // This proof grants no bounding rectangle or offset provenance.
        assert!(metrics::element_geometry(&snapshot, id).is_err());
    }
    let root = testing::id(&tree, "#root");
    let client = metrics::client_metrics(&snapshot, root).unwrap();
    assert_eq!([client.width, client.height], [200, 160]);
    let scroll = scroll::scroll_metrics(&snapshot, &styles, root).unwrap();
    assert_eq!([scroll.width, scroll.height], [200, 160]);
    // Chrome gives foreignObject/HTML descendants real CSS boxes; an unsupported
    // native box stays an error instead of being misclassified as zero.
    for selector in ["#foreign", "#html"] {
        let id = testing::id(&tree, selector);
        assert!(metrics::client_metrics(&snapshot, id).is_err());
        assert!(scroll::scroll_metrics(&snapshot, &styles, id).is_err());
    }
    let text = tree.children(tree.query_selector("#html").unwrap())[0];
    assert_eq!(
        metrics::client_metrics(&snapshot, crate::live::source_id(testing::DOCUMENT, text))
            .unwrap_err(),
        "native element geometry requires an Element receiver"
    );
    assert_eq!(
        metrics::client_metrics(&snapshot, crate::layout_snapshot::SourceId(u64::MAX)).unwrap_err(),
        "native layout source is missing"
    );
}

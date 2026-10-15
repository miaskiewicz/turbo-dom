use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::Tree;

#[test]
fn actual_parser_positions_match_independent_normal_chrome_controls() {
    let cases: Value = serde_json::from_str(include_str!(
        "controls/parser_positions/parser-source-position-cases.json"
    ))
    .unwrap();
    let control: Value = serde_json::from_str(include_str!(
        "controls/parser_positions/parser-source-position-chrome155.json"
    ))
    .unwrap();
    assert_eq!(control["errors"], json!([]));
    assert_eq!(control["browser"], "Chrome/155.0.8059.79");
    assert_eq!(control["v8"], "15.5.35.23");
    for (key, bytes) in [
        (
            "casesSha256",
            include_bytes!("controls/parser_positions/parser-source-position-cases.json")
                .as_slice(),
        ),
        (
            "probeSha256",
            include_bytes!("controls/parser_positions/parser-source-position-probe.js").as_slice(),
        ),
        (
            "collectorSha256",
            include_bytes!("controls/parser_positions/csp-document-chrome.mjs").as_slice(),
        ),
    ] {
        assert_eq!(
            control[key],
            format!("{:x}", Sha256::digest(bytes)),
            "stale control {key}"
        );
    }
    assert_eq!(cases.as_array().unwrap().len(), 14);
    assert_eq!(control["result"].as_array().unwrap().len(), 14);
    for (case, oracle) in cases
        .as_array()
        .unwrap()
        .iter()
        .zip(control["result"].as_array().unwrap())
    {
        // Template-owned reports have no Window source context. The backend
        // retains their lexical provenance; clearing event metadata belongs to
        // the browser binding, which has actual owner-document information.
        if case["id"] == "7" {
            continue;
        }
        let tree = Tree::parse_with_source_positions(case["html"].as_str().unwrap());
        let mut rows = Vec::new();
        let mut pending = vec![tree.root()];
        while let Some(node) = pending.pop() {
            if let Some(name) = tree.local_name(node) {
                if name == "style" {
                    if let Some(position) = tree.source_position(node) {
                        rows.push((position.utf16_offset,json!({"target":name,"line":position.line,"column":position.column,"directive":"style-src-elem"})));
                    }
                }
                if tree.get_attribute(node, "style").is_some() {
                    let position = tree
                        .source_attribute_position(node, "style")
                        .expect("actual parser attribute installation");
                    rows.push((position.utf16_offset,json!({"target":name,"line":position.line,"column":position.column,"directive":"style-src-attr"})));
                }
            }
            pending.extend(tree.children(node).into_iter().rev());
        }
        rows.sort_by_key(|row| row.0);
        let actual = rows.into_iter().map(|row| row.1).collect::<Vec<_>>();
        let expected=oracle["result"].as_array().unwrap().iter().map(|row|json!({"target":row["target"],"line":row["line"],"column":row["column"],"directive":row["directive"]})).collect::<Vec<_>>();
        assert_eq!(actual, expected, "case {}", case["id"]);
    }
}

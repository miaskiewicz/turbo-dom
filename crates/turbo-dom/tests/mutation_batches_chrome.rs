//! A multi-node ChildNode/ParentNode call queues ONE childList record, as in Chrome 156
//! (the nodes are inserted as a fragment). EXPECTED is a headless Chrome 156.0.8078.47
//! `MutationObserver` capture of the same operations on `<div id=r><a id=a></a><b id=b></b></div>`.
use serde_json::{json, Value};
use turbo_dom::rtdom::mutations::MutationEvent;
use turbo_dom::rtdom::{Handle, Tree};

const EXPECTED: &str = r#"{"before2":[{"target":"r","added":["x","y"],"removed":[],"previous":"a","next":"b"}],"after2":[{"target":"r","added":["x","y"],"removed":[],"previous":"a","next":"b"}],"replaceWith2":[{"target":"r","added":["x","y"],"removed":["a"],"previous":null,"next":"b"}],"replaceChildren2":[{"target":"r","added":["x","y"],"removed":["a","b"],"previous":null,"next":null}],"adjacentHtml":[{"target":"r","added":["x","y"],"removed":[],"previous":"b","next":null}],"adjacentHtmlAfterbegin":[{"target":"r","added":["x","y"],"removed":[],"previous":null,"next":"a"}],"replaceWithSelf":[{"target":"r","added":[],"removed":["a"],"previous":null,"next":"b"},{"target":"r","added":["x","a"],"removed":[],"previous":null,"next":"b"}],"appendFragment":[{"target":"r","added":["x","y"],"removed":[],"previous":"b","next":null}]}"#;

fn identity(tree: &Tree, node: Option<Handle>) -> Value {
    node.map_or(Value::Null, |node| json!(tree.get_attribute(node, "id")))
}
fn record(tree: &Tree, event: &MutationEvent) -> Value {
    let ids = |nodes: &[Handle]| {
        nodes
            .iter()
            .map(|n| identity(tree, Some(*n)))
            .collect::<Vec<_>>()
    };
    json!({"target": identity(tree, Some(event.record.target())), "added": ids(event.record.added()),
        "removed": ids(event.record.removed()), "previous": identity(tree, event.previous_sibling),
        "next": identity(tree, event.next_sibling)})
}
fn run(op: impl Fn(&mut Tree, Handle, Handle, Handle)) -> Value {
    let mut tree = Tree::parse("<div id=r><a id=a></a><b id=b></b></div>");
    let r = tree.query_selector("div").unwrap();
    let [x, y] = ["x", "y"].map(|tag| {
        let n = tree.create_element(tag);
        tree.set_attribute(n, "id", tag);
        n
    });
    tree.start_recording();
    op(&mut tree, r, x, y);
    let events = tree.take_mutation_events();
    // what a subtree observer on r sees
    json!(events
        .iter()
        .filter(|e| e.inclusive_ancestors.contains(&r))
        .map(|e| record(&tree, e))
        .collect::<Vec<_>>())
}

#[test]
fn multi_node_insertions_queue_one_child_list_record_like_chrome() {
    let first = |t: &Tree, r: Handle| t.children(r)[0];
    let last = |t: &Tree, r: Handle| *t.children(r).last().unwrap();
    let got = json!({
        "before2": run(|t, r, x, y| { let b = last(t, r); t.before(b, &[x, y]); }),
        "after2": run(|t, r, x, y| { let a = first(t, r); t.after(a, &[x, y]); }),
        "replaceWith2": run(|t, r, x, y| { let a = first(t, r); t.replace_with(a, &[x, y]); }),
        "replaceWithSelf": run(|t, r, x, _| { let a = first(t, r); t.replace_with(a, &[x, a]); }),
        "replaceChildren2": run(|t, r, x, y| t.replace_children(r, &[x, y])),
        "adjacentHtml": run(|t, r, _, _| t.insert_adjacent_html(r, "beforeend", "<x id=x></x><y id=y></y>")),
        "adjacentHtmlAfterbegin": run(|t, r, _, _| t.insert_adjacent_html(r, "afterbegin", "<x id=x></x><y id=y></y>")),
        "appendFragment": run(|t, r, x, y| {
            let f = t.create_document_fragment();
            t.append_child(f, x);
            t.append_child(f, y);
            t.take_mutation_events();
            t.append_child(r, f);
        }),
    });
    let expected: Value = serde_json::from_str(EXPECTED).unwrap();
    let wrong: Vec<_> = expected
        .as_object()
        .unwrap()
        .iter()
        .filter(|(name, want)| &got[name.as_str()] != *want)
        .map(|(name, want)| format!("{name}: got {} want {want}", got[name.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn attr_nodes_are_skipped_by_multi_node_inserts() {
    let mut tree = Tree::parse("<div id=r><a id=a></a></div>");
    let r = tree.query_selector("div").unwrap();
    let attr = tree.create_attribute(None, "title", "t");
    let x = tree.create_element("x");
    tree.replace_children(r, &[attr, x]);
    assert_eq!(tree.children(r), vec![x]);
    assert_eq!(tree.parent(attr), None);
}

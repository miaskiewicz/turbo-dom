//! The custom element state behind `:defined` (miaskiewicz/turbo-dom#24 item 3): derived from
//! the element's creation, overridden by the embedder's upgrade reports, seen by the DOM
//! APIs, the stylesheet matcher and `rtdom::cascade`. The V8 binding's end-to-end Chrome 156
//! control is `turbo-dom-browser-env/src/custom_elements_defined_chrome156.json`.
use turbo_dom::rtdom::cascade::computed_style;
use turbo_dom::rtdom::query::{match_for_cascade, parse_for_cascade, LiveNavigator};
use turbo_dom::rtdom::tree::{CollectionBudget, CustomElementState, Namespace};
use turbo_dom::{Handle, Tree};

const SHEET: &str = "<style>:not(:defined) { order: 2 } :defined { order: 1 }</style>";

fn id(tree: &Tree, id: &str) -> Handle {
    tree.get_element_by_id(id).unwrap()
}

fn defined(tree: &Tree, h: Handle) -> bool {
    let dom = tree.matches(h, ":defined");
    assert_eq!(tree.matches(h, ":not(:defined)"), !dom);
    assert_eq!(tree.matches_checked(h, ":defined"), Ok(dom));
    let sheet = parse_for_cascade(":defined").unwrap();
    assert_eq!(
        match_for_cascade(&LiveNavigator(tree), h, &sheet.selectors()[0]),
        dom
    );
    dom
}

#[test]
fn creation_time_state_without_an_embedder() {
    use CustomElementState::{Uncustomized, Undefined};
    let mut tree = Tree::parse(&format!(
        "<head>{SHEET}</head><body><x-a id=a></x-a><button id=b is=x-b></button><button id=e is></button>\
         <div id=d></div><svg><x-a id=s></x-a></svg><template id=t><x-a id=ta></x-a></template></body>"
    ));
    for (name, state) in [
        ("a", Undefined),
        ("b", Undefined),
        ("e", Undefined),
        ("d", Uncustomized),
        ("s", Uncustomized),
    ] {
        let h = id(&tree, name);
        assert_eq!(tree.custom_element_state(h), state, "{name}");
        assert_eq!(defined(&tree, h), state.is_defined(), "{name}");
    }
    // the `is` value is read once, at creation: later edits never change the state
    let b = id(&tree, "b");
    tree.remove_attribute(b, "is");
    assert!(!defined(&tree, b));
    let d = id(&tree, "d");
    tree.set_attribute(d, "is", "x-d");
    assert!(defined(&tree, d));
    let created = tree.create_element("button");
    tree.set_attribute(created, "is", "x-b");
    assert_eq!(tree.custom_element_state(created), Uncustomized);
    // fragment parsing reads `is` at creation like the initial parse (both parsers)
    tree.set_inner_html(
        d,
        "<button is=x-b></button><p is=q></p><x-c></x-c><svg is=x></svg>",
    );
    let kids = tree.children(d);
    let states: Vec<_> = kids.iter().map(|&k| tree.custom_element_state(k)).collect();
    assert_eq!(states, [Undefined, Undefined, Undefined, Uncustomized]);
    tree.set_inner_html_utf16(
        d,
        &"<button is=x-b></button><i></i>"
            .encode_utf16()
            .collect::<Vec<_>>(),
    );
    let states: Vec<_> = tree
        .children(d)
        .iter()
        .map(|&k| tree.custom_element_state(k))
        .collect();
    assert_eq!(states, [Undefined, Uncustomized]);
    // non-elements and foreign elements are uncustomized
    let text = tree.create_text_node("x");
    assert_eq!(tree.custom_element_state(text), Uncustomized);
    let foreign = tree.create_element_ns("x-a", Namespace::Svg);
    assert_eq!(tree.custom_element_state(foreign), Uncustomized);
}

#[test]
fn embedder_reports_override_and_invalidate_caches() {
    let mut tree = Tree::parse(&format!(
        "<head>{SHEET}</head><body><x-a id=a></x-a><x-a id=f></x-a><button id=b is=x-b></button></body>"
    ));
    let (a, f, b) = (id(&tree, "a"), id(&tree, "f"), id(&tree, "b"));
    let order = |tree: &Tree, h| computed_style(tree, h).get("order").cloned();
    assert_eq!(
        tree.query_selector_all("body :defined").to_vec(),
        Vec::<Handle>::new()
    );
    assert_eq!(order(&tree, a), Some("2".into()));
    let version = tree.version;
    assert!(tree.set_custom_element_state(a, CustomElementState::Precustomized));
    assert!(!defined(&tree, a), "the constructor is still running");
    assert!(tree.set_custom_element_state(a, CustomElementState::Custom));
    assert!(tree.version > version, "a state change bumps the version");
    assert!(defined(&tree, a));
    assert_eq!(
        order(&tree, a),
        Some("1".into()),
        "getComputedStyle sees it"
    );
    assert!(tree.set_custom_element_state(f, CustomElementState::Failed));
    assert!(!defined(&tree, f));
    assert!(tree.set_custom_element_state(b, CustomElementState::Custom));
    assert_eq!(
        tree.query_selector_all("body :defined").to_vec(),
        vec![a, b]
    );
    assert_eq!(
        tree.query_selector_all("body :not(:defined)").to_vec(),
        vec![f]
    );
    // an unchanged report does not bump; non-elements and dead handles are ignored
    let version = tree.version;
    assert!(!tree.set_custom_element_state(a, CustomElementState::Custom));
    assert_eq!(tree.version, version);
    let text = tree.create_text_node("t");
    assert!(!tree.set_custom_element_state(text, CustomElementState::Custom));
    assert_eq!(
        tree.custom_element_state(text),
        CustomElementState::Uncustomized
    );
    // copies (clone / import / adoption) start undefined unless the source is uncustomized
    assert_eq!(
        tree.custom_element_copy_state(a),
        CustomElementState::Undefined
    );
    assert_eq!(
        tree.custom_element_copy_state(f),
        CustomElementState::Undefined
    );
    let body = tree.query_selector("body").unwrap();
    assert_eq!(
        tree.custom_element_copy_state(body),
        CustomElementState::Uncustomized
    );
    // reports survive storage collection for retained nodes and go with collected ones
    let detached = tree.create_element("x-z");
    tree.set_custom_element_state(detached, CustomElementState::Custom);
    tree.collect_unreachable(
        &[],
        CollectionBudget {
            max_nodes: 1000,
            max_edges: 1000,
            max_payload_bytes: 100_000,
        },
    )
    .unwrap();
    assert!(!tree.is_live(detached));
    assert_eq!(tree.custom_element_state(a), CustomElementState::Custom);
    assert_eq!(tree.custom_element_state(f), CustomElementState::Failed);
}

/// miaskiewicz/turbo-surf#41: storage collection re-materializes the retained parsed nodes,
/// so a parser-created element's creation-time state must not depend on the parse buffer it
/// came from: an `is` element the parser created stays "undefined" (Chrome 156:
/// `custom_elements_defined_chrome156.json`, the binding's `before` snapshot), an `is`
/// attribute set later still changes nothing.
#[test]
fn parsed_is_state_survives_storage_collection() {
    let mut tree = Tree::parse(
        "<body><button id=b is=x-b></button><div id=d is=x-d></div><p id=e is></p>\
         <x-a id=a></x-a><div id=plain></div><div id=later></div></body>",
    );
    let later = id(&tree, "later");
    tree.set_attribute(later, "is", "x-later");
    let before: Vec<_> = ["b", "d", "e", "a", "plain", "later"]
        .iter()
        .map(|name| tree.custom_element_state(id(&tree, name)))
        .collect();
    tree.collect_unreachable(
        &[],
        CollectionBudget {
            max_nodes: 1000,
            max_edges: 1000,
            max_payload_bytes: 100_000,
        },
    )
    .unwrap();
    let after: Vec<_> = ["b", "d", "e", "a", "plain", "later"]
        .iter()
        .map(|name| tree.custom_element_state(id(&tree, name)))
        .collect();
    assert_eq!(after, before);
    for name in ["b", "d", "e", "a"] {
        assert!(!defined(&tree, id(&tree, name)), "{name}");
    }
    assert!(defined(&tree, id(&tree, "later")));
}

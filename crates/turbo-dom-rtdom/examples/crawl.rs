//! Minimal in-process Rust consumer of turbo-dom-rtdom. Run:
//!   cargo run --release --example crawl
//! Demonstrates the native API a crawler/extractor uses: parse → query →
//! read attrs/text → getComputedStyle → mutate via an event → serialize.

use turbo_dom_rtdom::rtdom::{cascade, serialize};
use turbo_dom_rtdom::{Dom, DocumentExt, Event};

fn main() {
    let html = r#"<!doctype html><html><head><style>
        .card { color: blue; padding: 8px }
        #hero { color: green }
    </style></head><body><main class="grid">
        <div class="card" id="hero" data-id="1"><h2 class="title">First</h2></div>
        <div class="card" data-id="2"><h2 class="title">Second</h2></div>
    </main></body></html>"#;

    let mut dom = Dom::parse(html);

    // query (version-cached) — zero boundary, plain Rust calls
    let cards = dom.tree.query_selector_all("div.card");
    println!("cards: {}", cards.len());
    for &c in cards.iter() {
        println!(
            "  <{}> id={:?} data-id={:?} text={:?}",
            dom.tree.local_name(c).unwrap_or(""),
            dom.tree.get_attribute(c, "id"),
            dom.tree.get_attribute(c, "data-id"),
            dom.tree.text_content(c),
        );
    }

    // getComputedStyle (partial honest cascade) — #hero (id) beats .card on color
    let hero = dom.tree.query_selector("#hero").unwrap();
    let style = cascade::computed_style(&dom.tree, hero);
    println!("#hero color = {:?}", cascade::get_property_value(&style, "color"));

    // ergonomic NodeRef facade
    if let Some(title) = dom.tree.query("div.card .title") {
        println!("first title via NodeRef: {:?}", title.text_content());
    }

    // mutate the tree from inside an event handler, then re-query (cache invalidates)
    let main = dom.tree.query_selector("main.grid").unwrap();
    dom.add_event_listener(main, "addcard", false, false, Box::new(|tree, _| {
        let m = tree.query_selector("main.grid").unwrap();
        let card = tree.create_element("div");
        tree.set_attribute(card, "class", "card");
        tree.append_child(m, card);
    }));
    let mut ev = Event::new("addcard", true, false);
    dom.dispatch_event(cards[0], &mut ev);
    println!("cards after event mutation: {}", dom.tree.query_selector_all("div.card").len());

    // serialize back to HTML
    let out = serialize::serialize_inner(&dom.tree, main);
    println!("serialized main has {} bytes", out.len());
}

use turbo_dom::rtdom::Tree;
fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}
#[test]
fn parser_transport_preserves_literal_markers_numeric_references_and_lone_units() {
    let marker = units("\u{f0000}");
    let code = units("\u{f0800}");
    let mut source = units("<a ");
    source.push(0xd800);
    source.extend(units("='high' "));
    source.push(0xdc00);
    source.extend(units("='low' �='replacement' data-u='"));
    let mut payload = vec![0xd800];
    payload.extend(&marker);
    payload.extend(&marker);
    payload.extend(&code);
    payload.push(0xdc00);
    source.extend(&payload);
    source.extend(units("&#983040;&#985088;'>"));
    source.extend(&payload);
    source.extend(units("&#983040;&#985088;&amp;</a><!--"));
    source.extend(&payload);
    source.extend(units("-->"));
    let tree = Tree::parse_utf16(&source);
    let node = tree.query_selector("a").unwrap();
    let mut expected = payload.clone();
    expected.extend(&marker);
    expected.extend(&code);
    assert_eq!(
        tree.get_attribute_units(node, &[0xd800]),
        Some(units("high"))
    );
    assert_eq!(
        tree.get_attribute_units(node, &[0xdc00]),
        Some(units("low"))
    );
    assert_eq!(
        tree.get_attribute_units(node, &[0xfffd]),
        Some(units("replacement"))
    );
    assert_eq!(
        tree.get_attribute_utf16(node, "data-u"),
        Some(expected.clone())
    );
    expected.push(38);
    assert_eq!(tree.text_content_utf16(node), expected);
    let body = tree.query_selector("body").unwrap();
    let comment = tree
        .children(body)
        .into_iter()
        .find(|&node| tree.node_type_id(node) == 8)
        .unwrap();
    assert_eq!(tree.node_value_utf16(comment), Some(payload));
}
#[test]
fn fragments_preserve_rawtext_rcdata_foreign_content_and_replacement_batches() {
    let mut tree = Tree::parse("<body><div><b>old</b></div><textarea></textarea><script></script>");
    let host = tree.query_selector("div").unwrap();
    let mut source = units("<a data-u='");
    source.push(0xd800);
    source.extend(units("'>"));
    source.push(0xdc00);
    source.extend(units("</a><svg><text>"));
    source.push(0xd800);
    source.extend(units("</text></svg>"));
    tree.start_recording();
    tree.set_inner_html_utf16(host, &source);
    let records = tree.take_mutation_events();
    assert_eq!(
        records
            .iter()
            .filter(|event| event.record.target() == host)
            .count(),
        1
    );
    let a = tree.query_selector("a").unwrap();
    assert_eq!(tree.get_attribute_utf16(a, "data-u"), Some(vec![0xd800]));
    assert_eq!(tree.text_content_utf16(a), vec![0xdc00]);
    let text = tree.query_selector("text").unwrap();
    assert_eq!(tree.text_content_utf16(text), vec![0xd800]);
    let textarea = tree.query_selector("textarea").unwrap();
    let script = tree.query_selector("script").unwrap();
    let mut source = vec![0xd800];
    source.extend(units("&#983040;"));
    tree.set_inner_html_utf16(textarea, &source);
    tree.set_inner_html_utf16(script, &source);
    assert_eq!(
        tree.text_content_utf16(textarea),
        [vec![0xd800], units("\u{f0000}")].concat()
    );
    assert_eq!(tree.text_content_utf16(script), source);
}
#[test]
fn every_lone_unit_is_native_and_remains_distinct_after_collection() {
    let mut source = units("<p>");
    let expected = (0xd800..=0xdfff)
        .flat_map(|unit| [unit, 124])
        .collect::<Vec<_>>();
    source.extend(&expected);
    source.extend(units("</p>"));
    let mut tree = Tree::parse_utf16(&source);
    let p = tree.query_selector("p").unwrap();
    assert_eq!(tree.text_content_utf16(p), expected);
    tree.collect_unreachable(
        &[],
        turbo_dom::rtdom::tree::CollectionBudget {
            max_nodes: 10_000,
            max_edges: 100_000,
            max_payload_bytes: 1_000_000,
        },
    )
    .unwrap();
    assert_eq!(tree.text_content_utf16(p), expected);
}

#[test]
fn dom_string_parser_keeps_leading_bom_in_valid_and_non_scalar_input() {
    for source in [vec![0xfeff, 0xd800], vec![0xfeff, 120]] {
        let mut tree = Tree::parse_utf16(&source);
        let body = tree.query_selector("body").unwrap();
        assert_eq!(tree.text_content_utf16(body), source);
        let host = tree.create_element("div");
        tree.set_inner_html_utf16(host, &source);
        assert_eq!(tree.text_content_utf16(host), source);
    }
}

#[test]
fn deeply_nested_document_and_fragments_use_heap_traversal() {
    // A small host stack makes accidental recursion fail far below this depth.
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            const DEPTH: usize = 8192;
            let opening = "<div>".repeat(DEPTH);
            let closing = "</div>".repeat(DEPTH);
            let source = format!("{opening}native{closing}");
            let mut scalar = Tree::parse(&source);
            assert_eq!(scalar.text_content(scalar.root()), "native");
            let body = scalar.query_selector("body").unwrap();
            assert_eq!(
                turbo_dom::rtdom::serialize::serialize_inner(&scalar, body),
                source
            );
            assert_eq!(scalar.text_content_utf16(scalar.root()), units("native"));
            let host = scalar.create_element("main");
            scalar.set_inner_html(host, &source);
            assert_eq!(scalar.text_content(host), "native");
            scalar.insert_adjacent_html(host, "beforeend", &source);
            assert_eq!(scalar.text_content(host), "nativenative");
            for payload in [vec![0xd800, 0xdc00, 0xd801], vec![0xfeff, 65]] {
                let mut encoded = units(&opening);
                encoded.extend(&payload);
                encoded.extend(units(&closing));
                let mut tree = Tree::parse_utf16(&encoded);
                assert_eq!(tree.text_content_utf16(tree.root()), payload);
                let body = tree.query_selector("body").unwrap();
                assert_eq!(
                    turbo_dom::rtdom::serialize::serialize_inner_utf16(&tree, body),
                    encoded
                );
                let host = tree.create_element("main");
                tree.set_inner_html_utf16(host, &encoded);
                assert_eq!(tree.text_content_utf16(host), payload);
                tree.insert_adjacent_html_utf16(host, "beforeend", &encoded);
                assert_eq!(tree.text_content_utf16(host), payload.repeat(2));
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn iterative_parser_keeps_template_projection_and_sibling_order() {
    let mut source = units("<div>A<template><b>B<template>C</template>D</b></template>E</div>");
    source.push(0xd800);
    let tree = Tree::parse_utf16(&source);
    let div = tree.query_selector("div").unwrap();
    assert_eq!(tree.text_content_utf16(div), units("AE"));
    let html = turbo_dom::rtdom::serialize::serialize_outer_utf16(&tree, div);
    assert_eq!(
        html,
        units("<div>A<template><b>B<template>C</template>D</b></template>E</div>")
    );
    let scalar = Tree::parse("<div>A<template><b>B<template>C</template>D</b></template>E</div>");
    assert_eq!(scalar.text_content(scalar.root()), "AE");
    assert_eq!(
        turbo_dom::rtdom::serialize::serialize_outer_utf16(
            &scalar,
            scalar.query_selector("div").unwrap()
        ),
        html
    );
}

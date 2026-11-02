//! The parser core against its two oracles, through BOTH outputs the front-ends ship
//! (`parse_html_document` / `parse_html_fragment_context` → `Node`, the napi `parse` /
//! `parseFragment`; `parse_html_soa_with_compat_mode` → `Soa`, the napi `parseBuffer` the JS
//! runtime builds its DOM from):
//! - html5lib-tests tree construction (`vendor/html5lib-tests/*.dat`): the same gate as
//!   `test/conformance.test.mjs` (>= 99.5%, no crash, every miss involves `<select>`);
//! - the Chrome 156 control `crates/turbo-dom/tests/fixtures/rtdom_core_chrome156.json`
//!   (`scripts/rtdom-core-chrome-control.mjs`): every document row's tree dump and
//!   `document.compatMode`, every fragment row's dump in its context element.
use super::*;
use std::fmt::Write as _;

const CHROME: &str =
    include_str!("../../crates/turbo-dom/tests/fixtures/rtdom_core_chrome156.json");

fn indent(depth: usize) -> String {
    format!("| {}", "  ".repeat(depth))
}

fn doctype_line(name: &str, public_id: &str, system_id: &str) -> String {
    if public_id.is_empty() && system_id.is_empty() {
        format!("<!DOCTYPE {name}>")
    } else {
        format!("<!DOCTYPE {name} \"{public_id}\" \"{system_id}\">")
    }
}

/// html5lib dump of a marshaled `Node`'s children (the format of `harness/serialize.mjs`).
pub(super) fn dump_node(root: &Node) -> String {
    fn walk(node: &Node, depth: usize, out: &mut Vec<String>) {
        match node.node_type {
            ELEMENT_NODE => {
                let ns = if node.namespace.is_empty() {
                    String::new()
                } else {
                    format!("{} ", node.namespace)
                };
                out.push(format!("{}<{ns}{}>", indent(depth), node.name));
                let mut attrs: Vec<(String, &str)> = node
                    .attrs
                    .iter()
                    .map(|a| {
                        let shown = if a.prefix.is_empty() {
                            a.name.clone()
                        } else {
                            format!("{} {}", a.prefix, a.name)
                        };
                        (shown, a.value.as_str())
                    })
                    .collect();
                attrs.sort_unstable();
                for (name, value) in attrs {
                    out.push(format!("{}{name}=\"{value}\"", indent(depth + 1)));
                }
            }
            TEXT_NODE => out.push(format!("{}\"{}\"", indent(depth), node.value)),
            COMMENT_NODE => out.push(format!("{}<!-- {} -->", indent(depth), node.value)),
            DOCUMENT_TYPE_NODE => out.push(format!(
                "{}{}",
                indent(depth),
                doctype_line(&node.name, &node.public_id, &node.system_id)
            )),
            DOCUMENT_FRAGMENT_NODE => out.push(format!("{}{}", indent(depth), node.name)),
            other => panic!("unexpected nodeType {other} below the root"),
        }
        for child in &node.children {
            walk(child, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    for child in &root.children {
        walk(child, 0, &mut out);
    }
    out.join("\n")
}

/// html5lib dump of the `Soa` document's children, read through the flat arrays only
/// (`first_child` / `next_sib` links, interned tables) the way the JS runtime does.
pub(super) fn dump_soa(soa: &Soa) -> String {
    fn walk(soa: &Soa, i: usize, depth: usize, out: &mut Vec<String>) {
        let string = |id: i32| soa.strings[usize::try_from(id).unwrap()].as_str();
        let tag = soa.tag_names[soa.tag_id[i] as usize].as_str();
        match soa.node_type[i] {
            ELEMENT_NODE => {
                let ns = ["", "svg ", "math "][soa.ns[i] as usize];
                out.push(format!("{}<{ns}{tag}>", indent(depth)));
                let mut attrs = Vec::new();
                if soa.attr_start[i] >= 0 {
                    let start = soa.attr_start[i] as usize;
                    for a in start..start + soa.attr_count[i] as usize {
                        let name = &soa.attr_names[soa.attr_name_id[a] as usize];
                        let prefix = &soa.attr_prefixes[soa.attr_prefix_id[a] as usize];
                        let shown = if prefix.is_empty() {
                            name.clone()
                        } else {
                            format!("{prefix} {name}")
                        };
                        attrs.push((
                            shown,
                            soa.attr_values[soa.attr_value_id[a] as usize].clone(),
                        ));
                    }
                } else {
                    assert_eq!(soa.attr_count[i], 0, "attr_count without attr_start");
                }
                attrs.sort_unstable();
                for (name, value) in attrs {
                    out.push(format!("{}{name}=\"{value}\"", indent(depth + 1)));
                }
            }
            TEXT_NODE => out.push(format!("{}\"{}\"", indent(depth), string(soa.text_id[i]))),
            COMMENT_NODE => out.push(format!(
                "{}<!-- {} -->",
                indent(depth),
                string(soa.text_id[i])
            )),
            DOCUMENT_TYPE_NODE => out.push(format!(
                "{}{}",
                indent(depth),
                doctype_line(
                    string(soa.text_id[i]),
                    string(soa.pub_id[i]),
                    string(soa.sys_id[i])
                )
            )),
            DOCUMENT_FRAGMENT_NODE => out.push(format!("{}{tag}", indent(depth))),
            other => panic!("unexpected nodeType {other} below the root"),
        }
        let mut child = soa.first_child[i];
        while child >= 0 {
            let c = child as usize;
            assert_eq!(soa.parent[c], i as i32, "parent link of node {c}");
            walk(soa, c, depth + 1, out);
            child = soa.next_sib[c];
        }
    }
    assert_eq!(soa.node_type[0], DOCUMENT_NODE);
    assert_eq!(soa.parent[0], -1);
    assert_eq!(soa.next_sib[0], -1);
    let mut out = Vec::new();
    let mut child = soa.first_child[0];
    while child >= 0 {
        walk(soa, child as usize, 0, &mut out);
        child = soa.next_sib[child as usize];
    }
    out.join("\n")
}

/// Nodes of a marshaled tree outside template contents (what `parse_html_document_count`
/// counts: the html5ever tree's own child links).
fn count_outside_templates(node: &Node) -> u32 {
    1 + node
        .children
        .iter()
        .filter(|c| c.node_type != DOCUMENT_FRAGMENT_NODE)
        .map(count_outside_templates)
        .sum::<u32>()
}

struct Case {
    data: String,
    document: String,
    context: Option<String>,
    script_on: bool,
}

/// html5lib-tests `.dat` cases (the format `harness/dat.mjs` reads).
fn parse_dat(text: &str) -> Vec<Case> {
    const HEADERS: [&str; 7] = [
        "#data",
        "#errors",
        "#new-errors",
        "#document",
        "#document-fragment",
        "#script-on",
        "#script-off",
    ];
    let mut raw: Vec<Vec<(&str, Vec<&str>)>> = Vec::new();
    for line in text.split('\n') {
        if HEADERS.contains(&line) {
            if line == "#data" {
                raw.push(Vec::new());
            }
            raw.last_mut().unwrap().push((line, Vec::new()));
        } else if let Some((_, buf)) = raw.last_mut().and_then(|c| c.last_mut()) {
            buf.push(line);
        }
    }
    raw.into_iter()
        .map(|sections| {
            let get = |h: &str| {
                sections.iter().find(|(name, _)| *name == h).map(|(_, b)| {
                    let mut b = b.clone();
                    while b.last() == Some(&"") {
                        b.pop();
                    }
                    b.join("\n")
                })
            };
            Case {
                // #data keeps its trailing lines except the blank case separator
                data: sections[0].1.join("\n"),
                document: get("#document").unwrap_or_default(),
                context: get("#document-fragment")
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty()),
                script_on: sections.iter().any(|(name, _)| *name == "#script-on"),
            }
        })
        .collect()
}

#[test]
fn html5lib_tree_construction_through_node_and_soa() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/html5lib-tests");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("vendored html5lib-tests")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "dat"))
        .collect();
    files.sort();
    let (mut pass, mut evaluated, mut skipped) = (0usize, 0usize, 0usize);
    let mut failures = String::new();
    for file in &files {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        for case in parse_dat(&std::fs::read_to_string(file).unwrap()) {
            if case.script_on {
                skipped += 1;
                continue;
            }
            evaluated += 1;
            let got = if let Some(context) = &case.context {
                dump_node(&parse_html_fragment_context(&case.data, context))
            } else {
                let node = dump_node(&parse_html_document(&case.data));
                assert_eq!(
                    dump_soa(&parse_html_soa(&case.data)),
                    node,
                    "[{name}] {:?}: Soa and Node trees differ",
                    case.data
                );
                let document = parse_html_document(&case.data);
                assert_eq!(
                    parse_html_document_count(&case.data),
                    count_outside_templates(&document),
                    "[{name}] {:?}: node count",
                    case.data
                );
                node
            };
            if got == case.document {
                pass += 1;
            } else if case.data.to_ascii_lowercase().contains("<select")
                || case.context.as_deref() == Some("select")
            {
                let _ = writeln!(failures, "select-family miss [{name}] {:?}", case.data);
            } else {
                panic!(
                    "[{name}] non-<select> miss {:?} (context {:?})\n--- expected\n{}\n--- got\n{got}",
                    case.data, case.context, case.document
                );
            }
        }
    }
    let rate = pass as f64 * 100.0 / evaluated as f64;
    eprintln!(
        "core html5lib: {pass}/{evaluated} = {rate:.2}% ({skipped} script-on skipped)\n{failures}"
    );
    // the JS gate's numbers (test/conformance.test.mjs, harness/conformance.mjs)
    assert_eq!((pass, evaluated, skipped), (1782, 1783, 8));
}

#[test]
fn chrome156_documents_and_fragments() {
    let fixture: serde_json::Value = serde_json::from_str(CHROME).unwrap();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    let (mut documents, mut fragments, mut quirks) = (0, 0, 0);
    let mut failures = Vec::new();
    for row in fixture["documents"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let html = row["html"].as_str().unwrap();
        let want = row["dump"].as_str().unwrap();
        if let Some(context) = row["context"].as_str() {
            fragments += 1;
            let got = dump_node(&parse_html_fragment_context(html, context));
            if got != want {
                failures.push(format!(
                    "{id} (in {context}):\n--- Chrome\n{want}\n--- core\n{got}"
                ));
            }
        } else {
            documents += 1;
            let got = dump_node(&parse_html_document(html));
            let (soa, mode) = parse_html_soa_with_compat_mode(html);
            let soa_got = dump_soa(&soa);
            let want_mode = row["compatMode"].as_str().unwrap();
            quirks += usize::from(want_mode == "BackCompat");
            if got != want || soa_got != want || mode != want_mode {
                failures.push(format!(
                    "{id}: mode {mode} (Chrome {want_mode})\n--- Chrome\n{want}\n--- core\n{got}\n--- soa\n{soa_got}"
                ));
            }
        }
    }
    assert_eq!((documents, fragments), (115, 83), "fixture rows");
    assert!(
        quirks > 0 && quirks < documents,
        "both compat modes are exercised"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

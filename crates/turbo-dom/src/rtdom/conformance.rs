//! DIRECT rtdom html5lib-tests conformance gate. Parses each fixture via the rtdom
//! `Tree`, dumps it through rtdom's traversal (`dump.rs`), and string-compares to the
//! fixture's `#document`. This proves rtdom's tree representation + traversal are
//! faithful — distinct from the transitive parser gate (`harness/conformance.mjs`),
//! which serializes the parser output in JS without going through rtdom.
//!
//! v1 scope: FULL-DOCUMENT tests. `#document-fragment` tests are skipped (counted)
//! — rtdom fragment parsing into a buffer isn't wired yet; those stay covered by the
//! transitive JS gate. `#script-on` fixtures are skipped (scripting flag is off).

use super::dump::dump_tree;
use super::tree::Tree;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

struct DatTest {
    data: String,
    document: String,
    fragment_context: Option<String>,
    script_on: bool,
}

const HEADERS: &[&str] = &[
    "#data",
    "#errors",
    "#new-errors",
    "#document",
    "#document-fragment",
    "#script-on",
    "#script-off",
];

fn trim_trailing_blank(lines: &[String]) -> Vec<String> {
    let mut v = lines.to_vec();
    while v.last().is_some_and(std::string::String::is_empty) {
        v.pop();
    }
    v
}

/// Port of `harness/dat.mjs` parseDatFile. Section keys are the `'static` HEADERS,
/// so the map never borrows from `text` (sidesteps lifetime tangles).
fn parse_dat(text: &str) -> Vec<DatTest> {
    let mut tests = Vec::new();
    let mut cur: HashMap<&'static str, Vec<String>> = HashMap::new();
    let mut have_cur = false;
    let mut section: Option<&'static str> = None;
    let mut buf: Vec<String> = Vec::new();

    for line in text.split('\n') {
        if let Some(&hdr) = HEADERS.iter().find(|&&h| h == line) {
            // flush the pending section's buffer into the current case
            if let Some(sec) = section {
                cur.insert(sec, std::mem::take(&mut buf));
            }
            if hdr == "#data" {
                if have_cur {
                    tests.push(normalize(&cur));
                    cur.clear();
                }
                have_cur = true;
            }
            section = Some(hdr);
            buf = Vec::new();
        } else {
            buf.push(line.to_string());
        }
    }
    if let Some(sec) = section {
        cur.insert(sec, std::mem::take(&mut buf));
    }
    if have_cur {
        tests.push(normalize(&cur));
    }
    tests
}

fn normalize(raw: &HashMap<&str, Vec<String>>) -> DatTest {
    let data = raw.get("#data").map(|v| v.join("\n")).unwrap_or_default();
    let document = raw
        .get("#document")
        .map(|v| trim_trailing_blank(v).join("\n"))
        .unwrap_or_default();
    let fragment_context = raw.get("#document-fragment").and_then(|v| {
        let s = trim_trailing_blank(v).join("\n").trim().to_string();
        if s.is_empty() { None } else { Some(s) }
    });
    let script_on = raw.contains_key("#script-on");
    DatTest { data, document, fragment_context, script_on }
}

#[test]
fn rtdom_html5lib_conformance_full_document() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/html5lib-tests");
    let Ok(entries) = fs::read_dir(&dir) else {
        eprintln!("rtdom conformance: fixtures not found at {dir:?} — skipping");
        return;
    };
    let mut dat_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "dat"))
        .collect();
    dat_files.sort();

    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut skip = 0usize;
    let mut bad: Vec<(String, String, String)> = Vec::new(); // (input, expected, actual)

    for path in &dat_files {
        let Ok(text) = fs::read_to_string(path) else { continue };
        for t in parse_dat(&text) {
            if t.script_on || t.fragment_context.is_some() {
                skip += 1;
                continue;
            }
            let actual = dump_tree(&Tree::parse(&t.data));
            if actual == t.document {
                pass += 1;
            } else {
                fail += 1;
                // The only accepted misses are the upstream <select>-family insertion
                // -mode divergences (same as the JS gate). A non-<select> miss means an
                // rtdom dump/traversal bug — fail loudly.
                if !t.data.contains("select") {
                    bad.push((t.data.clone(), t.document.clone(), actual));
                }
            }
        }
    }

    let total = pass + fail;
    let rate = if total > 0 { pass as f64 / total as f64 * 100.0 } else { 0.0 };
    eprintln!(
        "rtdom html5lib conformance (full-document): {pass}/{total} = {rate:.2}%  ({skip} skipped: fragment/script-on)"
    );

    for (input, expected, actual) in bad.iter().take(5) {
        eprintln!("--- NON-<select> FAILURE ---\ninput: {input}\nexpected:\n{expected}\nactual:\n{actual}\n");
    }

    assert!(
        bad.is_empty(),
        "{} non-<select> conformance failures — rtdom dump/traversal diverges from the fixtures",
        bad.len()
    );
    if total > 0 {
        assert!(rate >= 99.0, "rtdom conformance {rate:.2}% < 99% gate");
    }
}

#[cfg(test)]
mod dat_tests {
    use super::*;

    #[test]
    fn parses_sections_and_normalizes() {
        let text = "#data\n<div>x</div>\n#errors\n#document\n| <html>\n|   <head>\n|   <body>\n|     <div>\n|       \"x\"\n\n#data\n<p>\n#document-fragment\ntd\n#document\n| <p>\n";
        let tests = parse_dat(text);
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].data, "<div>x</div>");
        assert!(tests[0].document.starts_with("| <html>"));
        assert!(!tests[0].document.ends_with('\n')); // trailing blank trimmed
        assert_eq!(tests[0].fragment_context, None);
        assert_eq!(tests[1].fragment_context.as_deref(), Some("td"));
        assert!(!tests[1].script_on);
    }

    #[test]
    fn script_on_flag() {
        let text = "#data\n<noscript>x\n#script-on\n#document\n| <html>\n";
        let tests = parse_dat(text);
        assert_eq!(tests.len(), 1);
        assert!(tests[0].script_on);
    }
}

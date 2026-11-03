//! XML documents (turbo-dom#186) against the Chrome 156 control: the parsed XMLDocument
//! tree, `<parsererror>` documents, CDATA sections and the XMLSerializer round trip.
use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;
use sha2::Digest;

/// The embedder's `DOMParser` XML path: standalone V8 has no DOMParser, so the probe's
/// `new DOMParser().parseFromString(source, type)` calls this host function instead.
fn parse_xml(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let source = dom_units(scope, args.get(0)).unwrap();
    let content_type = args.get(1).to_rust_string_lossy(scope);
    rv.set(
        parse_xml_document_units(scope, &source, &content_type)
            .document
            .into(),
    );
}

fn run(probe: &str) -> serde_json::Value {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let global = context.global(scope);
    bind_method(scope, global, "__parseXML", parse_xml);
    let probe = probe.replace(
        "new DOMParser().parseFromString(source, type)",
        "__parseXML(source, type)",
    );
    let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, &source).unwrap();
    let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    let result =
        result.unwrap_or_else(|| panic!("XML probe failed: {}", thrown(scope.exception(), scope)));
    let result = serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
    reset();
    result
}

#[test]
fn xml_documents_match_the_chrome156_control() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("xml_documents_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let probe = include_str!("xml_documents_probe.js");
    let digest = sha2::Sha256::digest(probe.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        control["probeSha256"], hex,
        "recapture the control for the edited probe"
    );
    let actual = run(probe);
    let expected = control["result"].as_object().unwrap();
    let differences: Vec<_> = expected
        .iter()
        .filter(|(key, value)| actual.get(key.as_str()) != Some(*value))
        .map(|(key, value)| format!("{key}:\n  native={}\n  Chrome={value}", actual[key]))
        .collect();
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert_eq!(&actual, &control["result"]);
}

#[test]
fn inner_and_outer_html_of_xml_documents_match_the_chrome156_control() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("xml_inner_html_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let probe = include_str!("xml_inner_html_probe.js");
    let digest = sha2::Sha256::digest(probe.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        control["probeSha256"], hex,
        "recapture the control for the edited probe"
    );
    let actual = run(probe);
    for section in ["getters", "innerSet", "outerSet"] {
        let expected = control["result"][section].as_object().unwrap();
        let differences: Vec<_> = expected
            .iter()
            .filter(|(key, value)| actual[section].get(key.as_str()) != Some(*value))
            .map(|(key, value)| {
                format!(
                    "{section}.{key}:\n  native={}\n  Chrome={value}",
                    actual[section][key]
                )
            })
            .collect();
        assert!(differences.is_empty(), "{}", differences.join("\n"));
    }
    assert_eq!(&actual, &control["result"]);
}

/// Exponential entity amplification stays bounded: a billion-laughs document (3 * 10^9
/// characters when expanded) ends in a `<parsererror>` after a bounded amount of work,
/// while a document within the limits still expands its entities.
#[test]
fn entity_amplification_is_bounded() {
    let mut source = String::from("<!DOCTYPE r [<!ENTITY l0 \"lol\">");
    for level in 1..=9 {
        let reference = format!("&l{};", level - 1);
        source.push_str(&format!("<!ENTITY l{level} \"{}\">", reference.repeat(10)));
    }
    source.push_str("]><r>&l9;</r>");
    let started = std::time::Instant::now();
    let parsed = xml_documents::parse(&source);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    let text = xml_documents::serialize(&parsed.tree, parsed.tree.root(), None);
    assert!(text.contains("Entity too big"), "{text}");
    assert!(text.len() < 4096, "{}", text.len());
    let parsed = xml_documents::parse(
        "<!DOCTYPE r [<!ENTITY a \"lol\"><!ENTITY b \"&a;&a;\">]><r>&b;&b;</r>",
    );
    let text = xml_documents::serialize(&parsed.tree, parsed.tree.root(), None);
    assert_eq!(text, "<!DOCTYPE r><r>lollollollol</r>");
}

/// ATTLIST defaults cannot amplify: a default inserts each declared entity's replacement
/// text once, verbatim (Chrome 156; the control's `attlistDefault*` rows), so ten levels
/// of ten references, a self-referencing entity and a defaulted namespace declaration
/// (the pre-parse rewrite) each finish at once. What a default can still multiply — one
/// large entity referenced many times, per element — stops at the expansion budget (10x
/// the source plus 1 MB) with a `<parsererror>` (Chrome builds such a document; this
/// native parser stops instead).
#[test]
fn attlist_default_amplification_is_bounded() {
    let mut laughs = String::from("<!DOCTYPE r [<!ENTITY l0 \"lol\">");
    for level in 1..=9 {
        let reference = format!("&l{};", level - 1);
        laughs.push_str(&format!("<!ENTITY l{level} \"{}\">", reference.repeat(10)));
    }
    let l8 = "&amp;l8;".repeat(10);
    let self_reference = "<!DOCTYPE r [<!ENTITY e \"&e;&e;\">";
    let cases = [
        (
            format!("{laughs}<!ATTLIST r a CDATA \"&l9;\">]><r/>"),
            format!("<!DOCTYPE r><r a=\"{l8}\"/>"),
        ),
        (
            format!("{laughs}<!ATTLIST r xmlns:p CDATA \"&l9;\">]><r/>"),
            format!("<!DOCTYPE r><r xmlns:p=\"{l8}\"/>"),
        ),
        (
            format!("{self_reference}<!ATTLIST r a CDATA \"&e;\">]><r/>"),
            "<!DOCTYPE r><r a=\"&amp;e;&amp;e;\"/>".to_owned(),
        ),
        (
            format!("{self_reference}<!ATTLIST r xmlns:p CDATA \"&e;\">]><r><p:c/></r>"),
            "<!DOCTYPE r><r xmlns:p=\"&amp;e;&amp;e;\"><p:c/></r>".to_owned(),
        ),
    ];
    for (source, expected) in cases {
        let started = std::time::Instant::now();
        let parsed = xml_documents::parse(&source);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "{source}"
        );
        assert!(parsed.well_formed, "{source}");
        let text = xml_documents::serialize(&parsed.tree, parsed.tree.root(), None);
        assert_eq!(text, expected);
    }
    let big = "x".repeat(100_000);
    // within the budget: 50 x 10 KB = 500 KB of defaults from a ~10 KB source
    let source = format!(
        "<!DOCTYPE r [<!ENTITY e \"{}\"><!ATTLIST r a CDATA \"{}\">]><r/>",
        &big[..10_000],
        "&e;".repeat(50)
    );
    let parsed = xml_documents::parse(&source);
    assert!(parsed.well_formed);
    let text = xml_documents::serialize(&parsed.tree, parsed.tree.root(), None);
    assert_eq!(text.len(), "<!DOCTYPE r><r a=\"\"/>".len() + 500_000);
    // over it: 1000 x 100 KB (one default, a namespace default, per element)
    let over = [
        format!(
            "<!DOCTYPE r [<!ENTITY e \"{big}\"><!ATTLIST r a CDATA \"{}\">]><r/>",
            "&e;".repeat(1000)
        ),
        format!(
            "<!DOCTYPE r [<!ENTITY e \"{big}\"><!ATTLIST r xmlns:p CDATA \"{}\">]><r/>",
            "&e;".repeat(1000)
        ),
        format!(
            "<!DOCTYPE r [<!ENTITY e \"{big}\"><!ATTLIST s a CDATA \"&e;\">]><r>{}</r>",
            "<s/>".repeat(1000)
        ),
    ];
    for source in over {
        let started = std::time::Instant::now();
        let parsed = xml_documents::parse(&source);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        assert!(!parsed.well_formed);
        let text = xml_documents::serialize(&parsed.tree, parsed.tree.root(), None);
        assert!(
            text.contains("Entity too big"),
            "{}",
            &text[..text.len().min(2000)]
        );
        // what was built before the error stays within the budget
        assert!(text.len() < 12 * source.len() + 1_000_000, "{}", text.len());
    }
}

/// Specified attribute values (#268) insert an entity's replacement text once, verbatim,
/// like Chrome (the control's `attributeEntity*` rows), and share the ATTLIST defaults'
/// expansion budget (10x the source plus 1 MB): one large entity referenced many times,
/// in one value or across elements, stops with a `<parsererror>` (Chrome builds such a
/// document; this native parser stops instead), while values within it still expand.
#[test]
fn specified_attribute_expansion_is_bounded() {
    let big = "x".repeat(100_000);
    let source = format!(
        "<!DOCTYPE r [<!ENTITY e \"{}\">]><r a=\"{}\"/>",
        &big[..10_000],
        "&e;".repeat(50)
    );
    let parsed = xml_documents::parse(&source);
    assert!(parsed.well_formed);
    let text = xml_documents::serialize(&parsed.tree, parsed.tree.root(), None);
    assert_eq!(text.len(), "<!DOCTYPE r><r a=\"\"/>".len() + 500_000);
    let over = [
        format!(
            "<!DOCTYPE r [<!ENTITY e \"{big}\">]><r a=\"{}\"/>",
            "&e;".repeat(1000)
        ),
        format!(
            "<!DOCTYPE r [<!ENTITY e \"{big}\">]><r>{}</r>",
            "<s a=\"&e;\"/>".repeat(1000)
        ),
        format!(
            "<!DOCTYPE r [<!ENTITY e \"{big}\"><!ATTLIST s b CDATA \"&e;\">]><r>{}</r>",
            "<s a=\"&e;\"/>".repeat(1000)
        ),
    ];
    for source in over {
        let started = std::time::Instant::now();
        let parsed = xml_documents::parse(&source);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        assert!(!parsed.well_formed);
        let text = xml_documents::serialize(&parsed.tree, parsed.tree.root(), None);
        assert!(
            text.contains("Entity too big"),
            "{}",
            &text[..text.len().min(2000)]
        );
        // what was built before the error stays within the budget
        assert!(text.len() < 12 * source.len() + 1_000_000, "{}", text.len());
    }
}

/// `parse_xml_document_units` reports whether the source was well-formed, so an embedder
/// (turbo-surf's `responseXML`) need not look for a `<parsererror>` in the tree: a valid
/// document that itself holds a `parsererror` element is still well-formed.
#[test]
fn parse_xml_document_units_reports_well_formedness() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let count = v8::String::new(
        scope,
        "(doc) => doc.getElementsByTagNameNS('http://www.w3.org/1999/xhtml', 'parsererror').length",
    )
    .unwrap();
    let count = v8::Script::compile(scope, count, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        .unwrap();
    // (source, well-formed, <parsererror> elements in the resulting tree)
    let cases = [
        ("<a><b/></a>", true, 0),
        ("<a><b></a>", false, 1),
        ("", false, 1),
        (
            "<r><parsererror xmlns=\"http://www.w3.org/1999/xhtml\"><h3>x</h3></parsererror></r>",
            true,
            1,
        ),
    ];
    for (source, well_formed, errors) in cases {
        let units: Vec<u16> = source.encode_utf16().collect();
        let parsed = parse_xml_document_units(scope, &units, "application/xml");
        assert_eq!(parsed.well_formed, well_formed, "{source}");
        let undefined = v8::undefined(scope).into();
        let found = count
            .call(scope, undefined, &[parsed.document.into()])
            .and_then(|value| value.uint32_value(scope))
            .unwrap();
        assert_eq!(found, errors, "{source}");
    }
    reset();
}

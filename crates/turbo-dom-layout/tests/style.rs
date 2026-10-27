//! Style-system tests (§4): CSS parsing, selector matching + specificity,
//! tokens, node-style injection, the 6-level cascade, and inheritance.
//!
//! Split at the S1 relocation: the cases that need `TKind` directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/style.rs`.

mod common;

use turbo_dom_layout::style::{
    build_cascade, parse_stylesheet, StyledElement, StyledNode, TokenSet,
};

fn find<'a>(
    nodes: &'a [StyledNode],
    pred: &dyn Fn(&StyledElement) -> bool,
) -> Option<&'a StyledElement> {
    for node in nodes {
        if let StyledNode::Element(el) = node {
            if pred(el) {
                return Some(el);
            }
            if let Some(found) = find(&el.children, pred) {
                return Some(found);
            }
        }
    }
    None
}

fn by_id<'a>(nodes: &'a [StyledNode], id: &str) -> &'a StyledElement {
    find(nodes, &|el| {
        el.attrs.iter().any(|a| a.name == "id" && a.value == id)
    })
    .expect("element by id")
}

fn prop(nodes: &[StyledNode], id: &str, property: &str) -> Option<String> {
    by_id(nodes, id).style.get(property).map(str::to_string)
}

// --------------------------------------------------------------------------
// parser
// --------------------------------------------------------------------------

#[test]
fn parses_rules_comments_and_at_rules() {
    let sheet = parse_stylesheet(
        "/* c */ .a { color: red; font-weight: 700 !important } @page { size: A4 }",
    );
    assert_eq!(sheet.rules.len(), 1);
    assert_eq!(sheet.rules[0].declarations.len(), 2);
    assert!(sheet.rules[0].declarations[1].important);
    assert_eq!(sheet.at_rules.len(), 1);
    assert_eq!(sheet.at_rules[0].name, "page");
    assert_eq!(sheet.at_rules[0].body, "size: A4");
}

#[test]
fn malformed_declarations_are_skipped() {
    let sheet = parse_stylesheet(".a { ; color: red; novalue: ; :bad ; width: 10px }");
    assert_eq!(sheet.rules[0].declarations.len(), 2);
    let empty = parse_stylesheet("{ color: red }");
    assert!(empty.rules.is_empty());
}

#[test]
fn unterminated_comment_consumes_rest() {
    let sheet = parse_stylesheet(".a { color: red } /* dangling");
    assert_eq!(sheet.rules.len(), 1);
}

#[test]
fn nested_braces_in_at_rule_stay_intact() {
    let sheet = parse_stylesheet("@media print { .a { color: red } }");
    assert_eq!(sheet.rules.len(), 0);
    assert_eq!(sheet.at_rules.len(), 1);
    assert_eq!(sheet.at_rules[0].name, "media");
    assert!(sheet.at_rules[0].body.contains(".a"));
}

#[test]
fn at_rules_remember_their_source_position() {
    let mut sheet = parse_stylesheet(
        "@media screen { .a { color: red } } .b { color: blue } @page { size: A4 } .c { x: y }",
    );
    assert_eq!(sheet.rules.len(), 2);
    assert_eq!(sheet.at_rule_position(0), 0);
    assert_eq!(sheet.at_rule_position(1), 1);
    // An at-rule pushed by hand has no recorded position: it follows every rule.
    sheet.at_rules.push(sheet.at_rules[1].clone());
    assert_eq!(sheet.at_rule_position(2), 2);
    // A recorded position never points past the rules a caller kept.
    sheet.rules.clear();
    assert_eq!(sheet.at_rule_position(1), 0);
}

#[test]
fn conditional_rules_cascade_at_their_source_position() {
    // miaskiewicz/turbo-dom#24 item 1 (Chrome 156: crates/turbo-dom-layout/tests/
    // fixtures/html_document_chrome156.json `media-*` / `supports-*` cases).
    let order = |css: &str| -> Vec<String> {
        build_cascade(css, "", TokenSet::default())
            .rules
            .iter()
            .filter(|r| r.level == 1)
            .map(|r| r.rule.selectors.prelude().to_string())
            .collect()
    };
    assert_eq!(
        order(
            "@media screen { .m { a: b } } .p { a: b } @supports (display: grid) { .s { a: b } }"
        ),
        [".m", ".p", ".s"]
    );
    assert_eq!(
        order(
            "@supports (display: block) { @media screen { .n { a: b } } .s { a: b } } .p { a: b }"
        ),
        [".n", ".s", ".p"]
    );
    // A block whose condition fails keeps nothing, and leaves the order of the rest.
    assert_eq!(
        order(".p { a: b } @media print { .x { a: b } } .q { a: b } @font-face { a: b }"),
        [".p", ".q"]
    );
}

#[test]
fn empty_selector_list_produces_no_rule() {
    let sheet = parse_stylesheet(", { color: red }");
    assert!(sheet.rules.is_empty());
}

#[test]
fn root_pseudo_matches_html_element() {
    // `:root` (cascade.rs Root → ctx.tag == Some("html")). The compiler strips the
    // `<html>` wrapper before render, so build a tree with a top-level html element
    // by hand and style it directly.
    use turbo_dom_layout::test_support::literal::{Element, Node};
    use turbo_dom_layout::{Attr, Tag};
    let html = Node::Element(Element {
        tag: Tag::Html("html".to_string()),
        attrs: vec![Attr {
            name: "id".to_string(),
            value: "r".to_string(),
        }],
        children: vec![Node::Text("x".to_string())],
    });
    let cascade = build_cascade(":root { color: teal }", "", TokenSet::new());
    let styled = common::style_nodes(&[html], &cascade);
    assert_eq!(prop(&styled, "r", "color").as_deref(), Some("teal"));
}

// --------------------------------------------------------------------------
// tokens, presentational hints, font-size resolution, var(), @supports/@media
// --------------------------------------------------------------------------

fn styled_html(html: &str, cascade: &turbo_dom_layout::style::Cascade) -> Vec<StyledNode> {
    let nodes = turbo_dom_layout::test_support::literal::parse_html(html).unwrap();
    common::style_nodes(&nodes, cascade)
}

fn token(extends: &[&str], css: &str) -> turbo_dom_layout::style::StyleToken {
    turbo_dom_layout::style::StyleToken {
        extends: extends.iter().map(|s| (*s).to_string()).collect(),
        declarations: turbo_dom_layout::style::parse_declarations(css),
    }
}

#[test]
fn style_tokens_resolve_extends_in_application_order() {
    // §4.3b: `t:style="a b"` applies each token after the tokens it extends, later
    // tokens winning; an unknown name contributes nothing.
    let mut tokens = TokenSet::new();
    tokens.insert("base".into(), token(&[], "color: red; width: 10px"));
    tokens.insert("card".into(), token(&["base"], "color: blue"));
    tokens.insert("wide".into(), token(&[], "width: 30px"));
    let cascade = build_cascade("", "", tokens);
    let styled = styled_html(
        "<div id=a t:style='card'></div><div id=b t:style='card wide nope'></div><div id=c t:style='nope'></div><div id=d></div>",
        &cascade,
    );
    assert_eq!(prop(&styled, "a", "color").as_deref(), Some("blue"));
    assert_eq!(prop(&styled, "a", "width").as_deref(), Some("10px"));
    assert_eq!(prop(&styled, "b", "color").as_deref(), Some("blue"));
    assert_eq!(prop(&styled, "b", "width").as_deref(), Some("30px"));
    assert_eq!(prop(&styled, "c", "width"), None);
    assert_eq!(prop(&styled, "d", "color"), None);
}

#[test]
fn style_tokens_lose_to_inline_style_and_beat_author_rules() {
    // Tokens sit at level 2: above author rules (1), below inline style (4).
    let mut tokens = TokenSet::new();
    tokens.insert("t".into(), token(&[], "color: green; height: 5px"));
    let cascade = build_cascade("#a { color: red; height: 9px }", "", tokens);
    let styled = styled_html("<div id=a t:style=t style='height: 7px'></div>", &cascade);
    assert_eq!(prop(&styled, "a", "color").as_deref(), Some("green"));
    assert_eq!(prop(&styled, "a", "height").as_deref(), Some("7px"));
}

#[test]
fn cyclic_style_tokens_terminate_at_the_depth_cap() {
    // `a extends b extends a` stops at MAX_DEPTH (32) instead of recursing forever: the
    // deepest resolved token is applied first, so the outermost (`a`) still wins.
    let mut tokens = TokenSet::new();
    tokens.insert("a".into(), token(&["b"], "color: red"));
    tokens.insert("b".into(), token(&["a"], "color: blue; width: 4px"));
    let cascade = build_cascade("", "", tokens);
    let styled = styled_html("<p id=p t:style=a></p><p id=q t:style=b></p>", &cascade);
    assert_eq!(prop(&styled, "p", "color").as_deref(), Some("red"));
    assert_eq!(prop(&styled, "p", "width").as_deref(), Some("4px"));
    assert_eq!(prop(&styled, "q", "color").as_deref(), Some("blue"));
}

#[test]
fn legacy_align_font_color_and_cellpadding_hints() {
    // Presentational hints (Chrome: `align=justify` → `text-align: justify`, an unknown
    // keyword maps to nothing; `<font color>` → `color`; a table's `cellpadding` pads
    // its cells), all overridable by author CSS.
    let cascade = build_cascade("#o { padding: 9px }", "", TokenSet::new());
    let styled = styled_html(
        "<p id=j align=JUSTIFY>x</p><p id=u align=bogus>x</p><span id=s align=center>x</span>\
         <font id=f color=' red '>x</font><font id=e color='  '>x</font><b id=b color=red>x</b>\
         <table cellpadding=' 3 '><tr><td id=c>x</td><th id=h>y</th><td id=o>z</td></tr></table>\
         <table cellpadding=wide><tr><td id=w>x</td></tr></table>\
         <table><tr><td id=n>x</td></tr></table>",
        &cascade,
    );
    assert_eq!(prop(&styled, "j", "text-align").as_deref(), Some("justify"));
    assert_eq!(prop(&styled, "u", "text-align"), None);
    assert_eq!(prop(&styled, "s", "text-align"), None, "not an aligned tag");
    assert_eq!(prop(&styled, "f", "color").as_deref(), Some("red"));
    assert_eq!(prop(&styled, "e", "color"), None);
    assert_eq!(prop(&styled, "b", "color"), None, "only <font> maps color");
    assert_eq!(prop(&styled, "c", "padding").as_deref(), Some("3px"));
    assert_eq!(prop(&styled, "h", "padding").as_deref(), Some("3px"));
    assert_eq!(prop(&styled, "o", "padding").as_deref(), Some("9px"));
    assert_eq!(
        prop(&styled, "w", "padding").as_deref(),
        Some("1px"),
        "UA td padding"
    );
    assert_eq!(prop(&styled, "n", "padding").as_deref(), Some("1px"));
}

#[test]
fn cell_outside_any_table_gets_no_cellpadding() {
    use turbo_dom_layout::test_support::literal::{Element, Node};
    use turbo_dom_layout::{Attr, Tag};
    let attr = |name: &str, value: &str| Attr {
        name: name.to_string(),
        value: value.to_string(),
    };
    let td = Node::Element(Element {
        tag: Tag::Html("td".to_string()),
        attrs: vec![attr("id", "c")],
        children: vec![],
    });
    let div = Node::Element(Element {
        tag: Tag::Html("div".to_string()),
        attrs: vec![attr("cellpadding", "5")],
        children: vec![td],
    });
    let styled = common::style_nodes(&[div], &build_cascade("", "", TokenSet::new()));
    assert_eq!(prop(&styled, "c", "padding").as_deref(), Some("1px"));
}

#[test]
fn font_size_keywords_units_and_unparsable_values_resolve_to_px() {
    let cascade = build_cascade(
        "#k{font-size:larger} #s{font-size:smaller} #x{font-size:xx-large} #pt{font-size:12pt} \
         #in{font-size:0.5in} #pc{font-size:2pc} #mm{font-size:25.4mm} #cm{font-size:2.54cm} \
         #pct{font-size:50%} #em{font-size:2em} #bad{font-size:big} #unit{font-size:2ex}",
        "",
        TokenSet::new(),
    );
    let styled = styled_html(
        "<div style='font-size:20px'><i id=k></i><i id=s></i><i id=x></i><i id=pt></i><i id=in></i>\
         <i id=pc></i><i id=mm></i><i id=cm></i><i id=pct></i><i id=em></i><i id=bad></i><i id=unit></i></div>",
        &cascade,
    );
    let px = |id: &str| prop(&styled, id, "font-size").unwrap();
    assert_eq!(px("k"), "24px", "larger = parent x 1.2");
    assert_eq!(px("s"), format!("{}px", 20.0f32 / 1.2));
    assert_eq!(px("x"), "32px", "xx-large = 2 x medium");
    assert_eq!(px("pt"), "16px");
    assert_eq!(px("in"), "48px");
    assert_eq!(px("pc"), "32px");
    assert_eq!(px("mm"), "96px");
    assert_eq!(px("cm"), "96px");
    assert_eq!(px("pct"), "10px");
    assert_eq!(px("em"), "40px");
    assert_eq!(px("bad"), "20px", "an unparsable size keeps the parent's");
    assert_eq!(px("unit"), "20px", "an unknown unit keeps the parent's");
}

#[test]
fn var_references_inside_functions_fallbacks_and_unclosed() {
    let cascade = build_cascade(
        ":root{--w:10px} #a{width:calc(var(--w) + 1px)} #b{width:var(--missing, calc(2px + 3px))} \
         #c{width:var(--w, 99px)} #d{height:var(--w}",
        "",
        TokenSet::new(),
    );
    use turbo_dom_layout::test_support::literal::{Element, Node};
    use turbo_dom_layout::{Attr, Tag};
    let el = |tag: &str, id: &str, children| {
        Node::Element(Element {
            tag: Tag::Html(tag.to_string()),
            attrs: vec![Attr {
                name: "id".to_string(),
                value: id.to_string(),
            }],
            children,
        })
    };
    let html = el(
        "html",
        "r",
        vec![
            el("p", "a", vec![]),
            el("p", "b", vec![]),
            el("p", "c", vec![]),
            el("p", "d", vec![]),
        ],
    );
    let styled = common::style_nodes(&[html], &cascade);
    assert_eq!(
        prop(&styled, "a", "width").as_deref(),
        Some("calc(10px + 1px)")
    );
    assert_eq!(
        prop(&styled, "b", "width").as_deref(),
        Some("calc(2px + 3px)")
    );
    assert_eq!(prop(&styled, "c", "width").as_deref(), Some("10px"));
    assert_eq!(
        prop(&styled, "d", "height").as_deref(),
        Some("var(--w"),
        "an unclosed var() is kept verbatim"
    );
}

#[test]
fn supports_and_or_not_combinations() {
    let order = |css: &str| -> Vec<String> {
        build_cascade(css, "", TokenSet::default())
            .rules
            .iter()
            .filter(|r| r.level == 1)
            .map(|r| r.rule.selectors.prelude().to_string())
            .collect()
    };
    assert_eq!(
        order(
            "@supports (display: grid) and (gap: 1px) { .and { a: b } } \
             @supports (display: grid) and (not (gap: 1px)) { .andnot { a: b } } \
             @supports not (display: grid) or (gap: 1px) { .or { a: b } } \
             @supports not (display: grid) or not (gap: 1px) { .ornot { a: b } } \
             @supports ((a: b) and (c: d)) { .group { a: b } } \
             @supports ((a: b)) and ((c: d)) { .pair { a: b } } \
             @supports not ((a: b) or (c: d)) { .notgroup { a: b } }"
        ),
        [".and", ".or", ".group", ".pair"]
    );
}

#[test]
fn media_lengths_in_em_and_single_term_calc() {
    let order = |css: &str, width: f32| -> Vec<String> {
        turbo_dom_layout::style::build_cascade_with_width(css, "", TokenSet::default(), width)
            .rules
            .iter()
            .filter(|r| r.level == 1)
            .map(|r| r.rule.selectors.prelude().to_string())
            .collect()
    };
    let css =
        "@media (max-width: 40em) { .em { a: b } } @media (min-width: 2.5rem) { .rem { a: b } } \
               @media (max-width: calc(600px)) { .calc { a: b } }";
    assert_eq!(order(css, 640.0), [".em", ".rem"]);
    assert_eq!(order(css, 641.0), [".rem"]);
    assert_eq!(order(css, 600.0), [".em", ".rem", ".calc"]);
    assert_eq!(order(css, 39.0), [".em", ".calc"]);
}

#[test]
fn set_media_viewport_width_drives_the_next_cascade() {
    let css = "@media (max-width: 500px) { .narrow { a: b } }";
    let has = || {
        build_cascade(css, "", TokenSet::default())
            .rules
            .iter()
            .any(|r| r.level == 1 && r.rule.selectors.prelude() == ".narrow")
    };
    turbo_dom_layout::style::set_media_viewport_width(500.0);
    assert!(has());
    turbo_dom_layout::style::set_media_viewport_width(501.0);
    assert!(!has());
}

#[test]
fn rule_selector_validity() {
    use turbo_dom_layout::style::RuleSelectors;
    assert!(RuleSelectors::parse("div > p.a").is_valid());
    assert!(!RuleSelectors::parse("div >>> p").is_valid());
    assert_eq!(RuleSelectors::parse("div >>> p").prelude(), "div >>> p");
}

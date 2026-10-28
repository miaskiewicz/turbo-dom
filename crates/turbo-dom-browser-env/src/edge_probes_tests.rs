//! Edge probes of the DOM / CSSOM binding against their Chrome 156 controls
//! (`*_probe.js` → `*_chrome156.json`, recorded by `scripts/trusted-input-chrome-control.mjs`
//! with the frozen Chrome 156.0.8078.47): manual / named slot assignment, pre-insertion
//! validity of the mutation methods, selector admission and serialization, and the CSSOM
//! at-rule parser. Every leaf must equal Chrome's except the KNOWN_GAP rows, which must still
//! differ exactly so (a fix removes its rows).
use crate::browser_env::reference_target::probe_harness::{control, differences, run};

/// The leaves where `actual` differs from `expected` and is not a listed gap, then the gaps
/// that no longer differ as listed (stale rows).
fn mismatches(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    gaps: &[(&str, &str)],
) -> Vec<String> {
    let mut out = Vec::new();
    differences("result", actual, expected, &mut out);
    let mut seen = Vec::new();
    out.retain(|entry| {
        let (path, rest) = entry.split_once('\n').unwrap_or((entry, ""));
        let binding = rest
            .lines()
            .find_map(|line| line.trim().strip_prefix("binding: "))
            .unwrap_or("");
        let known = gaps.contains(&(path, binding));
        if known {
            seen.push(path.to_owned());
        }
        !known
    });
    for (path, _) in gaps {
        if !seen.iter().any(|seen| seen == path) {
            out.push(format!("stale known gap: {path}"));
        }
    }
    out
}

fn check(probe: &str, json: &str, gaps: &[(&str, &str)]) {
    let expected = control(json, probe);
    assert_eq!(
        mismatches(&run(probe), &expected, gaps),
        Vec::<String>::new()
    );
}

#[test]
fn mismatches_reports_differences_extras_and_stale_gaps() {
    let expected = serde_json::json!({"a": [1, 2], "b": "x", "c": {"d": true}});
    let same = expected.clone();
    assert_eq!(mismatches(&same, &expected, &[]), Vec::<String>::new());
    let actual = serde_json::json!({"a": [1, 3], "b": "y", "c": {"d": true, "e": 0}});
    assert_eq!(
        mismatches(
            &actual,
            &expected,
            &[("result.b", "\"y\""), ("result.c.d", "false")]
        ),
        vec![
            "result.a[1]\n  binding: 3\n  chrome:  2".to_owned(),
            "result.c.e: extra in binding".to_owned(),
            "stale known gap: result.c.d".to_owned(),
        ]
    );
    // A gap row only excuses the binding value it names.
    assert_eq!(
        mismatches(
            &actual,
            &expected,
            &[
                ("result.a[1]", "4"),
                ("result.b", "\"y\""),
                ("result.c.e: extra in binding", "")
            ]
        ),
        vec![
            "result.a[1]\n  binding: 3\n  chrome:  2".to_owned(),
            "stale known gap: result.a[1]".to_owned(),
        ]
    );
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#133): after a manual slot leaves its root, Chrome 156 can
/// still report it as the assignedSlot of the nodes it held. That is Blink's lazy slot-assignment
/// recalc: whether the stale slot is returned depends on the assignment order and on which node is
/// read first (any host mutation or layout clears it), so the binding keeps the spec answer (null).
const SLOT_GAPS: &[(&str, &str)] = &[
    ("result.slotRemoved[1]", "null"),
    ("result.slotRemoved[2]", "null"),
];

#[test]
fn shadow_slot_edges_match_chrome156() {
    check(
        include_str!("shadow_slots_edges_probe.js"),
        include_str!("shadow_slots_edges_chrome156.json"),
        SLOT_GAPS,
    );
}

/// Manual-assignment slots leaving and re-entering a shadow root (miaskiewicz/turbo-dom#133):
/// removing a slot that holds nodes signals slotchange at the removed slot; inserting a slot
/// into a manual root signals nothing (a named root still signals). Expected: Chrome
/// 156.0.8078.47 (frozen, headless).
#[test]
fn manual_slot_reinsertion_signals_like_chrome156() {
    let probe = r###"(async () => {
  // Manual-assignment slots leaving and re-entering their shadow root (turbo-dom#133):
  // removal signals the removed slot itself; re-insertion signals nothing.
  const tick = () => new Promise(resolve => setTimeout(resolve, 0));
  const ids = nodes => nodes.map(node => node.id || node.nodeName);
  const setup = () => {
    const host = document.body.appendChild(document.createElement('div'));
    host.innerHTML = '<b id=b1></b><i id=i1></i>';
    const root = host.attachShadow({ mode: 'open', slotAssignment: 'manual' });
    const s1 = root.appendChild(document.createElement('slot'));
    s1.id = 's1';
    const s2 = root.appendChild(document.createElement('slot'));
    s2.id = 's2';
    const log = [];
    root.addEventListener('slotchange', event => log.push('root:' + event.target.id));
    s1.addEventListener('slotchange', event => log.push('s1:' + event.target.id));
    const [b1, i1] = host.childNodes;
    return { host, root, s1, s2, b1, i1, log };
  };
  const out = {};
  {
    const t = setup();
    t.s1.assign(t.b1); await tick(); t.log.push('|');
    t.s1.remove(); await tick(); t.log.push('|');
    out.removed = ids(t.s1.assignedNodes());
    t.root.append(t.s1); await tick();
    out.readded = [ids(t.s1.assignedNodes()), t.b1.assignedSlot && t.b1.assignedSlot.id, t.log];
  }
  {
    const t = setup();
    t.s1.remove(); t.s1.assign(t.b1); await tick();
    out.assignedDetached = [ids(t.s1.assignedNodes()), t.b1.assignedSlot];
    t.root.append(t.s1); await tick();
    out.insertedAssigned = [ids(t.s1.assignedNodes()), t.b1.assignedSlot && t.b1.assignedSlot.id, t.log];
  }
  {
    const t = setup();
    t.s1.assign(t.b1); await tick();
    t.s1.remove(); await tick();
    t.s2.assign(t.b1); await tick();
    t.root.append(t.s1); await tick();
    out.reassigned = [ids(t.s1.assignedNodes()), ids(t.s2.assignedNodes()), t.b1.assignedSlot && t.b1.assignedSlot.id, t.log];
  }
  {
    const t = setup();
    const wrapper = t.root.appendChild(document.createElement('div'));
    wrapper.append(t.s1);
    t.s1.assign(t.b1, t.i1); await tick();
    wrapper.remove(); await tick();
    t.root.append(wrapper); await tick();
    out.nested = [ids(t.s1.assignedNodes()), t.log];
  }
  {
    const t = setup();
    t.s1.assign(t.b1); await tick();
    const other = document.body.appendChild(document.createElement('div')).attachShadow({ mode: 'open', slotAssignment: 'manual' });
    other.addEventListener('slotchange', event => t.log.push('other:' + event.target.id));
    other.append(t.s1); await tick();
    out.otherRoot = [ids(t.s1.assignedNodes()), t.log];
    t.root.append(t.s1); await tick();
    out.back = [ids(t.s1.assignedNodes()), t.log];
  }
  {
    // Named roots still signal a slot that enters with assigned nodes.
    const host = document.body.appendChild(document.createElement('div'));
    host.innerHTML = '<b></b>';
    const root = host.attachShadow({ mode: 'open' });
    const log = [];
    root.addEventListener('slotchange', event => log.push(event.target.id));
    const slot = document.createElement('slot');
    slot.id = 'named';
    root.append(slot); await tick();
    slot.remove(); await tick();
    root.append(slot); await tick();
    out.named = log;
  }
  return out;
})()
"###;
    let expected: serde_json::Value = serde_json::from_str(
        r###"{
 "removed": [],
 "readded": [
  [
   "b1"
  ],
  "s1",
  [
   "s1:s1",
   "root:s1",
   "|",
   "s1:s1",
   "|"
  ]
 ],
 "assignedDetached": [
  [],
  null
 ],
 "insertedAssigned": [
  [
   "b1"
  ],
  "s1",
  []
 ],
 "reassigned": [
  [],
  [
   "b1"
  ],
  "s2",
  [
   "s1:s1",
   "root:s1",
   "s1:s1",
   "root:s2"
  ]
 ],
 "nested": [
  [
   "b1",
   "i1"
  ],
  [
   "s1:s1",
   "root:s1",
   "s1:s1"
  ]
 ],
 "otherRoot": [
  [],
  [
   "s1:s1",
   "root:s1",
   "s1:s1",
   "other:s1",
   "s1:s1",
   "root:s1"
  ]
 ],
 "back": [
  [
   "b1"
  ],
  [
   "s1:s1",
   "root:s1",
   "s1:s1",
   "other:s1",
   "s1:s1",
   "root:s1"
  ]
 ],
 "named": [
  "named",
  "named"
 ]
}"###,
    )
    .unwrap();
    assert_eq!(differences_of(&run(probe), &expected), Vec::<String>::new());
}

#[test]
fn dom_mutation_validity_matches_chrome156() {
    check(
        include_str!("dom_mutation_validity_probe.js"),
        include_str!("dom_mutation_validity_chrome156.json"),
        &[],
    );
}

/// Document pre-insert validity counts the parent's current children: a single node argument
/// that is already a child still counts as one, and replaceChildren does not first remove the
/// existing children (miaskiewicz/turbo-dom#132). Expected values: Chrome 156.0.8078.47.
#[test]
fn document_pre_insert_validity_counts_current_children() {
    let probe = r#"(async () => {
      const attempt = run => { try { run(); return 'ok'; } catch (e) { return e.name; } };
      const names = p => Array.from(p.childNodes, n => n.nodeName).join(',');
      const cases = {
        append: d => d.append(d.documentElement),
        appendChild: d => d.appendChild(d.documentElement),
        prepend: d => d.prepend(d.documentElement),
        rcHtml: d => d.replaceChildren(d.documentElement),
        rcDoctype: d => d.replaceChildren(d.doctype),
        rcBoth: d => d.replaceChildren(d.doctype, d.documentElement),
        rcFrag: d => { const f = d.createDocumentFragment(); f.append(d.createElement('q')); d.replaceChildren(f); },
        doctypeReplaceWithHtml: d => d.doctype.replaceWith(d.documentElement),
        replaceChildHtmlForDoctype: d => d.replaceChild(d.documentElement, d.doctype),
        insertBeforeDoctype: d => d.insertBefore(d.doctype, d.documentElement),
        htmlReplaceWithSelf: d => d.documentElement.replaceWith(d.documentElement),
        moveBeforeHtml: d => d.moveBefore(d.documentElement, null),
        moveBeforeSelf: d => d.moveBefore(d.documentElement, d.documentElement),
        moveBeforeComment: d => { d.appendChild(d.createComment('c')); d.moveBefore(d.lastChild, d.documentElement); },
        moveBeforePi: d => { d.appendChild(d.createProcessingInstruction('a', 'b')); d.moveBefore(d.lastChild, d.doctype); },
        moveBeforeLone: d => { d.doctype.remove(); d.moveBefore(d.documentElement, null); },
        ibSelf: d => d.insertBefore(d.documentElement, d.documentElement),
        ibDoctypeSelf: d => d.insertBefore(d.doctype, d.doctype),
        rcSelf: d => d.replaceChild(d.documentElement, d.documentElement),
        rcDoctypeSelf: d => d.replaceChild(d.doctype, d.doctype),
        commentSelf: d => { const c = d.appendChild(d.createComment('c')); d.insertBefore(c, c); },
      };
      const out = {};
      for (const [k, f] of Object.entries(cases)) { const d = document.implementation.createHTMLDocument(''); out[k] = [attempt(() => f(d)), names(d)]; }
      return out;
    })()"#;
    let error = |name: &str| serde_json::json!([name, "html,HTML"]);
    assert_eq!(
        run(probe),
        serde_json::json!({
            "append": error("HierarchyRequestError"),
            "appendChild": error("HierarchyRequestError"),
            "prepend": error("HierarchyRequestError"),
            "rcHtml": error("HierarchyRequestError"),
            "rcDoctype": error("HierarchyRequestError"),
            "rcBoth": error("ok"),
            "rcFrag": error("HierarchyRequestError"),
            "doctypeReplaceWithHtml": error("HierarchyRequestError"),
            "replaceChildHtmlForDoctype": error("HierarchyRequestError"),
            "insertBeforeDoctype": error("HierarchyRequestError"),
            "htmlReplaceWithSelf": error("ok"),
            "moveBeforeHtml": error("HierarchyRequestError"),
            "moveBeforeSelf": error("HierarchyRequestError"),
            "moveBeforeComment": ["ok", "html,#comment,HTML"],
            "moveBeforePi": ["ok", "a,html,HTML"],
            "moveBeforeLone": ["HierarchyRequestError", "HTML"],
            "ibSelf": error("HierarchyRequestError"),
            "ibDoctypeSelf": error("HierarchyRequestError"),
            "rcSelf": error("ok"),
            "rcDoctypeSelf": error("ok"),
            "commentSelf": ["ok", "html,HTML,#comment"],
        })
    );
}

#[test]
fn selector_and_at_rule_edges_match_chrome156() {
    check(
        include_str!("selector_edges_probe.js"),
        include_str!("selector_edges_chrome156.json"),
        &[],
    );
}

/// CSSOM at-rule cssText (miaskiewicz/turbo-dom#134): @keyframes key / name / per-line
/// serialization and the declarations a keyframe drops, @font-face feature / variation
/// settings, reserved @counter-style names, @layer statements, grouping-rule bodies and
/// empty descriptor blocks. Expected: Chrome 156.0.8078.47 (frozen, headless).
#[test]
fn at_rule_css_text_matches_chrome156() {
    let probe = r###"(async () => {
  // CSSOM at-rule cssText (turbo-dom#134): each text through a <style> and insertRule.
  const sheet = document.head.appendChild(document.createElement('style')).sheet;
  const read = text => {
    const style = document.head.appendChild(document.createElement('style'));
    style.textContent = text;
    const parsed = style.sheet.cssRules.length ? style.sheet.cssRules[0].cssText : null;
    style.remove();
    let inserted;
    try { sheet.insertRule(text, 0); inserted = sheet.cssRules[0].cssText; sheet.deleteRule(0); } catch (e) { inserted = e.name; }
    return parsed === inserted ? parsed : [parsed, inserted];
  };
  const at = {};
  for (const text of ['@keyframes k { }', '@keyframes k { from { color: red } to { color: blue } }', '@keyframes k { from, to { color: red } 50.0% { color: blue } }',
    '@keyframes "a b" { }', '@keyframes "none" { }', '@keyframes "k" { }', '@keyframes K { 10% { } }', '@-webkit-keyframes w { from { color: red } }',
    '@keyframes k { 0% { color: red; color: blue } }', '@keyframes k { 0% { color: red !important } }', '@keyframes k { 0% { margin: 1px } }',
    '@font-face { font-feature-settings: "liga" on }', '@font-face { font-feature-settings: "liga" off }', '@font-face { font-feature-settings: "liga" 2, "kern" }',
    '@font-face { font-feature-settings: normal }', '@font-face { font-feature-settings: "lig" 1 }', '@font-face { font-feature-settings: "liga" -1 }', '@font-face { font-feature-settings: "liga" 1.5 }',
    '@font-face { font-feature-settings: "liga" 01 }', '@font-face { font-feature-settings: "LIGA" 1 }', "@font-face { font-feature-settings: 'liga' 0 }",
    '@font-face { font-variation-settings: "wght" 400.0, "wdth" -1e2 }', '@font-face { font-variation-settings: "wgh" 1 }', '@font-face { font-variation-settings: "wght" }', '@font-face { font-variation-settings: "wght" 4E2 }',
    '@font-face { font-family: x; font-feature-settings: "liga" 1; src: url(a) }',
    '@counter-style decimal { system: cyclic }', '@counter-style disc { system: cyclic }', '@counter-style NONE { system: cyclic }', '@counter-style inherit { system: cyclic }', '@counter-style default { system: cyclic }', '@counter-style square { system: cyclic }', '@counter-style Decimal { system: cyclic }',
    '@layer a;', '@layer a.b, c;', '@layer  a ,b ;', '@layer;', '@layer a { }', '@layer a { b { color: red } }', '@layer { }',
    '@media screen { }', '@media screen { a { color: red } b { color: blue } }', '@supports (color: red) { }', '@supports (color: red) { a { color: red } }',
    '@container (width > 1px) { a { color: red } }', '@container name (width > 1px) { }', '@scope (a) { }', '@scope { }', '@starting-style { }',
    '@starting-style { a { color: red } b { color: blue } }', '@media screen { @media print { a { color: red } } }',
    '@view-transition { }', '@view-transition { navigation: auto; types: a b }', '@page { }', '@page { margin: 1px }', '@font-face { }', '@counter-style c { }', '@property --q { syntax: "*"; inherits: true }',
    '@font-palette-values --p { font-family: x }', '@position-try --t { top: 1px }',
    "@counter-style circle { system: cyclic }","@counter-style disclosure-open { system: cyclic }","@counter-style unset { system: cyclic }","@counter-style revert-layer { system: cyclic }","@counter-style revert { system: cyclic }","@keyframes k { 12.5% { color: red } 33.3333% { color: red } TO { color: red } 0.0001% { color: red } }","@keyframes k { 1e1% { color: red } }","@layer a , b.c ;","@keyframes k { 0% { animation-name: x; color: red } }","@media screen { @font-face { font-family: x } }","@supports (color: red) { @keyframes q { } }","@keyframes k { 0% { color: red } 0% { color: blue } }","@keyframes \"\" { }","@keyframes \"1x\" { }","@keyframes \"inherit\" { }","@keyframes k { 50% { color: red; } from{color:blue} }","@keyframes k { 0% { color: red } } a { color: red }"]) at[text] = read(text);
  const keyframe = {};
  for (const declaration of ['animation-timing-function: linear', 'animation-composition: add', 'animation-timeline: none', 'animation-duration: 1s', 'animation: x 1s', 'animation-range: normal', 'ANIMATION-DELAY: 1s', 'transition: color 1s', '--x: 1']) keyframe[declaration] = read('@keyframes k { 0% { ' + declaration + '; color: red } }');
  return { at, keyframe };
})()"###;
    let expected: serde_json::Value = serde_json::from_str(
        r###"{
 "at": {
  "@keyframes k { }": "@keyframes k { \n}",
  "@keyframes k { from { color: red } to { color: blue } }": "@keyframes k { \n  0% { color: red; }\n  100% { color: blue; }\n}",
  "@keyframes k { from, to { color: red } 50.0% { color: blue } }": "@keyframes k { \n  0%, 100% { color: red; }\n  50% { color: blue; }\n}",
  "@keyframes \"a b\" { }": "@keyframes a\\ b { \n}",
  "@keyframes \"none\" { }": "@keyframes none { \n}",
  "@keyframes \"k\" { }": "@keyframes k { \n}",
  "@keyframes K { 10% { } }": "@keyframes K { \n  10% { }\n}",
  "@-webkit-keyframes w { from { color: red } }": "@-webkit-keyframes w { \n  0% { color: red; }\n}",
  "@keyframes k { 0% { color: red; color: blue } }": "@keyframes k { \n  0% { color: blue; }\n}",
  "@keyframes k { 0% { color: red !important } }": "@keyframes k { \n  0% { }\n}",
  "@keyframes k { 0% { margin: 1px } }": "@keyframes k { \n  0% { margin: 1px; }\n}",
  "@font-face { font-feature-settings: \"liga\" on }": "@font-face { font-feature-settings: \"liga\"; }",
  "@font-face { font-feature-settings: \"liga\" off }": "@font-face { font-feature-settings: \"liga\" 0; }",
  "@font-face { font-feature-settings: \"liga\" 2, \"kern\" }": "@font-face { font-feature-settings: \"liga\" 2, \"kern\"; }",
  "@font-face { font-feature-settings: normal }": "@font-face { font-feature-settings: normal; }",
  "@font-face { font-feature-settings: \"lig\" 1 }": "@font-face { }",
  "@font-face { font-feature-settings: \"liga\" -1 }": "@font-face { font-feature-settings: \"liga\" -1; }",
  "@font-face { font-feature-settings: \"liga\" 1.5 }": "@font-face { }",
  "@font-face { font-feature-settings: \"liga\" 01 }": "@font-face { font-feature-settings: \"liga\"; }",
  "@font-face { font-feature-settings: \"LIGA\" 1 }": "@font-face { font-feature-settings: \"LIGA\"; }",
  "@font-face { font-feature-settings: 'liga' 0 }": "@font-face { font-feature-settings: \"liga\" 0; }",
  "@font-face { font-variation-settings: \"wght\" 400.0, \"wdth\" -1e2 }": "@font-face { font-variation-settings: \"wght\" 400, \"wdth\" -100; }",
  "@font-face { font-variation-settings: \"wgh\" 1 }": "@font-face { }",
  "@font-face { font-variation-settings: \"wght\" }": "@font-face { font-variation-settings: \"wght\" 1; }",
  "@font-face { font-variation-settings: \"wght\" 4E2 }": "@font-face { font-variation-settings: \"wght\" 400; }",
  "@font-face { font-family: x; font-feature-settings: \"liga\" 1; src: url(a) }": "@font-face { font-family: x; font-feature-settings: \"liga\"; src: url(\"a\"); }",
  "@counter-style decimal { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style disc { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style NONE { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style inherit { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style default { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style square { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style Decimal { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@layer a;": "@layer a;",
  "@layer a.b, c;": "@layer a.b, c;",
  "@layer  a ,b ;": "@layer a, b;",
  "@layer;": [
   null,
   "SyntaxError"
  ],
  "@layer a { }": "@layer a {\n}",
  "@layer a { b { color: red } }": "@layer a {\n  b { color: red; }\n}",
  "@layer { }": "@layer {\n}",
  "@media screen { }": "@media screen {\n}",
  "@media screen { a { color: red } b { color: blue } }": "@media screen {\n  a { color: red; }\n  b { color: blue; }\n}",
  "@supports (color: red) { }": "@supports (color: red) {\n}",
  "@supports (color: red) { a { color: red } }": "@supports (color: red) {\n  a { color: red; }\n}",
  "@container (width > 1px) { a { color: red } }": "@container (width > 1px) {\n  a { color: red; }\n}",
  "@container name (width > 1px) { }": "@container name (width > 1px) {\n}",
  "@scope (a) { }": "@scope (a) {\n}",
  "@scope { }": "@scope {\n}",
  "@starting-style { }": "@starting-style {\n}",
  "@starting-style { a { color: red } b { color: blue } }": "@starting-style {\n  a { color: red; }\n  b { color: blue; }\n}",
  "@media screen { @media print { a { color: red } } }": "@media screen {\n  @media print {\n  a { color: red; }\n}\n}",
  "@view-transition { }": "@view-transition { }",
  "@view-transition { navigation: auto; types: a b }": "@view-transition { navigation: auto; types: a b; }",
  "@page { }": "@page { }",
  "@page { margin: 1px }": "@page { margin: 1px; }",
  "@font-face { }": "@font-face { }",
  "@counter-style c { }": "@counter-style c { }",
  "@property --q { syntax: \"*\"; inherits: true }": "@property --q { syntax: \"*\"; inherits: true; }",
  "@font-palette-values --p { font-family: x }": "@font-palette-values --p { font-family: x; }",
  "@position-try --t { top: 1px }": "@position-try --t { top: 1px; }",
  "@counter-style circle { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style disclosure-open { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style unset { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style revert-layer { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@counter-style revert { system: cyclic }": [
   null,
   "SyntaxError"
  ],
  "@keyframes k { 12.5% { color: red } 33.3333% { color: red } TO { color: red } 0.0001% { color: red } }": "@keyframes k { \n  12.5% { color: red; }\n  33.3333% { color: red; }\n  100% { color: red; }\n  0.0001% { color: red; }\n}",
  "@keyframes k { 1e1% { color: red } }": "@keyframes k { \n  10% { color: red; }\n}",
  "@layer a , b.c ;": "@layer a, b.c;",
  "@keyframes k { 0% { animation-name: x; color: red } }": "@keyframes k { \n  0% { color: red; }\n}",
  "@media screen { @font-face { font-family: x } }": "@media screen {\n  @font-face { font-family: x; }\n}",
  "@supports (color: red) { @keyframes q { } }": "@supports (color: red) {\n  @keyframes q { \n}\n}",
  "@keyframes k { 0% { color: red } 0% { color: blue } }": "@keyframes k { \n  0% { color: red; }\n  0% { color: blue; }\n}",
  "@keyframes \"\" { }": [
   null,
   "SyntaxError"
  ],
  "@keyframes \"1x\" { }": "@keyframes \\31 x { \n}",
  "@keyframes \"inherit\" { }": "@keyframes inherit { \n}",
  "@keyframes k { 50% { color: red; } from{color:blue} }": "@keyframes k { \n  50% { color: red; }\n  0% { color: blue; }\n}",
  "@keyframes k { 0% { color: red } } a { color: red }": [
   "@keyframes k { \n  0% { color: red; }\n}",
   "SyntaxError"
  ]
 },
 "keyframe": {
  "animation-timing-function: linear": "@keyframes k { \n  0% { animation-timing-function: linear; color: red; }\n}",
  "animation-composition: add": "@keyframes k { \n  0% { animation-composition: add; color: red; }\n}",
  "animation-timeline: none": "@keyframes k { \n  0% { animation-timeline: none; color: red; }\n}",
  "animation-duration: 1s": "@keyframes k { \n  0% { color: red; }\n}",
  "animation: x 1s": "@keyframes k { \n  0% { color: red; }\n}",
  "animation-range: normal": "@keyframes k { \n  0% { color: red; }\n}",
  "ANIMATION-DELAY: 1s": "@keyframes k { \n  0% { color: red; }\n}",
  "transition: color 1s": "@keyframes k { \n  0% { transition: color 1s; color: red; }\n}",
  "--x: 1": "@keyframes k { \n  0% { --x: 1; color: red; }\n}"
 }
}"###,
    )
    .unwrap();
    let actual = run(probe);
    assert_eq!(differences_of(&actual, &expected), Vec::<String>::new());
}

fn differences_of(actual: &serde_json::Value, expected: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    differences("result", actual, expected, &mut out);
    out
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#135): `<?target data?>` parses as a bogus comment.
const SANITIZER_GAPS: &[(&str, &str)] = &[
    (
        "result.parseHTML[2][0]",
        "\"<div id=\\\"h\\\"><span>light</span></div><!--c--><!--?pi x?-->\"",
    ),
    (
        "result.setHTMLComments[0]",
        "\"<div id=\\\"h\\\"><span>light</span></div><!--c--><!--?pi x?-->\"",
    ),
    (
        "result.setHTMLPi[0]",
        "\"<div id=\\\"h\\\"><span>light</span></div>\"",
    ),
    (
        "result.setHTMLUnsafe[0]",
        "\"<div id=\\\"h\\\"><span onclick=\\\"y\\\">light</span></div><!--c--><!--?pi x?-->\"",
    ),
];

#[test]
fn sanitizer_edges_match_chrome156() {
    check(
        include_str!("sanitizer_edges_probe.js"),
        include_str!("sanitizer_edges_chrome156.json"),
        SANITIZER_GAPS,
    );
}

#[test]
fn document_edges_match_chrome156() {
    check(
        include_str!("document_edges_probe.js"),
        include_str!("document_edges_chrome156.json"),
        &[],
    );
}

/// Chrome 156 admits every identifier as a view transition type in querySelector (as in
/// stylesheets); none of them matches outside an active transition.
#[test]
fn active_view_transition_type_admits_keyword_types() {
    let probe = r#"(async () => ['none', 'NONE', 'initial', 'inherit', 'unset', 'default', 'revert', 'revert-layer', 'auto'].map(type => {
      const selector = `:active-view-transition-type(${type})`;
      try { return [document.querySelectorAll(selector).length, document.documentElement.matches(selector)]; } catch (e) { return e.name; }
    }))()"#;
    assert_eq!(
        run(probe),
        serde_json::Value::Array(vec![serde_json::json!([0, false]); 9])
    );
}

/// DOM adopt: a shadow root throws HierarchyRequestError (a document NotSupportedError).
#[test]
fn adopting_a_shadow_root_throws_hierarchy_request_error() {
    let probe = r#"(async () => [document.createElement('div').attachShadow({ mode: 'open' }), document.implementation.createHTMLDocument('')].map(node => {
      try { document.adoptNode(node); return 'ok'; } catch (e) { return [e.name, e instanceof DOMException]; }
    }))()"#;
    assert_eq!(
        run(probe),
        serde_json::json!([["HierarchyRequestError", true], ["NotSupportedError", true]])
    );
}

/// KNOWN_GAP (miaskiewicz/turbo-dom#143): Text.prototype.wholeText is missing.
const RECEIVER_GAPS: &[(&str, &str)] = &[("result.Text.wholeText", "\"missing\"")];

#[test]
fn native_member_receivers_match_chrome156() {
    check(
        include_str!("receivers_edges_probe.js"),
        include_str!("receivers_edges_chrome156.json"),
        RECEIVER_GAPS,
    );
}

#[test]
fn event_member_receivers_and_listener_signals_match_chrome156() {
    check(
        include_str!("event_receivers_edges_probe.js"),
        include_str!("event_receivers_edges_chrome156.json"),
        &[],
    );
}

/// AddEventListenerOptions.signal is a non-nullable AbortSignal.
#[test]
fn add_event_listener_rejects_a_null_signal() {
    let probe = r#"(async () => [new EventTarget(), document.createElement('p')].map(target => {
      try { target.addEventListener('a', () => {}, { signal: null }); return 'ok'; } catch (e) { return e.name; }
    }))()"#;
    assert_eq!(run(probe), serde_json::json!(["TypeError", "TypeError"]));
}

/// miaskiewicz/turbo-dom#140: one DOM call queueing thousands of custom element reactions
/// and mutation records runs to completion (Chrome 156 has no per-call reaction cap).
#[test]
#[ignore = "turbo-dom#140: fails until the fix"]

fn custom_element_reaction_volume_matches_chrome156() {
    check(
        include_str!("reaction_volume_probe.js"),
        include_str!("reaction_volume_chrome156.json"),
        &[],
    );
}

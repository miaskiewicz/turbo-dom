// Chrome-156 control for turbo-dom's selector engine (consolidation S3a, miaskiewicz/turbo-dom#18).
//
// Records, for every case below, what Chrome for Testing 156 says about each selector:
//   - `valid`: whether `document.querySelector(sel)` accepts it (no SyntaxError),
//   - `matches`: `el.matches(sel)` for every element (document order, template content
//     excluded — it is a separate fragment in Chrome), as a 0/1 string,
//   - `qsa`: `document.querySelectorAll(sel)` as indices into that element list,
//   - `winner`: getComputedStyle(el).order for every element (a 0/1 string) when the case
//     document gets one extra `<style>SEL { order: 1 }</style>` as the last child of
//     <head> (the sentinel; 0 = the rule did not apply),
//   - `sheet_valid`: whether that sentinel sheet kept its rule (stylesheet validity, which
//     differs from `valid` for e.g. `::before:hover`).
// Plus `cascade_cases` (consolidation S3b, one per quirk class of the old html2pdf selector
// engine): each case's sheet is appended to <head> as one <style>, and `winners` records
// getComputedStyle(el).order for every element (the winning declaration of a cascade where
// several rules compete — validity, specificity, matching; P5b adds the declaration-order
// cases of miaskiewicz/turbo-dom#21).
// Plus `probes` (P5b): scripted DOM checks (template content traversal, `:root` outside the
// document element) whose named results the Rust test reproduces through the Tree API.
// Asserted by crates/turbo-dom/tests/cascade_selectors_chrome.rs (matches, cascade
// matching and specificity; cascade cases also through rtdom::cascade, P5b-X2), by
// test/selectors-chrome.test.mjs (the npm engine) and end to end through the layout HTML
// path by crates/turbo-dom-layout/tests/cascade_selectors_chrome.rs.
//
// Usage: node scripts/cascade-selectors-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156
//            (~/Library/Caches/ms-playwright/chromium-1248/...);
//   --out    defaults to crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

// `bug` names the miaskiewicz/turbo-dom#18 entry a case covers (the S3a bug list), or
// "guard" for nearby behaviour that must NOT change.
const DOC = (body, head = "") => `<!DOCTYPE html><html><head>${head}</head><body>${body}</body></html>`;
const CASES = [
  {
    id: "escapes",
    bug: "escapes",
    html: DOC(
      `<div class="md:flex" id="a"><p class="a.b">x</p></div><div id="123" class="10 @sm"><span class="md">y</span></div>`,
    ),
    selectors: [
      ".md\\:flex",
      "div.md\\:flex",
      "#\\61",
      "\\70",
      "\\64 iv",
      ".a\\.b",
      "#\\31 23",
      ".\\31 0",
      ".\\@sm",
      ".md\\:flex > \\70",
      ":is(.md\\:flex) p",
      ":not(.md\\:flex)",
      "[class=\"md:flex\"]",
      "[class=md\\:flex]",
      // guards
      ".md",
      "#a",
      "p",
      "div > p",
    ],
  },
  {
    id: "non-ascii",
    bug: "non-ascii",
    html: DOC(
      `<div class="café"><span class="日本 x">a</span></div><p id="ñ" class="caf">b</p><日本>c</日本>`,
    ),
    selectors: [".café", ".日本", "#ñ", "日本", "div.café > span.日本", ".日本.x", ":not(.café)", ".caf", "[class=café]"],
  },
  {
    id: "namespace",
    bug: "namespace",
    html: DOC(`<p class="x">a</p><svg><rect></rect></svg><div><p>b</p></div>`),
    selectors: ["*|p", "*|*", "*|p.x", "div > *|p", "*|rect", "|p", "svg|rect", ":is(*|p)", "p", "*"],
  },
  {
    id: "empty",
    bug: "empty",
    html: DOC(
      `<div id="c"><!--comment--></div><div id="s"> </div><div id="t">x</div><div id="m"><!--a-->x</div>` +
        `<div id="e"></div><div id="k"><span></span></div><template id="tp"></template><template id="tq"><p>x</p></template>` +
        `<div id="b"><?pi x?></div>`,
    ),
    selectors: [":empty", "div:empty", "template:empty", "#c:empty", "#s:empty", "#tq:empty", ":not(:empty)", "span:empty"],
  },
  {
    id: "read-only-read-write",
    bug: "read-only-read-write",
    html: DOC(
      `<div id="d">d</div><p>p</p>` +
        `<input id="text"><input id="ro" readonly><input id="dis" disabled><input id="disro" disabled readonly>` +
        `<input id="search" type="search"><input id="email" type="email"><input id="num" type="number"><input id="date" type="date">` +
        `<input id="pw" type="password"><input id="bogus" type="bogus">` +
        `<input id="cb" type="checkbox"><input id="radio" type="radio"><input id="hidden" type="hidden"><input id="range" type="range">` +
        `<input id="color" type="color"><input id="file" type="file"><input id="submit" type="submit"><input id="btn" type="button">` +
        `<textarea id="ta"></textarea><textarea id="taro" readonly></textarea><textarea id="tadis" disabled></textarea>` +
        `<select id="sel"><option>o</option></select><button id="b">b</button>` +
        `<fieldset disabled><input id="fsin"><legend><input id="legin"></legend></fieldset>` +
        `<fieldset disabled><legend><input id="firstlegin"></legend><input id="fsin2"></fieldset>` +
        `<div id="ce" contenteditable><span id="cechild">x</span><input id="cein" type="checkbox"></div>` +
        `<div id="cef" contenteditable="false">x</div><div contenteditable="true"><b id="cenest" contenteditable="false">x</b></div>` +
        `<div id="cepo" contenteditable="plaintext-only">x</div><div contenteditable><svg id="cesvg"><rect></rect></svg><select id="cesel"></select></div>` +
        `<svg id="svg"><rect></rect></svg><math><mi>x</mi></math>`,
    ),
    selectors: [":read-only", ":read-write", "div:read-only", "p:read-write", "input:read-write", "input:read-only", "textarea:read-write", "select:read-only", "button:read-only", "span:read-write", ":not(:read-write)"],
  },
  {
    id: "nth-of-s",
    bug: "nth-of-s",
    html: DOC(
      `<ul><li id="l1" class="x">1</li><li id="l2">2</li><li id="l3" class="x">3</li><li id="l4" class="x y">4</li><p id="l5" class="x">5</p></ul>`,
    ),
    selectors: [
      ":nth-child(2 of .x)",
      "li:nth-child(odd of .x)",
      ":nth-child(1 of li.y)",
      ":nth-last-child(1 of .x)",
      ":nth-last-child(2 of li)",
      ":nth-child(1 of p, .y)",
      ":nth-child(2n+1 of li)",
      ":nth-child(-n+2 of .x)",
      ":nth-child(2 of.x)",
      "li:not(:nth-child(1 of .x))",
      // guards
      ":nth-child(2)",
      "li:nth-child(2n+1)",
      ":nth-last-child(2)",
      "li:nth-of-type(odd)",
    ],
  },
  {
    id: "link",
    bug: "link",
    html: DOC(
      `<a id="a1" href="/x">a</a><a id="a2">no href</a><a id="a3" href="">empty</a>` +
        `<map name="m"><area id="ar" href="/y"><area id="ar2"></map><span id="s" href="/z">span</span>` +
        `<svg><a id="sa" href="/x"><rect></rect></a><a id="sx" xlink:href="/y"></a><a id="sn"></a></svg>`,
      `<link id="lk" rel="stylesheet" href="data:text/css,">`,
    ),
    selectors: [":link", ":any-link", "a:link", "area:any-link", "link:link", ":link:not(a)", ":visited", "a:not(:link)", "span:link", "a[href]"],
  },
  {
    id: "guards",
    bug: "guard",
    html: DOC(
      `<div class="a b" id="g"><p class="x">1</p><p>2</p><input type="checkbox" checked><input required><input disabled></div>` +
        `<section><h2>t</h2><p>after</p></section>`,
    ),
    selectors: [
      ".a.b",
      "#g > p",
      "p:first-child",
      "p:not(.x)",
      ":is(.x, h2)",
      "[type=\"checkbox\"]",
      ":checked",
      ":disabled",
      "input:enabled",
      ":required",
      "input:optional",
      "div:has(> p.x)",
      "h2 + p",
      "h2 ~ p",
      ":root",
      "p:hover",
      "p:empty",
      "*",
    ],
  },
  // P5b (miaskiewicz/turbo-dom#18 remainder): the pseudo-classes S3a left unevaluated, the
  // malformed selectors `_checked` accepted, rtdom::cascade's comma split and the
  // fieldset `:disabled` rule.
  {
    id: "lang",
    bug: "lang",
    html: DOC(
      `<div lang="en-US"><p id="a">x</p><span lang="" id="e">e</span></div><p lang="de-CH-1996" id="b">y</p>` +
        `<p lang="EN" id="c">z</p><p id="n">n</p><p lang="zh-Hant-TW" id="z">z</p><p xml:lang="ja" id="x">j</p>` +
        `<svg><g xml:lang="fr-CA" id="s"></g><g lang="fr" id="sl"></g></svg>`,
    ),
    selectors: [
      ":lang(en)",
      ":lang(EN-us)",
      ":lang(en-GB)",
      ":lang(e)",
      ":lang(de)",
      ":lang(de-CH)",
      ":lang(\"de\")",
      ":lang(en, de)",
      ":lang(zh-TW)",
      ":lang(\"*-CH\")",
      ":lang(\\*-CH)",
      ":lang(fr)",
      ":lang(ja)",
      ":lang(\"\")",
      ":lang()",
      ":lang(1)",
      ":lang(en,)",
      "p:not(:lang(en))",
      ":lang(en) > p",
      "span:lang(en)",
      ":LANG(en)",
    ],
  },
  {
    id: "dir",
    bug: "dir",
    html: DOC(
      `<div id="l"><p id="p1">a</p></div><div dir="rtl" id="r"><p id="p2">b</p><span dir="ltr" id="s1">c</span><span dir="bogus" id="s2">d</span></div>` +
        `<div dir="auto" id="a1">שלום abc</div><div dir="auto" id="a2">abc שלום</div><div dir="auto" id="a3">123</div>` +
        `<div dir="auto" id="a4"><span> </span><b>مرحبا</b></div><div dir="auto" id="a5"><bdi>שלום</bdi>abc</div>` +
        `<bdi id="b1">שלום</bdi><bdi id="b2">abc</bdi><div dir="AUTO" id="a6">ש</div><div dir="RTL" id="r2"><svg><g id="g"></g></svg></div>` +
        `<input dir="auto" id="i1" value="שלום"><input dir="auto" id="i2"><textarea dir="auto" id="t1">שלום</textarea>` +
        `<div dir="auto" id="a7"><script>ש</script>x</div><div dir="auto" id="a8"><span dir="ltr">ש</span>abc</div>` +
        `<div dir="rtl"><div dir="auto" id="a9">123</div><input type="tel" id="tel"><input dir="bogus" id="ib"></div>` +
        `<div dir="auto" id="a10"><span dir="bogus">ש</span>a</div><div dir="auto" id="a11"><textarea>ש</textarea>a</div>`,
    ),
    selectors: [
      ":dir(ltr)",
      ":dir(rtl)",
      ":dir(LTR)",
      ":dir(auto)",
      ":dir(foo)",
      ":dir()",
      ":dir(ltr, rtl)",
      ":dir(\"ltr\")",
      "p:dir(rtl)",
      ":not(:dir(ltr))",
      "div:dir(rtl) > span",
    ],
  },
  {
    id: "defined",
    bug: "defined",
    html: DOC(
      `<x-foo id="x"></x-foo><div id="d"></div><button is="x-b" id="bi">b</button><font-face id="ff"></font-face>` +
        `<svg><x-bar id="sx"></x-bar></svg><X-UP id="u"></X-UP><annotation-xml id="ax"></annotation-xml><p is="" id="pe">p</p>`,
    ),
    selectors: [":defined", ":not(:defined)", "x-foo:defined", "button:defined", "div:defined", "*:defined", ":DEFINED", ":defined()"],
  },
  {
    // P5b-X6 (miaskiewicz/turbo-dom#58): the fullscreen pseudo-classes; a static document has
    // no fullscreen element, so they are valid and match nothing.
    id: "fullscreen",
    bug: "fullscreen",
    html: DOC(`<div id="a"><p id="p">p</p></div>`),
    selectors: [":fullscreen", ":-webkit-full-screen", ":-webkit-full-screen-ancestor", ":-webkit-full-screen-document", ":FULLSCREEN", ":-WEBKIT-FULL-SCREEN-ANCESTOR", ":fullscreen()", "div:not(:fullscreen)", ":-webkit-full-screen-ancestor p", ":is(:fullscreen, p)", "p:not(:-webkit-full-screen-ancestor)"],
  },
  {
    id: "template",
    bug: "template",
    html: DOC(
      `<template id="t"><p class="t" id="in">x</p><div><span>s</span></div></template><p class="t" id="out">y</p>` +
        `<div id="w"><template><b id="deep">d</b></template></div>`,
    ),
    selectors: [".t", "p", "#in", "#deep", "b", "template p", "template > p", "span", "template:has(p)", ":has(> .t)", "div span", "div b", "template *"],
  },
  {
    id: "malformed",
    bug: "malformed",
    html: DOC(`<div id="a" class="x" a="1"><p class="x">p</p></div><select><option selected>o</option></select>`),
    selectors: [
      "p:nth-child(foo)",
      "[a]div",
      "::before.x",
      ".x*",
      ":not(p)x",
      "p:nth-child(2n+)",
      "p:nth-child(n-)",
      ":nth-child(+ 2n)",
      ":nth-child(- n+1)",
      ":nth-child(2 n)",
      ":nth-child(2.5n)",
      ":nth-child(1 of)",
      ":nth-child(of p)",
      ":nth-of-type(1 of p)",
      ":first-child(1)",
      ":bogus",
      "::bogus",
      "p..x",
      ".",
      "#",
      "p:",
      "p >",
      "",
      "::before:first-child",
      "::before span",
      ":not(::before)",
      ":nth-child(2n + 1)",
      ":nth-child( -n+3 )",
      ":nth-child(2n- 1)",
      ":nth-child(2n -1)",
      ":nth-child(2N+1)",
      ":nth-child(+n)",
      ":nth-child(-n)",
      ":nth-child(+5)",
      ":nth-child(ODD)",
      ":nth-last-of-type(n)",
      "::before",
      "p::after",
      "::before:hover",
      "::before:enabled",
      "p::marker:hover",
      "::placeholder:hover",
      "::file-selector-button:hover",
      "::selection:window-inactive",
      "::selection:hover",
      "::part(x):hover",
      "::part(x):first-child",
      "::slotted(p):hover",
      "::-webkit-scrollbar:hover",
      "::-webkit-scrollbar-thumb:horizontal",
      "::-webkit-scrollbar:disabled",
      "::-webkit-scrollbar:first-child",
      "::-webkit-scrollbar.x",
      "::-webkit-scrollbar:window-inactive",
      "::details-content:hover",
      "::picker(select):hover",
      "::backdrop:hover",
      "::scroll-marker:hover",
      "::part(x):focus-visible",
      "::part(x):active",
      "::part(x):enabled",
      "::file-selector-button:active",
      "::file-selector-button:focus",
      "p::-webkit-scrollbar",
      "::slotted(p)",
      "::part(x)",
      ":host",
      ":is(::before)",
      "option:selected",
      "#a#a",
      "*|*",
      // pseudo-class names are ASCII case-insensitive
      "P:FIRST-CHILD",
      "DIV:NOT(.y)",
      ":Nth-Child(ODD)",
      ":NTH-CHILD(2N OF .X)",
      ":Is(p)",
      "OPTION:CHECKED",
      ":EMPTY",
      // attribute selector grammar (unquoted values are identifiers)
      "[href=index.html]",
      "[a=]",
      "[x=\"a\"b]",
      "[a=1]",
      "[a=\"1\"]",
      "[a=-x]",
      "[a=\"1\" i]",
      "[a=b S]",
      "[a=\"b\" x]",
      "[*|a]",
      "[|a]",
      "[ns|a]",
      "[a|=b]",
      "[ a ]",
      "[a =\"1\"]",
      "[1a]",
      "[a='1'][class]",
      "[a=b c]",
      "[a=b s]",
      "[a=b I]",
      "[a=\"1\" s]",
      "[class=X i]",
      "[A]",
      "[a=\\31]",
      // :has() nesting
      ":has(:has(p))",
      "div:has(> p:has(x))",
      ":not(:has(p))",
      ":has(::before)",
      ":is(:has(p))",
      ":has(:is(:has(p)))",
      ":has(:is(p, :has(p)))",
      "div:has(:not(:has(p)))",
    ],
  },
  {
    id: "cascade-comma",
    bug: "cascade-comma",
    html: DOC(
      `<p class="a" id="pa">a</p><p class="b" id="pb">b</p><p class="c" title="a,b" id="pc">c</p>` +
        `<ul><li class="a">1</li><li class="b">2</li><li class="a">3</li></ul><div class="a"><span>s</span></div>`,
    ),
    selectors: [
      ":is(.a, .b)",
      ":not(.a, .b)",
      ":nth-child(2 of .a, .b)",
      "p, .x",
      ":where(.a, .b) span",
      "[title=\"a,b\"]",
      "li:is(.b, .c), p.c",
      ":has(> span, > em)",
      "p:not(.a, .c), li.b",
      ".a, :bogus",
    ],
  },
  {
    id: "disabled-fieldset",
    bug: "disabled-fieldset",
    html: DOC(
      `<fieldset disabled id="f1"><legend id="lg1"><input id="i1"></legend><input id="i2"><fieldset id="f2"><input id="i3"></fieldset>` +
        `<legend><input id="i4"></legend><button id="b1">b</button><select id="s1"><option id="o1">o</option></select>` +
        `<textarea id="t1"></textarea><div id="d1" disabled></div><output id="out1">o</output></fieldset>` +
        `<fieldset><legend disabled><input id="i5"></legend></fieldset>` +
        `<select><optgroup disabled id="og"><option id="o2">x</option></optgroup><option disabled id="o3">y</option></select>` +
        `<div disabled id="d2"></div><a disabled id="a1">a</a><input disabled id="i6">` +
        `<fieldset disabled><div><legend><input id="i7"></legend></div></fieldset><fieldset disabled><p>t</p><legend><input id="i8"></legend></fieldset>` +
        `<link disabled id="l1"><svg><g disabled id="sg"></g></svg>`,
    ),
    selectors: [":disabled", ":enabled", "input:disabled", "fieldset:disabled", ":not(:disabled)", "option:disabled", "legend :disabled", "input:enabled", "div:disabled", "button:enabled"],
  },
];


// S3b cascade cases: `class` names the old html2pdf engine's quirk class (docs/selector-engine.md,
// "What S3b changed") whose Chrome behaviour the case pins.
const CASCADE_CASES = [
  {
    id: "unknown-pseudo-class",
    class: "unknown-pseudo",
    html: DOC(`<p id="a" class="x">a</p><p id="b">b</p><a id="c" href="#">c</a><div class="y z">d</div><span class="w">e</span>`),
    css:
      "p { order: 1 } p:totallyunknown, .x { order: 2 } a:hover, #c { order: 3 } P:FIRST-CHILD { order: 4 } " +
      "div::-moz-selection, .y { order: 5 } .z::-webkit-scrollbar, .z:not(.q) { order: 6 } " +
      ":matches(span), .w { order: 7 } span:-moz-any(.w) { order: 8 } span:selected, .w { order: 9 }",
  },
  {
    id: "unsupported-pseudo-class",
    class: "unknown-pseudo",
    html: DOC(
      `<div id="g"><p class="x">1</p><p>2</p></div><div id="h"><span>3</span></div>` +
        `<input id="r" required><input id="o"><textarea id="t" readonly></textarea><textarea id="u"></textarea>`,
    ),
    css:
      "div:has(> p.x) { order: 1 } p:nth-last-child(1) { order: 2 } :required { order: 3 } input:optional { order: 4 } " +
      "textarea:read-only { order: 5 } textarea:read-write { order: 6 } p:nth-last-of-type(2) { order: 7 }",
  },
  {
    id: "attribute-namespace",
    class: "attribute-namespace",
    html: DOC(
      `<a id="h" href="#x">h</a><svg><a id="s" xlink:href="#y"><text>s</text></a><a id="p" href="#z"><text>p</text></a>` +
        `<use id="u" xlink:href="#u"></use></svg>`,
    ),
    css: `[href] { order: 1 } a[href^="#"] { order: 2 }`,
  },
  {
    id: "specificity-where",
    class: "specificity",
    html: DOC(`<p id="a" class="c">a</p><p id="b" class="c d">b</p>`),
    css: "p:where(.c) { order: 1 } p { order: 2 } .d:where(#b) { order: 3 } p.d { order: 4 }",
  },
  {
    id: "specificity-is-not-has",
    class: "specificity",
    html: DOC(`<p id="a" class="c d">a</p><p id="b" class="c">b</p><div id="g" class="k"><span class="s">x</span></div>`),
    css:
      ":is(#a, .zz) { order: 1 } p.c.d { order: 2 } p:not(#z) { order: 3 } p.c { order: 4 } " +
      "div:has(> #nope, .s) { order: 5 } div.k { order: 6 } div:not(.a.b.c) { order: 7 } div.k { order: 8 }",
  },
  {
    id: "specificity-pseudo-element",
    class: "specificity",
    html: DOC(`<p class="c">a</p><div class="bar">b</div>`),
    css:
      "p { order: 1 } .c::after { order: 2 } .c:before { order: 3 } .bar::first-line { order: 4 } div { order: 5 } " +
      ".bar::-webkit-scrollbar { order: 6 } .bar::-webkit-scrollbar-thumb:hover { order: 7 } .c::marker { order: 8 }",
  },
  // P5b-X2 (miaskiewicz/turbo-dom#24 item 2): selectors whose Selectors 4 specificity
  // differs from a per-token count — `:nth-child(… of S)`, `:not()` / `:is()` / `:has()`
  // taking their most specific argument, escaped `:` in a class, `#` inside an attribute
  // value, a type selector inside a pseudo-class argument. Each pair's later rule wins only
  // under spec specificity.
  {
    id: "specificity-arguments",
    class: "specificity",
    html: DOC(
      `<div id="w" class="k"><p id="a" class="c d">a</p><p id="b" class="c" title="#t">b</p>` +
        `<span id="s" class="md:x e">s</span><p id="q" class="c k">q</p><i id="t">t</i></div>`,
    ),
    css:
      ":nth-child(1 of #a, .c) { order: 1 } #a.c { order: 2 } " +
      ".md\\:x { order: 3 } span.e { order: 4 } " +
      'p[title="#t"] { order: 5 } p[title] { order: 6 } ' +
      "#q:not(.zz) { order: 7 } #q.k { order: 8 } " +
      "div :is(i) { order: 9 } div > i { order: 10 } " +
      "div:has(> #a) { order: 11 } #w.k { order: 12 }",
  },
  {
    id: "cascade-only-pseudos",
    class: "unknown-pseudo",
    html: DOC(
      `<div lang="en-US"><p id="a" class="c">a</p><x-foo id="x">x</x-foo></div><p id="b" lang="de">b</p>` +
        `<span id="s" class="c">s</span>`,
    ),
    css:
      ":lang(en) { order: 1 } :lang(de) { order: 2 } x-foo:defined { order: 3 } x-foo:not(:defined) { order: 4 } " +
      ":scope span { order: 5 } :-webkit-any(#s) { order: 6 } span.c { order: 7 }",
  },
  // P5b (miaskiewicz/turbo-dom#21): declarations are ordered by origin, importance,
  // specificity, then the RULE's source order (never the declaration's index in its rule).
  {
    id: "declaration-order",
    class: "cascade-order",
    html: DOC(`<p id="a">a</p><p id="b" class="k">b</p><div id="d">d</div><span id="s">s</span>`),
    css:
      "p { margin-top: 1px; margin-bottom: 1px; order: 1 } p { order: 2 } " +
      ".k { color: red; margin: 0; padding: 0; order: 3 } p.k { order: 4 } " +
      "div { color: red; margin: 0; padding: 0; border: 0; order: 5 } div { order: 6 } div { color: blue } " +
      "span { order: 7; order: 8 } span { margin: 0 }",
  },
  {
    id: "importance",
    class: "cascade-order",
    html: DOC(`<p id="a">a</p><p id="b" class="k">b</p><div id="d">d</div>`),
    css:
      "p { order: 6 !important; color: red } p { order: 7 } #b { order: 8 } p.k { margin: 0; padding: 0; order: 9 !important } " +
      "p { color: blue; margin: 0; order: 10 !important } div { margin: 0; padding: 0; order: 11 !important } div { order: 12 }",
  },
  {
    id: "inline-importance",
    class: "cascade-order",
    // the minimal cascade of crates/turbo-dom/tests/cascade_selectors_chrome.rs has no
    // inline-style origin: asserted end to end by the layout test only
    layout_only: true,
    html: DOC(`<p id="i" style="order: 13">a</p><p id="j" style="color: red; order: 14 !important">b</p><p id="k" style="order: 15">c</p>`),
    css: "#i { order: 16 } p { order: 17 !important; color: blue } #j { order: 18 !important } #k { margin: 0; padding: 0; order: 19 }",
  },
];

// P5b (miaskiewicz/turbo-dom#18): scripted DOM probes for what a per-selector row cannot
// express — template content traversal and `:root` outside the document element. Each
// probe returns named values; crates/turbo-dom/tests/cascade_selectors_chrome.rs rebuilds
// the same nodes through the Tree API (one arm per probe id).
const PROBES = [
  {
    id: "template-traversal",
    bug: "template",
    html: DOC(`<template id="t"><p class="t" id="in">x</p><div><span>s</span></div></template><p class="t" id="out">y</p>`),
    script: () => {
      const t = document.getElementById("t");
      const inner = t.content.querySelector("p");
      return {
        by_id_in_content: document.getElementById("in") === null ? "null" : "found",
        by_tag_span: document.getElementsByTagName("span").length,
        by_tag_p: document.getElementsByTagName("p").length,
        qsa_from_template: t.querySelectorAll("p").length,
        qsa_from_content: t.content.querySelectorAll("p").length,
        content_p_matches_template_p: inner.matches("template p"),
        content_p_matches_root: inner.matches(":root"),
        content_p_matches_first_child: inner.matches(":first-child"),
        content_span_matches_div_span: t.content.querySelector("span").matches("div span"),
        template_matches_has_p: t.matches(":has(p)"),
        template_matches_empty: t.matches(":empty"),
      };
    },
  },
  {
    id: "root",
    bug: "root",
    html: DOC(`<div id="host"></div><p id="p">p</p>`),
    script: () => {
      const detached = document.createElement("div");
      detached.append(document.createElement("span"));
      const fragment = document.createDocumentFragment();
      fragment.append(document.createElement("p"));
      const shadow = document.getElementById("host").attachShadow({ mode: "open" });
      shadow.innerHTML = "<p>s</p>";
      const template = document.createElement("template");
      template.innerHTML = "<p>t</p>";
      return {
        document_element: document.documentElement.matches(":root"),
        body: document.body.matches(":root"),
        qsa_root: document.querySelectorAll(":root").length,
        detached: detached.matches(":root"),
        detached_child: detached.firstChild.matches(":root"),
        fragment_child: fragment.firstChild.matches(":root"),
        shadow_child: shadow.firstChild.matches(":root"),
        template_content_child: template.content.firstChild.matches(":root"),
        fragment_qsa_root: fragment.querySelectorAll(":root").length,
        root_child_combinator: document.body.matches(":root > body"),
      };
    },
  },
];

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage();
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "domcontentloaded" });
  const recorded = await page.evaluate((selectors) => {
    const elements = [];
    const walk = (el) => {
      elements.push(el);
      for (const child of el.children) walk(child);
    };
    walk(document.documentElement);
    const rows = selectors.map((selector) => {
      let valid = true;
      try {
        document.querySelector(selector);
      } catch {
        valid = false;
      }
      return {
        selector,
        valid,
        matches: valid ? elements.map((el) => (el.matches(selector) ? "1" : "0")).join("") : "0".repeat(elements.length),
        qsa: valid ? [...document.querySelectorAll(selector)].map((el) => elements.indexOf(el)) : [],
      };
    });
    // The sentinel sheet is a <style> appended as the LAST child of <head> only while the
    // winners are read (matches/qsa above see the case document unchanged); the Rust test
    // reproduces exactly that document for its cascade check.
    const sheet = document.createElement("style");
    document.head.appendChild(sheet);
    for (const row of rows) {
      sheet.textContent = `${row.selector} { order: 1 }`;
      row.winner = elements.map((el) => getComputedStyle(el).order).join("");
      row.sheet_valid = sheet.sheet.cssRules.length === 1;
    }
    sheet.remove();
    return { elements: elements.map((el) => el.localName), rows };
  }, testCase.selectors);
  result.push({ id: testCase.id, bug: testCase.bug, html: testCase.html, elements: recorded.elements, selectors: recorded.rows });
}
const cascadeResult = [];
for (const testCase of CASCADE_CASES) {
  await page.setContent(testCase.html, { waitUntil: "domcontentloaded" });
  const recorded = await page.evaluate((css) => {
    const sheet = document.createElement("style");
    sheet.textContent = css;
    document.head.appendChild(sheet);
    const elements = [];
    const walk = (el) => {
      elements.push(el);
      for (const child of el.children) walk(child);
    };
    walk(document.documentElement);
    const listed = elements.filter((el) => el !== sheet);
    const winners = listed.map((el) => getComputedStyle(el).order);
    sheet.remove();
    return { elements: listed.map((el) => el.localName), winners };
  }, testCase.css);
  cascadeResult.push({
    id: testCase.id,
    class: testCase.class,
    ...(testCase.layout_only ? { layout_only: true } : {}),
    html: testCase.html,
    css: testCase.css,
    ...recorded,
  });
}
const probeResult = [];
for (const probe of PROBES) {
  await page.setContent(probe.html, { waitUntil: "domcontentloaded" });
  const values = await page.evaluate(`(${probe.script.toString()})()`);
  probeResult.push({ id: probe.id, bug: probe.bug, html: probe.html, values });
}
await browser.close();
const fixture = {
  generator: "scripts/cascade-selectors-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  sentinel: "order",
  cases: result,
  cascade_cases: cascadeResult,
  probes: probeResult,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(
  `Chrome ${version}: ${result.length} cases, ${result.reduce((n, c) => n + c.selectors.length, 0)} selectors, ` +
    `${cascadeResult.length} cascade cases, ${probeResult.length} probes -> ${out}`,
);

// Chrome-156 control for the HTML drive's document model (P5b, miaskiewicz/turbo-html2pdf#29):
// `<head>` styles apply, and `<html>`/`<body>` are real boxes with Chrome's UA semantics
// (`body { margin: 8px }`, root margins never collapse, `rem` against the root font size,
// the root/body background propagated to the canvas). P5b-W1 adds the cascade cases of
// miaskiewicz/turbo-dom#24 item 1 (`@media` / `@supports` rules in source order) and
// miaskiewicz/turbo-dom#25 item 2 (elements the drive does not lay out still count for
// selectors). P5b-W3 adds the layout cases of miaskiewicz/turbo-dom#28 (one group per item).
// P5b-W5 adds the table caption cases of miaskiewicz/turbo-dom#51 (`caption-*`) and the
// cases of the bugs found while fixing it (one group per issue, labelled).
//
// For every case below, Chrome for Testing 156 (viewport 1280x800) records:
//   - `boxes`: getBoundingClientRect() of every `[data-cid]` element (document order), as
//     `{cid, x, y, width, height}` — the shape of turbo-dom-layout's `layout_boxes`;
//   - `canvas`: the colour of the canvas (a 1x1 screenshot at (1278,798), below all content),
//     as `#rrggbb`;
//   - `computed`: getComputedStyle(html/body) background-color + font-size.
// Asserted by crates/turbo-dom-layout/tests/html_document_chrome.rs (layout_boxes + the
// galley's canvas background) and crates/turbo-dom-raster/tests (the screenshot canvas).
//
// Usage: node scripts/html-document-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/html_document_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { inflateSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/html_document_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const BOX = `<div data-cid="d" style="height:10px"></div>`;
const DOC = (head, body, htmlAttrs = "", bodyAttrs = "") =>
  `<!DOCTYPE html><html${htmlAttrs}><head>${head}</head><body${bodyAttrs}>${body}</body></html>`;
const S = (css) => `<style>${css}</style>`;
const ROOTS = ` data-cid="html"`;
const BODY = ` data-cid="body"`;
// A caption's text-free content block.
const CAP = (w, h) => `<div style="width:${w}px;height:${h}px"></div>`;

// `measure` lists what the test compares per box ("xywh" by default; text boxes skip the
// font-dependent height).
const CASES = [
  { id: "ua-body-margin", html: DOC("", BOX, ROOTS, BODY) },
  { id: "head-style", html: DOC(S("div{width:96px;height:60px;margin:8px}"), `<div data-cid="d"></div>`, ROOTS, BODY) },
  { id: "head-then-body-order", html: DOC(S("div{width:10px;height:10px}"), S("div{width:20px}") + `<div data-cid="d"></div>`) },
  { id: "body-margin", html: DOC(S("body{margin:20px}"), BOX, ROOTS, BODY) },
  { id: "html-padding", html: DOC(S("html{padding:30px}"), BOX, ROOTS, BODY) },
  { id: "html-margin-no-collapse", html: DOC(S("html{margin:10px}"), BOX, ROOTS, BODY) },
  { id: "body-width", html: DOC(S("body{width:300px}"), BOX, ROOTS, BODY) },
  { id: "body-width-percent", html: DOC(S("body{width:50%;margin:0 auto}"), BOX, ROOTS, BODY) },
  { id: "body-padding-border", html: DOC(S("body{margin:0;padding:5px;border:3px solid #000}"), BOX, ROOTS, BODY) },
  { id: "rem-root-font-size", html: DOC(S("html{font-size:40px} div{height:2rem;width:3rem}"), `<div data-cid="d"></div>`) },
  { id: "rem-root-percent", html: DOC(S("html{font-size:62.5%} div{height:2rem;width:10rem;margin-left:1rem}"), `<div data-cid="d"></div>`) },
  { id: "rem-font-size", html: DOC(S("html{font-size:20px} div{font-size:2rem;height:1em;width:1rem}"), `<div data-cid="d"></div>`) },
  { id: "em-body-font-size", html: DOC(S("body{font-size:40px} div{height:2em}"), `<div data-cid="d"></div>`) },
  { id: "body-flex", html: DOC(S("body{display:flex} div{width:10px;height:10px}"), `<div data-cid="d"></div><div data-cid="e"></div>`) },
  {
    id: "first-child-margin-collapses-with-body",
    html: DOC(S("h1{margin:30px 0}"), `<h1 data-cid="h">x</h1>`, ROOTS, BODY),
    measure: "xyw",
  },
  { id: "body-structural-selectors", html: DOC(S("html:first-child div{width:11px} body:first-child div{height:99px} head + body div{height:7px}"), `<div data-cid="d"></div>`) },
  { id: "style-and-script-in-body-not-rendered", html: DOC("", `<style>div{width:40px;height:40px}</style><script>var x = 1;</script><div data-cid="d"></div>`) },
  { id: "canvas-from-body", html: DOC(S("body{background:#123456}"), BOX, ROOTS, BODY) },
  { id: "canvas-from-html", html: DOC(S("html{background-color:#abcdef} body{background-color:#123456}"), BOX, ROOTS, BODY) },
  { id: "canvas-default-white", html: DOC(S("div{background:#ff0000}"), BOX) },
  { id: "canvas-translucent-body", html: DOC(S("body{background:rgba(0,0,255,0.5)}"), BOX) },
  { id: "canvas-shorthand-keeps-image", html: DOC(S("body{background:#00ff00 linear-gradient(#ff0000,#ff0000) no-repeat 0 0/10px 10px}"), BOX) },
  { id: "canvas-wrapper-not-propagated", html: DOC(S("body{margin:0} div{background:#000000;height:100px}"), `<div data-cid="d"></div>`) },
  // P5b-W1, miaskiewicz/turbo-dom#24 item 1: `@media` / `@supports` rules take their place in
  // the cascade by source position, not after the sheet's top-level rules.
  { id: "media-rule-before-plain-rule", html: DOC(S("@media screen{div{width:10px}} div{width:20px;height:10px}"), `<div data-cid="d"></div>`) },
  { id: "supports-rule-before-plain-rule", html: DOC(S("@supports (display:grid){div{width:10px}} div{width:20px;height:10px}"), `<div data-cid="d"></div>`) },
  { id: "media-rule-after-plain-rule", html: DOC(S("div{width:20px;height:10px} @media screen{div{width:10px}}"), `<div data-cid="d"></div>`) },
  { id: "media-in-supports-source-order", html: DOC(S("@supports (display:block){@media screen{div{width:10px}} div{width:30px}} div{height:10px}"), `<div data-cid="d"></div>`) },
  { id: "media-sheet-before-later-sheet", html: DOC(S("@media screen{div{width:10px;height:10px}}"), S("div{width:20px}") + `<div data-cid="d"></div>`) },
  // P5b-W1, miaskiewicz/turbo-dom#25 item 2: the elements the HTML drive does not lay out
  // (style, script, noscript, template, title, meta, link) are still children for selectors.
  { id: "style-child-is-not-empty", html: DOC(S("div{height:10px} div:empty{width:10px}"), `<div data-cid="d"><style></style></div><div data-cid="e"></div>`) },
  { id: "script-child-is-not-empty", html: DOC(S("div{height:10px} div:empty{width:10px}"), `<div data-cid="d"><script></script></div>`) },
  { id: "style-sibling-before-first-child", html: DOC(S("p{margin:0;height:10px} p:first-child{width:10px}"), `<div><style></style><p data-cid="p"></p></div>`) },
  { id: "script-counts-for-nth-child", html: DOC(S("p{margin:0;height:10px} p:nth-child(2){width:10px}"), `<div><script></script><p data-cid="p"></p></div>`) },
  { id: "template-sibling-after-last-child", html: DOC(S("p{margin:0;height:10px} p:last-child{width:10px}"), `<div><p data-cid="p"></p><template><i></i></template></div>`) },
  { id: "noscript-adjacent-sibling", html: DOC(S("p{margin:0;height:10px} noscript + p{width:10px}"), `<div><noscript></noscript><p data-cid="p"></p></div>`) },
  { id: "script-general-sibling", html: DOC(S("p{margin:0;height:10px} script ~ p{width:10px}"), `<div><script></script><i></i><p data-cid="p"></p></div>`) },
  { id: "style-sibling-breaks-only-child", html: DOC(S("p{margin:0;height:10px} p:only-child{width:10px}"), `<div><p data-cid="p"></p><style></style></div>`) },
  { id: "has-style-child", html: DOC(S("div{height:10px} div:has(> style){width:10px}"), `<div data-cid="d"><style></style></div>`) },
  { id: "meta-link-title-in-body-count", html: DOC(S("p{margin:0;height:10px} p:nth-child(4){width:10px}"), `<div><meta><link><title></title><p data-cid="p"></p></div>`) },
  // P5b-W3, miaskiewicz/turbo-dom#28 item 2: an out-of-flow box's own margins are added once —
  // to its static position (the margin edge of its hypothetical box) or inside a `right` inset.
  { id: "abs-static-position-margins", html: DOC(S("body{margin:0}"), `<div data-cid="p" style="padding-left:3px"><div data-cid="d" style="height:10px;margin-bottom:20px"></div><div data-cid="a" style="position:absolute;margin:5px 0 0 7px;width:10px;height:10px"></div><div data-cid="e" style="height:10px;margin-top:4px"></div></div>`) },
  { id: "abs-static-position-own-margin-top", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="height:10px"></div><div data-cid="a" style="position:absolute;margin-top:5px;width:10px;height:10px"></div>`) },
  { id: "abs-static-position-negative-margins", html: DOC(S("body{margin:0}"), `<div data-cid="p" style="padding:20px"><div data-cid="a" style="position:absolute;margin:-1px;width:1px;height:1px"></div><div data-cid="b" style="height:10px;margin-top:6px"></div><div data-cid="c" style="position:absolute;margin:-1px 0 0 -2px;width:1px;height:1px"></div></div>`) },
  { id: "abs-inset-margins", html: DOC(S("body{margin:0}"), `<div data-cid="a" style="position:absolute;right:10px;top:0;margin:0 3px 0 7px;width:10px;height:10px"></div><div data-cid="b" style="position:absolute;left:10px;top:20px;margin:2px 3px 0 7px;width:10px;height:10px"></div><div data-cid="r" style="position:relative;height:100px"><div data-cid="d" style="position:absolute;bottom:10px;margin-bottom:5px;margin-left:4px;width:10px;height:10px"></div></div>`) },
  { id: "abs-static-position-in-line", html: DOC(S("body{margin:0}"), `<div data-cid="p"><span style="display:inline-block;width:16px;height:10px"></span><span data-cid="a" style="position:absolute;margin-left:7px;margin-top:5px;width:10px;height:10px"></span></div>`), measure: "xyw" },
  // P5b-W3, miaskiewicz/turbo-dom#28 item 3: the anonymous block around inline content next to
  // a block takes only its parent's inherited properties, not its frame, size or margins.
  { id: "anonymous-block-frame", html: DOC(S("body{margin:0}#w{width:200px;padding:10px;border:2px solid #000;margin-left:30px;font-size:0}p{margin:0;height:10px}i{display:inline-block;width:8px;height:10px}"), `<div id="w" data-cid="w"><i data-cid="s"></i><p data-cid="p"></p></div>`) },
  { id: "anonymous-block-height", html: DOC(S("body{margin:0}#w{height:50px;min-height:50px;font-size:0}p{margin:0;height:10px}i{display:inline-block;width:8px;height:10px}"), `<div id="w" data-cid="w"><i data-cid="s"></i><p data-cid="p"></p></div>`) },
  { id: "anonymous-block-margins", html: DOC(S("body{margin:0}#w{margin:20px 0;overflow:hidden;font-size:0}p{margin:0;height:10px}i{display:inline-block;width:8px;height:10px}"), `<div id="w" data-cid="w"><i data-cid="s"></i><p data-cid="p"></p><i data-cid="t"></i></div>`) },
  // P5b-W3, miaskiewicz/turbo-dom#28 item 4: a float is blockified (a floated span is a
  // shrink-to-fit block) and `inline-table` is an atomic inline-level table.
  { id: "float-span-blockified", html: DOC(S("body{margin:0}"), `<div data-cid="d"><span data-cid="f" style="float:left;width:30px;height:20px"></span><span data-cid="g" style="float:right;width:40px;height:25px;margin-top:3px"></span></div><div data-cid="e" style="height:5px"></div>`) },
  { id: "float-span-beside-line", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><span data-cid="f" style="float:left;width:30px;height:20px"></span><i data-cid="i" style="display:inline-block;width:10px;height:10px"></i></div>`) },
  { id: "float-span-shrinks-to-content", html: DOC(S("body{margin:0}"), `<div data-cid="d"><span data-cid="f" style="float:left"><i style="display:block;width:30px;height:20px"></i><i style="display:block;width:20px;height:20px"></i></span></div>`) },
  { id: "float-in-inline", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><a href="#"><span data-cid="f" style="float:right;width:30px;height:20px"></span></a></div>`) },
  { id: "float-span-mid-line", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><i data-cid="i" style="display:inline-block;width:10px;height:10px"></i><span data-cid="f" style="float:left;width:30px;height:20px"></span><i data-cid="j" style="display:inline-block;width:10px;height:10px"></i></div>`) },
  { id: "float-in-line-wraps-following-lines", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="width:100px;font-size:0"><i data-cid="a" style="display:inline-block;width:30px;height:10px"></i><span data-cid="f" style="float:right;width:40px;height:25px"></span><i data-cid="b" style="display:inline-block;width:30px;height:10px"></i><i data-cid="c" style="display:inline-block;width:30px;height:10px"></i><i data-cid="e" style="display:inline-block;width:30px;height:10px"></i><i data-cid="g" style="display:inline-block;width:30px;height:10px"></i></div>`) },
  { id: "inline-table-is-inline-level", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><div data-cid="t" style="display:inline-table;width:40px;height:20px"></div><div data-cid="t2" style="display:inline-table;width:40px;height:20px"></div></div>`) },
  { id: "inline-table-cells", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><i style="display:inline-block;width:15px;height:10px"></i><table data-cid="t" style="display:inline-table;border-spacing:2px"><tr><td data-cid="td" style="padding:1px"><div style="width:30px;height:10px"></div></td></tr></table></div>`) },
  { id: "inline-table-blockified", html: DOC(S("body{margin:0}"), `<div data-cid="t" style="display:inline-table;float:left;width:40px;height:20px"></div><div data-cid="u" style="display:inline-table;position:absolute;left:100px;top:50px;width:40px;height:20px"></div><span data-cid="v" style="display:inline-block;float:right;width:10px;height:10px"></span>`) },
  // P5b-W3, miaskiewicz/turbo-dom#28 item 6: a line holding only absolutely positioned (or
  // floated) boxes, collapsible spaces and empty inlines is a zero-height line box.
  { id: "abs-only-line-in-inline", html: DOC(S("body{margin:0}"), `<div data-cid="d"><span><span data-cid="a" style="position:absolute;width:5px;height:5px"></span></span></div><div data-cid="e" style="height:5px"></div>`) },
  { id: "abs-only-line-with-collapsible-spaces", html: DOC(S("body{margin:0}"), `<div data-cid="d"> <span>  <i data-cid="a" style="position:absolute;width:5px;height:5px"></i> </span> <b data-cid="b" style="position:absolute;left:20px;width:5px;height:5px"></b> </div><div data-cid="e" style="height:5px"></div>`) },
  { id: "abs-only-line-between-empty-inlines", html: DOC(S("body{margin:0}"), `<div data-cid="d"><span></span><b data-cid="a" style="position:absolute;width:5px;height:5px"></b><span></span></div><div data-cid="e" style="height:5px"></div>`) },
  { id: "float-only-line-in-inline", html: DOC(S("body{margin:0}"), `<div data-cid="d"><a href="#"><span data-cid="f" style="float:left;width:30px;height:20px"></span></a></div><div data-cid="e" style="height:5px"></div>`) },
  // P5b-W3, miaskiewicz/turbo-dom#28 item 7: a `width: auto` table is shrink-to-fit on the
  // HTML (PDF / screenshot) path too — its columns' max-content plus spacing and frame,
  // clamped by min/max-width, never below its min-content.
  { id: "table-auto-width", html: DOC(S("body{margin:0}"), `<table data-cid="table"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-two-columns", html: DOC(S("body{margin:0}"), `<table data-cid="table"><tr><td data-cid="a"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:50px;height:20px"></div></td></tr><tr><td data-cid="c"><div style="width:10px;height:5px"></div></td><td data-cid="d"><div style="width:10px;height:5px"></div></td></tr></table>`) },
  { id: "table-auto-spacing-padding-border", html: DOC(S("body{margin:0}table{border-spacing:6px 4px;border:3px solid #000;padding:5px}td{padding:2px;border:1px solid #000}"), `<table data-cid="table"><tr><td data-cid="a"><div style="width:20px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-margin-centered", html: DOC(S("body{margin:0}table{margin:0 auto}"), `<table data-cid="table"><tr><td data-cid="td"><div style="width:40px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-wide-content-fills", html: DOC(S("body{margin:0}"), `<table data-cid="table"><tr><td data-cid="td"><div style="width:2000px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-fixed-layout", html: DOC(S("body{margin:0}table{table-layout:fixed}"), `<table data-cid="table"><tr><td data-cid="a"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-fixed-collapsed", html: DOC(S("body{margin:0}table{table-layout:fixed;border-collapse:collapse}td{border:1px solid #000}"), `<table data-cid="table"><tr><td data-cid="a" style="width:50px"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-px-cell", html: DOC(S("body{margin:0}"), `<table data-cid="table"><tr><td data-cid="a" style="width:100px"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-min-width", html: DOC(S("body{margin:0}table{min-width:100px}"), `<table data-cid="table"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-max-width-floors-at-content", html: DOC(S("body{margin:0}table{max-width:20px}"), `<table data-cid="table"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-in-flex-item", html: DOC(S("body{margin:0}div{display:flex}"), `<div><table data-cid="table"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table></div>`) },
  { id: "table-auto-nested", html: DOC(S("body{margin:0}"), `<table data-cid="outer"><tr><td data-cid="cell"><table data-cid="inner"><tr><td><div style="width:30px;height:10px"></div></td></tr></table></td></tr></table>`) },
  // miaskiewicz/turbo-dom#53: `inline-flex` / `inline-grid` are atomic inline-level boxes.
  { id: "inline-flex-is-inline-level", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><div data-cid="a" style="display:inline-flex;width:40px;height:20px"></div><div data-cid="b" style="display:inline-flex;width:40px;height:20px"></div></div>`) },
  { id: "inline-flex-shrinks-to-content", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><span data-cid="a" style="display:inline-flex"><i style="display:block;width:30px;height:10px"></i><i style="display:block;width:20px;height:10px"></i></span><i data-cid="b" style="display:inline-block;width:10px;height:10px"></i></div>`) },
  { id: "inline-grid-is-inline-level", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><div data-cid="a" style="display:inline-grid;width:40px;height:20px"></div><div data-cid="b" style="display:inline-grid;grid-template-columns:15px 25px"><i style="display:block;height:10px"></i><i style="display:block;height:10px"></i></div></div>`) },
  { id: "inline-flex-grid-blockified", html: DOC(S("body{margin:0}"), `<div data-cid="f" style="display:inline-flex;float:left;width:40px;height:20px"></div><div data-cid="g" style="display:inline-grid;position:absolute;left:100px;top:50px;width:40px;height:20px"></div>`) },
  // miaskiewicz/turbo-dom#48: an absolutely positioned `width: auto` box is shrink-to-fit
  // unless both `left` and `right` are set.
  { id: "abs-auto-width-shrinks-to-fit", html: DOC(S("body{margin:0}div div{height:10px}"), `<div data-cid="a" style="position:absolute;left:0;top:0"><div style="width:30px"></div></div><div data-cid="b" style="position:absolute;right:0;top:20px"><div style="width:30px"></div></div><div data-cid="c" style="position:absolute;top:40px"><div style="width:30px"></div></div><div data-cid="d" style="position:absolute;left:0;right:0;top:60px"><div style="width:30px"></div></div><div data-cid="e" style="position:absolute;left:0;top:80px;padding:0 5px;border:2px solid"><div style="width:30px"></div></div>`) },
  { id: "abs-auto-width-min-max", html: DOC(S("body{margin:0}div div{height:10px}"), `<div data-cid="a" style="position:absolute;left:0;top:0;min-width:50px"><div style="width:30px"></div></div><div data-cid="b" style="position:absolute;left:0;top:20px;max-width:20px"><div style="width:30px"></div></div>`) },
  { id: "abs-auto-width-wider-than-cb", html: DOC(S("body{margin:0}"), `<div data-cid="p" style="position:relative;width:200px;height:50px"><div data-cid="a" style="position:absolute;left:10px;top:0"><div style="width:300px;height:10px"></div></div><div data-cid="b" style="position:absolute;left:50px;top:20px"><div style="display:inline-block;width:100px;height:10px"></div><div style="display:inline-block;width:100px;height:10px"></div></div></div>`), measure: "xyw" },
  { id: "fixed-auto-width-shrinks", html: DOC(S("body{margin:0}"), `<div data-cid="f" style="position:fixed;right:10px;top:10px"><div style="width:25px;height:15px"></div></div>`) },
  { id: "abs-auto-width-margins", html: DOC(S("body{margin:0}"), `<div data-cid="a" style="position:absolute;left:0;top:0;margin:0 4px 0 6px"><div style="width:30px;height:10px"></div></div><div data-cid="b" style="position:absolute;right:0;top:20px;margin:0 4px 0 6px"><div style="width:30px;height:10px"></div></div>`) },
  { id: "abs-auto-width-both-insets", html: DOC(S("body{margin:0}"), `<div style="position:relative;width:300px;height:40px"><div data-cid="a" style="position:absolute;left:10px;right:20px;top:0;padding:0 3px"><div style="width:30px;height:10px"></div></div><div data-cid="b" style="position:absolute;left:10px;right:20px;top:20px;margin:0 5px"><div style="width:30px;height:10px"></div></div></div>`) },
  // miaskiewicz/turbo-dom#50: percentage cell widths in the automatic table layout.
  { id: "table-percent-column-half", html: DOC(S("body{margin:0}"), `<table data-cid="t"><tr><td data-cid="a" style="width:50%"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-percent-column-full", html: DOC(S("body{margin:0}"), `<table data-cid="t"><tr><td data-cid="a" style="width:100%"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-percent-columns-two-quarters", html: DOC(S("body{margin:0}"), `<table data-cid="t"><tr><td data-cid="a" style="width:25%"><div style="width:30px;height:10px"></div></td><td data-cid="b" style="width:25%"><div style="width:20px;height:10px"></div></td><td data-cid="c"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-percent-column-explicit-table-width", html: DOC(S("body{margin:0}table{width:400px}"), `<table data-cid="t"><tr><td data-cid="a" style="width:25%"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td><td data-cid="c"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-percent-column-attribute", html: DOC(S("body{margin:0}"), `<table data-cid="t" width="300"><tr><td data-cid="a" width="60%"><div style="width:10px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  // miaskiewicz/turbo-dom#49: auto-layout collapsed tables merge shared borders.
  { id: "table-collapse-auto-cells", html: DOC(S("body{margin:0}table{border-collapse:collapse}td{border:2px solid #000;padding:3px}"), `<table data-cid="t"><tr><td data-cid="a"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr><tr><td data-cid="c"><div style="width:10px;height:5px"></div></td><td data-cid="d"><div style="width:40px;height:5px"></div></td></tr></table>`) },
  { id: "table-collapse-auto-table-border", html: DOC(S("body{margin:0}table{border-collapse:collapse}td{border:2px solid #000;padding:3px}table{border:4px solid #000}"), `<table data-cid="t"><tr><td data-cid="a"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-collapse-auto-mixed-borders", html: DOC(S("body{margin:0}table{border-collapse:collapse}td{padding:1px}"), `<table data-cid="t"><tr><td data-cid="a" style="border:6px solid #000"><div style="width:30px;height:10px"></div></td><td data-cid="b" style="border:1px solid #000"><div style="width:20px;height:10px"></div></td></tr><tr><td data-cid="c" style="border:1px solid #000"><div style="width:10px;height:10px"></div></td><td data-cid="d" style="border:1px solid #000"><div style="width:10px;height:10px"></div></td></tr></table>`) },
  { id: "table-collapse-auto-colspan", html: DOC(S("body{margin:0}table{border-collapse:collapse}td{border:2px solid #000;padding:3px}"), `<table data-cid="t"><tr><td data-cid="s" colspan="2"><div style="width:100px;height:10px"></div></td></tr><tr><td data-cid="a"><div style="width:20px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  // miaskiewicz/turbo-dom#56: spanning cells spread their excess over their columns.
  { id: "table-colspan-spreads-in-proportion", html: DOC(S("body{margin:0}td{padding:0}"), `<table data-cid="t"><tr><td data-cid="s" colspan="2"><div style="width:102px;height:10px"></div></td></tr><tr><td data-cid="a"><div style="width:20px;height:10px"></div></td><td data-cid="b"><div style="width:60px;height:10px"></div></td></tr></table>`) },
  { id: "table-colspan-counts-inner-spacing", html: DOC(S("body{margin:0}td{padding:0}table{border-spacing:5px}"), `<table data-cid="t"><tr><td data-cid="s" colspan="2"><div style="width:125px;height:10px"></div></td></tr><tr><td data-cid="a"><div style="width:10px;height:10px"></div></td><td data-cid="b"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-colspan-empty-columns-evenly", html: DOC(S("body{margin:0}td{padding:0}"), `<table data-cid="t"><tr><td data-cid="s" colspan="2"><div style="width:62px;height:10px"></div></td><td data-cid="c"></td></tr><tr><td data-cid="a"></td><td data-cid="b"></td><td><div style="width:10px;height:10px"></div></td></tr></table>`) },
  // P5b-W5, miaskiewicz/turbo-dom#51: a caption is not part of the grid. The table wrapper box
  // (the `<table>`'s box) holds the captions — `caption-side: top` above the grid, `bottom`
  // below, each in source order — and the grid box, which stretches to the wrapper's width:
  // the grid's own width or the widest caption's min-content (margin box), whichever is wider.
  // Caption margins do not collapse with the table's; `height` sizes the grid.
  ...[
    ["caption-wider-than-grid", "", `<caption data-cid="c">${CAP(100, 10)}</caption>`],
    ["caption-narrower-than-grid", "", `<caption data-cid="c">${CAP(10, 10)}</caption>`],
    ["caption-table-width", "width:100px", `<caption data-cid="c">${CAP(10, 10)}</caption>`],
    ["caption-wider-than-table-width", "width:100px", `<caption data-cid="c">${CAP(150, 10)}</caption>`],
    ["caption-frame-margins", "border:3px solid #000;padding:5px;background:#f00;margin:7px", `<caption data-cid="c" style="padding:2px;border:1px solid #00f;margin:4px">${CAP(100, 10)}</caption>`],
    ["caption-side-bottom", "caption-side:bottom", `<caption data-cid="c">${CAP(100, 10)}</caption>`],
    ["caption-multiple", "", `<caption data-cid="c">${CAP(40, 10)}</caption>`, `<caption data-cid="c2" style="caption-side:bottom">${CAP(50, 7)}</caption><caption data-cid="c3">${CAP(20, 3)}</caption>`],
    ["caption-own-width", "", `<caption data-cid="c" style="width:200px">${CAP(10, 10)}</caption>`],
    ["caption-percent-table-width", "width:50%", `<caption data-cid="c">${CAP(100, 10)}</caption>`],
    ["caption-centered-table", "margin:0 auto", `<caption data-cid="c">${CAP(100, 10)}</caption>`],
    ["caption-absolute-table", "position:absolute;left:10px;top:20px", `<caption data-cid="c">${CAP(100, 10)}</caption>`],
    ["caption-table-height", "height:100px", `<caption data-cid="c">${CAP(100, 10)}</caption>`],
    ["caption-margins", "", `<caption data-cid="c" style="margin:10px 0 5px">${CAP(100, 10)}</caption>`],
    ["caption-collapsed-borders", "border-collapse:collapse;border:2px solid #000", `<caption data-cid="c">${CAP(100, 10)}</caption>`, "", "border:1px solid #000"],
  ].map(([id, table, caption, after = "", cell = ""]) => ({
    id,
    html: DOC(S("body{margin:0}"), `<table data-cid="t" style="${table}">${caption}<tr><td data-cid="td" style="${cell}"><div style="width:30px;height:10px"></div></td></tr>${after}</table><div data-cid="after" style="height:5px"></div>`),
  })),
  { id: "caption-no-rows", html: DOC(S("body{margin:0}"), `<table data-cid="t"><caption data-cid="c">${CAP(100, 10)}</caption></table>`) },
  { id: "caption-narrow-container", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="width:60px"><table data-cid="t"><caption data-cid="c">${CAP(100, 10)}</caption><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table></div>`) },
  { id: "caption-inline-table", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><i style="display:inline-block;width:5px;height:5px"></i><table data-cid="t" style="display:inline-table"><caption data-cid="c">${CAP(100, 10)}</caption><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table></div>`) },
  { id: "caption-in-flex-item", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="display:flex"><table data-cid="t"><caption data-cid="c">${CAP(100, 10)}</caption><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table></div>`) },
  { id: "caption-float-table", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="float:left;width:50px"><caption data-cid="c">${CAP(80, 10)}</caption><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table><div data-cid="d" style="overflow:hidden;height:10px"></div>`) },
  { id: "caption-nested-table", html: DOC(S("body{margin:0}"), `<table data-cid="o"><tr><td data-cid="oc"><table data-cid="t"><caption data-cid="c">${CAP(100, 10)}</caption><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table></td></tr></table>`) },
  { id: "caption-display-table-caption", html: DOC(S("body{margin:0}"), `<div data-cid="t" style="display:table"><div data-cid="c" style="display:table-caption">${CAP(100, 10)}</div><div style="display:table-row"><div data-cid="td" style="display:table-cell"><div style="width:30px;height:10px"></div></div></div></div>`) },
  // A text caption wraps to its min-content (its widest word): font-dependent sizes, so only
  // the caption's box and the x edges are compared.
  { id: "caption-text", html: DOC(S("body{margin:0}"), `<table data-cid="t"><caption data-cid="c">A rather long caption text that is wider than the table</caption><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`), measure: "x" },
  // P5b-W5, miaskiewicz/turbo-dom#65: a `px` width is the border box under
  // `box-sizing: border-box` in intrinsic sizes (floats, inline-blocks, shrink-to-fit
  // tables) and in table column widths.
  { id: "border-box-float-child", html: DOC(S("body{margin:0}"), `<div data-cid="f" style="float:left"><div data-cid="c" style="box-sizing:border-box;width:100px;padding:10px;height:10px"></div></div>`) },
  { id: "border-box-inline-block-child", html: DOC(S("body{margin:0}"), `<div data-cid="d" style="font-size:0"><div data-cid="i" style="display:inline-block"><div style="box-sizing:border-box;width:100px;padding:10px;height:10px"></div></div><i data-cid="j" style="display:inline-block;width:10px;height:10px"></i></div>`) },
  { id: "border-box-table-content", html: DOC(S("body{margin:0}td{padding:0}"), `<table data-cid="t" style="border-spacing:0"><tr><td data-cid="td"><div data-cid="c" style="box-sizing:border-box;width:100px;padding:10px;height:10px"></div></td></tr></table>`) },
  { id: "border-box-table-min-content", html: DOC(S("body{margin:0}td{padding:0}"), `<table data-cid="t" style="width:1px;border-spacing:0"><tr><td data-cid="td"><div data-cid="c" style="box-sizing:border-box;width:100px;padding:10px;height:10px"></div></td></tr></table>`) },
  { id: "border-box-cell-width", html: DOC(S("body{margin:0}"), `<table data-cid="t"><tr><td data-cid="a" style="box-sizing:border-box;width:100px;padding:10px"></td><td data-cid="b"><div style="width:10px;height:10px"></div></td></tr></table>`) },
  { id: "border-box-cell-width-collapsed", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="border-collapse:collapse"><tr><td data-cid="a" style="box-sizing:border-box;width:100px;padding:10px;border:2px solid"></td><td data-cid="b"><div style="width:10px;height:10px"></div></td></tr></table>`) },
  { id: "border-box-cell-width-fixed", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="table-layout:fixed;width:300px"><tr><td data-cid="a" style="box-sizing:border-box;width:100px;padding:10px"></td><td data-cid="b"><div style="width:10px;height:10px"></div></td></tr></table>`) },
  { id: "border-box-cell-width-fixed-collapsed", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="table-layout:fixed;width:300px;border-collapse:collapse"><tr><td data-cid="a" style="box-sizing:border-box;width:100px;padding:10px;border:2px solid"></td><td data-cid="b"><div style="width:10px;height:10px"></div></td></tr></table>`) },
  { id: "border-box-wider-than-frame", html: DOC(S("body{margin:0}"), `<div data-cid="f" style="float:left"><div data-cid="c" style="box-sizing:border-box;width:10px;padding:10px;height:10px"></div></div>`) },
  // P5b-W5, miaskiewicz/turbo-dom#62: Chrome's UA sizes `<table>` as `box-sizing: border-box`.
  { id: "table-border-box-width", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="width:120px;border:4px solid;padding:3px"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-border-box-width-collapsed", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="width:120px;border:4px solid;border-collapse:collapse"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-border-box-percent-width", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="width:50%;border:10px solid"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-border-box-min-width", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="min-width:100px;border:5px solid"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-border-box-max-width", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="width:300px;max-width:100px;border:5px solid"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-content-box-author", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="box-sizing:content-box;width:120px;border:4px solid"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-border-box-in-float", html: DOC(S("body{margin:0}"), `<div data-cid="f" style="float:left"><table data-cid="t" style="width:120px;border:4px solid"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table></div>`) },
  { id: "table-border-box-fixed", html: DOC(S("body{margin:0}"), `<table data-cid="t" style="table-layout:fixed;width:120px;border:4px solid;padding:2px"><tr><td data-cid="a"><div style="width:10px;height:10px"></div></td><td data-cid="b"><div style="width:10px;height:10px"></div></td></tr></table>`) },
];

/** The RGB of a 1x1 PNG screenshot (any filter: a single pixel's filter adds 0). */
function onePixel(png) {
  let off = 8;
  let idat = Buffer.alloc(0);
  let colorType = 2;
  while (off < png.length) {
    const len = png.readUInt32BE(off);
    const type = png.toString("latin1", off + 4, off + 8);
    const data = png.subarray(off + 8, off + 8 + len);
    if (type === "IHDR") colorType = data[9];
    if (type === "IDAT") idat = Buffer.concat([idat, data]);
    off += 12 + len;
  }
  const raw = inflateSync(idat);
  if (![2, 6].includes(colorType)) throw new Error(`unexpected PNG colour type ${colorType}`);
  const hex = (n) => n.toString(16).padStart(2, "0");
  return `#${hex(raw[1])}${hex(raw[2])}${hex(raw[3])}`;
}

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  const probe = await page.evaluate(() => {
    const r = (n) => Math.round(n * 1000) / 1000;
    const boxes = [...document.querySelectorAll("[data-cid]")].map((el) => {
      const b = el.getBoundingClientRect();
      return { cid: el.dataset.cid, x: r(b.x), y: r(b.y), width: r(b.width), height: r(b.height) };
    });
    const cs = (el) => {
      const s = getComputedStyle(el);
      return { "background-color": s.backgroundColor, "font-size": s.fontSize };
    };
    return { boxes, computed: { html: cs(document.documentElement), body: cs(document.body) } };
  });
  const shot = await page.screenshot({ clip: { x: 1278, y: 798, width: 1, height: 1 } });
  result.push({ ...testCase, measure: testCase.measure || "xywh", ...probe, canvas: onePixel(shot) });
}
await browser.close();

const fixture = {
  generator: "scripts/html-document-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${result.length} cases -> ${out}`);

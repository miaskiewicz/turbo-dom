// Chrome-156 control for turbo-dom-layout's live-DOM geometry (P5b-W1): the CSSOM-View
// reads turbo-surf answers from `turbo_dom_layout::live` snapshots
// (`cssom_view::metrics::element_geometry`, falling back to `cssom_view::inline::geometry`).
//   - miaskiewicz/turbo-surf#31: an absolutely/fixed positioned inline is blockified
//     (CSS 2.1 §9.7) and laid out at its static position without splitting its inline
//     ancestor, and an unsupported inline construct fails that element only, instead of
//     failing the whole snapshot; a table row is a box spanning its cells (inside the
//     border-spacing) with used box metrics, and a row group (`<tbody>`/`<thead>`/
//     `<tfoot>`) is a box: the union of its rows;
//   - miaskiewicz/turbo-surf#36 items 3 and 4: an auto-width table shrinks to fit its
//     columns, and a `<template>` (display:none) has an all-zero rect.
// P5b-W3 adds the cases of miaskiewicz/turbo-dom#28 (one group per item, labelled).
//
// For every case, Chrome for Testing 156 (viewport 1280x800) records the
// getBoundingClientRect() of every `[data-cid]` element (document order) as
// `{cid, x, y, width, height, display, position}` (the last two from getComputedStyle),
// and, for a case with `hits` (viewport points), `document.elementFromPoint` of each as
// `{x, y, hit}` (the element's `data-cid`, else its lower-case tag name, else null).
// Boxes are compared exactly; a case's `measure` maps a cid to the fields compared for it
// ("xywh" when absent) where text metrics (font-dependent) would move the rest.
// Asserted by crates/turbo-dom-layout/src/live/chrome_tests.rs.
//
// Usage: node scripts/live-geometry-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/live_geometry_chrome156.json.
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/live_geometry_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DOC = (css, body) => `<!doctype html><style>${css}</style>${body}`;
// A text-free cell content box of a known size.
const BOX = (w, h, cid = "") => `<div${cid ? ` data-cid="${cid}"` : ""} style="width:${w}px;height:${h}px"></div>`;
const CELL = (w, h, cid) => `<td data-cid="${cid}">${BOX(w, h)}</td>`;

const CASES = [
  // turbo-surf#31 C: position:absolute/fixed blockify an inline (display computes to
  // block; CSS 2.1 §9.7) and it lays out as an out-of-flow box.
  {
    id: "absolute-span-blockified",
    html: DOC("#a{position:absolute;top:5px;left:7px;width:20px;height:9px}", `${BOX(10, 40, "d")}<span id="a" data-cid="a"></span>`),
  },
  {
    id: "absolute-span-static-position",
    html: DOC("#a{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0)}", `${BOX(10, 40, "d")}<span id="a" data-cid="a">screen-reader only</span>`),
  },
  {
    id: "fixed-span-blockified",
    html: DOC("#f{position:fixed;right:10px;bottom:20px;width:30px;height:15px}", `${BOX(10, 10, "d")}<span id="f" data-cid="f"></span>`),
  },
  {
    // The Bootstrap .sr-only span inside a link: only its size is font-independent.
    id: "sr-only-in-link",
    html: DOC(
      ".sr-only{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}",
      `<p data-cid="p">Read <a href="#">more<span data-cid="s" class="sr-only"> about tables</span></a> here.</p>`,
    ),
    measure: { p: "xw", s: "wh" },
  },
  // turbo-surf#31: an inline construct the native inline layout does not support fails
  // that element only; the rest of the document still answers.
  {
    id: "unsupported-inline-is-per-element",
    html: DOC("p{margin:0}", `${BOX(20, 10, "a")}<p data-cid="p">x<span data-cid="s" style="vertical-align:top">y</span></p>${BOX(20, 10, "b")}`),
    measure: { p: "xw", b: "xwh" },
  },
  // turbo-surf#31 A: a table row is a box spanning its cells (inside the border-spacing)
  // with used box metrics.
  { id: "table-width-percent", html: DOC("table{width:50%}", `<table data-cid="table"><tr data-cid="tr">${CELL(30, 10, "td")}</tr></table>`) },
  { id: "table-width-px", html: DOC("table{width:300px}", `<table data-cid="table"><tr data-cid="tr">${CELL(30, 10, "td")}</tr></table>`) },
  // turbo-surf#36 item 3: an auto-width table shrinks to fit its columns (and is centred
  // by auto margins), but never below its content.
  {
    id: "table-auto-width",
    html: DOC("", `<table data-cid="table"><tr data-cid="tr">${CELL(30, 10, "td")}</tr></table>`),
  },
  {
    id: "table-auto-two-columns",
    html: DOC(
      "",
      `<table data-cid="table"><tr data-cid="tr">${CELL(30, 10, "a")}${CELL(50, 20, "b")}</tr><tr data-cid="tr2">${CELL(10, 5, "c")}${CELL(10, 5, "d")}</tr></table>`,
    ),
  },
  {
    id: "table-auto-spacing-padding-border",
    html: DOC(
      "table{border-spacing:6px 4px;border:3px solid #000;padding:5px}td{padding:2px;border:1px solid #000}",
      `<table data-cid="table"><tr data-cid="tr">${CELL(20, 10, "a")}${CELL(20, 10, "b")}</tr></table>`,
    ),
  },
  {
    id: "table-auto-margin-centered",
    html: DOC("table{margin:0 auto}", `<table data-cid="table"><tr data-cid="tr">${CELL(40, 10, "td")}</tr></table>`),
  },
  {
    id: "table-auto-wide-content-fills",
    html: DOC("", `<table data-cid="table"><tr data-cid="tr">${CELL(2000, 10, "td")}</tr></table>`),
  },
  // turbo-surf#31 B: a row group (`<tbody>`/`<thead>`/`<tfoot>`) is a box: the union of
  // its rows.
  {
    id: "table-sections",
    html: DOC(
      "",
      `<table data-cid="table"><thead data-cid="thead"><tr data-cid="h">${CELL(30, 10, "hc")}</tr></thead><tbody data-cid="tbody"><tr data-cid="r1">${CELL(30, 20, "c1")}</tr><tr data-cid="r2">${CELL(30, 5, "c2")}</tr></tbody><tfoot data-cid="tfoot"><tr data-cid="f">${CELL(30, 7, "fc")}</tr></tfoot></table>`,
    ),
  },
  {
    id: "table-two-bodies",
    html: DOC(
      "",
      `<table data-cid="table"><tbody data-cid="b1"><tr data-cid="r1">${CELL(10, 10, "c1")}</tr></tbody><tbody data-cid="b2"><tr data-cid="r2">${CELL(10, 10, "c2")}</tr></tbody></table>`,
    ),
  },
  {
    id: "table-empty-cell",
    html: DOC("", `<table data-cid="table"><tbody data-cid="tbody"><tr data-cid="tr"><td data-cid="td"></td></tr></tbody></table>`),
  },
  {
    // turbo-surf#31's decorated fixed table (collapsed borders; the cells' text does not
    // move any box: fixed widths, explicit heights).
    id: "table-decorated-collapsed",
    html: DOC(
      "#t{border-collapse:collapse;table-layout:fixed;width:240px}#t td{width:120px;height:30px;padding:6px;border:1px solid #333}",
      `<table id="t" data-cid="table"><tbody data-cid="tbody"><tr data-cid="r1"><td data-cid="a">A1</td><td data-cid="b">B1</td></tr><tr data-cid="r2"><td data-cid="c">A2</td><td data-cid="d">B2</td></tr></tbody></table>`,
    ),
  },
  // turbo-surf#36 item 4: a <template> is display:none, so its rect is all zero — also in
  // a table cell.
  {
    id: "template-in-cell",
    html: DOC("", `<table><tr><td data-cid="td"><template data-cid="t"><b>x</b></template>${BOX(8, 8)}</td></tr></table>`),
  },
  { id: "template-in-body", html: DOC("", `${BOX(10, 10, "d")}<template data-cid="t"><b>x</b></template>`) },
  // turbo-dom#28 item 1: `clip: rect()` clips an absolutely positioned box and its
  // descendants out of hit testing (`auto` = the border-box edge); `clip` on a box that
  // is not absolutely positioned does nothing.
  {
    id: "clip-rect-hit-testing",
    html: DOC(
      "body{margin:0}.a{position:absolute;top:0;width:50px;height:50px}i{display:block;width:50px;height:50px}",
      `<div data-cid="under" style="height:100px;padding-top:60px"><span data-cid="none" class="a" style="left:0;clip:rect(0 0 0 0)"></span><span data-cid="part" class="a" style="left:100px;clip:rect(10px,40px,40px,10px)"><i data-cid="kid"></i></span><span data-cid="auto" class="a" style="left:200px;clip:rect(auto, 20px, auto, auto)"></span><div data-cid="fixed" style="position:fixed;left:300px;top:0;width:50px;height:50px;clip:rect(0 0 0 0)"></div><div data-cid="flow" style="width:20px;height:20px;clip:rect(0 0 0 0)"></div></div>`,
    ),
    hits: [[5, 5], [105, 5], [115, 15], [139, 39], [145, 15], [115, 45], [205, 5], [205, 45], [225, 5], [305, 5], [5, 65]],
  },
  {
    // The sr-only pattern: its text overflows the 1px box but is clipped away too.
    id: "clip-rect-sr-only-text",
    html: DOC(
      "body{margin:0}p{margin:0}.sr{position:absolute;width:1px;height:1px;padding:0;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}",
      `<p data-cid="p"><span data-cid="s" class="sr">hidden text</span>visible</p>`,
    ),
    measure: { p: "xyw", s: "xywh" },
    hits: [[0.5, 0.5], [3, 3]],
  },
  // turbo-dom#28 item 2: an out-of-flow box's own margins are added once — to its static
  // position (the margin edge of its hypothetical box: the flow's content edge and the
  // margins collapsed before it) or inside a `right` inset.
  { id: "abs-static-position-margins", html: DOC("body{margin:0}", `<div data-cid="p" style="padding-left:3px"><div data-cid="d" style="height:10px;margin-bottom:20px"></div><div data-cid="a" style="position:absolute;margin:5px 0 0 7px;width:10px;height:10px"></div><div data-cid="e" style="height:10px;margin-top:4px"></div></div>`) },
  { id: "abs-static-position-own-margin-top", html: DOC("body{margin:0}", `<div data-cid="d" style="height:10px"></div><div data-cid="a" style="position:absolute;margin-top:5px;width:10px;height:10px"></div>`) },
  { id: "abs-static-position-negative-margins", html: DOC("body{margin:0}", `<div data-cid="p" style="padding:20px"><div data-cid="a" style="position:absolute;margin:-1px;width:1px;height:1px"></div><div data-cid="b" style="height:10px;margin-top:6px"></div><div data-cid="c" style="position:absolute;margin:-1px 0 0 -2px;width:1px;height:1px"></div></div>`) },
  { id: "abs-inset-margins", html: DOC("body{margin:0}", `<div data-cid="a" style="position:absolute;right:10px;top:0;margin:0 3px 0 7px;width:10px;height:10px"></div><div data-cid="b" style="position:absolute;left:10px;top:20px;margin:2px 3px 0 7px;width:10px;height:10px"></div><div data-cid="r" style="position:relative;height:100px"><div data-cid="d" style="position:absolute;bottom:10px;margin-bottom:5px;margin-left:4px;width:10px;height:10px"></div></div>`) },
  { id: "abs-static-position-in-line", html: DOC("body{margin:0}", `<div data-cid="p"><span style="display:inline-block;width:16px;height:10px"></span><span data-cid="a" style="position:absolute;margin-left:7px;margin-top:5px;width:10px;height:10px"></span></div>`), measure: { p: "xyw" } },
  // turbo-dom#28 item 3: the anonymous block around inline content next to a block takes only
  // the inherited properties of its parent (CSS 2.1 §9.2.1.1), not its frame, size or margins.
  { id: "anonymous-block-frame", html: DOC("body{margin:0}#w{width:200px;padding:10px;border:2px solid #000;margin-left:30px;font-size:0}p{margin:0;height:10px}i{display:inline-block;width:8px;height:10px}", `<div id="w" data-cid="w"><i data-cid="s"></i><p data-cid="p"></p></div>`) },
  { id: "anonymous-block-height", html: DOC("body{margin:0}#w{height:50px;min-height:50px;font-size:0}p{margin:0;height:10px}i{display:inline-block;width:8px;height:10px}", `<div id="w" data-cid="w"><i data-cid="s"></i><p data-cid="p"></p></div>`) },
  { id: "anonymous-block-margins", html: DOC("body{margin:0}#w{margin:20px 0;overflow:hidden;font-size:0}p{margin:0;height:10px}i{display:inline-block;width:8px;height:10px}", `<div id="w" data-cid="w"><i data-cid="s"></i><p data-cid="p"></p><i data-cid="t"></i></div>`) },
  // turbo-dom#28 item 4: a float is blockified like an absolutely positioned box (CSS 2.1
  // §9.7: a floated span is a block, shrink-to-fit), and `inline-table` is an atomic
  // inline-level table (`table` once blockified).
  { id: "float-span-blockified", html: DOC("body{margin:0}", `<div data-cid="d"><span data-cid="f" style="float:left;width:30px;height:20px"></span><span data-cid="g" style="float:right;width:40px;height:25px;margin-top:3px"></span></div><div data-cid="e" style="height:5px"></div>`) },
  { id: "float-span-beside-line", html: DOC("body{margin:0}", `<div data-cid="d" style="font-size:0"><span data-cid="f" style="float:left;width:30px;height:20px"></span><i data-cid="i" style="display:inline-block;width:10px;height:10px"></i></div>`) },
  { id: "float-span-shrinks-to-content", html: DOC("body{margin:0}", `<div data-cid="d"><span data-cid="f" style="float:left"><i style="display:block;width:30px;height:20px"></i><i style="display:block;width:20px;height:20px"></i></span></div>`) },
  { id: "float-in-inline", html: DOC("body{margin:0}", `<div data-cid="d" style="font-size:0"><a data-cid="a" href="#"><span data-cid="f" style="float:right;width:30px;height:20px"></span></a></div>`) },
  { id: "float-span-mid-line", html: DOC("body{margin:0}", `<div data-cid="d" style="font-size:0"><i data-cid="i" style="display:inline-block;width:10px;height:10px"></i><span data-cid="f" style="float:left;width:30px;height:20px"></span><i data-cid="j" style="display:inline-block;width:10px;height:10px"></i></div>`) },
  { id: "float-in-line-wraps-following-lines", html: DOC("body{margin:0}", `<div data-cid="d" style="width:100px;font-size:0"><i data-cid="a" style="display:inline-block;width:30px;height:10px"></i><span data-cid="f" style="float:right;width:40px;height:25px"></span><i data-cid="b" style="display:inline-block;width:30px;height:10px"></i><i data-cid="c" style="display:inline-block;width:30px;height:10px"></i><i data-cid="e" style="display:inline-block;width:30px;height:10px"></i><i data-cid="g" style="display:inline-block;width:30px;height:10px"></i></div>`) },
  { id: "inline-table-is-inline-level", html: DOC("body{margin:0}", `<div data-cid="d" style="font-size:0"><div data-cid="t" style="display:inline-table;width:40px;height:20px"></div><div data-cid="t2" style="display:inline-table;width:40px;height:20px"></div></div>`) },
  { id: "inline-table-cells", html: DOC("body{margin:0}", `<div data-cid="d" style="font-size:0"><i style="display:inline-block;width:15px;height:10px"></i><table data-cid="t" style="display:inline-table;border-spacing:2px"><tr><td data-cid="td" style="padding:1px"><div style="width:30px;height:10px"></div></td></tr></table></div>`) },
  { id: "inline-table-blockified", html: DOC("body{margin:0}", `<div data-cid="t" style="display:inline-table;float:left;width:40px;height:20px"></div><div data-cid="u" style="display:inline-table;position:absolute;left:100px;top:50px;width:40px;height:20px"></div><span data-cid="v" style="display:inline-block;float:right;width:10px;height:10px"></span>`) },
  // turbo-dom#28 item 5: an empty row group (`<tbody></tbody>`) is a zero-height box as wide
  // as the rows, at the bottom of the rows before it (the first row edge when none).
  { id: "empty-row-group-only", html: DOC("body{margin:0}", `<table data-cid="t"><tbody data-cid="b"></tbody></table>`) },
  { id: "empty-row-group-only-framed", html: DOC("body{margin:0}table{border-spacing:5px 7px;padding:3px;border:2px solid}", `<table data-cid="t"><tbody data-cid="b"></tbody></table>`) },
  { id: "empty-row-group-before-rows", html: DOC("body{margin:0}table{border-spacing:5px 7px}", `<table data-cid="t"><thead data-cid="h"></thead><tbody data-cid="b"><tr data-cid="r"><td data-cid="c"><div style="width:30px;height:10px"></div></td></tr></tbody></table>`) },
  { id: "empty-row-group-between", html: DOC("body{margin:0}table{border-spacing:5px 7px}", `<table data-cid="t"><tbody data-cid="b1"><tr><td><div style="width:30px;height:10px"></div></td></tr></tbody><tbody data-cid="e"></tbody><tbody data-cid="b2"><tr><td><div style="width:30px;height:10px"></div></td></tr></tbody></table>`) },
  { id: "empty-row-group-after-rows", html: DOC("body{margin:0}table{border-spacing:5px 7px}", `<table data-cid="t"><tbody data-cid="b"><tr><td><div style="width:30px;height:10px"></div></td></tr></tbody><tfoot data-cid="f"></tfoot></table>`) },
  { id: "empty-row-group-collapsed-fixed", html: DOC("body{margin:0}table{border-collapse:collapse;table-layout:fixed;width:200px}td{border:2px solid}", `<table data-cid="t"><tbody data-cid="b"><tr><td><div style="width:30px;height:10px"></div></td></tr></tbody><tbody data-cid="e"></tbody></table>`) },
  // turbo-dom#28 item 6: a line whose only content is absolutely positioned (or floated)
  // boxes, collapsible spaces and empty inlines is a zero-height line box (CSS 2.1 §9.4.2).
  { id: "abs-only-line-in-inline", html: DOC("body{margin:0}", `<div data-cid="d"><span><span data-cid="a" style="position:absolute;width:5px;height:5px"></span></span></div><div data-cid="e" style="height:5px"></div>`) },
  { id: "abs-only-line-with-collapsible-spaces", html: DOC("body{margin:0}", `<div data-cid="d"> <span>  <i data-cid="a" style="position:absolute;width:5px;height:5px"></i> </span> <b data-cid="b" style="position:absolute;left:20px;width:5px;height:5px"></b> </div><div data-cid="e" style="height:5px"></div>`) },
  { id: "abs-only-line-between-empty-inlines", html: DOC("body{margin:0}", `<div data-cid="d"><span></span><b data-cid="a" style="position:absolute;width:5px;height:5px"></b><span></span></div><div data-cid="e" style="height:5px"></div>`) },
  { id: "float-only-line-in-inline", html: DOC("body{margin:0}", `<div data-cid="d"><a href="#"><span data-cid="f" style="float:left;width:30px;height:20px"></span></a></div><div data-cid="e" style="height:5px"></div>`) },
  // turbo-dom#28 item 7: shrink-to-fit auto-width tables on every path — more shapes (the
  // table-auto-* cases above are turbo-surf#36 item 3's).
  { id: "table-auto-fixed-layout", html: DOC("body{margin:0}table{table-layout:fixed}", `<table data-cid="table"><tr><td data-cid="a"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-fixed-collapsed", html: DOC("body{margin:0}table{table-layout:fixed;border-collapse:collapse}td{border:1px solid #000}", `<table data-cid="table"><tr><td data-cid="a" style="width:50px"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-px-cell", html: DOC("body{margin:0}", `<table data-cid="table"><tr><td data-cid="a" style="width:100px"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-min-width", html: DOC("body{margin:0}table{min-width:100px}", `<table data-cid="table"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-max-width-floors-at-content", html: DOC("body{margin:0}table{max-width:20px}", `<table data-cid="table"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-auto-in-flex-item", html: DOC("body{margin:0}div{display:flex}", `<div><table data-cid="table"><tr><td data-cid="td"><div style="width:30px;height:10px"></div></td></tr></table></div>`) },
  { id: "table-auto-nested", html: DOC("body{margin:0}", `<table data-cid="outer"><tr><td data-cid="cell"><table data-cid="inner"><tr><td><div style="width:30px;height:10px"></div></td></tr></table></td></tr></table>`) },
  // turbo-dom#53: `inline-flex` / `inline-grid` are atomic inline-level boxes (shrink-to-fit,
  // in the line); blockified (float / absolute) they are `flex` / `grid`.
  { id: "inline-flex-is-inline-level", html: DOC("body{margin:0}", `<div data-cid="d" style="font-size:0"><div data-cid="a" style="display:inline-flex;width:40px;height:20px"></div><div data-cid="b" style="display:inline-flex;width:40px;height:20px"></div></div>`) },
  { id: "inline-flex-shrinks-to-content", html: DOC("body{margin:0}", `<div data-cid="d" style="font-size:0"><span data-cid="a" style="display:inline-flex"><i style="display:block;width:30px;height:10px"></i><i style="display:block;width:20px;height:10px"></i></span><i data-cid="b" style="display:inline-block;width:10px;height:10px"></i></div>`) },
  { id: "inline-grid-is-inline-level", html: DOC("body{margin:0}", `<div data-cid="d" style="font-size:0"><div data-cid="a" style="display:inline-grid;width:40px;height:20px"></div><div data-cid="b" style="display:inline-grid;grid-template-columns:15px 25px"><i style="display:block;height:10px"></i><i style="display:block;height:10px"></i></div></div>`) },
  { id: "inline-flex-grid-blockified", html: DOC("body{margin:0}", `<div data-cid="f" style="display:inline-flex;float:left;width:40px;height:20px"></div><div data-cid="g" style="display:inline-grid;position:absolute;left:100px;top:50px;width:40px;height:20px"></div>`) },
  // turbo-dom#48: an absolutely positioned non-replaced `width: auto` box is shrink-to-fit
  // (CSS 2.1 §10.3.7) unless both `left` and `right` are set (then it stretches between them).
  { id: "abs-auto-width-shrinks-to-fit", html: DOC("body{margin:0}div div{height:10px}", `<div data-cid="a" style="position:absolute;left:0;top:0"><div style="width:30px"></div></div><div data-cid="b" style="position:absolute;right:0;top:20px"><div style="width:30px"></div></div><div data-cid="c" style="position:absolute;top:40px"><div style="width:30px"></div></div><div data-cid="d" style="position:absolute;left:0;right:0;top:60px"><div style="width:30px"></div></div><div data-cid="e" style="position:absolute;left:0;top:80px;padding:0 5px;border:2px solid"><div style="width:30px"></div></div>`) },
  { id: "abs-auto-width-min-max", html: DOC("body{margin:0}div div{height:10px}", `<div data-cid="a" style="position:absolute;left:0;top:0;min-width:50px"><div style="width:30px"></div></div><div data-cid="b" style="position:absolute;left:0;top:20px;max-width:20px"><div style="width:30px"></div></div>`) },
  { id: "abs-auto-width-wider-than-cb", html: DOC("body{margin:0}", `<div data-cid="p" style="position:relative;width:200px;height:50px"><div data-cid="a" style="position:absolute;left:10px;top:0"><div style="width:300px;height:10px"></div></div><div data-cid="b" style="position:absolute;left:50px;top:20px"><div style="display:inline-block;width:100px;height:10px"></div><div style="display:inline-block;width:100px;height:10px"></div></div></div>`), measure: { b: "xyw" } },
  { id: "fixed-auto-width-shrinks", html: DOC("body{margin:0}", `<div data-cid="f" style="position:fixed;right:10px;top:10px"><div style="width:25px;height:15px"></div></div>`) },
  { id: "abs-auto-width-margins", html: DOC("body{margin:0}", `<div data-cid="a" style="position:absolute;left:0;top:0;margin:0 4px 0 6px"><div style="width:30px;height:10px"></div></div><div data-cid="b" style="position:absolute;right:0;top:20px;margin:0 4px 0 6px"><div style="width:30px;height:10px"></div></div>`) },
  { id: "abs-auto-width-both-insets", html: DOC("body{margin:0}", `<div style="position:relative;width:300px;height:40px"><div data-cid="a" style="position:absolute;left:10px;right:20px;top:0;padding:0 3px"><div style="width:30px;height:10px"></div></div><div data-cid="b" style="position:absolute;left:10px;right:20px;top:20px;margin:0 5px"><div style="width:30px;height:10px"></div></div></div>`) },
  // turbo-dom#50: a percentage cell width makes its column that share of the table, and
  // an auto table's max-content grows so it can (unbounded at >= 100% beside other content).
  { id: "table-percent-column-half", html: DOC("body{margin:0}", `<table data-cid="t"><tr><td data-cid="a" style="width:50%"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-percent-column-full", html: DOC("body{margin:0}", `<table data-cid="t"><tr><td data-cid="a" style="width:100%"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-percent-columns-two-quarters", html: DOC("body{margin:0}", `<table data-cid="t"><tr><td data-cid="a" style="width:25%"><div style="width:30px;height:10px"></div></td><td data-cid="b" style="width:25%"><div style="width:20px;height:10px"></div></td><td data-cid="c"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-percent-column-explicit-table-width", html: DOC("body{margin:0}table{width:400px}", `<table data-cid="t"><tr><td data-cid="a" style="width:25%"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td><td data-cid="c"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-percent-column-attribute", html: DOC("body{margin:0}", `<table data-cid="t" width="300"><tr><td data-cid="a" width="60%"><div style="width:10px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  // turbo-dom#49: an auto-layout `border-collapse: collapse` table merges shared borders onto
  // the grid lines (one border per line, half on each side) like a fixed-layout one.
  { id: "table-collapse-auto-cells", html: DOC("body{margin:0}table{border-collapse:collapse}td{border:2px solid #000;padding:3px}", `<table data-cid="t"><tr><td data-cid="a"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr><tr><td data-cid="c"><div style="width:10px;height:5px"></div></td><td data-cid="d"><div style="width:40px;height:5px"></div></td></tr></table>`) },
  { id: "table-collapse-auto-table-border", html: DOC("body{margin:0}table{border-collapse:collapse}td{border:2px solid #000;padding:3px}table{border:4px solid #000}", `<table data-cid="t"><tr><td data-cid="a"><div style="width:30px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-collapse-auto-mixed-borders", html: DOC("body{margin:0}table{border-collapse:collapse}td{padding:1px}", `<table data-cid="t"><tr><td data-cid="a" style="border:6px solid #000"><div style="width:30px;height:10px"></div></td><td data-cid="b" style="border:1px solid #000"><div style="width:20px;height:10px"></div></td></tr><tr><td data-cid="c" style="border:1px solid #000"><div style="width:10px;height:10px"></div></td><td data-cid="d" style="border:1px solid #000"><div style="width:10px;height:10px"></div></td></tr></table>`) },
  { id: "table-collapse-auto-colspan", html: DOC("body{margin:0}table{border-collapse:collapse}td{border:2px solid #000;padding:3px}", `<table data-cid="t"><tr><td data-cid="s" colspan="2"><div style="width:100px;height:10px"></div></td></tr><tr><td data-cid="a"><div style="width:20px;height:10px"></div></td><td data-cid="b"><div style="width:20px;height:10px"></div></td></tr></table>`) },
  { id: "table-collapse-auto-row-groups", html: DOC("body{margin:0}table{border-collapse:collapse}td{border:2px solid #000;padding:3px}", `<table data-cid="t"><tbody data-cid="e0"></tbody><tbody data-cid="b"><tr data-cid="r"><td data-cid="a"><div style="width:30px;height:10px"></div></td></tr></tbody><tbody data-cid="e"></tbody></table>`) },
  // turbo-dom#56: a spanning cell's excess width spreads over its columns in proportion
  // (evenly when empty), the spacing between them counting towards the span.
  { id: "table-colspan-spreads-in-proportion", html: DOC("body{margin:0}td{padding:0}", `<table data-cid="t"><tr><td data-cid="s" colspan="2"><div style="width:102px;height:10px"></div></td></tr><tr><td data-cid="a"><div style="width:20px;height:10px"></div></td><td data-cid="b"><div style="width:60px;height:10px"></div></td></tr></table>`) },
  { id: "table-colspan-counts-inner-spacing", html: DOC("body{margin:0}td{padding:0}table{border-spacing:5px}", `<table data-cid="t"><tr><td data-cid="s" colspan="2"><div style="width:125px;height:10px"></div></td></tr><tr><td data-cid="a"><div style="width:10px;height:10px"></div></td><td data-cid="b"><div style="width:30px;height:10px"></div></td></tr></table>`) },
  { id: "table-colspan-empty-columns-evenly", html: DOC("body{margin:0}td{padding:0}", `<table data-cid="t"><tr><td data-cid="s" colspan="2"><div style="width:62px;height:10px"></div></td><td data-cid="c"></td></tr><tr><td data-cid="a"></td><td data-cid="b"></td><td><div style="width:10px;height:10px"></div></td></tr></table>`) },
];

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  const { boxes, hits } = await page.evaluate((points) => {
    const boxes = [...document.querySelectorAll("[data-cid]")].map((el) => {
      const b = el.getBoundingClientRect();
      const s = getComputedStyle(el);
      return { cid: el.dataset.cid, x: b.x, y: b.y, width: b.width, height: b.height, display: s.display, position: s.position };
    });
    const hits = points.map(([x, y]) => {
      const el = document.elementFromPoint(x, y);
      return { x, y, hit: el ? el.dataset.cid || el.localName : null };
    });
    return { boxes, hits };
  }, testCase.hits || []);
  const { hits: _points, ...rest } = testCase;
  result.push({ ...rest, measure: testCase.measure || {}, boxes, ...(hits.length ? { hits } : {}) });
}
await browser.close();

const fixture = {
  generator: "scripts/live-geometry-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${result.length} cases -> ${out}`);

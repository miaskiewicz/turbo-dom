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

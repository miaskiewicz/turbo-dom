// Chrome-156 control for turbo-dom-layout's CSSOM-View element metrics and hit testing
// (the coverage phase, C2): the reads turbo-surf answers from `turbo_dom_layout::live`
// snapshots through `cssom_view::{metrics, inline, hit}`:
//   - offsetLeft / offsetTop / offsetParent (`metrics::offset_position`): the body
//     fallback in document coordinates, positioned / transformed / table-cell parents
//     measured from their padding edge, `display: contents` skipped, fixed boxes and
//     `display: none` with no parent;
//   - offsetWidth / offsetHeight and clientLeft / clientTop / clientWidth / clientHeight
//     (`metrics::element_geometry`; `inline::geometry` for a flattened plain inline);
//   - getClientRects() / getBoundingClientRect() of plain inlines over one and several
//     lines (`inline::geometry`);
//   - document.elementsFromPoint() (`hit::elements_at_point`): text hits, overflow and
//     `clip` clipping, pointer-events / visibility, points outside the viewport.
//
// Text is set in the crate's bundled Liberation Serif: the control loads the same TTF
// with @font-face (a data: URL) so Chrome shapes with the font the layout uses.
// For every case, Chrome for Testing 156 (viewport 1280x800) records each `[data-cid]`
// element (document order) as `{cid, offset: [left, top, width, height], parent,
// client: [left, top, width, height], rects: [[x, y, w, h]...], bounds: [x, y, w, h]}`
// (`parent` is the offsetParent's data-cid, else its local name, else null), and for a
// case with `hits` (viewport points) `{x, y, hits}`: elementsFromPoint as data-cid / local
// names. Asserted by crates/turbo-dom-layout/src/cssom_view/metrics_chrome_tests.rs.
//
// Usage: node scripts/cssom-metrics-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/cssom_metrics_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { readFileSync, writeFileSync } from "node:fs";
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/cssom_metrics_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const FONT = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf")).toString("base64");
const FACE = `@font-face{font-family:"Liberation Serif";src:url(data:font/ttf;base64,${FONT})}`;
const DOC = (css, body) => `<!doctype html><style>${css}</style>${body}`;
const TEXT = 'body{margin:8px;font-family:"Liberation Serif";font-size:16px}';
const BOX = (w, h, cid = "", style = "") => `<div${cid ? ` data-cid="${cid}"` : ""} style="width:${w}px;height:${h}px${style ? `;${style}` : ""}"></div>`;

const CASES = [
  // offsetParent / offsetLeft / offsetTop.
  { id: "offset-static-in-body", html: DOC("body{margin:8px}", `${BOX(10, 20, "a")}<div data-cid="w" style="margin-left:5px">${BOX(10, 10, "b")}</div>`) },
  {
    id: "offset-positioned-parent-padding-edge",
    html: DOC("body{margin:0}", `<div data-cid="p" style="position:relative;border:5px solid;padding:3px;margin:10px;width:100px">${BOX(10, 10, "c", "margin-left:4px")}<div data-cid="n">${BOX(5, 5, "g")}</div></div>`),
  },
  {
    id: "offset-transformed-parent",
    html: DOC("body{margin:0}", `<div style="height:7px"></div><div data-cid="t" style="transform:translateX(20px);border:2px solid;padding:1px;width:60px">${BOX(10, 10, "c", "margin-left:3px")}</div>`),
  },
  {
    id: "offset-table-cell-parent",
    html: DOC("body{margin:0}td{padding:4px;border:1px solid}", `<table data-cid="t"><tr data-cid="r"><td data-cid="td">${BOX(10, 10, "c")}</td></tr></table>`),
  },
  {
    id: "offset-display-contents-skipped",
    html: DOC("body{margin:0}", `<div data-cid="r" style="position:relative;left:6px;top:4px"><div data-cid="k" style="display:contents;position:relative">${BOX(10, 10, "c", "margin-top:3px")}</div></div>`),
  },
  {
    id: "offset-fixed-and-hidden",
    html: DOC("body{margin:0}", `<div style="position:relative">${BOX(10, 10, "f", "position:fixed;left:30px;top:40px")}<div data-cid="n" style="display:none">${BOX(5, 5, "nn")}</div></div>`),
  },
  {
    id: "offset-body-and-root",
    html: `<!doctype html><html data-cid="html"><style>html{padding:3px}body{margin:5px}</style><body data-cid="body"><div data-cid="d">${BOX(10, 10, "c")}</div></body></html>`,
  },
  {
    id: "offset-absolute-in-static-body",
    html: DOC("body{margin:8px}", `<div style="margin:10px">${BOX(10, 10, "a", "position:absolute;left:12px;top:14px")}</div>`),
  },
  {
    id: "offset-nested-positioned",
    html: DOC("body{margin:0}", `<div data-cid="o" style="position:absolute;left:20px;top:30px;border:4px solid"><div data-cid="i" style="position:relative;left:2px;top:3px;padding:6px">${BOX(8, 8, "c")}</div></div>`),
  },
  {
    id: "offset-positioned-inline-parent",
    html: DOC(TEXT, `<p style="margin:0">x <span data-cid="s" style="position:relative;border-left:3px solid;padding-left:4px">ab<b data-cid="b" style="display:inline-block;width:10px;height:10px"></b></span></p>`),
  },
  {
    // `transform` does not apply to a non-replaced inline: it is no offsetParent.
    id: "offset-transformed-inline-is-not-a-parent",
    html: DOC(TEXT, `<p style="margin:0">x <span data-cid="s" style="transform:translateX(5px)">ab<b data-cid="b" style="display:inline-block;width:10px;height:10px"></b></span></p>`),
  },
  // offsetWidth/Height and client* of boxes.
  {
    id: "client-metrics-boxes",
    html: DOC("body{margin:0}", `<div data-cid="a" style="width:50px;height:20px;padding:3px 5px;border-width:1px 2px 3px 4px;border-style:solid"></div><div data-cid="b" style="box-sizing:border-box;width:40px;height:30px;padding:2px;border:6px solid"></div>`),
  },
  // Plain inlines (inline::geometry): one line, several lines, nested, transformed.
  {
    id: "inline-one-line",
    html: DOC(TEXT, `<p data-cid="p" style="margin:0">Hello <span data-cid="s">brave new</span> world</p>`),
  },
  {
    id: "inline-wraps-over-lines",
    html: DOC(TEXT, `<p data-cid="p" style="margin:0;width:120px">Some words <span data-cid="s">that wrap over two lines here</span> end</p>`),
  },
  {
    id: "inline-nested-same-font",
    html: DOC(TEXT, `<div data-cid="d">a <span data-cid="o">outer <span data-cid="i">inner</span> tail</span></div>`),
  },
  {
    id: "inline-in-transformed-block",
    html: DOC(TEXT, `<div data-cid="d" style="transform:translate(30px,10px)">x <span data-cid="s">moved text</span></div>`),
  },
  {
    // A line of only preserved spaces (and an empty inline) keeps its line box.
    id: "preserved-space-only-line",
    html: DOC(TEXT, `<div data-cid="d" style="white-space:pre"><span data-cid="s"></span>   </div>${BOX(10, 5, "e")}`),
  },
  // elementsFromPoint.
  {
    id: "hits-boxes-and-outside-viewport",
    html: DOC("body{margin:0}", `${BOX(100, 50, "a")}<div data-cid="b" style="position:absolute;left:50px;top:20px;width:100px;height:60px"></div>`),
    hits: [[10, 10], [60, 30], [120, 70], [500, 500], [0, 0], [0, 10], [10, 0], [-1, 10], [10, -1], [1280, 10], [10, 800], [1279, 799]],
  },
  {
    id: "hits-text",
    html: DOC(TEXT, `<p data-cid="p" style="margin:0">Hello <span data-cid="s">brave new</span> world</p>`),
    hits: [[3, 9], [12, 9], [60, 9], [300, 9], [60, 40]],
  },
  {
    // Text hits its inline ancestors up to the first non-inline one (whose box hits).
    id: "hits-nested-inline-text",
    html: DOC(TEXT, `<p data-cid="p" style="margin:0"><span data-cid="o">a <span data-cid="i">bold</span></span> <span data-cid="ib" style="display:inline-block;width:30px;height:10px"></span></p>`),
    hits: [[20, 9], [3, 9], [60, 9]],
  },
  {
    // miaskiewicz/turbo-dom#142: an inline-block's baseline is its last line box's.
    id: "inline-block-text-baseline",
    html: DOC(TEXT, `<p data-cid="p" style="margin:0"><span data-cid="o">a</span> <span data-cid="ib" style="display:inline-block"><span data-cid="t">box</span></span></p>`),
    hits: [[3, 9], [12, 9], [30, 9]],
  },
  {
    // Text overflowing its block hits its inline ancestors but not the block (whose box
    // the point is outside); a display:contents ancestor has no box of its own.
    id: "hits-overflowing-text",
    html: DOC(TEXT, `<p data-cid="p" style="margin:0;width:10px;white-space:nowrap">Hello <span data-cid="s">world</span></p><div data-cid="d" style="width:10px;white-space:nowrap"><span data-cid="c" style="display:contents">contents text</span></div>`),
    hits: [[5, 9], [30, 9], [50, 9], [200, 9], [5, 27], [40, 27]],
  },
  {
    id: "hits-hidden-text",
    html: DOC(TEXT, `<p data-cid="p" style="margin:0"><span data-cid="s" style="visibility:hidden">Hello</span> <span data-cid="v">world</span></p>`),
    hits: [[10, 9], [50, 9]],
  },
  {
    id: "hits-overflow-clip",
    html: DOC("body{margin:0}", `<div data-cid="c" style="width:50px;height:50px;overflow:hidden;border:5px solid">${BOX(200, 200, "k")}</div>`),
    hits: [[20, 20], [52, 20], [57, 20], [100, 20], [20, 100]],
  },
  {
    id: "hits-overflow-clip-text",
    html: DOC(TEXT, `<div data-cid="c" style="width:30px;height:18px;overflow:hidden;white-space:nowrap"><span data-cid="s">clipped text that overflows</span></div>`),
    hits: [[10, 9], [100, 9]],
  },
  {
    id: "hits-pointer-events-visibility",
    html: DOC(
      "body{margin:0}",
      `${BOX(100, 30, "a")}${BOX(100, 30, "b", "pointer-events:none")}<div data-cid="v" style="visibility:hidden;width:100px;height:30px">${BOX(50, 10, "vv", "visibility:visible")}</div>`,
    ),
    hits: [[10, 10], [10, 40], [10, 65], [70, 65]],
  },
  {
    id: "hits-text-pointer-events-none",
    html: DOC(TEXT, `<p data-cid="p" style="margin:0"><span data-cid="s" style="pointer-events:none">Hello</span> world</p>`),
    hits: [[10, 9]],
  },
  {
    id: "hits-display-contents-overflow",
    html: DOC("body{margin:0}", `<div data-cid="k" style="display:contents;overflow:hidden">${BOX(50, 50, "a")}</div>`),
    hits: [[10, 10], [60, 10]],
  },
  {
    id: "hits-list-marker",
    html: DOC(TEXT, `<ol style="margin:0"><li data-cid="li">item</li></ol>`),
    hits: [[30, 9], [45, 9]],
  },
  {
    id: "hits-clip-rect",
    html: DOC("body{margin:0}", `<div data-cid="a" style="position:absolute;left:0;top:0;width:100px;height:100px;clip:rect(10px,60px,60px,10px)">${BOX(100, 100, "k")}</div>`),
    hits: [[5, 5], [20, 20], [70, 20], [20, 70]],
  },
];

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  await page.addStyleTag({ content: FACE });
  await page.evaluate(() => document.fonts.ready);
  const { boxes, hits } = await page.evaluate((points) => {
    const name = (el) => (el ? el.dataset.cid || el.localName : null);
    const rect = (r) => [r.x, r.y, r.width, r.height];
    const boxes = [...document.querySelectorAll("[data-cid]")].map((el) => ({
      cid: el.dataset.cid,
      offset: [el.offsetLeft, el.offsetTop, el.offsetWidth, el.offsetHeight],
      parent: name(el.offsetParent),
      client: [el.clientLeft, el.clientTop, el.clientWidth, el.clientHeight],
      rects: [...el.getClientRects()].map(rect),
      bounds: rect(el.getBoundingClientRect()),
    }));
    const hits = points.map(([x, y]) => ({ x, y, hits: document.elementsFromPoint(x, y).map(name) }));
    return { boxes, hits };
  }, testCase.hits || []);
  const { hits: _points, ...rest } = testCase;
  result.push({ ...rest, boxes, ...(hits.length ? { hits } : {}) });
}
await browser.close();

const fixture = {
  generator: "scripts/cssom-metrics-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${result.length} cases -> ${out}`);

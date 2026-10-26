// Chrome-156 control for turbo-dom-layout's live-DOM geometry (P5b-W1): the CSSOM-View
// reads turbo-surf answers from `turbo_dom_layout::live` snapshots
// (`cssom_view::metrics::element_geometry`, falling back to `cssom_view::inline::geometry`).
//   - miaskiewicz/turbo-surf#31: an absolutely/fixed positioned inline is blockified
//     (CSS 2.1 §9.7) and laid out at its static position without splitting its inline
//     ancestor, and an unsupported inline construct fails that element only, instead of
//     failing the whole snapshot; a table row is a box spanning its cells (inside the
//     border-spacing) with used box metrics.
//
// For every case, Chrome for Testing 156 (viewport 1280x800) records the
// getBoundingClientRect() of every `[data-cid]` element (document order) as
// `{cid, x, y, width, height, display, position}` (the last two from getComputedStyle).
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
];

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  const boxes = await page.evaluate(() =>
    [...document.querySelectorAll("[data-cid]")].map((el) => {
      const b = el.getBoundingClientRect();
      const s = getComputedStyle(el);
      return { cid: el.dataset.cid, x: b.x, y: b.y, width: b.width, height: b.height, display: s.display, position: s.position };
    }),
  );
  result.push({ ...testCase, measure: testCase.measure || {}, boxes });
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

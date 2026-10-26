// Chrome-156 control for the HTML drive's document model (P5b, miaskiewicz/turbo-html2pdf#29):
// `<head>` styles apply, and `<html>`/`<body>` are real boxes with Chrome's UA semantics
// (`body { margin: 8px }`, root margins never collapse, `rem` against the root font size,
// the root/body background propagated to the canvas). P5b-W1 adds the cascade cases of
// miaskiewicz/turbo-dom#24 item 1 (`@media` / `@supports` rules in source order).
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

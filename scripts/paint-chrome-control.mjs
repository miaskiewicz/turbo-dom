// Chrome-156 pixel control for turbo-dom-raster's screenshots (P5b-W5):
//   - miaskiewicz/turbo-dom#51 (`caption-*`): a table caption is laid out in the table
//     wrapper box, above (`caption-side: top`) or below (`bottom`) the table grid box; the
//     table's background and border paint the grid box only, never the caption.
//
// Every case is one page (viewport 1280x800, `body{margin:0;background:#00ffff}`). Chrome for
// Testing 156 records, per case, the getBoundingClientRect() of every `[data-cid]` element
// (`boxes`: `{cid, x, y, width, height}`) and the colour class at each listed probe point
// (`probes`: `{x, y, class}`) of a screenshot: R/G/B/Y/W, T = the page's #00ffff backdrop,
// `?` = anything else. `assets` lists the case's `<img src>` `data:` URLs (the screenshot API
// takes image bytes from the caller, keyed by the raw `src`). Asserted by
// crates/turbo-dom-raster/tests/paint_chrome.rs (pixels) and
// crates/turbo-dom-layout/tests/html_document_chrome.rs (boxes, through `layout_boxes`).
//
// Usage: node scripts/paint-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/paint_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { mkdirSync, writeFileSync } from "node:fs";
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/paint_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DOC = (body) =>
  `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>${body}</body></html>`;
const at = (x, y) => `position:absolute;left:${x}px;top:${y}px`;
// A text-free coloured block.
const BLOCK = (w, h, bg, cid = "") =>
  `<div${cid ? ` data-cid="${cid}"` : ""} style="width:${w}px;height:${h}px${bg ? `;background:${bg}` : ""}"></div>`;
// A red table with a 4px blue border, no spacing, one 40x20 cell (no background).
const TABLE = (style, caption, cellStyle = "") =>
  `<table data-cid="t" style="${style};background:#ff0000;border:4px solid #0000ff;border-spacing:0">${caption}<tr><td data-cid="td" style="padding:0${cellStyle}">${BLOCK(40, 20, "")}</td></tr></table>`;

const CASES = [
  // miaskiewicz/turbo-dom#51.
  {
    // The caption (100 wide, 5px margins) sits above the grid; the grid is stretched to the
    // wrapper's 110px; the table's red background and blue border stay off the caption and
    // its margins.
    id: "caption-top",
    html: DOC(TABLE(at(20, 10), `<caption data-cid="c" style="margin:5px">${BLOCK(100, 20, "#00ff00", "cd")}</caption>`)),
    probes: [[70, 25], [22, 12], [70, 36], [120, 25], [60, 47], [110, 47], [21, 47], [70, 65]],
  },
  {
    id: "caption-bottom",
    html: DOC(TABLE(`${at(20, 10)};caption-side:bottom`, `<caption data-cid="c">${BLOCK(100, 20, "#00ff00", "cd")}</caption>`)),
    probes: [[70, 20], [110, 20], [21, 20], [70, 45], [70, 55], [118, 45], [70, 9], [70, 59]],
  },
  {
    // A caption narrower than the grid takes the grid's width; the grid keeps its own.
    id: "caption-narrow",
    html: DOC(TABLE(`${at(20, 10)};width:120px`, `<caption data-cid="c">${BLOCK(30, 10, "#00ff00", "cd")}</caption>`)),
    probes: [[30, 15], [100, 15], [100, 30], [130, 30], [139, 30]],
  },
];

/** Decode an 8-bit RGB/RGBA PNG (all five filters) to px(x, y) -> [r,g,b]. */
function decodePng(png) {
  let off = 8;
  let idat = Buffer.alloc(0);
  let width = 0;
  let height = 0;
  let channels = 3;
  while (off < png.length) {
    const len = png.readUInt32BE(off);
    const type = png.toString("latin1", off + 4, off + 8);
    const data = png.subarray(off + 8, off + 8 + len);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      channels = data[9] === 6 ? 4 : 3;
    }
    if (type === "IDAT") idat = Buffer.concat([idat, data]);
    off += 12 + len;
  }
  const raw = inflateSync(idat);
  const stride = width * channels;
  const pix = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    for (let i = 0; i < stride; i++) {
      const x = raw[y * (stride + 1) + 1 + i];
      const a = i >= channels ? pix[y * stride + i - channels] : 0;
      const b = y > 0 ? pix[(y - 1) * stride + i] : 0;
      const c = i >= channels && y > 0 ? pix[(y - 1) * stride + i - channels] : 0;
      let v;
      if (filter === 0) v = x;
      else if (filter === 1) v = x + a;
      else if (filter === 2) v = x + b;
      else if (filter === 3) v = x + ((a + b) >> 1);
      else {
        const p = a + b - c;
        const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
        v = x + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c);
      }
      pix[y * stride + i] = v & 255;
    }
  }
  return (x, y) => [...pix.subarray(y * stride + x * channels, y * stride + x * channels + 3)];
}

function classOf([r, g, b]) {
  const hi = (v) => v > 160, lo = (v) => v < 96;
  if (hi(r) && hi(g) && hi(b)) return "W";
  if (lo(r) && hi(g) && hi(b)) return "T";
  if (hi(r) && lo(g) && lo(b)) return "R";
  if (lo(r) && hi(g) && lo(b)) return "G";
  if (lo(r) && lo(g) && hi(b)) return "B";
  if (hi(r) && hi(g) && lo(b)) return "Y";
  return "?";
}

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const cases = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  const boxes = await page.evaluate(async () => {
    for (const img of document.images) await img.decode().catch(() => {});
    const r = (n) => Math.round(n * 1000) / 1000;
    return [...document.querySelectorAll("[data-cid]")].map((el) => {
      const b = el.getBoundingClientRect();
      return { cid: el.dataset.cid, x: r(b.x), y: r(b.y), width: r(b.width), height: r(b.height) };
    });
  });
  const px = decodePng(await page.screenshot());
  const probes = testCase.probes.map(([x, y]) => ({ x, y, class: classOf(px(x, y)) }));
  cases.push({ id: testCase.id, html: testCase.html, assets: testCase.assets || [], boxes, probes });
}
await browser.close();

const fixture = {
  generator: "scripts/paint-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases,
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) {
  console.log(c.id.padEnd(22), c.probes.map((p) => `${p.x},${p.y}:${p.class}`).join(" "));
  console.log("".padEnd(22), c.boxes.map((b) => `${b.cid}:${b.x},${b.y} ${b.width}x${b.height}`).join("  "));
}

// Chrome-156 pixel control for turbo-dom-raster's screenshots (P5b-W5):
//   - miaskiewicz/turbo-dom#51 (`caption-*`): a table caption is laid out in the table
//     wrapper box, above (`caption-side: top`) or below (`bottom`) the table grid box; the
//     table's background and border paint the grid box only, never the caption;
//   - miaskiewicz/turbo-dom#52 (`clip-*`): an absolutely positioned box's `clip: rect()`
//     clips the box and its descendants; a zero-area clip paints nothing (the box keeps its
//     border box, miaskiewicz/turbo-dom#75); `clip` does not
//     apply to a box that is not absolutely positioned.
//
// Every case is one page (viewport 1280x800, `body{margin:0;background:#00ffff}`). Chrome for
// Testing 156 records, per case, the getBoundingClientRect() of every `[data-cid]` element
// (`boxes`: `{cid, x, y, width, height}`) and the colour class at each listed probe point
// (`probes`: `{x, y, class, rgb}`) of a screenshot: R/G/B/Y/W, T = the page's #00ffff
// backdrop, `?` = anything else; `rgb` is the raw screenshot colour there. `assets` lists the case's `<img src>` `data:` URLs (the screenshot API
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
// A 100x40 PNG sprite: left half red, right half blue.
const SPRITE =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAGQAAAAoCAIAAACHGsgUAAAAU0lEQVR42u3QMQ0AAAzDsPIn3cFYD0sBEDlNBpucamDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggUL1ncHs7SRQvFoC8gAAAAASUVORK5CYII=";

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
  // miaskiewicz/turbo-dom#52.
  {
    // The issue's repro: a 30x30 red square at (10,10); the border box stays 50x50.
    id: "clip-partial",
    html: DOC(`<div style="position:relative;height:100px"><div data-cid="c" style="${at(0, 0)};width:50px;height:50px;background:#ff0000;clip:rect(10px,40px,40px,10px)"></div></div>`),
    probes: [[25, 25], [11, 11], [38, 38], [5, 5], [45, 45], [25, 5], [25, 45], [5, 25], [45, 25]],
  },
  {
    // `auto` is the border-box edge; commas are optional.
    id: "clip-auto-edges",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:60px;height:40px;background:#ff0000;clip:rect(auto 30px auto auto)"></div>`),
    probes: [[25, 30], [48, 58], [55, 30], [75, 50]],
  },
  {
    // The clip applies to the descendants: an overflowing child is cut at the clip edge.
    id: "clip-descendants",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:50px;height:50px;clip:rect(0,60px,60px,0)">${BLOCK(100, 100, "#00ff00", "k")}</div>`),
    probes: [[50, 50], [75, 75], [100, 50], [50, 100], [110, 110]],
  },
  {
    // A transformed box clips in its own (transformed) coordinate space.
    id: "clip-transformed",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:50px;height:50px;background:#ff0000;transform:translate(100px,0);clip:rect(0,25px,50px,0)"></div>`),
    probes: [[130, 45], [160, 45], [40, 45]],
  },
  {
    // The sr-only pattern paints nothing.
    id: "clip-zero-area",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:50px;height:50px;background:#ff0000;clip:rect(0,0,0,0)"></div><div style="${at(100, 20)};width:1px;height:1px;overflow:hidden;clip:rect(1px,1px,1px,1px)">${BLOCK(40, 40, "#ff0000")}</div>`),
    probes: [[45, 45], [110, 30], [101, 21]],
  },
  {
    // A zero-width clip whose left edge is past the box's right edge (`auto` right).
    id: "clip-zero-area-auto",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:50px;height:50px;background:#ff0000;clip:rect(0,auto,auto,60px)"></div>`),
    probes: [[45, 45], [25, 25]],
  },
  {
    // `clip` applies only to absolutely positioned boxes: the static and relative boxes paint
    // whole.
    id: "clip-not-positioned",
    html: DOC(`<div data-cid="s" style="width:50px;height:50px;background:#ff0000;clip:rect(0,0,0,0)"></div><div data-cid="r" style="position:relative;width:50px;height:50px;background:#0000ff;clip:rect(0,10px,10px,0)"></div>`),
    probes: [[25, 25], [25, 75], [40, 90]],
  },
  {
    // The CSS-sprite pattern: a clipped `<img>` shows only its left half.
    id: "clip-image-sprite",
    html: DOC(`<img data-cid="i" src="${SPRITE}" style="${at(20, 20)};clip:rect(0,50px,40px,0)">`),
    assets: [SPRITE],
    probes: [[45, 40], [95, 40], [21, 21], [69, 59]],
  },
  // P5-cov: the raster painter's box decoration paths.
  {
    // A rounded box with a uniform border: the border band follows the radius and the
    // corners outside it stay the backdrop.
    id: "paint-round-border",
    html: DOC(`<div style="${at(20, 20)};width:60px;height:40px;background:#ff0000;border:4px solid #0000ff;border-radius:12px"></div>`),
    probes: [[54, 44], [54, 21], [21, 44], [86, 44], [54, 66], [21, 21], [86, 66]],
  },
  {
    // A linear gradient clipped to the rounded border box.
    id: "paint-round-gradient",
    html: DOC(`<div style="${at(20, 90)};width:100px;height:50px;border-radius:20px;background:linear-gradient(to right, #ff0000, #0000ff)"></div>`),
    probes: [[25, 115], [115, 115], [70, 93], [21, 91], [118, 138]],
  },
  {
    // An unblurred offset shadow is the box's silhouette in the shadow colour.
    id: "paint-shadow-hard",
    html: DOC(`<div style="${at(20, 160)};width:40px;height:40px;background:#00ff00;box-shadow:10px 10px 0 #ff0000"></div>`),
    probes: [[40, 180], [65, 205], [25, 205], [65, 165]],
  },
  {
    // A negative spread larger than half the box leaves no shadow.
    id: "paint-shadow-collapsed",
    html: DOC(`<div style="${at(100, 160)};width:20px;height:20px;background:#00ff00;box-shadow:30px 0 0 -15px #ff0000"></div>`),
    probes: [[110, 170], [135, 170], [145, 170]],
  },
  {
    // A spread grows the rounded silhouette (radius + spread) on every side.
    id: "paint-shadow-spread-round",
    html: DOC(`<div style="${at(200, 170)};width:40px;height:40px;background:#00ff00;border-radius:10px;box-shadow:0 0 0 6px #ff0000"></div>`),
    probes: [[196, 190], [220, 190], [243, 190], [220, 166], [195, 165]],
  },
  {
    // Transparent text paints nothing over its box.
    id: "paint-text-transparent",
    html: DOC(`<div style="${at(300, 20)};font-size:40px;color:transparent;background:#00ff00">MMMM</div>`),
    probes: [[310, 40], [330, 45], [350, 40], [370, 45]],
  },
  {
    // Blurred shadows, square and rounded: the shadow's core is its colour, the blur
    // fades out past its edge.
    id: "paint-shadow-blur",
    html: DOC(
      `<div style="${at(300, 100)};width:40px;height:40px;background:#00ff00;box-shadow:50px 0 4px #ff0000"></div>` +
        `<div style="${at(300, 160)};width:40px;height:40px;background:#00ff00;border-radius:8px;box-shadow:50px 0 4px #0000ff"></div>`,
    ),
    probes: [[320, 120], [370, 120], [400, 120], [370, 92], [370, 180], [400, 180], [352, 162]],
  },
  {
    // Translucent colours: a half-transparent background and shadow over the backdrop
    // (`rgb` carries the composited values).
    id: "paint-alpha",
    html: DOC(`<div style="${at(420, 20)};width:40px;height:40px;background:rgba(255,0,0,0.5);box-shadow:0 50px 0 rgba(0,0,255,0.5)"></div>`),
    probes: [[440, 40], [440, 90], [440, 130]],
  },
  {
    // A gradient in a square box (top to bottom).
    id: "paint-gradient-square",
    html: DOC(`<div style="${at(500, 20)};width:40px;height:100px;background:linear-gradient(#ff0000, #0000ff)"></div>`),
    probes: [[520, 23], [520, 117], [520, 70], [545, 70]],
  },
  // miaskiewicz/turbo-dom#95: blurred shadows of boxes far wider than the viewport, and the
  // blur radius Chrome caps (every blur from 256px up paints the same pixels).
  {
    id: "shadow-wide",
    html: DOC(`<div style="${at(20, 300)};width:9000px;height:40px;background:#00ff00;box-shadow:0 50px 4px #ff0000"></div>`),
    probes: [[100, 320], [100, 370], [10, 370], [100, 400], [1270, 370], [640, 345]],
  },
  {
    // A spread grows the shadow's corner radius by the spread scaled by 1 + (r/s - 1)^3 when
    // the radius r is below the spread s (CSS Backgrounds 3 §7.1): a square box keeps a square
    // shadow; a 2px radius with an 8px spread rounds to about 6.6px, not 10px.
    id: "shadow-spread-small-radius",
    html: DOC(
      `<div style="${at(600, 60)};width:40px;height:40px;background:#00ff00;box-shadow:0 0 0 8px #ff0000"></div>` +
        `<div style="${at(700, 60)};width:40px;height:40px;background:#00ff00;border-radius:2px;box-shadow:0 0 0 8px #ff0000"></div>`,
    ),
    probes: [[593, 53], [646, 53], [593, 106], [646, 106], [620, 53], [694, 54], [745, 54], [694, 105], [745, 105], [720, 53]],
  },
  {
    id: "shadow-blur-256",
    html: DOC(`<div style="${at(600, 300)};width:100px;height:100px;background:#0000ff;box-shadow:0 0 256px #000000"></div>`),
    probes: [[550, 350], [500, 350], [450, 350], [650, 250], [650, 150], [400, 600]],
  },
  {
    id: "shadow-blur-400",
    html: DOC(`<div style="${at(600, 300)};width:100px;height:100px;background:#0000ff;box-shadow:0 0 400px #000000"></div>`),
    probes: [[550, 350], [500, 350], [450, 350], [650, 250], [650, 150], [400, 600]],
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
  const probes = testCase.probes.map(([x, y]) => ({ x, y, class: classOf(px(x, y)), rgb: px(x, y) }));
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

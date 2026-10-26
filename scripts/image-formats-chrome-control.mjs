// Chrome-156 control for the image formats Chrome decodes (P5b, miaskiewicz/turbo-surf#30):
// AVIF, BMP, ICO, CUR and SVG `<img>` sources (base64 and percent-encoded / utf8 `data:`
// URIs), with TIFF as the unsupported control. The images are
// crates/turbo-dom-layout/tests/fixtures/images/formats (scripts/make_format_fixtures.py:
// a 64x40 image in four colour quadrants — top-left red, top-right green, bottom-left
// blue, bottom-right yellow).
//
// One page (viewport 1280x800) shows every source as an absolutely positioned <img>; Chrome
// for Testing 156 records per image: `natural` ([naturalWidth, naturalHeight]), `complete`,
// `event` (load / error), `decoded` (img.decode() resolves / rejects), `rect`
// (getBoundingClientRect) and `samples` — the colour class (R/G/B/Y/W/T(ransparent: the
// page's #00ffff backdrop)/?) at the centres of the four quadrants of the box, read from a
// screenshot. Asserted by crates/turbo-dom-layout/tests/image_formats_chrome.rs (probe +
// box geometry + the `data:` URI decoder) and crates/turbo-dom-raster/tests/
// image_formats_chrome.rs (pixels).
//
// Usage: node scripts/image-formats-chrome-control.mjs [--chrome PATH] [--out PATH]
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { readFileSync, writeFileSync } from "node:fs";
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/image_formats_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DIR = join(repo, "crates/turbo-dom-layout/tests/fixtures/images/formats");
const read = (f) => readFileSync(join(DIR, f));
const b64 = (mime, f) => `data:${mime};base64,${read(f).toString("base64")}`;
const svg = read("q.svg").toString("utf8").trim();
const viewBoxOnly = svg.replace('width="64" height="40"', 'viewBox="0 0 64 40"');

// [id, src]: base64 `data:` URIs of every fixture, plus the SVG as the `data:` forms pages
// use for inline icons (percent-encoded, raw utf8 with only `#` escaped, base64).
const SOURCES = [
  ["avif", b64("image/avif", "q.avif")],
  ["avif-alpha", b64("image/avif", "q-alpha.avif")],
  ["bmp", b64("image/bmp", "q.bmp")],
  ["bmp-alpha", b64("image/bmp", "q-alpha.bmp")],
  ["bmp-8bit", b64("image/bmp", "q-8bit.bmp")],
  ["ico", b64("image/x-icon", "q.ico")],
  ["ico-sizes", b64("image/x-icon", "q-sizes.ico")],
  ["ico-png", b64("image/x-icon", "q-png.ico")],
  ["cur", b64("image/x-icon", "q.cur")],
  ["svg-base64", b64("image/svg+xml", "q.svg")],
  ["svg-percent", `data:image/svg+xml,${encodeURIComponent(svg)}`],
  ["svg-utf8", `data:image/svg+xml;utf8,${svg.replaceAll("#", "%23")}`],
  ["svg-viewbox-only", `data:image/svg+xml,${encodeURIComponent(viewBoxOnly)}`],
  ["svg-width-only", `data:image/svg+xml,${encodeURIComponent(svg.replace('height="40"', 'viewBox="0 0 64 40"'))}`],
  ["svg-fractional", `data:image/svg+xml,${encodeURIComponent(svg.replace('width="64" height="40"', 'width="10.5" height="7.25" viewBox="0 0 64 40"'))}`],
  ["svg-no-size", `data:image/svg+xml,${encodeURIComponent(svg.replace(' width="64" height="40"', ""))}`],
  ["tiff", b64("image/tiff", "q.tiff")],
];

const ROW = 70;
let body = "";
const boxes = SOURCES.map(([id, src], i) => {
  const [x, y] = [20 + (i % 2) * 400, 10 + Math.floor(i / 2) * ROW];
  body += `<img data-cid="${id}" src="${src.replaceAll('"', "&quot;")}" style="position:absolute;left:${x}px;top:${y}px">`;
  return { id, src, x, y };
});
const html = `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>${body}</body></html>`;

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
await page.setContent(html, { waitUntil: "load" });
const measured = await page.evaluate(async () => {
  const out = [];
  for (const el of document.querySelectorAll("img")) {
    const decoded = await el.decode().then(() => true, () => false);
    const r = el.getBoundingClientRect();
    out.push({
      id: el.dataset.cid,
      natural: [el.naturalWidth, el.naturalHeight],
      complete: el.complete,
      // An <img> that failed reports complete with naturalWidth 0 (no load/error
      // listener is needed after `load`: decode() settles the outcome).
      event: decoded ? "load" : "error",
      decoded,
      rect: [r.x, r.y, r.width, r.height],
    });
  }
  return out;
});
const px = decodePng(await page.screenshot({ fullPage: true }));
await browser.close();

const cases = boxes.map((box) => {
  const m = measured.find((e) => e.id === box.id);
  const [x, y, w, h] = m.rect;
  const points = [
    [x + w / 4, y + h / 4],
    [x + (3 * w) / 4, y + h / 4],
    [x + w / 4, y + (3 * h) / 4],
    [x + (3 * w) / 4, y + (3 * h) / 4],
  ].map(([a, b]) => [Math.floor(a), Math.floor(b)]);
  const samples = w > 0 && h > 0 ? points.map(([a, b]) => classOf(px(a, b))).join("") : "";
  return { ...box, ...m, points, samples };
});

const fixture = {
  generator: "scripts/image-formats-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  html,
  cases,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} images -> ${out}`);
for (const c of cases) console.log(c.id.padEnd(18), JSON.stringify(c.natural), c.event, c.rect.join(","), c.samples);

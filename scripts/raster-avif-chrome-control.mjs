// Chrome-156 pixel control for turbo-dom-raster's AVIF decode (`avif` feature, P5-cov): the
// AVIFs of scripts/make_raster_avif_fixtures.py (crates/turbo-dom-raster/tests/fixtures/
// images/avif — the 64x40 quadrant image in every chroma layout, each matrix-coefficients
// family, both ranges, and without an `nclx` box) as `<img src="data:…">` over a `#00ffff`
// backdrop. Chrome for Testing 156 records, per image, `natural` ([naturalWidth,
// naturalHeight]), `rect` (getBoundingClientRect) and, at the centres of the four quadrants
// of its box (`points`), the screenshot's colour class (`samples`: R/G/B/Y/W/T = backdrop/?)
// and raw `rgb`. Asserted by crates/turbo-dom-raster/tests/avif_decode_chrome.rs.
//
// Usage: node scripts/raster-avif-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/avif_decode_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/avif_decode_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DIR = join(repo, "crates/turbo-dom-raster/tests/fixtures/images/avif");
// The deep (10/12-bit) images were added later: they come last, so the earlier cells keep
// their places.
const deep = (f) => Number(/-\d+bit-/.test(f));
const files = readdirSync(DIR)
  .filter((f) => f.endsWith(".avif"))
  .sort((a, b) => deep(a) - deep(b) || a.localeCompare(b));
const sources = files.map((f) => [f.replace(/\.avif$/, ""), `data:image/avif;base64,${readFileSync(join(DIR, f)).toString("base64")}`]);
// One absolutely positioned image per 100x60 cell, four per row.
const pos = (i) => [10 + (i % 4) * 100, 10 + Math.floor(i / 4) * 60];
const html =
  `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}img{position:absolute}</style></head><body>` +
  sources.map(([id, src], i) => `<img id="${id}" src="${src}" style="left:${pos(i)[0]}px;top:${pos(i)[1]}px">`).join("") +
  `</body></html>`;

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
const info = await page.evaluate(async () => {
  for (const img of document.images) await img.decode().catch(() => {});
  return [...document.images].map((img) => {
    const r = img.getBoundingClientRect();
    return { id: img.id, natural: [img.naturalWidth, img.naturalHeight], rect: [r.x, r.y, r.width, r.height] };
  });
});
const px = decodePng(await page.screenshot());
await browser.close();

const cases = info.map(({ id, natural, rect: [x, y, w, h] }) => {
  const points = [[x + w / 4, y + h / 4], [x + (3 * w) / 4, y + h / 4], [x + w / 4, y + (3 * h) / 4], [x + (3 * w) / 4, y + (3 * h) / 4]].map(
    ([a, b]) => [Math.floor(a), Math.floor(b)],
  );
  const rgb = points.map(([a, b]) => px(a, b));
  return { id, src: sources.find(([s]) => s === id)[1], natural, rect: [x, y, w, h], points, samples: rgb.map(classOf).join(""), rgb };
});
writeFileSync(
  out,
  `${JSON.stringify({ generator: "scripts/raster-avif-chrome-control.mjs", chrome: version, playwright_core: playwrightVersion, viewport: [1280, 800], html, cases }, null, 1)}\n`,
);
console.log(`Chrome ${version}: ${cases.length} images -> ${out}`);
for (const c of cases) console.log(c.id.padEnd(24), c.natural.join("x"), c.samples, JSON.stringify(c.rgb));

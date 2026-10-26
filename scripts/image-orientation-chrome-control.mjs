// Chrome-156 control for EXIF image orientation (P5b, miaskiewicz/turbo-html2pdf#30).
//
// Every image of crates/turbo-dom-layout/tests/fixtures/images/orientation (a 64x40 stored
// image in four colour quadrants — top-left red, top-right green, bottom-left blue,
// bottom-right yellow — with EXIF Orientation 1..8 in JPEG APP1, PNG eXIf and WebP EXIF;
// see scripts/make_orientation_fixtures.py) is shown four ways on one page (viewport
// 1280x800, every box absolutely positioned so the page is engine-independent):
//   - `img`:      <img>, default `image-orientation: from-image`
//   - `img-none`: <img style="image-orientation: none">
//   - `bg`:       an 80x80 <div> with the image as its background, stretched to the box
//                 (`no-repeat 0 0/100% 100%`: the engine paints a background stretched)
//   - `bg-none`:  the same with `image-orientation: none`
// Chrome for Testing 156 records, per box: `natural` ([naturalWidth, naturalHeight] for
// <img>), `rect` (getBoundingClientRect) and `samples`: the colour class (R/G/B/Y, W for
// white, ? otherwise) at a list of points (page px) read from a full-page screenshot —
// the centres of the four quadrants of the box (the image is stretched over it, so they
// read the displayed image's four quadrants).
// Asserted by crates/turbo-dom-layout/tests/image_orientation_chrome.rs (natural size,
// box geometry) and crates/turbo-dom-raster/tests/image_orientation_chrome.rs (pixels).
//
// Usage: node scripts/image-orientation-chrome-control.mjs [--chrome PATH] [--out PATH]
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/image_orientation_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DIR = join(repo, "crates/turbo-dom-layout/tests/fixtures/images/orientation");
const MIME = { jpg: "image/jpeg", png: "image/png", webp: "image/webp" };
const files = readdirSync(DIR).filter((f) => /\.(jpg|png|webp)$/.test(f)).sort();

// One row (y) per image, one column (x) per mode.
const IMG = (id, src, x, y, extra = "") =>
  `<img data-cid="${id}" src="${src}" style="position:absolute;left:${x}px;top:${y}px${extra}">`;
const BG = (id, src, x, y, extra = "") =>
  `<div data-cid="${id}" style="position:absolute;left:${x}px;top:${y}px;width:80px;height:80px;background:url(${src}) no-repeat 0 0/100% 100%${extra}"></div>`;
const MODES = [
  ["img", 20, (id, src, x, y) => IMG(id, src, x, y)],
  ["img-none", 160, (id, src, x, y) => IMG(id, src, x, y, ";image-orientation:none")],
  ["bg", 300, (id, src, x, y) => BG(id, src, x, y)],
  ["bg-none", 440, (id, src, x, y) => BG(id, src, x, y, ";image-orientation:none")],
];
const ROW = 90;
const boxes = [];
let body = "";
files.forEach((file, row) => {
  const src = `data:${MIME[file.split(".").pop()]};base64,${readFileSync(join(DIR, file)).toString("base64")}`;
  for (const [mode, x, tag] of MODES) {
    const id = `${file}|${mode}`;
    const y = 10 + row * ROW;
    body += tag(id, src, x, y);
    boxes.push({ id, file, mode, x, y });
  }
});
const html = `<!DOCTYPE html><html><head><style>body{margin:0}</style></head><body>${body}</body></html>`;

/** Decode an 8-bit RGB/RGBA PNG (all five filters) to {width, height, px(x, y) -> [r,g,b]}. */
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
      if (data[8] !== 8 || ![2, 6].includes(data[9])) throw new Error("unexpected PNG format");
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
  return { width, height, px: (x, y) => [...pix.subarray(y * stride + x * channels, y * stride + x * channels + 3)] };
}

/** The colour class of a pixel: R/G/B/Y quadrant colours, W white, ? anything else. */
function classOf([r, g, b]) {
  const hi = (v) => v > 160, lo = (v) => v < 96;
  if (hi(r) && hi(g) && hi(b)) return "W";
  if (hi(r) && lo(g) && lo(b)) return "R";
  if (lo(r) && hi(g) && lo(b)) return "G";
  if (lo(r) && lo(g) && hi(b)) return "B";
  if (hi(r) && hi(g) && lo(b)) return "Y";
  return "?";
}

const quadrantCentres = (x, y, w, h) => [
  [x + w / 4, y + h / 4],
  [x + (3 * w) / 4, y + h / 4],
  [x + w / 4, y + (3 * h) / 4],
  [x + (3 * w) / 4, y + (3 * h) / 4],
].map(([px, py]) => [Math.floor(px), Math.floor(py)]);

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
await page.setContent(html, { waitUntil: "load" });
const measured = await page.evaluate(() =>
  [...document.querySelectorAll("[data-cid]")].map((el) => {
    const r = el.getBoundingClientRect();
    return {
      id: el.dataset.cid,
      rect: [r.x, r.y, r.width, r.height],
      natural: el.tagName === "IMG" ? [el.naturalWidth, el.naturalHeight] : null,
    };
  }),
);
const shot = decodePng(await page.screenshot({ fullPage: true }));
await browser.close();

const cases = boxes.map((box) => {
  const m = measured.find((e) => e.id === box.id);
  const orientation = Number((box.file.match(/-o(\d)/) || [0, 1])[1]);
  const points = quadrantCentres(...m.rect);
  const samples = points.map(([x, y]) => classOf(shot.px(x, y))).join("");
  return { ...box, orientation, natural: m.natural, rect: m.rect, points, samples };
});

const fixture = {
  generator: "scripts/image-orientation-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  html,
  cases,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} boxes -> ${out}`);
for (const c of cases) console.log(c.id.padEnd(28), JSON.stringify(c.natural), c.rect.join(","), c.samples);

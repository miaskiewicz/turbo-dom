// Chrome-156 control for turbo-dom-raster's inline `<svg>` screenshots (P5b,
// miaskiewicz/turbo-dom#25 item 1): the screenshot paints each parsed `<svg>` ELEMENT of the
// document (the HTML parser decides what is an element), never a `<svg` that only appears
// in source text — inside an attribute value (a raw-UTF-8 SVG `data:` URL), a `<script>`
// string, or a `title`.
//
// Every case is one page (viewport 1280x800, `body{margin:0;background:#00ffff}`) of
// absolutely positioned boxes tagged `data-cid`. Chrome for Testing 156 records per box:
// `rect` (getBoundingClientRect) and `samples` — the colour class (R/G/B/Y/W/T(ransparent:
// the page's #00ffff backdrop)/?) at the centres of the box's four quadrants, read from a
// screenshot (`points` are those pixel coordinates). `assets` lists the case's `<img src>`
// `data:` URLs: the screenshot API takes image bytes from the caller (keyed by the raw
// `src`), so the test supplies them, as turbo-surf does for fetched images.
// Asserted by crates/turbo-dom-raster/tests/inline_svg_chrome.rs.
//
// Usage: node scripts/inline-svg-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/inline_svg_chrome156.json.
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/inline_svg_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const at = (x, y, w = 64, h = 40) => `position:absolute;left:${x}px;top:${y}px;width:${w}px;height:${h}px`;
const pos = (x, y) => `position:absolute;left:${x}px;top:${y}px`;
const DOC = (body) =>
  `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>${body}</body></html>`;
const RECT = (fill) => `<rect width="64" height="40" fill="${fill}"/>`;
// A raw-UTF-8 SVG `data:` URL (only `#` escaped): its `<svg` sits inside the attribute value.
const UTF8_SVG = `data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='64' height='40'><rect width='64' height='40' fill='%23ff0000'/></svg>`;

const CASES = [
  {
    id: "inline-basic",
    html: DOC(`<svg data-cid="s" width="64" height="40" style="${pos(20, 10)}">${RECT("#ff0000")}</svg>`),
  },
  {
    // `xlink:href` with no `xmlns:xlink` declaration: the HTML parser puts it in the XLink
    // namespace; a verbatim copy of the source is not namespace-well-formed XML.
    id: "inline-xlink-use",
    html: DOC(
      `<svg data-cid="s" width="64" height="40" style="${pos(20, 10)}"><defs><rect id="r" width="64" height="40" fill="#00ff00"/></defs><use xlink:href="#r"/></svg>`,
    ),
  },
  {
    // An HTML named character reference inside the SVG (not an XML entity).
    id: "inline-html-entity",
    html: DOC(`<svg data-cid="s" width="64" height="40" style="${pos(20, 10)}"><title>a&nbsp;b</title>${RECT("#0000ff")}</svg>`),
  },
  {
    // Lowercase source names the HTML parser adjusts to SVG's camelCase (`viewBox`,
    // `linearGradient`), and an uppercase tag name.
    id: "inline-adjusted-names",
    html: DOC(
      `<SVG data-cid="s" width="64" height="40" viewbox="0 0 32 20" style="${pos(20, 10)}"><lineargradient id="g"><stop offset="0" stop-color="#00ff00"/><stop offset="1" stop-color="#00ff00"/></lineargradient><rect width="32" height="20" fill="url(#g)"/></SVG>`,
    ),
  },
  {
    id: "inline-nested",
    html: DOC(
      `<svg data-cid="s" width="64" height="40" style="${pos(20, 10)}"><rect width="32" height="40" fill="#ff0000"/><svg x="32" width="32" height="40"><rect width="32" height="40" fill="#0000ff"/></svg></svg>`,
    ),
  },
  {
    // `<svg` / `</svg>` inside script strings are not elements: the box between them paints.
    id: "script-strings",
    html: DOC(
      `<script>var a = '<svg width="5">';</script><div data-cid="d" style="${at(20, 10)};background:#ffff00"></div><script>var b = '</svg>';</script>`,
    ),
  },
  {
    // `<svg>` / `</svg>` inside attribute values are not elements either.
    id: "attribute-values",
    html: DOC(
      `<div data-cid="a" title="<svg>" style="${at(20, 10)};background:#ff0000"></div><div data-cid="b" style="${at(120, 10)};background:#0000ff"></div><p title="</svg>"></p>`,
    ),
  },
  {
    // A raw-UTF-8 SVG `data:` URL next to an inline `<svg>`: both paint.
    id: "data-url-and-inline",
    html: DOC(
      `<img data-cid="i" src="${UTF8_SVG}" style="${pos(20, 10)}"><svg data-cid="s" width="64" height="40" style="${pos(120, 10)}">${RECT("#00ff00")}</svg>`,
    ),
    assets: [UTF8_SVG],
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
  const rects = await page.evaluate(async () => {
    for (const img of document.images) await img.decode().catch(() => {});
    return [...document.querySelectorAll("[data-cid]")].map((el) => {
      const r = el.getBoundingClientRect();
      return { cid: el.dataset.cid, rect: [r.x, r.y, r.width, r.height] };
    });
  });
  const px = decodePng(await page.screenshot());
  const boxes = rects.map(({ cid, rect: [x, y, w, h] }) => {
    const points = [
      [x + w / 4, y + h / 4],
      [x + (3 * w) / 4, y + h / 4],
      [x + w / 4, y + (3 * h) / 4],
      [x + (3 * w) / 4, y + (3 * h) / 4],
    ].map(([a, b]) => [Math.floor(a), Math.floor(b)]);
    return { cid, rect: [x, y, w, h], points, samples: points.map(([a, b]) => classOf(px(a, b))).join("") };
  });
  cases.push({ id: testCase.id, html: testCase.html, assets: testCase.assets || [], boxes });
}
await browser.close();

const fixture = {
  generator: "scripts/inline-svg-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases,
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) console.log(c.id.padEnd(22), c.boxes.map((b) => `${b.cid}:${b.rect.join(",")}:${b.samples}`).join("  "));

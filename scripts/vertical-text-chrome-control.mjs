// Chrome-156 pixel control for vertical writing modes in turbo-dom-raster's screenshots
// (miaskiewicz/turbo-dom#234): text in a `writing-mode: vertical-rl` / `vertical-lr` box
// paints its Latin glyphs turned a quarter clockwise (`text-orientation: mixed`), its
// lines stacked right to left (vertical-rl) or left to right (vertical-lr).
//
// Every case is one page (viewport 800x600, device scale factor 1) in the repo's fixture
// Liberation Serif (crates/turbo-dom-layout/assets/fonts, loaded through @font-face; the
// bundled face turbo-dom-raster's font registry resolves), red text on white. Per case
// Chrome records:
//   boxes  the getBoundingClientRect() of every `[data-cid]` element, as [x, y, w, h];
//   ink    per box, the bounding box [x0, y0, x1, y1] (exclusive) of the screenshot pixels
//          within it that differ from the white page by more than 96 in a channel, and
//          their count (`null` when none): the painted text.
// Asserted by crates/turbo-dom-raster/tests/vertical_text_chrome.rs (PNG and SVG).
//
// Usage: node scripts/vertical-text-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156 headless shell;
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/vertical_text_chrome156.json.
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
  join(homedir(), ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell");
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/vertical_text_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const FONT = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf")).toString("base64");
const FACE = `@font-face{font-family:"Liberation Serif";src:url(data:font/ttf;base64,${FONT})}`;
const DOC = (body) =>
  `<!doctype html><style>body{margin:8px;font-family:"Liberation Serif";font-size:16px;line-height:20px;color:#ff0000}</style>${body}`;

// miaskiewicz/turbo-dom#239 upright text: the generated box-glyph Ic Fixture face
// (scripts/make-ic-fixture-font.py; U+6C34 a 0.9em box, `a` / `b` boxes, no vmtx so the
// vertical advance is the 1em ascent + descent). Cases marked `ic` use it.
const IC = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/ic-fixture/IcFixture.ttf")).toString("base64");
const IC_FACE = `@font-face{font-family:"Ic Fixture";src:url(data:font/ttf;base64,${IC})}`;
const IC_DOC = (body) =>
  `<!doctype html><style>body{margin:8px;font-family:"Ic Fixture";font-size:16px;line-height:20px;color:#ff0000}</style>${body}`;

const CASES = [
  { id: "ic-rl-cjk", ic: true, html: IC_DOC(`<div data-cid="v" style="writing-mode:vertical-rl;height:120px">\u6c34\u6c34</div>`) },
  { id: "ic-lr-upright", ic: true, html: IC_DOC(`<div data-cid="v" style="writing-mode:vertical-lr;height:120px;text-orientation:upright">ab</div>`) },
  { id: "ic-rl-mixed", ic: true, html: IC_DOC(`<div data-cid="v" style="writing-mode:vertical-rl;height:120px">a\u6c34b</div>`) },
  { id: "ic-rl-lines", ic: true, html: IC_DOC(`<div data-cid="v" style="writing-mode:vertical-rl;height:40px">\u6c34\u6c34\u6c34\u6c34</div>`) },
  { id: "rl-word", html: DOC(`<div data-cid="v" style="writing-mode:vertical-rl;height:120px">Hello</div>`) },
  { id: "lr-word", html: DOC(`<div data-cid="v" style="writing-mode:vertical-lr;height:120px">Hello</div>`) },
  { id: "rl-lines", html: DOC(`<div data-cid="v" style="writing-mode:vertical-rl;height:50px">lll mmm</div>`) },
  { id: "lr-lines", html: DOC(`<div data-cid="v" style="writing-mode:vertical-lr;height:50px">lll mmm</div>`) },
  {
    id: "rl-blocks",
    html: DOC(
      `<div style="writing-mode:vertical-rl;height:100px;padding:4px 6px"><p data-cid="a" style="margin:0 0 0 10px">Ab</p><p data-cid="b" style="margin:0;font-size:24px">Cd</p></div>`,
    ),
  },
];

// Decode a PNG screenshot (8-bit RGB/RGBA, no interlace) into { width, height, rgba }.
function decodePng(buffer) {
  let offset = 8;
  let width = 0;
  let height = 0;
  let channels = 4;
  const idat = [];
  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("ascii", offset + 4, offset + 8);
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      channels = data[9] === 6 ? 4 : 3;
    } else if (type === "IDAT") idat.push(data);
    offset += 12 + length;
  }
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const rgba = new Uint8Array(width * height * 4);
  let prev = new Uint8Array(stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const cur = new Uint8Array(stride);
    for (let i = 0; i < stride; i++) {
      const a = i >= channels ? cur[i - channels] : 0;
      const b = prev[i];
      const c = i >= channels ? prev[i - channels] : 0;
      let v = line[i];
      if (filter === 1) v += a;
      else if (filter === 2) v += b;
      else if (filter === 3) v += (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      cur[i] = v & 255;
    }
    for (let x = 0; x < width; x++) {
      for (let k = 0; k < 3; k++) rgba[(y * width + x) * 4 + k] = cur[x * channels + k];
      rgba[(y * width + x) * 4 + 3] = 255;
    }
    prev = cur;
  }
  return { width, height, rgba };
}

// The ink bounding box of `png` within `rect`.
function ink(png, [x, y, w, h]) {
  const x0 = Math.max(0, Math.floor(x));
  const y0 = Math.max(0, Math.floor(y));
  const x1 = Math.min(png.width, Math.ceil(x + w));
  const y1 = Math.min(png.height, Math.ceil(y + h));
  let box = null;
  let count = 0;
  for (let py = y0; py < y1; py++) {
    for (let px = x0; px < x1; px++) {
      const i = (py * png.width + px) * 4;
      if (![0, 1, 2].some((k) => 255 - png.rgba[i + k] > 96)) continue;
      count++;
      box = box
        ? [Math.min(box[0], px), Math.min(box[1], py), Math.max(box[2], px + 1), Math.max(box[3], py + 1)]
        : [px, py, px + 1, py + 1];
    }
  }
  return box && { box, count };
}

const browser = await chromium.launch({ executablePath: chromePath, headless: true, args: ["--use-mock-keychain", "--password-store=basic"] });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 });
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  await page.addStyleTag({ content: testCase.ic ? IC_FACE : FACE });
  const family = testCase.ic ? '16px "Ic Fixture"' : '16px "Liberation Serif"';
  await page.evaluate((f) => document.fonts.load(f).then(() => document.fonts.ready), family);
  const boxes = await page.evaluate(() =>
    [...document.querySelectorAll("[data-cid]")].map((el) => {
      const r = el.getBoundingClientRect();
      return { cid: el.dataset.cid, rect: [r.x, r.y, r.width, r.height] };
    }),
  );
  const png = decodePng(await page.screenshot());
  for (const b of boxes) b.ink = ink(png, b.rect);
  result.push({ ...testCase, boxes });
}
await browser.close();

const fixture = {
  generator: "scripts/vertical-text-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [800, 600],
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${result.length} cases -> ${out}`);

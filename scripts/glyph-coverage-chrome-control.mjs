// Chrome-156 glyph coverage control for turbo-dom-raster's text painter
// (miaskiewicz/turbo-dom#266: Chrome's CoreText-smoothed, pre-blended glyph edges).
//
// Each case is one page (viewport 1280x800, DSF 1) with a line of the fixture Liberation
// Serif (`@font-face`, crates/turbo-dom-layout/assets/fonts) absolutely positioned at
// (10, 10). Recorded per case: `crop` [x, y, w, h], the screenshot region around the
// text's box, and `rgb`, its pixels as base64 RGB rows. Two families:
//   - `text-<size>-<fg>`: "HIlmn gy oe Wx" at 12-64px in black on white, blue on cyan
//     and white on black (the colour sets Skia's luminance pre-blend);
//   - `edge-<glyph>-<phase>`: a 64px `l` (vertical stem edges, the x response) or `-`
//     (horizontal bar edges, the y response) with the pen at x + 0, 0.25, 0.5, 0.75
//     (Skia's four subpixel positions).
// Asserted by crates/turbo-dom-raster/tests/glyph_coverage_chrome.rs.
//
// Usage: node scripts/glyph-coverage-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156 headless shell (~/.cache/turbo-consolidation);
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/glyph_coverage_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/glyph_coverage_chrome156.json"));
const playwrightCore = process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const FONT = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf")).toString("base64");
const FACE = `@font-face{font-family:"Liberation Serif";src:url(data:font/ttf;base64,${FONT})}`;
const DOC = (bg, body) =>
  `<!DOCTYPE html><html><head><style>body{margin:0;background:${bg}}</style></head><body>${body}</body></html>`;
const LINE = (left, size, fg, text) =>
  `<div data-cid="t" style="position:absolute;left:${left}px;top:10px;font-family:'Liberation Serif';font-size:${size}px;line-height:${Math.round(1.3 * size)}px;color:${fg};white-space:nowrap">${text}</div>`;

const COLOURS = [["000000", "#ffffff"], ["0000ff", "#00ffff"], ["ffffff", "#000000"]];
const CASES = [];
for (const size of [12, 16, 20, 32, 40, 64]) {
  for (const [fg, bg] of COLOURS) {
    CASES.push({ id: `text-${size}-${fg}`, html: DOC(bg, LINE(10, size, `#${fg}`, "HIlmn gy oe Wx")) });
  }
}
for (const [name, glyph] of [["l", "l"], ["hyphen", "-"]]) {
  for (const phase of [0, 0.25, 0.5, 0.75]) {
    CASES.push({ id: `edge-${name}-${phase}`, html: DOC("#ffffff", LINE(10 + phase, 64, "#000000", glyph)) });
  }
}

/** Decode an 8-bit RGB/RGBA PNG (all five filters) to { width, channels, pix }. */
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
  return { width, channels, pix };
}

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const cases = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  await page.addStyleTag({ content: FACE });
  await page.evaluate(() => document.fonts.ready);
  const box = await page.evaluate(() => {
    const b = document.querySelector("[data-cid]").getBoundingClientRect();
    return [b.x, b.y, b.width, b.height];
  });
  const crop = [
    Math.max(0, Math.floor(box[0]) - 4),
    Math.max(0, Math.floor(box[1]) - 4),
    Math.ceil(box[2]) + 8,
    Math.ceil(box[3]) + 8,
  ];
  const { width, channels, pix } = decodePng(await page.screenshot());
  const rgb = Buffer.alloc(crop[2] * crop[3] * 3);
  for (let y = 0; y < crop[3]; y++) {
    for (let x = 0; x < crop[2]; x++) {
      const at = ((crop[1] + y) * width + crop[0] + x) * channels;
      pix.copy(rgb, (y * crop[2] + x) * 3, at, at + 3);
    }
  }
  cases.push({ id: testCase.id, html: testCase.html, crop, rgb: rgb.toString("base64") });
}
await browser.close();

mkdirSync(dirname(out), { recursive: true });
writeFileSync(
  out,
  `${JSON.stringify({ generator: "scripts/glyph-coverage-chrome-control.mjs", chrome: version, playwright_core: playwrightVersion, viewport: [1280, 800], cases }, null, 1)}\n`,
);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);

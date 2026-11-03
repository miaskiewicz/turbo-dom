// Chrome-156 control for an infinite `border-radius` (R2 review #5): a radius that
// overflows to infinity (`1` followed by 400 zeros, px) is clamped to a finite length,
// so the overlap rule turns the box into a pill, not NaN radii.
//
// Per case (viewport 300x120, device scale factor 1, white page, a red box at 10,10) the
// screenshot is probed on a 4px grid over the box's border box plus 4px around it; each
// probe records whether the pixel is red ("r": R > 200, G and B < 60), white ("w": all
// channels > 230) or an anti-aliased edge ("-", not asserted).
// Asserted by crates/turbo-dom-raster/tests/infinite_radius_chrome.rs (PNG and SVG).
//
// Usage: node scripts/infinite-radius-chrome-control.mjs [--chrome PATH] [--out PATH]
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
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
    ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
  );
const out = resolve(
  values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/infinite_radius_chrome156.json"),
);
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const HUGE = `1${"0".repeat(400)}px`;
const BOX = { x: 10, y: 10, width: 200, height: 50 };
const DOC = (style) =>
  `<!doctype html><style>body{margin:0;background:#fff}div{position:absolute;left:10px;top:10px;width:200px;height:50px;background:#f00;${style}}</style><div></div>`;
const CASES = [
  { id: "all-corners", html: DOC(`border-radius:${HUGE}`) },
  { id: "one-corner", html: DOC(`border-top-left-radius:${HUGE}`) },
  { id: "elliptical", html: DOC(`border-radius:${HUGE} / 10px`) },
];

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
  const rgb = new Uint8Array(width * height * 3);
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
    for (let x = 0; x < width; x++) for (let k = 0; k < 3; k++) rgb[(y * width + x) * 3 + k] = cur[x * channels + k];
    prev = cur;
  }
  return { width, height, rgb };
}

const classify = ([r, g, b]) => (r > 200 && g < 60 && b < 60 ? "r" : r > 230 && g > 230 && b > 230 ? "w" : "-");

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 300, height: 120 }, deviceScaleFactor: 1 });
const cases = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  const png = decodePng(await page.screenshot({ type: "png" }));
  const rows = [];
  for (let y = BOX.y - 4; y <= BOX.y + BOX.height + 4; y += 4) {
    let row = "";
    for (let x = BOX.x - 4; x <= BOX.x + BOX.width + 4; x += 4) {
      const i = (y * png.width + x) * 3;
      row += classify([png.rgb[i], png.rgb[i + 1], png.rgb[i + 2]]);
    }
    rows.push(row);
  }
  cases.push({ id: testCase.id, html: testCase.html, origin: [BOX.x - 4, BOX.y - 4], step: 4, rows });
}
await browser.close();
writeFileSync(
  out,
  `${JSON.stringify({ generator: "scripts/infinite-radius-chrome-control.mjs", chrome: version, playwright_core: playwrightVersion, viewport: [300, 120], cases }, null, 1)}\n`,
);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);

// Chrome-156 pixel control for turbo-dom-raster's `<canvas>` content in page screenshots
// (miaskiewicz/turbo-dom#291): the embedder supplies what a page drew into each canvas (the
// render tier's `ctx._ops` op log, or a straight-alpha RGBA bitmap) and the screenshot paints
// it in the canvas box like Chrome — at the canvas's bitmap size (its `width` / `height`
// attributes), scaled into the content box (`object-fit` / `object-position`), inside the
// box's border and padding, clipped to its content box and corner radii, under its transform.
//
// Every case is one page (viewport 400x300, `body{margin:0;background:#00ffff}`) whose
// `<canvas>` elements Chrome for Testing 156 draws into: `canvases[i]` is the content of the
// page's i-th `<canvas>` in tree order (`index`), either `ops` (each tuple
// `[name, args, fillStyle, strokeStyle, font, textBaseline, textAlign, globalAlpha,
// globalCompositeOperation]`: the carried state is assigned, then `ctx[name](...args)`) or
// `bitmap` (`size` and four `quadrants` colours `[r, g, b, a]`, top-left, top-right,
// bottom-left, bottom-right, written with `putImageData`). A canvas with no entry is left
// blank. `probes` holds the screenshot's RGB at each probe point.
// Asserted by crates/turbo-dom-raster/tests/canvas_content_chrome.rs.
//
// Usage: node scripts/canvas-content-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/canvas_content_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { decodePng } from "./lib/png-samples.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/canvas_content_chrome156.json"));
const playwrightCore = process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const VIEWPORT = [400, 300];
const DOC = (body) =>
  `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>${body}</body></html>`;
const pos = (x, y) => `position:absolute;left:${x}px;top:${y}px`;
const canvas = (attrs, style) => `<canvas ${attrs} style="${style}"></canvas>`;

const RED = [255, 0, 0, 255];
const GREEN = [0, 255, 0, 255];
const BLUE = [0, 0, 255, 255];
const YELLOW = [255, 255, 0, 255];
const QUADRANTS = [RED, GREEN, BLUE, YELLOW];
/** A `w x h` bitmap of the four quadrant colours. */
const bitmap = (index, [w, h], quadrants = QUADRANTS) => ({ index, bitmap: { size: [w, h], quadrants } });

// One recorded tuple with the default state except `fill` / `alpha`.
const op = (name, args, { fill = "#000000", alpha = 1 } = {}) => [
  name,
  args,
  fill,
  "#000000",
  "10px sans-serif",
  "alphabetic",
  "start",
  alpha,
  "source-over",
];
/** The four quadrant colours filled into a `w x h` canvas as op tuples. */
const quadrantOps = (index, [w, h]) => ({
  index,
  ops: [
    op("fillRect", [0, 0, w / 2, h / 2], { fill: "#ff0000" }),
    op("fillRect", [w / 2, 0, w / 2, h / 2], { fill: "#00ff00" }),
    op("fillRect", [0, h / 2, w / 2, h / 2], { fill: "#0000ff" }),
    op("fillRect", [w / 2, h / 2, w / 2, h / 2], { fill: "#ffff00" }),
  ],
});
/** Probe points: the quadrant centres of the `w x h` area at `(x, y)`. */
const quads = (x, y, w, h) => [
  [x + w / 4, y + h / 4],
  [x + (3 * w) / 4, y + h / 4],
  [x + w / 4, y + (3 * h) / 4],
  [x + (3 * w) / 4, y + (3 * h) / 4],
];

const CASES = [
  {
    // An op log replayed at the canvas's 64x40 bitmap size, painted 1:1; the edge pixels
    // inside and outside the box.
    id: "ops-1x",
    html: DOC(canvas(`width="64" height="40"`, pos(20, 10))),
    canvases: [quadrantOps(0, [64, 40])],
    probes: [...quads(20, 10, 64, 40), [20, 10], [83, 10], [20, 49], [83, 49], [19, 10], [84, 49], [20, 50]],
  },
  {
    id: "bitmap-1x",
    html: DOC(canvas(`width="64" height="40"`, pos(20, 10))),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(20, 10, 64, 40), [20, 10], [83, 10], [20, 49], [83, 49], [19, 10], [84, 49], [20, 50]],
  },
  {
    // No `width` / `height`: the 300x150 default bitmap.
    id: "ops-default-size",
    html: DOC(canvas("", pos(0, 0))),
    canvases: [quadrantOps(0, [300, 150])],
    probes: [...quads(0, 0, 300, 150), [299, 149], [300, 10], [10, 150]],
  },
  {
    // A 32x20 bitmap stretched by CSS to 128x80.
    id: "bitmap-css-scaled",
    html: DOC(canvas(`width="32" height="20"`, `${pos(20, 10)};width:128px;height:80px`)),
    canvases: [bitmap(0, [32, 20])],
    probes: [...quads(20, 10, 128, 80), [19, 50], [148, 50]],
  },
  {
    // Only a CSS width: the height follows the bitmap's aspect ratio (128x80).
    id: "bitmap-css-width",
    html: DOC(canvas(`width="64" height="40"`, `${pos(20, 10)};width:128px`)),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(20, 10, 128, 80), [50, 92]],
  },
  {
    // Border and padding frame the content box the bitmap fills.
    id: "bitmap-border-padding",
    html: DOC(
      canvas(`width="64" height="40"`, `${pos(20, 10)};border:4px solid #000000;padding:6px;background:#ffffff`),
    ),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(30, 20, 64, 40), [22, 30], [26, 30], [60, 15], [99, 45], [30, 20], [29, 20]],
  },
  {
    id: "fit-fill",
    html: DOC(canvas(`width="64" height="40"`, `${pos(20, 10)};width:128px;height:128px;object-fit:fill`)),
    canvases: [bitmap(0, [64, 40])],
    probes: quads(20, 10, 128, 128),
  },
  {
    // 64x40 contained in 128x128: 128x80, centred (24px bands above and below).
    id: "fit-contain",
    html: DOC(
      canvas(`width="64" height="40"`, `${pos(20, 10)};width:128px;height:128px;object-fit:contain;background:#ffffff`),
    ),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(20, 34, 128, 80), [84, 20], [84, 130]],
  },
  {
    // 64x40 covering 80x80: 128x80, centred, its sides clipped to the box.
    id: "fit-cover",
    html: DOC(canvas(`width="64" height="40"`, `${pos(30, 10)};width:80px;height:80px;object-fit:cover`)),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(30, 10, 80, 80), [28, 30], [112, 30], [31, 11], [109, 89]],
  },
  {
    // `none` at its 64x40 size in a 32x20 box: the centre of the bitmap.
    id: "fit-none",
    html: DOC(canvas(`width="64" height="40"`, `${pos(20, 10)};width:32px;height:20px;object-fit:none`)),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(20, 10, 32, 20), [18, 15], [54, 15], [30, 32]],
  },
  {
    // `scale-down` of a bitmap smaller than the box is `none`: 64x40 centred in 128x128.
    id: "fit-scale-down",
    html: DOC(
      canvas(
        `width="64" height="40"`,
        `${pos(20, 10)};width:128px;height:128px;object-fit:scale-down;background:#ffffff`,
      ),
    ),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(52, 54, 64, 40), [30, 20], [130, 130]],
  },
  {
    // `object-position: 0 100%`: the contained 128x80 sits at the bottom.
    id: "fit-contain-position",
    html: DOC(
      canvas(
        `width="64" height="40"`,
        `${pos(20, 10)};width:128px;height:128px;object-fit:contain;object-position:0 100%;background:#ffffff`,
      ),
    ),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(20, 58, 128, 80), [84, 30]],
  },
  {
    // `none` positioned at the left top: the bitmap's top-left 32x20.
    id: "fit-none-position",
    html: DOC(
      canvas(
        `width="64" height="40"`,
        `${pos(20, 10)};width:48px;height:30px;object-fit:none;object-position:left top`,
      ),
    ),
    canvases: [bitmap(0, [64, 40])],
    probes: [[30, 20], [60, 20], [30, 35], [60, 35]],
  },
  {
    // Three canvases; the second has no content (blank, transparent).
    id: "keyed-by-document-order",
    html: DOC(
      canvas(`width="64" height="40"`, pos(20, 10)) +
        canvas(`width="64" height="40"`, pos(120, 10)) +
        `<div>${canvas(`width="64" height="40"`, pos(220, 10))}</div>`,
    ),
    canvases: [quadrantOps(0, [64, 40]), bitmap(2, [64, 40], [BLUE, BLUE, GREEN, GREEN])],
    probes: [...quads(20, 10, 64, 40), ...quads(120, 10, 64, 40), ...quads(220, 10, 64, 40)],
  },
  {
    // Translucent canvas pixels composite over the page.
    id: "alpha",
    html: DOC(canvas(`width="64" height="40"`, pos(20, 10))),
    canvases: [
      {
        index: 0,
        ops: [
          op("fillRect", [0, 0, 32, 40], { fill: "rgba(255, 0, 0, 0.5)" }),
          op("fillRect", [32, 0, 32, 40], { fill: "#0000ff", alpha: 0.25 }),
        ],
      },
    ],
    probes: [
      [36, 30],
      [68, 30],
    ],
  },
  {
    id: "bitmap-alpha",
    html: DOC(canvas(`width="64" height="40"`, pos(20, 10))),
    canvases: [bitmap(0, [64, 40], [[255, 0, 0, 128], [0, 0, 255, 64], [0, 0, 0, 0], [255, 255, 0, 192]])],
    probes: quads(20, 10, 64, 40),
  },
  {
    // Rounded corners clip the content.
    id: "border-radius",
    html: DOC(canvas(`width="64" height="40"`, `${pos(20, 10)};border-radius:16px`)),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(20, 10, 64, 40), [21, 11], [82, 11], [21, 48], [82, 48]],
  },
  {
    // The content turns with the box.
    id: "transform",
    html: DOC(canvas(`width="64" height="40"`, `${pos(20, 10)};transform:rotate(180deg)`)),
    canvases: [bitmap(0, [64, 40])],
    probes: quads(20, 10, 64, 40),
  },
  {
    // In a vertical writing mode the bitmap keeps its physical orientation.
    id: "vertical-rl",
    html: DOC(`<div style="${pos(20, 10)};writing-mode:vertical-rl">${canvas(`width="64" height="40"`, "")}</div>`),
    canvases: [bitmap(0, [64, 40])],
    probes: quads(20, 10, 64, 40),
  },
  {
    // `object-fit` / `object-position` are physical in a vertical writing mode too: the
    // contained 80x50 sits at the physical bottom of the 80x80 box.
    id: "vertical-rl-contain-position",
    html: DOC(
      `<div style="${pos(20, 10)};writing-mode:vertical-rl">` +
        canvas(`width="64" height="40"`, "width:80px;height:80px;object-fit:contain;object-position:0 100%") +
        "</div>",
    ),
    canvases: [bitmap(0, [64, 40])],
    probes: [...quads(20, 40, 80, 50), [60, 20]],
  },
  {
    id: "vertical-lr-cover-position",
    html: DOC(
      `<div style="${pos(20, 10)};writing-mode:vertical-lr">` +
        canvas(`width="64" height="40"`, "width:40px;height:80px;object-fit:cover;object-position:100% 0") +
        "</div>",
    ),
    canvases: [bitmap(0, [64, 40])],
    probes: [[30, 30], [50, 30], [30, 70], [50, 70], [62, 30]],
  },
  {
    // An inline canvas in a block's flow.
    id: "inline-in-flow",
    html: DOC(`<div style="${pos(20, 10)};line-height:0">${canvas(`width="64" height="40"`, "")}</div>`),
    canvases: [quadrantOps(0, [64, 40])],
    probes: quads(20, 10, 64, 40),
  },
];

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const version = browser.version();
const page = await browser.newPage({ viewport: { width: VIEWPORT[0], height: VIEWPORT[1] }, deviceScaleFactor: 1 });
const cases = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  await page.evaluate((canvases) => {
    const all = document.querySelectorAll("canvas");
    for (const content of canvases) {
      const ctx = all[content.index].getContext("2d");
      if (content.ops) {
        for (const [name, args, fill, stroke, font, , , alpha] of content.ops) {
          ctx.fillStyle = fill;
          ctx.strokeStyle = stroke;
          ctx.font = font;
          ctx.globalAlpha = alpha;
          ctx[name](...args);
        }
      } else {
        const [w, h] = content.bitmap.size;
        const data = new ImageData(w, h);
        for (let y = 0; y < h; y++)
          for (let x = 0; x < w; x++) {
            const q = (y < h / 2 ? 0 : 2) + (x < w / 2 ? 0 : 1);
            data.data.set(content.bitmap.quadrants[q], (y * w + x) * 4);
          }
        ctx.putImageData(data, 0, 0);
      }
    }
  }, testCase.canvases);
  const px = decodePng(await page.screenshot());
  const probes = testCase.probes.map(([x, y]) => {
    const [a, b] = [Math.floor(x), Math.floor(y)];
    return { x: a, y: b, rgb: px(a, b) };
  });
  cases.push({ id: testCase.id, html: testCase.html, canvases: testCase.canvases, probes });
}
await browser.close();

const fixture = {
  generator: "scripts/canvas-content-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: VIEWPORT,
  cases,
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) {
  console.log(c.id.padEnd(26), c.probes.map((p) => `${p.x},${p.y}:${p.rgb.join("/")}`).join(" "));
}

// Chrome-156 pixel control for turbo-dom-raster's 2D-canvas op-log replay
// (`canvas_ops_rgba` / `canvas_ops_png`, P5-cov).
//
// Each case is a canvas size plus an op log in the render tier's `ctx._ops` tuple format —
// `[name, args, fillStyle, strokeStyle, font, textBaseline, textAlign, globalAlpha,
// globalCompositeOperation]` — that Chrome for Testing 156 replays into a real
// `getContext('2d')`: per tuple it assigns the carried `fillStyle` / `strokeStyle` / `font` /
// `globalAlpha` (a value the canvas rejects leaves the previous state, as it does in a page)
// and then calls `ctx[name](...args)`. `getImageData` is read at each probe point (`rgba`,
// straight alpha). Shapes only: canvas text uses the platform fonts in Chrome and the bundled
// faces here, so text is asserted by the raster's own tests, not against glyph pixels.
// Asserted by crates/turbo-dom-raster/tests/canvas_ops_chrome.rs.
//
// Usage: node scripts/canvas-ops-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/canvas_ops_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/canvas_ops_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

// One recorded tuple with the default state except `fill` / `stroke` / `alpha`.
const op = (name, args = [], { fill = "#000000", stroke = "#000000", alpha = 1, font = "10px sans-serif" } = {}) => [
  name,
  args,
  fill,
  stroke,
  font,
  "alphabetic",
  "start",
  alpha,
  "source-over",
];
// The centre of every 8x8 cell of a 64-wide grid, row-major.
const cell = (i) => [(i % 8) * 8 + 4, Math.floor(i / 8) * 8 + 4];

// fillStyle strings the op-log parser maps (hex 3/4/6/8, rgb()/rgba() comma, space and
// slash forms with number or % alpha, out-of-range channels, the named keywords) and ones
// Chrome rejects too (a 5-digit hex, bad hex digits, rgb() with too few channels), each filling
// one 8x8 cell over a grey cell drawn first, so a rejected colour shows the previous fill.
const COLOURS = [
  "#f00", "#0f08", "#00ff00", "#0000ff80", "rgb(255, 255, 0)", "rgba(255,0,255,0.5)",
  "rgb(0 255 255 / 50%)", "rgba(10, 20, 30, 25%)", "transparent", "black", "white", "red",
  "green", "lime", "blue", "yellow", "cyan", "aqua", "magenta", "fuchsia", "gray", "grey",
  "orange", "  RED  ", "#ABCDEF", "#12345", "rgb(1,2)", "#ggg",
  "rgb(300, -5, 128)", "rgba(0,0,255,2)", "rgb(10 20 30 / 0.25)",
];
const cellOps = (colours) =>
  colours.flatMap((c, i) => {
    const [x, y] = cell(i).map((v) => v - 4);
    return [op("fillRect", [x, y, 8, 8], { fill: "#808080" }), op("fillRect", [x, y, 8, 8], { fill: c })];
  });
const colourOps = cellOps(COLOURS);
// CSS colour syntaxes beyond those (miaskiewicz/turbo-dom#95): hsl()/hsla() (comma, space and
// slash forms, `deg` hues, hue wrap), percentage rgb() channels and alpha, fractional channels
// (rounded), the full named-colour set in any case, and syntaxes Chrome rejects.
const CSS_COLOURS = [
  "hsl(0, 100%, 50%)", "hsl(120deg 100% 25%)", "hsla(240, 100%, 50%, 0.5)", "hsl(60 100% 50% / 25%)",
  "hsl(-120, 100%, 50%)", "hsl(480, 100%, 50%)", "hsl(0, 0%, 75%)", "hsl(200, 60%, 70%)",
  "rgb(50%, 0%, 100%)", "rgb(10% 20% 30% / 50%)", "rgba(100%, 50%, 0%, 0.25)", "rgb(12.9, 100.5, 200.4)",
  "rebeccapurple", "CornflowerBlue", "darkslategray", "LightGoldenRodYellow", "navy", "teal",
  "olive", "maroon", "silver", "purple", "indigo", "tomato",
  "hsl(0, 100%)", "hsl(x, 100%, 50%)", "rgb(1, 2, 3, 4, 5)", "notacolour",
];

const CASES = [
  {
    id: "fill-style-colours",
    size: [64, 64],
    ops: colourOps,
    probes: COLOURS.map((_, i) => cell(i)),
  },
  {
    id: "fill-style-css-colours",
    size: [64, 64],
    ops: cellOps(CSS_COLOURS),
    probes: CSS_COLOURS.map((_, i) => cell(i)),
  },
  {
    // save/restore around a translate; scale; rotate; transform; setTransform; resetTransform.
    id: "transforms",
    size: [96, 96],
    ops: [
      op("save"),
      op("translate", [40, 0]),
      op("fillRect", [0, 0, 10, 10], { fill: "#ff0000" }),
      op("restore"),
      op("fillRect", [0, 0, 10, 10], { fill: "#00ff00" }),
      op("restore"),
      op("save"),
      op("scale", [2, 3]),
      op("fillRect", [0, 10, 5, 5], { fill: "#0000ff" }),
      op("restore"),
      op("save"),
      op("translate", [70, 20]),
      op("rotate", [Math.PI / 2]),
      op("fillRect", [0, 0, 20, 10], { fill: "#ffff00" }),
      op("restore"),
      op("transform", [1, 0, 0, 1, 20, 60]),
      op("transform", [2, 0, 0, 2, 0, 0]),
      op("fillRect", [0, 0, 5, 5], { fill: "#ff00ff" }),
      op("setTransform", [1, 0, 0, 1, 60, 60]),
      op("fillRect", [0, 0, 10, 10], { fill: "#00ffff" }),
      op("resetTransform"),
      op("fillRect", [86, 86, 10, 10], { fill: "#000000" }),
    ],
    probes: [[45, 5], [5, 5], [5, 35], [12, 42], [65, 30], [55, 30], [22, 62], [28, 68], [65, 65], [90, 90], [30, 5]],
  },
  {
    // Paths: a closed triangle fill, a rect, a full circle, an open polyline stroke on
    // pixel centres, a bare lineTo (acts as moveTo), and a fill of two subpaths.
    id: "paths",
    size: [96, 96],
    ops: [
      op("beginPath"),
      op("moveTo", [4, 4]),
      op("lineTo", [40, 4]),
      op("lineTo", [4, 40]),
      op("closePath"),
      op("fill", [], { fill: "#ff0000" }),
      op("beginPath"),
      op("rect", [50, 4, 20, 10]),
      op("fill", [], { fill: "#0000ff" }),
      op("beginPath"),
      op("arc", [70, 50, 15, 0, 2 * Math.PI]),
      op("fill", [], { fill: "#00ff00" }),
      op("beginPath"),
      op("lineTo", [4, 60.5]),
      op("lineTo", [40, 60.5]),
      op("stroke", [], { stroke: "#ff00ff" }),
      op("beginPath"),
      op("rect", [4, 70, 10, 10]),
      op("rect", [20, 70, 10, 10]),
      op("fill", [], { fill: "#00ffff" }),
      op("beginPath"),
      op("closePath"),
      op("fill", [], { fill: "#000000" }),
    ],
    probes: [[10, 10], [35, 30], [60, 9], [75, 50], [70, 38], [56, 50], [88, 50], [20, 60], [20, 62], [9, 75], [25, 75], [17, 75], [45, 60]],
  },
  {
    // Arc sweeps: clockwise and anticlockwise half discs, an end angle more than a full turn
    // past the start (one full turn is drawn), an anticlockwise sweep from 0 to a negative
    // end, and a zero radius (nothing).
    id: "arcs",
    size: [96, 96],
    ops: [
      op("beginPath"),
      op("arc", [20, 20, 15, 0, Math.PI, false]),
      op("fill", [], { fill: "#ff0000" }),
      op("beginPath"),
      op("arc", [60, 20, 15, 0, Math.PI, true]),
      op("fill", [], { fill: "#0000ff" }),
      op("beginPath"),
      op("arc", [20, 60, 15, 0, 7 * Math.PI, false]),
      op("fill", [], { fill: "#00ff00" }),
      op("beginPath"),
      op("arc", [60, 60, 15, Math.PI / 2, -Math.PI / 2, true]),
      op("fill", [], { fill: "#ffff00" }),
      op("beginPath"),
      op("arc", [85, 85, 0, 0, 2 * Math.PI]),
      op("fill", [], { fill: "#000000" }),
      op("beginPath"),
      op("moveTo", [80, 5]),
      op("arc", [88, 12, 5, Math.PI / 2, 0, false]),
      op("stroke", [], { stroke: "#ff00ff" }),
    ],
    probes: [[20, 28], [20, 12], [60, 12], [60, 28], [20, 52], [20, 68], [52, 60], [68, 60], [85, 85]],
  },
  {
    // strokeRect on pixel centres (a crisp 1px outline), a transparent stroke, a stroke
    // under a 2x scale (device line width 2), and clearRect under a rotation.
    id: "stroke-and-clear",
    size: [96, 96],
    ops: [
      op("strokeRect", [4.5, 4.5, 30, 20], { stroke: "#0000ff" }),
      op("strokeRect", [40.5, 4.5, 30, 20], { stroke: "rgba(0,0,0,0)" }),
      op("save"),
      op("scale", [2, 2]),
      op("strokeRect", [2.5, 20.5, 10, 10], { stroke: "#ff0000" }),
      op("restore"),
      op("fillRect", [50, 50, 40, 40], { fill: "#00ff00" }),
      op("save"),
      op("translate", [70, 70]),
      op("rotate", [Math.PI / 4]),
      op("clearRect", [-5, -5, 10, 10]),
      op("restore"),
    ],
    probes: [[4, 14], [19, 4], [34, 14], [19, 14], [55, 4], [5, 50], [6, 50], [70, 70], [52, 52], [88, 88]],
  },
  {
    // globalAlpha folds into each fill's alpha (and clamps); straight-alpha readback.
    id: "global-alpha",
    size: [64, 32],
    ops: [
      op("fillRect", [0, 0, 16, 16], { fill: "#ff0000", alpha: 0.5 }),
      op("fillRect", [16, 0, 16, 16], { fill: "rgba(0,0,255,0.5)", alpha: 0.5 }),
      op("fillRect", [32, 0, 16, 16], { fill: "#00ff00", alpha: 0 }),
      op("fillRect", [48, 0, 16, 16], { fill: "#000000", alpha: 0.25 }),
      op("fillRect", [0, 16, 16, 16], { fill: "#0000ff" }),
      op("fillRect", [0, 16, 16, 16], { fill: "#ff0000", alpha: 0.5 }),
    ],
    probes: [[8, 8], [24, 8], [40, 8], [56, 8], [8, 24], [40, 24]],
  },
];

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage();
const cases = [];
for (const testCase of CASES) {
  const probes = await page.evaluate(({ size, ops, probes }) => {
    const canvas = document.createElement("canvas");
    [canvas.width, canvas.height] = size;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    for (const [name, args, fill, stroke, font, , , alpha] of ops) {
      ctx.fillStyle = fill;
      ctx.strokeStyle = stroke;
      ctx.font = font;
      ctx.globalAlpha = alpha;
      ctx[name](...args);
    }
    return probes.map(([x, y]) => ({ x, y, rgba: [...ctx.getImageData(x, y, 1, 1).data] }));
  }, testCase);
  cases.push({ id: testCase.id, size: testCase.size, ops: testCase.ops, probes });
}
await browser.close();

const fixture = {
  generator: "scripts/canvas-ops-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  cases,
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) {
  console.log(c.id.padEnd(20), c.probes.map((p) => `${p.x},${p.y}:${p.rgba.join("/")}`).join(" "));
}

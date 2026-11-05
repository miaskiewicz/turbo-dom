// Chrome-156 pixel control for the native form-control and media-control looks
// turbo-dom-layout draws into the galley (miaskiewicz/turbo-dom#293, #292):
//   - `progress-*` / `meter-*`: <progress> and <meter> as native controls (Chrome's
//     10em / 5em x 1em boxes, the rounded track and its value bar; the meter's optimum /
//     suboptimal / even-less-good colours);
//   - `indeterminate-*`: a checkbox whose `indeterminate` IDL flag a script set (marked
//     here with `data-indeterminate`; the screenshot test hands the flag to layout as
//     `turbo_dom_layout::INDETERMINATE_ATTR`);
//   - `vertical-*`: every control in `vertical-lr` / `vertical-rl`, turned as Chrome
//     turns it (its frame, a range / progress / meter running down the inline axis, a
//     menu list's chevron pointing along the block axis, unturned check glyphs);
//   - `audio-*`: the `<audio controls>` panel (play button, timeline, volume, menu).
//
// Every case is one page (viewport 1280x800, `body{margin:0;background:#00ffff}`) with
// its control (`data-cid="m"`) at (20, 20). Chrome 156 records the getBoundingClientRect()
// of every `[data-cid]` element (`boxes`) and, at probe points, the colour class
// (R/G/B/Y/W, T = the #00ffff backdrop, `?` = anything else) and raw RGB of a
// screenshot (`probes`). Probes are the case's listed points plus a grid over the
// control's box (and 4px around it; `gridProbes`) kept where Chrome's 5x5 neighbourhood
// is one colour, so no probe sits on an anti-aliased edge; `exclude` drops
// grid points in a region (an animated or text area). Asserted by
// crates/turbo-dom-raster/src/form_native_chrome_tests.rs (PNG and SVG screenshots and
// `layout_boxes`).
//
// Usage: node scripts/form-native-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156 headless shell (~/.cache/turbo-consolidation);
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/form_native_chrome156.json.
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
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/form_native_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DOC = (body) =>
  `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>${body}</body></html>`;
const AT = "position:absolute;left:20px;top:20px";
// The fixture Liberation Serif (the bundled face the screenshot's font registry resolves),
// injected as an @font-face after load so the recorded `html` stays small.
const FONT = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf")).toString("base64");
const FACE = `@font-face{font-family:"Liberation Serif";src:url(data:font/ttf;base64,${FONT})}`;

const progress = (attrs, style = "") => `<progress data-cid="m" ${attrs} style="${AT}${style}"></progress>`;
const meter = (attrs, style = "") => `<meter data-cid="m" ${attrs} style="${AT}${style}"></meter>`;
// The bar of a default 16px-tall progress / meter: the 8px-tall track's middle row.
const BAR = (xs) => xs.map((x) => [x, 8]);

const CASES = [
  // <progress>: its value / max (max 1 when absent or not positive; the value clamped to
  // 0..max, 0 when invalid). Without a value it is indeterminate: Chrome animates a bar
  // over the track, so only the box around it is probed.
  ["progress-value", progress('value="0.3"'), { probes: BAR([4, 30, 46, 50, 80, 155]) }],
  ["progress-value-max", progress('value="3" max="4"'), { probes: BAR([110, 118, 122, 130]) }],
  ["progress-over-max", progress('value="2"'), { probes: BAR([4, 150, 155]) }],
  ["progress-negative", progress('value="-1"'), { probes: BAR([4, 10, 80]) }],
  ["progress-max-zero", progress('value="0.5" max="0"'), { probes: BAR([70, 78, 82, 90]) }],
  ["progress-invalid-value", progress('value="abc"'), { probes: BAR([4, 10, 80]) }],
  ["progress-indeterminate", progress(""), { exclude: [[16, 22, 184, 34]] }],
  ["progress-font", progress('value="0.5"', ";font-size:20px"), {}],
  ["progress-size", progress('value="0.5"', ";width:50px;height:30px"), {}],
  ["progress-children", `<progress data-cid="m" value="0.5" style="${AT}"><span style="display:block;width:300px;height:40px;background:#ff0000"></span></progress>`, {}],
  // <meter>: the value's region against low / high / optimum picks the bar's colour.
  ["meter-value", meter('value="0.6"'), { probes: BAR([4, 40, 46, 50, 60, 75]) }],
  ["meter-default", meter(""), { probes: BAR([4, 40, 75]) }],
  ["meter-full", meter('value="1"'), { probes: BAR([4, 40, 75]) }],
  ["meter-even-less-good", meter('value="0.9" low="0.3" high="0.7" optimum="0.1"'), { probes: BAR([4, 40, 70]) }],
  ["meter-suboptimal-low", meter('value="0.5" low="0.3" high="0.7" optimum="0.1"'), { probes: BAR([4, 30, 50]) }],
  ["meter-optimum-low", meter('value="0.2" low="0.3" high="0.7" optimum="0.1"'), { probes: BAR([4, 10, 30]) }],
  ["meter-suboptimal-high", meter('value="0.5" low="0.3" high="0.7" optimum="0.9"'), { probes: BAR([4, 30, 50]) }],
  ["meter-even-less-good-high", meter('value="0.2" low="0.3" high="0.7" optimum="0.9"'), { probes: BAR([4, 10, 30]) }],
  ["meter-optimum-high", meter('value="0.8" low="0.3" high="0.7" optimum="0.9"'), { probes: BAR([4, 50, 70]) }],
  ["meter-suboptimal-mid", meter('value="0.9" low="0.3" high="0.7"'), { probes: BAR([4, 50, 70]) }],
  ["meter-optimum-mid", meter('value="0.5" low="0.3" high="0.7"'), { probes: BAR([4, 30, 50]) }],
  ["meter-min-max", meter('min="10" max="20" value="15"'), { probes: BAR([4, 30, 50, 70]) }],
  ["meter-max-below-min", meter('min="10" max="5" value="7"'), { probes: BAR([4, 30, 70]) }],
  ["meter-font", meter('value="0.5"', ";font-size:10px"), { probes: [[8, 5], [20, 5], [30, 5], [44, 5]] }],
  // A progress / meter on a line of text: its baseline is its bottom, lowered by Chrome's
  // `vertical-align: -0.2em`.
  [
    "progress-meter-baseline",
    `<div data-cid="line" style="${AT};font:20px/30px 'Liberation Serif';color:transparent;width:600px">x<progress data-cid="m" value="0.5"></progress>x<meter data-cid="n" value="0.5"></meter>x</div>`,
    {},
  ],
  // A script-set indeterminate checkbox paints Chrome's dash, checked or not; a radio
  // button's flag changes nothing.
  ["indeterminate-checkbox", `<input data-cid="m" type="checkbox" data-indeterminate style="${AT};margin:0">`, { probes: [[2, 2], [7, 6], [5, 6], [9, 6], [7, 3], [7, 10], [11, 11], [2, 11]] }],
  ["indeterminate-checked", `<input data-cid="m" type="checkbox" checked data-indeterminate style="${AT};margin:0">`, { probes: [[2, 2], [7, 6], [5, 6], [9, 6], [7, 3], [7, 10], [5, 8], [10, 3]] }],
  ["indeterminate-radio", `<input data-cid="m" type="radio" data-indeterminate style="${AT};margin:0">`, { probes: [[6, 6], [3, 6]] }],
  // `<audio controls>`: Chrome's panel (no source: the play button and volume greyed out).
  // The time display's text is the host's system font: not probed.
  ["audio-panel", `<audio data-cid="m" controls style="${AT}"></audio>`, { exclude: [[58, 34, 142, 59]], probes: [[25, 27], [27, 25], [150, 26], [180, 27], [236, 27], [240, 27], [244, 27], [248, 26], [273, 22], [274, 27], [273, 32], [6, 27], [294, 27], [150, 10], [150, 45]] }],
  ["audio-panel-wide", `<audio data-cid="m" controls style="${AT};width:400px"></audio>`, { exclude: [[58, 34, 142, 59]] }],
  ["audio-panel-tall", `<audio data-cid="m" controls style="${AT};height:80px"></audio>`, { exclude: [[58, 60, 142, 85]] }],
  ["audio-panel-frame", `<audio data-cid="m" controls style="${AT};border:4px solid #0000ff;padding:6px;background:#00ff00"></audio>`, { exclude: [[68, 44, 152, 69]] }],
];

// Every control in both vertical writing modes, in a wrapper at (20, 20) of the fixture
// serif: block-level, so no line places it, but a progress / meter also on a line (its
// baseline lowered toward the line-under side, the left in both modes).
const VERTICAL = [
  ["text", `<input data-cid="m" style="display:block;font-family:'Liberation Serif';width:30px;height:16px">`, []],
  ["textarea", `<textarea data-cid="m" style="display:block;font-family:'Liberation Serif';width:30px;height:16px"></textarea>`, []],
  ["button", `<button data-cid="m" style="display:block;font-family:'Liberation Serif';width:42px;height:22px"></button>`, []],
  ["select", `<select data-cid="m" style="display:block;font-family:'Liberation Serif';width:42px;height:22px"></select>`, []],
  ["checkbox", `<input data-cid="m" type="checkbox" checked style="display:block;margin:0">`, [[2, 2], [10, 10], [5, 8], [8, 5], [2, 9], [10, 3]]],
  ["radio", `<input data-cid="m" type="radio" checked style="display:block;margin:0">`, [[6, 6], [4, 6], [6, 4]]],
  ["range", `<input data-cid="m" type="range" value="20" style="display:block;margin:0">`, []],
  ["color", `<input data-cid="m" type="color" style="display:block;margin:0">`, []],
  ["color-value", `<input data-cid="m" type="color" value="#ff0000" style="display:block;margin:0">`, []],
  ["progress", `<progress data-cid="m" value="0.3" style="display:block"></progress>`, []],
  ["meter", `<meter data-cid="m" value="0.3" style="display:block"></meter>`, []],
  ["progress-inline", `<progress data-cid="m" value="0.3"></progress>`, []],
  ["meter-inline", `<meter data-cid="m" value="0.3"></meter>`, []],
  ["indeterminate", `<input data-cid="m" type="checkbox" data-indeterminate style="display:block;margin:0">`, [[2, 2], [7, 6], [7, 3], [7, 10]]],
];
for (const mode of ["vertical-lr", "vertical-rl"]) {
  for (const [kind, control, probes] of VERTICAL) {
    CASES.push([`vertical-${kind}-${mode.slice(9)}`, `<div style="${AT};writing-mode:${mode};font-family:'Liberation Serif'">${control}</div>`, { probes }]);
  }
}

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

// Grid points over `box` (and 4px around it) whose 5x5 neighbourhood is one colour:
// 2px apart (3px on a box over 600px², 6px over 4000px²), the backdrop's twice as far.
function gridProbes(px, box, exclude) {
  const area = box.width * box.height;
  const step = area > 4000 ? 6 : area > 600 ? 3 : 2;
  const points = [];
  const same = (x, y) => {
    const c = px(x, y).join();
    for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) if (px(x + dx, y + dy).join() !== c) return false;
    return true;
  };
  const x0 = Math.floor(box.x) - 4, y0 = Math.floor(box.y) - 4;
  for (let y = y0; y <= box.y + box.height + 4; y += step) {
    for (let x = x0; x <= box.x + box.width + 4; x += step) {
      if (exclude.some(([ex0, ey0, ex1, ey1]) => x >= ex0 && x <= ex1 && y >= ey0 && y <= ey1)) continue;
      const backdrop = classOf(px(x, y)) === "T";
      if (backdrop && ((x - x0) % (2 * step) || (y - y0) % (2 * step))) continue;
      if (same(x, y)) points.push([x, y]);
    }
  }
  return points;
}

const browser = await chromium.launch({ executablePath: chromePath, headless: true, args: ["--use-mock-keychain", "--password-store=basic"] });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const cases = [];
for (const [id, body, { probes = [], exclude = [] }] of CASES) {
  const html = DOC(body);
  await page.setContent(html, { waitUntil: "load" });
  await page.addStyleTag({ content: FACE });
  await page.evaluate(async () => {
    await document.fonts.ready;
    for (const el of document.querySelectorAll("[data-indeterminate]")) el.indeterminate = true;
  });
  const boxes = await page.evaluate(() => {
    const r = (n) => Math.round(n * 1000) / 1000;
    return [...document.querySelectorAll("[data-cid]")].map((el) => {
      const b = el.getBoundingClientRect();
      return { cid: el.dataset.cid, x: r(b.x), y: r(b.y), width: r(b.width), height: r(b.height) };
    });
  });
  const px = decodePng(await page.screenshot());
  const control = boxes.find((b) => b.cid === "m");
  const listed = probes.map(([dx, dy]) => [Math.floor(control.x) + dx, Math.floor(control.y) + dy]);
  const seen = new Set(listed.map((p) => p.join()));
  const grid = gridProbes(px, control, exclude).filter((p) => !seen.has(p.join()));
  const points = [...listed, ...grid];
  cases.push({
    id,
    html,
    boxes,
    probes: points.map(([x, y]) => ({ x, y, class: classOf(px(x, y)), rgb: px(x, y) })),
  });
}
await browser.close();

const fixture = {
  generator: "scripts/form-native-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases,
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 0).replaceAll('},{"x"', '},\n{"x"')}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) {
  const classes = {};
  for (const p of c.probes) classes[p.class] = (classes[p.class] || 0) + 1;
  console.log(c.id.padEnd(32), JSON.stringify(classes), c.boxes.map((b) => `${b.cid}:${b.x},${b.y} ${b.width}x${b.height}`).join("  "));
}

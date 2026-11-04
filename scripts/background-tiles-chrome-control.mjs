// Chrome-156 control for where a tiled background's tiles land once its box has been moved
// after layout or laid out in a vertical writing mode:
//   - miaskiewicz/turbo-dom#274 (`moved-*`): a flex item, a float and a horizontal box in a
//     vertical-rl flow, each laid out at its container's origin and then moved into place,
//     keep their tiles in phase;
//   - miaskiewicz/turbo-dom#277 (`ortho-*`): in an orthogonal flow (a horizontal box in a
//     vertical container, or a vertical box itself) percentage and keyword
//     `background-position`s and percentage `background-size`s resolve against the box's
//     physical size.
//
// Every case is one page (viewport 800x600, `body{margin:0;background:#fff}`) holding one
// `[data-cid=t]` box (no border, no padding) whose background is a 2px red stripe at each
// tile's left edge (`axis: "x"`) or top edge (`axis: "y"`). Chrome records the box's
// getBoundingClientRect() (`box`: [x, y, width, height]) and `stripes`: the start of every
// red run exactly 2px long along the box's middle row (x) or column (y) of a screenshot —
// the tiles whose whole stripe lies in the box. Asserted by
// crates/turbo-dom-layout/tests/background_tiles_moved.rs.
//
// Usage: node scripts/background-tiles-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156 headless shell (~/.cache/turbo-consolidation);
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/background_tiles_chrome156.json.
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
  join(homedir(), ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell");
const out = resolve(
  values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/background_tiles_chrome156.json"),
);
const playwrightCore = process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const STRIPE = { x: "linear-gradient(to right,red 0 2px,transparent 2px)", y: "linear-gradient(to bottom,red 0 2px,transparent 2px)" };
/** The target box: `size` (CSS), the background `layer` (position/size) and `extra` style. */
const box = (axis, size, layer, extra = "") =>
  `<div data-cid="t" style="${size};background:${STRIPE[axis]} ${layer}${extra ? `;${extra}` : ""}"></div>`;
const H = "width:100px;height:50px";
const RL = (w, h, inner) => `<div style="writing-mode:vertical-rl;width:${w}px;height:${h}px">${inner}</div>`;
const ATOM = '<span style="display:inline-block;width:20px;height:30px"></span>';
/** An inline box of five atoms with a 5px padding and the stripe background `layer`. */
const INLINE = (axis, layer, extra = "") =>
  `<span style="font-size:0"><span data-cid="t" style="padding:5px;background:${STRIPE[axis]} ${layer}${extra ? `;${extra}` : ""}">${ATOM.repeat(5)}</span></span>`;
const LR = (w, h, inner) => `<div style="writing-mode:vertical-lr;width:${w}px;height:${h}px">${inner}</div>`;

// [id, axis, body]
const CASES = [
  ["moved-flex", "x", `<div style="display:flex;padding:13px 7px">${box("x", H, "0 0/25px 25px")}</div>`],
  ["moved-float", "x", `<div style="padding:13px 7px">${box("x", H, "0 0/25px 25px", "float:left")}</div>`],
  ["moved-ortho-px-x", "x", RL(313, 200, box("x", H, "5px 3px/10px 4px"))],
  ["moved-ortho-px-y", "y", RL(313, 200, box("y", H, "5px 3px/10px 7px"))],
  ["ortho-pct-x", "x", RL(310, 100, box("x", H, "100% 100%/30px 4px"))],
  ["ortho-pct-y", "y", RL(310, 100, box("y", H, "100% 100%/4px 15px"))],
  ["ortho-keyword-x", "x", RL(310, 100, box("x", H, "right 7px bottom 3px/30px 4px"))],
  ["ortho-keyword-y", "y", RL(310, 100, box("y", H, "right 7px bottom 3px/4px 15px"))],
  ["ortho-center-x", "x", RL(310, 100, box("x", H, "center/30px 4px"))],
  ["ortho-size-pct-x", "x", RL(310, 100, box("x", H, "0 0/30% 20%"))],
  ["ortho-size-pct-y", "y", RL(310, 100, box("y", H, "0 0/30% 30%"))],
  ["ortho-lr-pct-x", "x", LR(310, 100, box("x", H, "100% 0/30px 4px"))],
  ["ortho-lr-pct-y", "y", LR(310, 100, box("y", H, "0 100%/4px 15px"))],
  ["ortho-self-pct-x", "x", box("x", H, "100% 0/30px 4px", "writing-mode:vertical-rl")],
  ["ortho-self-pct-y", "y", box("y", H, "0 100%/4px 15px", "writing-mode:vertical-rl")],
  ["ortho-self-lr-size-x", "x", box("x", H, "0 0/30% 20%", "writing-mode:vertical-lr")],
  // miaskiewicz/turbo-dom#278: an inline box in vertical text (font-size 0, so its
  // fragments are its padding around inline-block atoms: whole px), sliced across lines
  // or cloned per line.
  ["inline-rl-pct-y", "y", RL(300, 100, INLINE("y", "100% 100%/4px 15px"))],
  ["inline-rl-pct-x", "x", RL(300, 100, INLINE("x", "100% 100%/7px 4px"))],
  ["inline-rl-keyword-y", "y", RL(300, 100, INLINE("y", "right 3px bottom 4px/4px 15px"))],
  ["inline-rl-size-pct-y", "y", RL(300, 100, INLINE("y", "0 0/4px 10%"))],
  ["inline-rl-clone-pct-y", "y", RL(300, 100, INLINE("y", "100% 100%/4px 15px", "box-decoration-break:clone"))],
  ["inline-rl-clone-size-pct-x", "x", RL(300, 100, INLINE("x", "0 0/30% 4px", "box-decoration-break:clone"))],
  ["inline-lr-pct-y", "y", LR(300, 100, INLINE("y", "0 100%/4px 15px"))],
  ["inline-lr-clone-keyword-x", "x", LR(300, 100, INLINE("x", "right 2px top/7px 4px", "box-decoration-break:clone"))],
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

const isRed = ([r, g, b]) => r > 160 && g < 96 && b < 96;

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 });
const cases = [];
/** The starts of the 2px red runs along `axis` through the middle of rect `r`. */
function stripesIn(px, axis, [bx, by, bw, bh]) {
  const [from, len] = axis === "x" ? [bx, bw] : [by, bh];
  const at = (i) => (axis === "x" ? px(i, Math.floor(by + bh / 2)) : px(Math.floor(bx + bw / 2), i));
  const stripes = [];
  for (let i = from; i < from + len; ) {
    if (!isRed(at(i))) {
      i++;
      continue;
    }
    let end = i;
    while (end < from + len && isRed(at(end))) end++;
    if (end - i === 2) stripes.push(i);
    i = end;
  }
  return stripes;
}

for (const [id, axis, body] of CASES) {
  const html = `<!DOCTYPE html><style>body{margin:0;background:#fff}</style><body>${body}</body>`;
  await page.setContent(html, { waitUntil: "load" });
  const rects = await page.evaluate(() => {
    const t = document.querySelector("[data-cid=t]");
    const b = t.getBoundingClientRect();
    return {
      box: [b.x, b.y, b.width, b.height],
      fragments: [...t.getClientRects()].map((f) => [f.x, f.y, f.width, f.height]),
    };
  });
  const px = decodePng(await page.screenshot({ type: "png" }));
  if (id.startsWith("inline-")) {
    const fragments = rects.fragments.map((box) => ({ box, stripes: stripesIn(px, axis, box) }));
    cases.push({ id, axis, html, fragments });
  } else {
    cases.push({ id, axis, html, box: rects.box, stripes: stripesIn(px, axis, rects.box) });
  }
}
await browser.close();

const fixture = {
  generator: "scripts/background-tiles-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  cases,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) console.log(c.id.padEnd(22), JSON.stringify(c.fragments || [{ box: c.box, stripes: c.stripes }]));

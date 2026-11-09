// Chrome-156 paint control for turbo-dom-raster's multicol `column-rule-*`
// painter (miaskiewicz/turbo-dom#398 PR-D).
//
// Shape matches `scripts/paint-chrome-control.mjs`: each case is a full HTML
// page over a `#00ffff` backdrop (`body{margin:0;background:#00ffff}`) with
// Chrome 156 recording, per case, the `getBoundingClientRect()` of every
// `[data-cid]` element and the colour class (`R`/`G`/`B`/`Y`/`W`/`T`/`?`) and
// raw RGB at each probe point on the screenshot. Painters (PNG + SVG) are
// asserted against this fixture by
// `crates/turbo-dom-raster/tests/multicol_rule_chrome.rs`.
//
// Cases (CSS Multicol 1 §5 "The column-rule properties"):
//   * `multicol-rule-solid-1col-gap`: `columns:2;column-gap:20px;column-rule:
//     4px solid #333;width:400px;height:300px` — one centred solid rule in the
//     gap.
//   * `multicol-rule-dotted-3col`: `columns:3;column-gap:16px;column-rule:2px
//     dotted #c00` — two dotted rules between three columns.
//   * `multicol-rule-double-thick`: `columns:2;column-gap:40px;column-rule:12px
//     double #004;width:600px;height:200px` — a thick double rule centred in a
//     wide gap.
//   * `multicol-rule-none`: `columns:2;column-gap:20px;column-rule:4px none
//     #333` — `none` suppresses the rule.
//   * `multicol-rule-wider-than-gap`: `columns:2;column-gap:4px;
//     column-rule-width:12px;column-rule-style:solid;column-rule-color:#000` —
//     a rule wider than the gap: Chrome keeps it centred and lets it overlap.
//
// Usage: node crates/turbo-dom-raster/scripts/multicol-rule-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the cached Chrome 156 headless shell
//     (~/.cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64);
//   --out    defaults to
//     crates/turbo-dom-raster/tests/fixtures/multicol_rule_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE (default: a sibling turbo-surf
// checkout's node_modules/playwright-core).
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { inflateSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
  );
const out = resolve(
  values.out ||
    join(repo, "crates/turbo-dom-raster/tests/fixtures/multicol_rule_chrome156.json"),
);
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DOC = (body) =>
  `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>${body}</body></html>`;

// A fixed-height block inside the multicol flow: the children let Chrome pick
// column strips wide enough for a visible rule. `#eef` is a light wash the
// probes do not cross.
const BLK = (h, i) => `<div data-cid="i${i}" style="height:${h}px;background:#eef"></div>`;
const stack = (heights) => heights.map((h, i) => BLK(h, i)).join("");

const CASES = [
  {
    // Two equal columns separated by a 20 px gap; one 4 px centred rule.
    // Content-box 20..420, column-width (400 - 20) / 2 = 190: col 0 = 20..210,
    // gap = 210..230 (centre 220), col 1 = 230..420. Rule 4 px at 218..222.
    id: "multicol-rule-solid-1col-gap",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:300px;columns:2;column-gap:20px;column-rule:4px solid #333333">${stack([80, 80, 80, 80])}</div>`,
    ),
    probes: [
      // Deep inside the rule (dark grey `?`).
      [220, 100],
      [219, 100],
      [221, 100],
      // One pixel outside the rule in the gap — the cyan backdrop shows
      // through (the container has no background).
      [217, 100],
      [223, 100],
      // In a column block (its `#eef` wash shows as `W`).
      [50, 60],
      [370, 60],
    ],
  },
  {
    // Three columns, two 2 px dotted rules, 16 px gaps. Content-box 20..620,
    // column-width (600 - 32) / 3 ≈ 189.328 (Blink's LayoutUnit truncation):
    // col 0 = 20..209.328, gap 0 = 209.328..225.328 (centre 217.328),
    // col 1 = 225.328..414.656, gap 1 = 414.656..430.656 (centre 422.656),
    // col 2 = 430.656..620. Blink stamps dots of diameter `width` centred
    // along the rule; both painters agree on the first dot at the rule top
    // and the gap that follows. Chrome's exact phase after the first dot
    // follows LayoutUnit arithmetic beyond this PR's scope (probes stop at
    // the first dot).
    id: "multicol-rule-dotted-3col",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:600px;height:200px;columns:3;column-gap:16px;column-rule:2px dotted #cc0000">${stack([60, 60, 60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Rule 0 centre (x = 217), the first dot's y row.
      [217, 21],
      // One pixel past the first dot: the gap.
      [217, 22],
      // Just above the rule — backdrop.
      [217, 16],
      // Rule 1 centre (x = 422), the same first dot row.
      [422, 21],
      // Far backdrop.
      [10, 10],
      // Column block shows its `#eef` wash.
      [50, 40],
    ],
  },
  {
    // A thick double rule centred in a 40 px gap. Content-box 20..620,
    // column-width (600 - 40) / 2 = 280: col 0 = 20..300, gap = 300..340
    // (centre 320), col 1 = 340..620. Rule 12 px at 314..326; double bands
    // of 4 px thick at each long edge (left 314..318, hollow 318..322,
    // right 322..326).
    id: "multicol-rule-double-thick",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:600px;height:200px;columns:2;column-gap:40px;column-rule:12px double #000044">${stack([60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Left band centre (dark navy `?`).
      [316, 40],
      // Hollow centre (backdrop shows through).
      [320, 40],
      // Right band centre (dark navy `?`).
      [324, 40],
      // Gap just before the left band — cyan backdrop.
      [310, 40],
      // Column block — `#eef` wash.
      [50, 40],
    ],
  },
  {
    // `column-rule-style: none` suppresses the rule entirely. Content-box
    // and column layout as `multicol-rule-solid-1col-gap`.
    id: "multicol-rule-none",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:300px;columns:2;column-gap:20px;column-rule:4px none #333333">${stack([80, 80, 80, 80])}</div>`,
    ),
    probes: [
      // Where the rule would have been: cyan backdrop shows through.
      [220, 100],
      // Column blocks still paint their own wash.
      [50, 60],
      [370, 60],
    ],
  },
  {
    // Rule wider than the gap: 12 px declared width, 4 px gap. Chrome 156
    // clamps the painted rule to the gap (Blink's multicol painter never
    // paints outside the gap). Content-box 20..420, column-width
    // (400 - 4) / 2 = 198: col 0 = 20..218, gap = 218..222 (centre 220),
    // col 1 = 222..420. Painted rule: 4 px at 218..222.
    id: "multicol-rule-wider-than-gap",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:200px;columns:2;column-gap:4px;column-rule-width:12px;column-rule-style:solid;column-rule-color:#000000">${stack([60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Deep inside the rule (black `?`).
      [220, 50],
      [219, 50],
      [221, 50],
      // Outside the rule, inside the adjacent columns — block `#eef`.
      [215, 50],
      [225, 50],
    ],
  },
  {
    // `dashed` style: 2 px rule, 20 px gap. The painters stamp dashes of
    // `2 * width` px length on period `3 * width` (a `width` gap between).
    // Content-box 20..420, column-width (400 - 20) / 2 = 190: col 0 =
    // 20..210, gap = 210..230 (centre 220), col 1 = 230..420. Rule 2 px at
    // 219..221.
    id: "multicol-rule-dashed",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:200px;columns:2;column-gap:20px;column-rule:2px dashed #008800">${stack([60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Rule centre, first-dash row — green.
      [219, 21],
      // Column block — `#eef` wash.
      [50, 40],
      // Backdrop outside.
      [10, 10],
    ],
  },
  {
    // `currentcolor` resolves to the container's own `color` (initial
    // black) — a plain `column-rule-color: currentcolor` on a `color: red`
    // container paints a red rule.
    id: "multicol-rule-currentcolor",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:200px;columns:2;column-gap:20px;column-rule-width:4px;column-rule-style:solid;column-rule-color:currentcolor;color:#ff0000">${stack([60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Rule centre — red.
      [220, 60],
      // Column block — `#eef` wash.
      [50, 60],
      // Backdrop outside.
      [10, 10],
    ],
  },
  // #424: dotted with a declared width smaller than 1px (gap-clamped).
  // Chrome stamps a dot pattern centred in the gap; the probe at the
  // first dot row's rule centre records whatever Chrome paints — this
  // pins the sub-pixel fallback (dot visible vs collapsed to backdrop).
  // Content-box 20..420, column-gap:2, col-width 199: col0 20..219,
  // gap 219..221 (centre 220), col1 221..420. Declared 0.5px rule
  // clamped to the 2px gap.
  {
    id: "multicol-rule-dotted-subpixel",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:200px;columns:2;column-gap:2px;column-rule-width:0.5px;column-rule-style:dotted;column-rule-color:#000000">${stack([60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // First dot centre row (y ~ container top + radius ~ 20+0.25).
      [220, 20],
      // One row down.
      [220, 21],
      // Column block — `#eef` wash.
      [50, 60],
      // Backdrop outside.
      [10, 10],
    ],
  },
  // #424: double rule at 2px declared width — floor(2/3)=0, so Chrome
  // paints a solid rule rather than invisible bands. Gap 6, col-width
  // 197, col0 20..217, gap 217..223 (centre 220), col1 223..420. Rule
  // 2px at 219..221.
  {
    id: "multicol-rule-double-thin",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:200px;columns:2;column-gap:6px;column-rule-width:2px;column-rule-style:double;column-rule-color:#000000">${stack([60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Rule centre — black.
      [220, 40],
      // One px left of the rule — in the gap band, backdrop.
      [218, 40],
      // One px right — in the gap band, backdrop.
      [222, 40],
      // Column block — `#eef` wash.
      [50, 40],
      // Backdrop outside.
      [10, 10],
    ],
  },
  // #457: broader colour / width / gap — green 6px solid, gap 10.
  // col-width 195, col0 20..215, gap 215..225 (centre 220), col1
  // 225..420. Rule 6px at 217..223.
  {
    id: "multicol-rule-solid-wide-green",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:200px;columns:2;column-gap:10px;column-rule:6px solid #00aa00">${stack([60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Rule centre — green.
      [220, 40],
      [218, 40],
      [222, 40],
      // Column block — `#eef` wash.
      [50, 40],
      // Backdrop outside.
      [10, 10],
    ],
  },
  // #457: three columns + solid blue rules, wide 30 px gaps.
  // content-box 20..620, col-width (600-60)/3 = 180, col0 20..200,
  // gap0 200..230 (centre 215), col1 230..410, gap1 410..440
  // (centre 425), col2 440..620. Rule 4px at 213..217 and 423..427.
  {
    id: "multicol-rule-solid-3col-wide-gap",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:600px;height:200px;columns:3;column-gap:30px;column-rule:4px solid #0000aa">${stack([60, 60, 60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Left rule centre — blue.
      [215, 40],
      // Right rule centre — blue.
      [425, 40],
      // Column block — `#eef` wash.
      [50, 40],
      // Backdrop outside.
      [10, 10],
    ],
  },
  // #457: thin 1px solid yellow with an 8 px gap.
  // col-width 196, col0 20..216, gap 216..224 (centre 220), col1
  // 224..420. Rule 1px at 219.5..220.5.
  {
    id: "multicol-rule-solid-thin-yellow",
    html: DOC(
      `<div data-cid="c" style="position:absolute;left:20px;top:20px;width:400px;height:200px;columns:2;column-gap:8px;column-rule:1px solid #cccc00">${stack([60, 60, 60, 60])}</div>`,
    ),
    probes: [
      // Rule centre — yellow.
      [220, 40],
      // Column block — `#eef` wash.
      [50, 40],
      // Backdrop outside.
      [10, 10],
    ],
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
        const pa = Math.abs(p - a),
          pb = Math.abs(p - b),
          pc = Math.abs(p - c);
        v = x + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c);
      }
      pix[y * stride + i] = v & 255;
    }
  }
  return (x, y) => [...pix.subarray(y * stride + x * channels, y * stride + x * channels + 3)];
}

function classOf([r, g, b]) {
  const hi = (v) => v > 160,
    lo = (v) => v < 96;
  if (hi(r) && hi(g) && hi(b)) return "W";
  if (lo(r) && hi(g) && hi(b)) return "T";
  if (hi(r) && lo(g) && lo(b)) return "R";
  if (lo(r) && hi(g) && lo(b)) return "G";
  if (lo(r) && lo(g) && hi(b)) return "B";
  if (hi(r) && hi(g) && lo(b)) return "Y";
  return "?";
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
  const boxes = await page.evaluate(() => {
    const r = (n) => Math.round(n * 1000) / 1000;
    return [...document.querySelectorAll("[data-cid]")].map((el) => {
      const b = el.getBoundingClientRect();
      return { cid: el.dataset.cid, x: r(b.x), y: r(b.y), width: r(b.width), height: r(b.height) };
    });
  });
  const px = decodePng(await page.screenshot());
  const probes = testCase.probes.map(([x, y]) => ({
    x,
    y,
    class: classOf(px(x, y)),
    rgb: px(x, y),
  }));
  cases.push({ id: testCase.id, html: testCase.html, assets: [], boxes, probes });
}
await browser.close();

const fixture = {
  generator: "crates/turbo-dom-raster/scripts/multicol-rule-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases,
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) {
  console.log(c.id.padEnd(32), c.probes.map((p) => `${p.x},${p.y}:${p.class} ${p.rgb.join(",")}`).join("  "));
}

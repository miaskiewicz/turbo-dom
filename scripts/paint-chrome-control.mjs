// Chrome-156 pixel control for turbo-dom-raster's screenshots (P5b-W5):
//   - miaskiewicz/turbo-dom#51 (`caption-*`): a table caption is laid out in the table
//     wrapper box, above (`caption-side: top`) or below (`bottom`) the table grid box; the
//     table's background and border paint the grid box only, never the caption;
//   - miaskiewicz/turbo-dom#52 (`clip-*`): an absolutely positioned box's `clip: rect()`
//     clips the box and its descendants; a zero-area clip paints nothing (the box keeps its
//     border box, miaskiewicz/turbo-dom#75); `clip` does not
//     apply to a box that is not absolutely positioned.
// P5b-W7 adds miaskiewicz/turbo-dom#81 (`icb-*`): the screenshot's initial containing block
// is its viewport, so `position: fixed` boxes and absolutely positioned ones it contains
// anchor `bottom` / `%` insets to the viewport's bottom edge.
//
// Every case is one page (viewport 1280x800, `body{margin:0;background:#00ffff}`). Chrome for
// Testing 156 records, per case, the getBoundingClientRect() of every `[data-cid]` element
// (`boxes`: `{cid, x, y, width, height}`) and the colour class at each listed probe point
// (`probes`: `{x, y, class, rgb}`) of a screenshot: R/G/B/Y/W, T = the page's #00ffff
// backdrop, `?` = anything else; `rgb` is the raw screenshot colour there. `assets` lists the case's `<img src>` `data:` URLs (the screenshot API
// takes image bytes from the caller, keyed by the raw `src`). Asserted by
// crates/turbo-dom-raster/tests/paint_chrome.rs (pixels) and
// crates/turbo-dom-layout/tests/html_document_chrome.rs (boxes, through `layout_boxes`).
//
// Usage: node scripts/paint-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156 headless shell (~/.cache/turbo-consolidation);
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/paint_chrome156.json.
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/paint_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DOC = (body) =>
  `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>${body}</body></html>`;
const at = (x, y) => `position:absolute;left:${x}px;top:${y}px`;
// A text-free coloured block.
const BLOCK = (w, h, bg, cid = "") =>
  `<div${cid ? ` data-cid="${cid}"` : ""} style="width:${w}px;height:${h}px${bg ? `;background:${bg}` : ""}"></div>`;
// A red table with a 4px blue border, no spacing, one 40x20 cell (no background).
const TABLE = (style, caption, cellStyle = "") =>
  `<table data-cid="t" style="${style};background:#ff0000;border:4px solid #0000ff;border-spacing:0">${caption}<tr><td data-cid="td" style="padding:0${cellStyle}">${BLOCK(40, 20, "")}</td></tr></table>`;
// A 100x40 PNG sprite: left half red, right half blue.
const SPRITE =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAGQAAAAoCAIAAACHGsgUAAAAU0lEQVR42u3QMQ0AAAzDsPIn3cFYD0sBEDlNBpucamDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggUL1ncHs7SRQvFoC8gAAAAASUVORK5CYII=";

// The fixture Liberation Serif (the bundled face the screenshot's font registry resolves),
// injected as an @font-face after load so the recorded `html` stays small.
const FONT = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf")).toString("base64");
const FACE = `@font-face{font-family:"Liberation Serif";src:url(data:font/ttf;base64,${FONT})}`;
const SERIF = `font-family:'Liberation Serif';font-size:20px;line-height:30px;color:transparent`;
// A `w`x`h` inline-block, transparent unless `bg` is given.
const ATOM = (w, h, bg = "", cid = "") => `<span${cid ? ` data-cid="${cid}"` : ""} style="display:inline-block;width:${w}px;height:${h}px${bg ? `;background:${bg}` : ""}"></span>`;
const RTL_OVERRIDE = (inner) => `<span style="direction:rtl;unicode-bidi:bidi-override">${inner}</span>`;

// A 40x40 inline-block of colour `bg`.
const IB = (bg, cid) => `<span data-cid="${cid}" style="display:inline-block;width:40px;height:40px;background:${bg}"></span>`;

const CASES = [
  // miaskiewicz/turbo-dom#51.
  {
    // The caption (100 wide, 5px margins) sits above the grid; the grid is stretched to the
    // wrapper's 110px; the table's red background and blue border stay off the caption and
    // its margins.
    id: "caption-top",
    html: DOC(TABLE(at(20, 10), `<caption data-cid="c" style="margin:5px">${BLOCK(100, 20, "#00ff00", "cd")}</caption>`)),
    probes: [[70, 25], [22, 12], [70, 36], [120, 25], [60, 47], [110, 47], [21, 47], [70, 65]],
  },
  {
    id: "caption-bottom",
    html: DOC(TABLE(`${at(20, 10)};caption-side:bottom`, `<caption data-cid="c">${BLOCK(100, 20, "#00ff00", "cd")}</caption>`)),
    probes: [[70, 20], [110, 20], [21, 20], [70, 45], [70, 55], [118, 45], [70, 9], [70, 59]],
  },
  {
    // A caption narrower than the grid takes the grid's width; the grid keeps its own.
    id: "caption-narrow",
    html: DOC(TABLE(`${at(20, 10)};width:120px`, `<caption data-cid="c">${BLOCK(30, 10, "#00ff00", "cd")}</caption>`)),
    probes: [[30, 15], [100, 15], [100, 30], [130, 30], [139, 30]],
  },
  // miaskiewicz/turbo-dom#52.
  {
    // The issue's repro: a 30x30 red square at (10,10); the border box stays 50x50.
    id: "clip-partial",
    html: DOC(`<div style="position:relative;height:100px"><div data-cid="c" style="${at(0, 0)};width:50px;height:50px;background:#ff0000;clip:rect(10px,40px,40px,10px)"></div></div>`),
    probes: [[25, 25], [11, 11], [38, 38], [5, 5], [45, 45], [25, 5], [25, 45], [5, 25], [45, 25]],
  },
  {
    // `auto` is the border-box edge; commas are optional.
    id: "clip-auto-edges",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:60px;height:40px;background:#ff0000;clip:rect(auto 30px auto auto)"></div>`),
    probes: [[25, 30], [48, 58], [55, 30], [75, 50]],
  },
  {
    // The clip applies to the descendants: an overflowing child is cut at the clip edge.
    id: "clip-descendants",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:50px;height:50px;clip:rect(0,60px,60px,0)">${BLOCK(100, 100, "#00ff00", "k")}</div>`),
    probes: [[50, 50], [75, 75], [100, 50], [50, 100], [110, 110]],
  },
  {
    // A transformed box clips in its own (transformed) coordinate space.
    id: "clip-transformed",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:50px;height:50px;background:#ff0000;transform:translate(100px,0);clip:rect(0,25px,50px,0)"></div>`),
    probes: [[130, 45], [160, 45], [40, 45]],
  },
  {
    // The sr-only pattern paints nothing.
    id: "clip-zero-area",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:50px;height:50px;background:#ff0000;clip:rect(0,0,0,0)"></div><div style="${at(100, 20)};width:1px;height:1px;overflow:hidden;clip:rect(1px,1px,1px,1px)">${BLOCK(40, 40, "#ff0000")}</div>`),
    probes: [[45, 45], [110, 30], [101, 21]],
  },
  {
    // A zero-width clip whose left edge is past the box's right edge (`auto` right).
    id: "clip-zero-area-auto",
    html: DOC(`<div data-cid="c" style="${at(20, 20)};width:50px;height:50px;background:#ff0000;clip:rect(0,auto,auto,60px)"></div>`),
    probes: [[45, 45], [25, 25]],
  },
  {
    // `clip` applies only to absolutely positioned boxes: the static and relative boxes paint
    // whole.
    id: "clip-not-positioned",
    html: DOC(`<div data-cid="s" style="width:50px;height:50px;background:#ff0000;clip:rect(0,0,0,0)"></div><div data-cid="r" style="position:relative;width:50px;height:50px;background:#0000ff;clip:rect(0,10px,10px,0)"></div>`),
    probes: [[25, 25], [25, 75], [40, 90]],
  },
  {
    // The CSS-sprite pattern: a clipped `<img>` shows only its left half.
    id: "clip-image-sprite",
    html: DOC(`<img data-cid="i" src="${SPRITE}" style="${at(20, 20)};clip:rect(0,50px,40px,0)">`),
    assets: [SPRITE],
    probes: [[45, 40], [95, 40], [21, 21], [69, 59]],
  },
  // miaskiewicz/turbo-dom#81.
  {
    // A fixed red square 10px off the bottom-right corner, below a taller-than-viewport block.
    id: "icb-fixed-bottom-right",
    html: DOC(`${BLOCK(10, 2000, "")}<div data-cid="c" style="position:fixed;right:10px;bottom:10px;width:20px;height:20px;background:#ff0000"></div>`),
    probes: [[1260, 780], [1251, 771], [1260, 765], [1260, 795], [1275, 780], [1260, 10]],
  },
  {
    // An absolutely positioned blue bar at the bottom of the initial containing block, and a
    // green one at its vertical middle (`top: 50%`).
    id: "icb-absolute-bottom-and-percent",
    html: DOC(`<div data-cid="b" style="position:absolute;left:100px;bottom:0;width:200px;height:20px;background:#0000ff"></div><div data-cid="m" style="position:absolute;left:100px;top:50%;width:200px;height:20px;background:#00ff00"></div>`),
    probes: [[200, 790], [200, 779], [200, 410], [200, 399], [200, 10]],
  },
  // P5-cov: the raster painter's box decoration paths.
  {
    // A rounded box with a uniform border: the border band follows the radius and the
    // corners outside it stay the backdrop.
    id: "paint-round-border",
    html: DOC(`<div style="${at(20, 20)};width:60px;height:40px;background:#ff0000;border:4px solid #0000ff;border-radius:12px"></div>`),
    probes: [[54, 44], [54, 21], [21, 44], [86, 44], [54, 66], [21, 21], [86, 66]],
  },
  {
    // A linear gradient clipped to the rounded border box.
    id: "paint-round-gradient",
    html: DOC(`<div style="${at(20, 90)};width:100px;height:50px;border-radius:20px;background:linear-gradient(to right, #ff0000, #0000ff)"></div>`),
    probes: [[25, 115], [115, 115], [70, 93], [21, 91], [118, 138]],
  },
  {
    // An unblurred offset shadow is the box's silhouette in the shadow colour.
    id: "paint-shadow-hard",
    html: DOC(`<div style="${at(20, 160)};width:40px;height:40px;background:#00ff00;box-shadow:10px 10px 0 #ff0000"></div>`),
    probes: [[40, 180], [65, 205], [25, 205], [65, 165]],
  },
  {
    // A negative spread larger than half the box leaves no shadow.
    id: "paint-shadow-collapsed",
    html: DOC(`<div style="${at(100, 160)};width:20px;height:20px;background:#00ff00;box-shadow:30px 0 0 -15px #ff0000"></div>`),
    probes: [[110, 170], [135, 170], [145, 170]],
  },
  {
    // A spread grows the rounded silhouette (radius + spread) on every side.
    id: "paint-shadow-spread-round",
    html: DOC(`<div style="${at(200, 170)};width:40px;height:40px;background:#00ff00;border-radius:10px;box-shadow:0 0 0 6px #ff0000"></div>`),
    probes: [[196, 190], [220, 190], [243, 190], [220, 166], [195, 165]],
  },
  {
    // Transparent text paints nothing over its box.
    id: "paint-text-transparent",
    html: DOC(`<div style="${at(300, 20)};font-size:40px;color:transparent;background:#00ff00">MMMM</div>`),
    probes: [[310, 40], [330, 45], [350, 40], [370, 45]],
  },
  {
    // Blurred shadows, square and rounded: the shadow's core is its colour, the blur
    // fades out past its edge.
    id: "paint-shadow-blur",
    html: DOC(
      `<div style="${at(300, 100)};width:40px;height:40px;background:#00ff00;box-shadow:50px 0 4px #ff0000"></div>` +
        `<div style="${at(300, 160)};width:40px;height:40px;background:#00ff00;border-radius:8px;box-shadow:50px 0 4px #0000ff"></div>`,
    ),
    probes: [[320, 120], [370, 120], [400, 120], [370, 92], [370, 180], [400, 180], [352, 162]],
  },
  {
    // Translucent colours: a half-transparent background and shadow over the backdrop
    // (`rgb` carries the composited values).
    id: "paint-alpha",
    html: DOC(`<div style="${at(420, 20)};width:40px;height:40px;background:rgba(255,0,0,0.5);box-shadow:0 50px 0 rgba(0,0,255,0.5)"></div>`),
    probes: [[440, 40], [440, 90], [440, 130]],
  },
  {
    // A gradient in a square box (top to bottom).
    id: "paint-gradient-square",
    html: DOC(`<div style="${at(500, 20)};width:40px;height:100px;background:linear-gradient(#ff0000, #0000ff)"></div>`),
    probes: [[520, 23], [520, 117], [520, 70], [545, 70]],
  },
  // miaskiewicz/turbo-dom#95: blurred shadows of boxes far wider than the viewport, and the
  // blur radius Chrome caps (every blur from 256px up paints the same pixels).
  {
    id: "shadow-wide",
    html: DOC(`<div style="${at(20, 300)};width:9000px;height:40px;background:#00ff00;box-shadow:0 50px 4px #ff0000"></div>`),
    probes: [[100, 320], [100, 370], [10, 370], [100, 400], [1270, 370], [640, 345]],
  },
  {
    // A spread grows the shadow's corner radius by the spread scaled by 1 + (r/s - 1)^3 when
    // the radius r is below the spread s (CSS Backgrounds 3 §7.1): a square box keeps a square
    // shadow; a 2px radius with an 8px spread rounds to about 6.6px, not 10px.
    id: "shadow-spread-small-radius",
    html: DOC(
      `<div style="${at(600, 60)};width:40px;height:40px;background:#00ff00;box-shadow:0 0 0 8px #ff0000"></div>` +
        `<div style="${at(700, 60)};width:40px;height:40px;background:#00ff00;border-radius:2px;box-shadow:0 0 0 8px #ff0000"></div>`,
    ),
    probes: [[593, 53], [646, 53], [593, 106], [646, 106], [620, 53], [694, 54], [745, 54], [694, 105], [745, 105], [720, 53]],
  },
  {
    id: "shadow-blur-256",
    html: DOC(`<div style="${at(600, 300)};width:100px;height:100px;background:#0000ff;box-shadow:0 0 256px #000000"></div>`),
    probes: [[550, 350], [500, 350], [450, 350], [650, 250], [650, 150], [400, 600]],
  },
  {
    id: "shadow-blur-400",
    html: DOC(`<div style="${at(600, 300)};width:100px;height:100px;background:#0000ff;box-shadow:0 0 400px #000000"></div>`),
    probes: [[550, 350], [500, 350], [450, 350], [650, 250], [650, 150], [400, 600]],
  },
  // miaskiewicz/turbo-dom#192: inset shadows paint inside the padding box, over the
  // background and under the border, with offset / spread / blur.
  {
    // An offset hard inset shadow is a band along the top and left padding edges.
    id: "inset-hard-offset",
    html: DOC(`<div style="${at(20, 20)};width:60px;height:60px;background:#00ff00;box-shadow:inset 10px 10px 0 #ff0000"></div>`),
    probes: [[25, 50], [50, 25], [25, 25], [50, 50], [77, 50], [50, 77], [15, 50], [85, 85]],
  },
  {
    // A spread ring sits inside a 4px border, which paints over nothing of it.
    id: "inset-spread-border",
    html: DOC(`<div style="${at(120, 20)};width:60px;height:60px;background:#00ff00;border:4px solid #0000ff;box-shadow:inset 0 0 0 6px #ff0000"></div>`),
    probes: [[122, 54], [127, 54], [133, 54], [137, 54], [154, 54], [154, 27], [154, 33], [181, 54], [154, 86], [154, 82]],
  },
  {
    // A rounded box: the ring follows the padding-box radius; corners outside stay the backdrop.
    id: "inset-spread-round",
    html: DOC(`<div style="${at(220, 20)};width:60px;height:60px;background:#00ff00;border-radius:20px;box-shadow:inset 0 0 0 6px #ff0000"></div>`),
    probes: [[221, 21], [223, 50], [250, 23], [250, 50], [278, 79], [229, 29], [235, 35], [250, 77]],
  },
  {
    // A blurred inset shadow: dark at the padding edge, fading to the background inside.
    id: "inset-blur",
    html: DOC(`<div style="${at(320, 20)};width:80px;height:80px;background:#00ff00;box-shadow:inset 0 0 10px 4px #ff0000"></div>`),
    probes: [[321, 60], [323, 60], [360, 21], [360, 60], [345, 60], [340, 40], [398, 98]],
  },
  {
    // A spread larger than half the box fills the whole padding box.
    id: "inset-full",
    html: DOC(`<div style="${at(420, 20)};width:40px;height:40px;background:#00ff00;border:2px solid #0000ff;box-shadow:inset 0 0 0 30px #ff0000"></div>`),
    probes: [[442, 42], [425, 25], [421, 42], [442, 63]],
  },
  // miaskiewicz/turbo-dom#238: the screenshot paints bidi-reordered runs. An override
  // reverses its atoms (U+FFFC is a neutral, so `bidi-override` makes it R), an RTL
  // paragraph places its neutral atoms right to left, and `<bdo>` overrides in line.
  {
    id: "bidi-override-atoms",
    html: DOC(`<div style="${at(0, 0)};font-size:0"><span style="direction:rtl;unicode-bidi:bidi-override">${IB("#ff0000", "a")}${IB("#00ff00", "b")}${IB("#0000ff", "c")}</span></div>`),
    probes: [[20, 20], [60, 20], [100, 20], [130, 20]],
  },
  {
    id: "bidi-rtl-paragraph-atoms",
    html: DOC(`<div dir="rtl" style="${at(0, 50)};width:200px;font-size:0">${IB("#ff0000", "d")}${IB("#0000ff", "e")}</div>`),
    probes: [[180, 70], [140, 70], [100, 70], [20, 70]],
  },
  {
    // `<bdo dir=rtl>` is an inline isolate-override: its atoms reverse in place.
    id: "bidi-bdo-atoms",
    html: DOC(`<div style="${at(0, 110)};font-size:0"><bdo dir="rtl">${IB("#ff0000", "f")}${IB("#00ff00", "g")}</bdo>${IB("#0000ff", "h")}</div>`),
    probes: [[20, 130], [60, 130], [100, 130], [140, 130]],
  },
  // miaskiewicz/turbo-dom#244: an inline box paints its background, padding and border
  // per line fragment (box-decoration-break: slice): the start edge on its first
  // fragment, the end edge on its last; padding and border also push its neighbours.
  // Text is transparent so the probes see the decoration only. The text cases' boxes
  // follow the text advances in Chrome's 1/64px LayoutUnits (miaskiewicz/turbo-dom#246).
  {
    id: "inline-background-text",
    html: DOC(`<div style="${at(20, 20)};${SERIF}">aaa <span data-cid="s" style="background:#ff0000">MMMM</span> bbb${ATOM(10, 10, "", "a")}</div>`),
    probes: [[87, 35], [53, 35], [121, 35], [49, 35], [125, 35], [87, 25], [87, 22], [87, 45], [87, 48]],
  },
  {
    id: "inline-edges-text",
    html: DOC(`<div style="${at(20, 20)};${SERIF}">aa<span data-cid="s" style="background:#ff0000;padding:4px 8px;border:3px solid #0000ff;margin:0 10px">MM</span>${ATOM(40, 40, "#00ff00", "a")}</div>`),
    probes: [[49, 50], [52, 50], [104, 50], [87, 36], [87, 40], [87, 69], [87, 73], [45, 50], [110, 50], [135, 40]],
  },
  {
    id: "inline-slice-atoms",
    html: DOC(`<div style="${at(20, 20)};width:100px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 6px;border-left:4px solid #0000ff;border-right:4px solid #0000ff">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    probes: [[22, 50], [27, 50], [108, 50], [112, 50], [60, 40], [22, 80], [104, 80], [108, 80], [112, 80], [60, 65]],
  },
  {
    id: "inline-bidi-split-atoms",
    html: DOC(`<div style="${at(20, 20)};font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 0;border-left:4px solid #0000ff;border-right:4px solid #0000ff">${ATOM(40, 30)}${RTL_OVERRIDE(ATOM(40, 30))}</span>${RTL_OVERRIDE(ATOM(40, 30, "#00ff00"))}</div>`),
    probes: [[22, 50], [40, 50], [62, 50], [84, 52], [84, 40], [106, 52], [142, 52], [146, 52], [150, 52]],
  },
  // `box-decoration-break: clone`: every fragment has both edges, which also take line
  // space (two atoms still fit each 100px line with 10px edges on both sides).
  {
    id: "inline-clone-atoms",
    html: DOC(`<div style="${at(20, 20)};width:100px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 6px;border-left:4px solid #0000ff;border-right:4px solid #0000ff;box-decoration-break:clone;-webkit-box-decoration-break:clone">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    probes: [[22, 50], [27, 50], [100, 50], [113, 50], [118, 50], [122, 50], [22, 80], [27, 80], [113, 80], [118, 80], [122, 80], [60, 65]],
  },
  {
    // The line keeps room for the cloned end edge: two atoms (90px with the start edge)
    // fit a 95px line, but not with the 10px end edge, so each line holds one atom
    // (mutation audit 0.7.0: Placer::clone_reserve).
    id: "inline-clone-reserve",
    html: DOC(`<div style="${at(20, 20)};width:95px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 6px;border-left:4px solid #0000ff;border-right:4px solid #0000ff;box-decoration-break:clone;-webkit-box-decoration-break:clone">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    probes: [[22, 50], [78, 50], [90, 50], [22, 80], [78, 80], [90, 80], [78, 140], [78, 170]],
  },
  {
    // The same with text: "xx xx" (55px with the start edge, Liberation Serif 20px) stays
    // on a 60px line although the end edge then overflows it.
    id: "inline-clone-reserve-text",
    html: DOC(`<div style="${at(20, 20)};width:60px;font:20px/30px 'Liberation Serif'"><span data-cid="s" style="background:#ff0000;padding:5px 6px;border-left:4px solid #0000ff;border-right:4px solid #0000ff;box-decoration-break:clone;-webkit-box-decoration-break:clone">xx xx xx xx</span></div>`),
    probes: [[22, 35], [92, 35], [22, 65], [92, 65], [22, 95]],
  },
  {
    // No break opportunity before an end tag: the last atom moves to a new line with the
    // box's end edge when both do not fit (80px + 10px on an 85px line), sliced or not.
    id: "inline-slice-close-glued",
    html: DOC(`<div style="${at(20, 20)};width:85px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 6px;border-left:4px solid #0000ff;border-right:4px solid #0000ff">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    probes: [[22, 50], [68, 50], [78, 50], [22, 80], [58, 80], [68, 80], [22, 110], [68, 110]],
  },
  {
    // Nested end tags glue to the atom before them together (3px + 5px): two atoms on a
    // 95px line wrap the second; three atoms exactly fill an 88px line's 5 + 3 + 80px.
    id: "inline-close-glued-nested",
    html: DOC(`<div style="${at(20, 20)};width:88px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px"><span style="padding:0 3px">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></span></div><div style="${at(20, 200)};width:95px;font-size:0"><span data-cid="t" style="background:#ff0000;padding:5px"><span style="padding:0 3px">${ATOM(40, 30)}${ATOM(40, 30)}</span></span></div>`),
    probes: [[22, 50], [100, 50], [106, 50], [22, 80], [64, 80], [72, 80], [22, 230], [64, 230], [72, 230], [22, 260], [64, 260]],
  },
  {
    // A forced break inside a cloned box: both lines' fragments repeat both edges.
    id: "inline-clone-br",
    html: DOC(`<div style="${at(20, 20)};width:200px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 6px;border-left:4px solid #0000ff;border-right:4px solid #0000ff;box-decoration-break:clone;-webkit-box-decoration-break:clone">${ATOM(40, 30)}${ATOM(40, 30)}<br>${ATOM(40, 30)}</span></div>`),
    probes: [[22, 50], [113, 50], [118, 50], [122, 50], [22, 80], [27, 80], [73, 80], [78, 80], [82, 80]],
  },
  // miaskiewicz/turbo-dom#248: background-size / -position / -repeat / -origin / -clip.
  // SPRITE is 100x40, left half red, right half blue; the boxes' colour is green.
  {
    id: "bg-size-position-no-repeat",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:100px;background:#00ff00 url(${SPRITE}) no-repeat;background-size:50px 20px;background-position:30px 10px"></div>`),
    assets: [SPRITE],
    probes: [[60, 40], [90, 40], [110, 40], [40, 25], [60, 55], [52, 32], [98, 48]],
  },
  {
    id: "bg-default-repeat",
    html: DOC(`<div style="${at(20, 20)};width:250px;height:90px;background:#00ff00 url(${SPRITE})"></div>`),
    assets: [SPRITE],
    probes: [[40, 40], [100, 40], [140, 40], [200, 40], [240, 40], [260, 40], [40, 80], [100, 80], [240, 105]],
  },
  {
    id: "bg-repeat-x",
    html: DOC(`<div style="${at(20, 20)};width:300px;height:100px;background:#00ff00 url(${SPRITE}) repeat-x 0 50%/40px 16px"></div>`),
    assets: [SPRITE],
    probes: [[30, 70], [50, 70], [70, 70], [90, 70], [310, 70], [30, 50], [30, 90]],
  },
  {
    id: "bg-repeat-y",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:200px;background:#00ff00 url(${SPRITE}) repeat-y right top/40px 16px"></div>`),
    assets: [SPRITE],
    probes: [[190, 28], [210, 28], [190, 44], [210, 200], [100, 28], [170, 28]],
  },
  {
    id: "bg-contain",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:100px;background:#00ff00 url(${SPRITE}) no-repeat;background-size:contain"></div>`),
    assets: [SPRITE],
    probes: [[60, 60], [160, 60], [100, 110], [100, 98]],
  },
  {
    id: "bg-cover-center",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:100px;background:#00ff00 url(${SPRITE}) no-repeat center/cover"></div>`),
    assets: [SPRITE],
    probes: [[25, 70], [110, 70], [130, 70], [215, 70], [120, 22], [120, 118]],
  },
  {
    id: "bg-position-keywords",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:100px;background:#00ff00 url(${SPRITE}) no-repeat right 25% bottom 10px/40px 20px"></div>`),
    assets: [SPRITE],
    probes: [[145, 100], [165, 100], [175, 100], [125, 100], [155, 85], [155, 112]],
  },
  {
    // A transparent border and padding: the image's origin is the content box, the
    // painting area (colour included) the padding box.
    id: "bg-origin-clip",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:60px;padding:10px;border:10px solid transparent;background:#00ff00 url(${SPRITE}) no-repeat content-box padding-box;background-size:20px 20px"></div>`),
    assets: [SPRITE],
    probes: [[25, 50], [35, 50], [45, 45], [55, 45], [65, 45], [75, 75], [145, 50]],
  },
  {
    id: "bg-gradient-tiles",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:60px;background:#00ff00 linear-gradient(to right,#ff0000,#0000ff) 0 0/40px 30px"></div>`),
    assets: [],
    probes: [[22, 30], [58, 30], [62, 30], [98, 30], [22, 65], [58, 65]],
  },
  // miaskiewicz/turbo-dom#264: gradient colour stops at lengths (px, em), hard stops
  // (two stops at one position), double-position stops, and repeating gradients.
  {
    id: "gradient-length-stops",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:40px;font-size:10px;background:linear-gradient(90deg,#ff0000 20px,#0000ff 20px,#0000ff 5em,#00ff00 5em)"></div>`),
    assets: [],
    probes: [[21, 30], [38, 30], [41, 30], [68, 30], [71, 30], [218, 30]],
  },
  {
    id: "gradient-hard-stops",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:100px;background:linear-gradient(#ff0000 30%,#00ff00 30% 60%,#0000ff 60%)"></div>`),
    assets: [],
    probes: [[50, 21], [50, 48], [50, 51], [50, 78], [50, 81], [50, 118]],
  },
  {
    id: "gradient-repeating",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:40px;background:repeating-linear-gradient(90deg,#ff0000 0 10px,#0000ff 10px 20px)"></div>`),
    assets: [],
    probes: [[21, 30], [28, 30], [31, 30], [38, 30], [121, 30], [131, 30], [218, 30]],
  },
  {
    // The review's grid: a 1px #ccc line every 4px, both axes (two layers).
    id: "gradient-grid-tiles",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:100px;background:#00ff00 linear-gradient(#cccccc 1px,transparent 1px) 0 0/4px 4px"></div>`),
    assets: [],
    probes: [[30, 20], [30, 21], [30, 23], [30, 24], [218, 116], [218, 117], [218, 119]],
  },
  {
    // `space`: 5 whole 40px tiles over 220px, 5px apart, the first at the edge.
    id: "bg-space",
    html: DOC(`<div style="${at(20, 20)};width:220px;height:60px;background:#00ff00 url(${SPRITE}) space;background-size:40px 20px"></div>`),
    assets: [SPRITE],
    probes: [[25, 30], [55, 30], [62, 30], [67, 30], [97, 30], [102, 30], [205, 30], [235, 30], [237, 30], [25, 70]],
  },
  {
    // `round`: 230 / 40 rounds to 6 tiles of 38.33px (20 / 20 rows stay 20px).
    id: "bg-round",
    html: DOC(`<div style="${at(20, 20)};width:230px;height:60px;background:#00ff00 url(${SPRITE}) round;background-size:40px 20px"></div>`),
    assets: [SPRITE],
    probes: [[25, 30], [56, 30], [60, 30], [76, 30], [96, 30], [98, 30], [138, 30], [244, 30], [248, 30], [25, 75]],
  },
  {
    // A rounded box: the layer is clipped to the border box's outer radius, and the
    // (opaque) border paints over it.
    id: "bg-radius-border-box",
    html: DOC(`<div style="${at(20, 20)};width:80px;height:80px;border:10px solid #ffff00;border-radius:30px;background:#00ff00 url(${SPRITE}) center/40px 40px"></div>`),
    assets: [SPRITE],
    probes: [[21, 21], [24, 24], [31, 31], [25, 70], [70, 70], [80, 70], [118, 118]],
  },
  {
    // `background-clip: padding-box` clips to the inner radius (30 - 10px border).
    id: "bg-radius-padding-box",
    html: DOC(`<div style="${at(20, 20)};width:80px;height:80px;border:10px solid transparent;border-radius:30px;background:#00ff00 url(${SPRITE}) padding-box;background-size:40px 40px"></div>`),
    assets: [SPRITE],
    probes: [[25, 70], [31, 31], [33, 33], [40, 40], [70, 40], [109, 109], [106, 106]],
  },
  {
    // 75 x 75 = 5625 tiles: every one paints (the far corner too).
    id: "bg-many-tiles",
    html: DOC(`<div style="${at(20, 20)};width:600px;height:300px;background:#00ff00 url(${SPRITE});background-size:8px 4px"></div>`),
    assets: [SPRITE],
    probes: [[21, 21], [25, 21], [613, 317], [617, 317], [21, 317], [617, 21]],
  },
  {
    // 0.7.0 round-2 review, finding 1: 640 x 400 = 256000 two-pixel gradient tiles (red
    // top row) fill the whole viewport-sized box, down to its bottom-right tile.
    id: "bg-tiles-fill-a-big-box",
    html: DOC(`<div style="${at(0, 0)};width:1280px;height:800px;background:#00ff00 linear-gradient(#cccccc 1px,transparent 1px) 0 0/2px 2px"></div>`),
    assets: [],
    probes: [[1, 0], [1, 1], [640, 400], [640, 401], [5, 600], [5, 601], [1279, 796], [1279, 798], [1279, 799]],
  },
  {
    // 0.7.0 round-3 review, finding 3: a scaled box's gradient tiles are drawn at device
    // resolution, smooth (a 4px red-to-blue tile under scale(2): eight shades per 8px
    // period, not four 2px bands).
    id: "bg-tiles-scaled-2x",
    html: DOC(`<div style="${at(0, 0)};width:16px;height:8px;transform:scale(2);transform-origin:0 0;background:linear-gradient(to right,#ff0000,#0000ff) 0 0/4px 4px"></div>`),
    assets: [],
    probes: [[0, 4], [1, 4], [2, 4], [3, 4], [4, 4], [5, 4], [6, 4], [7, 4], [24, 12], [31, 12]],
  },
  {
    // The same under scale(4): a 10px tile, 40 device px per period.
    id: "bg-tiles-scaled-4x",
    html: DOC(`<div style="${at(0, 100)};width:40px;height:20px;transform:scale(4);transform-origin:0 0;background:linear-gradient(to right,#ff0000,#0000ff) 0 0/10px 10px"></div>`),
    assets: [],
    probes: [[0, 110], [4, 110], [8, 110], [12, 110], [16, 110], [20, 110], [24, 110], [28, 110], [32, 110], [36, 110], [156, 170]],
  },
  {
    // A rotated box's tiles: the hard red/blue stop inside each 20px tile is
    // anti-aliased along the rotated edge, not stair-stepped.
    id: "bg-tiles-rotated",
    html: DOC(`<div style="${at(300, 300)};width:80px;height:80px;transform:rotate(30deg);background:#00ff00 linear-gradient(to right,#ff0000 50%,#0000ff 50%) 0 0/20px 20px"></div>`),
    assets: [],
    probes: [
      [332, 292], [332, 293], [331, 294], [330, 296], [341, 297], [329, 298],
      [340, 298], [328, 299], [340, 299], [328, 300], [339, 300], [327, 301], [339, 301],
      [340, 340], [325, 320],
    ],
  },
  {
    // miaskiewicz/turbo-dom#269: an `<img>` in a scale(4) box is sampled at device
    // resolution: its red/blue edge (sprite x=50) blends over a few device px. (The
    // transform is on a wrapper: a transform on the `<img>` itself is not applied yet.)
    id: "img-scaled-4x",
    html: DOC(`<div style="${at(0, 0)};transform:scale(4);transform-origin:0 0"><img src="${SPRITE}" style="display:block;width:100px;height:40px"></div>`),
    assets: [SPRITE],
    probes: [[190, 80], [196, 80], [197, 80], [198, 80], [199, 80], [200, 80], [201, 80], [202, 80], [203, 80], [210, 80]],
  },
  {
    // miaskiewicz/turbo-dom#272: a transform on the `<img>` itself scales it (paint and
    // getBoundingClientRect), here about its default centre origin: the 100x40 box
    // becomes 400x160 at (50,140). The edge blend is img-scaled-4x's.
    id: "img-transformed",
    html: DOC(`<img data-cid="i" src="${SPRITE}" style="${at(200, 200)};width:100px;height:40px;transform:scale(4)">`),
    assets: [SPRITE],
    probes: [[60, 160], [100, 220], [240, 220], [248, 220], [252, 220], [400, 220], [449, 299], [451, 220], [100, 301]],
  },
  {
    // The same for image tiles: 50x20 sprite tiles under scale(4), edges at device x=100
    // (red to blue inside a tile) and x=200 (blue to the next tile's red).
    id: "bg-image-tiles-scaled-4x",
    html: DOC(`<div style="${at(0, 300)};width:100px;height:20px;transform:scale(4);transform-origin:0 0;background:url(${SPRITE}) 0 0/50px 20px"></div>`),
    assets: [SPRITE],
    probes: [[97, 340], [98, 340], [99, 340], [100, 340], [101, 340], [102, 340], [197, 340], [198, 340], [199, 340], [200, 340], [201, 340], [202, 340]],
  },
  {
    // miaskiewicz/turbo-dom#270: 2.5px tiles (red 40%, blue after) are not snapped to
    // whole pixels: Chrome draws the tile once and repeats it every 2.5px, blending the
    // pixels the tile edges cross.
    id: "bg-tiles-fractional-blend",
    html: DOC(`<div style="${at(0, 0)};width:200px;height:200px;background:#00ff00 linear-gradient(to right,#ff0000 40%,#0000ff 40%) 0 0/2.5px 2.5px"></div>`),
    assets: [],
    probes: [[0, 0], [1, 0], [2, 0], [3, 0], [4, 0], [5, 0], [7, 0], [8, 0], [102, 50], [103, 50], [197, 199], [198, 199]],
  },
  {
    // The same with 1.5px tiles over a small box (fewer than 4096 tiles).
    id: "bg-tiles-fractional-blend-few",
    html: DOC(`<div style="${at(0, 0)};width:30px;height:30px;background:#00ff00 linear-gradient(to right,#ff0000 50%,#0000ff 50%) 0 0/1.5px 1.5px"></div>`),
    assets: [],
    probes: [[0, 5], [1, 5], [2, 5], [3, 5], [4, 5], [5, 5], [28, 29], [29, 29]],
  },
  {
    // 0.7.0 round-3 review, finding 4: about 7.3 million 1.5px tiles (no whole-pixel
    // period) shown under scale(0.25) cover the whole viewport, down to its bottom-right
    // corner.
    id: "bg-tiles-shrunk-fractional",
    html: DOC(`<div style="${at(0, 0)};width:5120px;height:3200px;transform:scale(0.25);transform-origin:0 0;background:#00ff00 linear-gradient(#ff0000,#ff0000) 0 0/1.5px 1.5px"></div>`),
    assets: [],
    probes: [[0, 0], [640, 400], [1279, 0], [0, 799], [640, 700], [1279, 700], [1279, 799]],
  },
  {
    // An inline `content-box` origin: the first fragment's content starts after its
    // left padding, the last ends before its right padding; the 20px image sits at the
    // strip's content end.
    id: "inline-bg-content-box",
    html: DOC(`<div style="${at(20, 20)};width:100px;font-size:0"><span style="background:#00ff00 url(${SPRITE}) no-repeat content-box right center/20px 10px;background-clip:border-box;padding:5px 10px">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    assets: [SPRITE],
    probes: [[85, 80], [95, 80], [105, 80], [75, 80], [25, 50]],
  },
  {
    // An inline box: the positioning area is its fragments' strip (160px over two
    // lines); a right-aligned 40px no-repeat image lands at the end of the last one.
    id: "inline-bg-position",
    html: DOC(`<div style="${at(20, 20)};width:80px;font-size:0"><span style="background:#00ff00 url(${SPRITE}) no-repeat right 0/40px 100%;padding:5px 0">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    assets: [SPRITE],
    probes: [[25, 52], [95, 52], [25, 82], [65, 82], [95, 82]],
  },
  // An inline box's background image / gradient (stretched over its padding box here):
  // sliced, the fragments show consecutive pieces of one image laid over their strip;
  // cloned, each fragment shows the whole image.
  {
    id: "inline-image-slice",
    html: DOC(`<div style="${at(20, 20)};width:80px;font-size:0"><span style="background:url(${SPRITE}) 0 0/100% 100% no-repeat;padding:5px 0">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    assets: [SPRITE],
    probes: [[25, 52], [55, 52], [95, 52], [25, 82], [55, 82], [95, 82]],
  },
  {
    id: "inline-image-clone",
    html: DOC(`<div style="${at(20, 20)};width:80px;font-size:0"><span style="background:url(${SPRITE}) 0 0/100% 100% no-repeat;padding:5px 0;box-decoration-break:clone;-webkit-box-decoration-break:clone">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    assets: [SPRITE],
    probes: [[25, 52], [95, 52], [25, 82], [95, 82]],
  },
  {
    id: "inline-gradient-slice",
    html: DOC(`<div style="${at(20, 20)};width:80px;font-size:0"><span style="background:linear-gradient(to right,#ff0000,#0000ff);padding:5px 0">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    probes: [[22, 52], [98, 52], [22, 82], [98, 82]],
  },
  {
    id: "inline-gradient-clone",
    html: DOC(`<div style="${at(20, 20)};width:80px;font-size:0"><span style="background:linear-gradient(to right,#ff0000,#0000ff);padding:5px 0;box-decoration-break:clone;-webkit-box-decoration-break:clone">${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}${ATOM(40, 30)}</span></div>`),
    probes: [[22, 52], [98, 52], [22, 82], [98, 82]],
  },
  // Round-1 review of 0.7.0, finding 18: a cloned inline box's radius, background image and
  // inset shadow repeat on every fragment (two lines of two 40x60 atoms; with 12px 6px
  // padding around the zero-height content area each fragment is 92x24 at x 20..112, y
  // 68..92 and 128..152).
  {
    id: "inline-clone-radius",
    html: DOC(`<div style="${at(20, 20)};width:100px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:12px 6px;border-radius:10px;box-decoration-break:clone;-webkit-box-decoration-break:clone">${ATOM(40, 60)}${ATOM(40, 60)}${ATOM(40, 60)}${ATOM(40, 60)}</span></div>`),
    probes: [[21, 69], [111, 69], [21, 91], [111, 91], [21, 129], [111, 129], [21, 151], [111, 151], [60, 80], [60, 140], [22, 80], [110, 140]],
  },
  {
    id: "inline-clone-image-radius",
    html: DOC(`<div style="${at(20, 20)};width:100px;font-size:0"><span style="background:url(${SPRITE}) 0 0/100% 100% no-repeat;padding:12px 6px;border-radius:10px;box-decoration-break:clone;-webkit-box-decoration-break:clone">${ATOM(40, 60)}${ATOM(40, 60)}${ATOM(40, 60)}${ATOM(40, 60)}</span></div>`),
    assets: [SPRITE],
    probes: [[30, 80], [100, 80], [30, 140], [100, 140], [21, 69], [111, 91], [21, 129], [111, 151]],
  },
  {
    id: "inline-clone-inset-shadow",
    html: DOC(`<div style="${at(20, 20)};width:100px;font-size:0"><span style="background:#ffff00;padding:12px 6px;box-shadow:inset 0 0 0 5px #0000ff;box-decoration-break:clone;-webkit-box-decoration-break:clone">${ATOM(40, 60)}${ATOM(40, 60)}${ATOM(40, 60)}${ATOM(40, 60)}</span></div>`),
    probes: [[22, 80], [110, 80], [60, 70], [60, 90], [60, 130], [60, 150], [22, 140], [110, 140], [60, 80], [60, 140], [30, 80], [100, 140]],
  },
  // Round-1 review of 0.7.0, finding 7: `overflow: hidden` clips glyph ink past the line
  // box (a 40px "gy" in a 10px-high line); unclipped, the same ink reaches below the box.
  // (23,42) and (24,44) are solid ink of the descender's left part in both engines; its
  // edge pixels (22,44) and (28,44) stay, toleranced in paint_chrome.rs.
  ...["hidden", "visible"].map((overflow) => ({
    id: `overflow-glyph-ink-${overflow}`,
    html: DOC(`<div data-cid="m" style="${at(20, 20)};width:100px;height:10px;line-height:10px;font-size:40px;overflow:${overflow};font-family:'Liberation Serif';color:#0000ff">gy</div>`),
    probes: [[22, 12], [28, 12], [34, 12], [40, 12], [46, 12], [52, 12], [22, 25], [28, 25], [34, 25], [40, 25], [46, 25], [52, 25], [22, 36], [28, 36], [34, 36], [40, 36], [46, 36], [52, 36], [23, 42], [22, 44], [24, 44], [28, 44], [34, 44], [40, 44], [46, 44], [52, 44]],
  })),
  // miaskiewicz/turbo-dom#253: Chrome 156's full border-radius model. Per-corner and
  // elliptical radii, percentages against the border box's own axes, the overlap scale-down,
  // inner (padding / content edge) radii, logical corners under RTL and vertical writing
  // modes, and the radius clipping backgrounds, borders, shadows and overflow.
  {
    // Shorthand order TL TR BR BL: a square TL, a small TR, larger BR and BL.
    id: "radius-per-corner",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:100px;background:#ff0000;border-radius:0 10px 30px 50px"></div>`),
    probes: [[21, 21], [118, 21], [115, 115], [100, 100], [30, 110], [40, 100], [70, 70], [112, 28]],
  },
  {
    // Longhands, one of them elliptical (h 40px, v 10px).
    id: "radius-longhands",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:60px;background:#00ff00;border-top-left-radius:40px;border-bottom-right-radius:40px 10px"></div>`),
    probes: [[25, 25], [45, 30], [119, 21], [21, 79], [115, 77], [100, 76], [110, 74], [70, 50]],
  },
  {
    // `h / v` in the shorthand.
    id: "radius-elliptical",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:100px;background:#0000ff;border-radius:50px / 20px"></div><div style="${at(300, 20)};width:200px;height:100px;background:#ff0000;border-radius:10px 60px / 40px 10px"></div>`),
    probes: [[25, 25], [30, 45], [40, 28], [22, 30], [215, 115], [120, 70], [302, 24], [325, 25], [496, 22], [470, 22], [302, 116], [498, 117], [400, 70]],
  },
  {
    // `50%` on a 200x100 box is a 100x50 ellipse at each corner (an ellipse, not a pill).
    id: "radius-percent",
    html: DOC(`<div style="${at(20, 20)};width:200px;height:100px;background:#ff0000;border-radius:50%"></div><div style="${at(300, 20)};width:100px;height:100px;background:#00ff00;border-radius:20% 0 0 0 / 40% 0 0 0"></div>`),
    probes: [[70, 25], [120, 23], [120, 70], [30, 70], [45, 40], [302, 22], [302, 60], [319, 30], [330, 60], [398, 22]],
  },
  {
    // The overlap rule: TL 150 + TR 50 on a 100px top side scale every radius by 0.5.
    id: "radius-overlap",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:200px;background:#ff0000;border-top-left-radius:150px;border-top-right-radius:50px"></div><div style="${at(200, 20)};width:100px;height:100px;background:#0000ff;border-radius:100px 100px 100px 100px / 50px 150px 50px 150px"></div>`),
    probes: [[23, 65], [112, 28], [117, 25], [70, 120], [30, 30], [210, 22], [298, 22], [250, 70], [202, 70]],
  },
  {
    // Different border widths and colours per side at a rounded corner: the inner (padding
    // edge) radius is the outer one less each side's width (here 10px wide, 26px tall).
    id: "radius-border-sides",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:100px;background:#00ff00;border-style:solid;border-width:4px 20px 4px 20px;border-color:#ff0000 #0000ff #ff0000 #0000ff;border-radius:30px"></div>`),
    probes: [[90, 22], [30, 74], [150, 74], [90, 126], [90, 74], [21, 21], [42, 30], [60, 22], [45, 26], [41, 40], [158, 118]],
  },
  {
    // A uniform rounded border, and a border wider than the radius (square inner corner).
    id: "radius-border-uniform",
    html: DOC(`<div style="${at(20, 20)};width:60px;height:60px;background:#00ff00;border:10px solid #0000ff;border-radius:40px"></div><div style="${at(200, 20)};width:60px;height:60px;background:#00ff00;border:20px solid #0000ff;border-radius:15px"></div>`),
    probes: [[22, 22], [30, 30], [60, 25], [60, 60], [42, 42], [36, 36], [201, 21], [203, 38], [202, 22], [221, 41], [241, 41], [290, 22]],
  },
  {
    // Logical corners under `direction: rtl`: start-start is the top-right corner, end-end
    // the bottom-left; a physical longhand stays put.
    id: "radius-rtl",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:60px;background:#ff0000;direction:rtl;border-start-start-radius:40px;border-end-end-radius:40px"></div><div style="${at(200, 20)};width:100px;height:60px;background:#ff0000;direction:rtl;border-top-left-radius:40px"></div>`),
    probes: [[22, 22], [118, 22], [22, 78], [118, 78], [70, 50], [202, 22], [298, 22], [202, 78]],
  },
  {
    // Logical corners in `vertical-rl` (block-start = right, inline-start = top) and
    // `vertical-lr`; a physical elliptical longhand keeps its corner and axes; the
    // block-start border is the right side in vertical-rl.
    id: "radius-vertical",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:60px;background:#00ff00;writing-mode:vertical-rl;border-start-start-radius:40px;border-start-end-radius:20px;border-top-left-radius:30px 10px;border-right:10px solid #0000ff"></div><div style="${at(200, 20)};width:100px;height:60px;background:#00ff00;writing-mode:vertical-lr;border-start-end-radius:40px;border-end-start-radius:40px"></div>`),
    probes: [[128, 22], [128, 78], [25, 22], [25, 29], [124, 50], [60, 50], [22, 78], [202, 22], [202, 78], [298, 22], [298, 78], [298, 50]],
  },
  {
    // `overflow: hidden` clips descendants to the rounded padding box; square too.
    id: "radius-overflow",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:100px;overflow:hidden;border-radius:50px">${BLOCK(100, 100, "#ff0000")}</div><div style="${at(200, 20)};width:50px;height:50px;overflow:hidden">${BLOCK(100, 100, "#00ff00")}</div><div style="${at(400, 20)};width:80px;height:80px;overflow:hidden;border:10px solid #0000ff;border-radius:40px">${BLOCK(200, 200, "#ff0000")}</div>`),
    probes: [[25, 25], [70, 70], [115, 115], [70, 22], [240, 40], [260, 40], [240, 90], [440, 60], [402, 60], [413, 60], [418, 33], [470, 112], [460, 170]],
  },
  {
    // Background layers (miaskiewicz/turbo-dom#248) clip to the radius: gradient tiles over
    // the border box and a colour clipped to the content box with its inner radius.
    id: "radius-background-layers",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:100px;border-radius:50px;background:linear-gradient(#ff0000,#ff0000) 0 0 / 50px 50px repeat"></div><div style="${at(200, 20)};width:80px;height:80px;padding:10px;border-radius:40px;background:#0000ff;background-clip:content-box"></div>`),
    probes: [[25, 25], [70, 70], [45, 25], [115, 115], [70, 22], [212, 32], [221, 41], [205, 60], [250, 25], [250, 60]],
  },
  {
    // Shadows follow each corner's radius: a spread shadow grows a rounded corner by the
    // spread and keeps a square one square; an inset shadow follows an elliptical padding
    // box.
    id: "radius-shadows",
    html: DOC(`<div style="${at(100, 100)};width:60px;height:60px;background:#00ff00;border-radius:30px 0 0 0;box-shadow:0 0 0 10px #ff0000"></div><div style="${at(300, 100)};width:100px;height:60px;background:#00ff00;border-radius:50px / 30px;box-shadow:inset 0 0 0 8px #ff0000"></div>`),
    probes: [[92, 92], [167, 92], [95, 130], [167, 167], [130, 95], [130, 130], [302, 102], [304, 130], [350, 104], [350, 130], [312, 112]],
  },
  {
    // A background image clipped to the padding box of a box with per-corner elliptical
    // radii: each corner's inner radii (outer less the adjacent border widths) clip it.
    id: "radius-background-image-corners",
    html: DOC(`<div style="${at(20, 20)};width:100px;height:60px;border:10px solid transparent;border-radius:40px 0 10px 0 / 20px 0 10px 0;background:url(${SPRITE}) 0 0 / 100% 100% no-repeat;background-clip:padding-box"></div>`),
    assets: [SPRITE],
    probes: [[35, 31], [45, 38], [128, 88], [128, 32], [22, 50], [31, 88], [70, 60], [100, 60]],
  },
  // miaskiewicz/turbo-dom#252 (`media-*`): <canvas>, <video>, <audio controls> and <object>
  // are replaced boxes that paint their own background and border (a 4px blue border, a
  // 6px green padding and background, in horizontal and vertical-rl writing modes); a
  // <video> paints its poster image. Chrome's 2D canvas content is the embedder's
  // (`canvas_ops_png`), so an empty canvas shows its background.
  ...["canvas width=40 height=20", "video width=40 height=20", "audio controls", "object width=40 height=20"].flatMap((element) => {
    const tag = element.split(" ")[0];
    // Chrome paints the audio controls' panel over the content box, which a screenshot
    // here does not: only the frame is probed there.
    const probes = tag === "audio" ? [[22, 22], [28, 28], [26, 40], [21, 60]] : [[22, 22], [28, 28], [26, 40], [34, 34], [21, 60], [90, 30]];
    const media = `<${element} data-cid="m" style="background:#00ff00;border:4px solid #0000ff;padding:6px"></${tag}>`;
    return [
      { id: `media-${tag}-frame`, html: DOC(`<div data-cid="p" style="${at(20, 20)};font-size:0">${media}</div>`), probes },
      { id: `media-${tag}-frame-rl`, html: DOC(`<div data-cid="p" style="${at(20, 20)};font-size:0;writing-mode:vertical-rl">${media}</div>`), probes },
    ];
  }),
  // miaskiewicz/turbo-dom#239 (`form-*`): form controls paint Chrome's field / button
  // backgrounds inside their frames at Chrome's sizes (fixture font).
  ...[
    ["input", `<input data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:30px;height:16px">`],
    ["textarea", `<textarea data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:30px;height:16px"></textarea>`],
    ["button", `<button data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:42px;height:22px"></button>`],
    ["select", `<select data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:42px;height:22px"></select>`],
  ].map(([tag, html]) => ({
    id: `form-${tag}`,
    html: DOC(html),
    // Chrome paints the native frames (rounded, thinner than their 2px border box):
    // only the field inside and the page around them are probed.
    probes: [[19, 30], [24, 30], [40, 30], [50, 36], [70, 30], [40, 50]],
  })),
  // miaskiewicz/turbo-dom#255 (`form-native-*`): a control with its default appearance
  // paints Chrome's native look (a 1px #767676 frame with rounded corners on white or
  // #efefef, a menu list's chevron, the checkbox / radio glyphs, the range track and
  // thumb, the colour swatch); `appearance: none` or an author border keeps the CSS
  // painting. Probes are box-relative offsets from the control's (20, 20) corner.
  ...[
    ["text", `<input data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:30px;height:16px">`, [[0, 0], [15, 0], [15, 1], [15, 2], [0, 11], [1, 11], [2, 11], [37, 11], [36, 11], [15, 21], [15, 20]]],
    ["textarea", `<textarea data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:30px;height:16px"></textarea>`, [[0, 0], [15, 0], [15, 1], [0, 10], [1, 10], [35, 10], [34, 10], [15, 21], [15, 20]]],
    ["button", `<button data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:42px;height:22px"></button>`, [[0, 0], [20, 0], [20, 1], [0, 11], [1, 11], [41, 11], [40, 11], [20, 21], [20, 20]]],
    ["select", `<select data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:42px;height:22px"></select>`, [[0, 0], [20, 0], [20, 1], [0, 11], [1, 11], [20, 21], [20, 20], [35, 12], [31, 9], [38, 9], [35, 8], [28, 11]]],
    ["checkbox", `<input data-cid="m" type="checkbox" style="${at(20, 20)};margin:0">`, [[0, 0], [6, 0], [6, 1], [0, 6], [1, 6], [6, 6], [12, 6], [11, 6], [6, 12], [6, 11]]],
    ["checkbox-checked", `<input data-cid="m" type="checkbox" checked style="${at(20, 20)};margin:0">`, [[0, 0], [6, 0], [6, 1], [2, 2], [10, 10], [5, 8], [8, 5], [2, 9], [10, 3], [6, 12]]],
    ["radio", `<input data-cid="m" type="radio" style="${at(20, 20)};margin:0">`, [[0, 0], [1, 1], [6, 0], [6, 1], [6, 6], [0, 6], [1, 6], [12, 6], [11, 6], [6, 12], [6, 11]]],
    ["radio-checked", `<input data-cid="m" type="radio" checked style="${at(20, 20)};margin:0">`, [[0, 0], [1, 1], [6, 0], [6, 6], [4, 6], [6, 4], [1, 6], [6, 1], [11, 6], [6, 11]]],
    ["range", `<input data-cid="m" type="range" style="${at(20, 20)};margin:0;width:60px">`, [[0, 0], [5, 1], [5, 8], [20, 8], [30, 8], [30, 1], [30, 15], [45, 8], [55, 8], [45, 2], [45, 14], [58, 8]]],
    ["range-value", `<input data-cid="m" type="range" min="10" max="20" value="18" style="${at(20, 20)};margin:0;width:60px">`, [[5, 8], [20, 8], [30, 8], [46, 8], [46, 1], [56, 8], [58, 8]]],
    // A max below the min is the min (the thumb sits at the start); a value, min or max
    // that is not a finite valid floating-point number is ignored (round 1 review #1).
    ["range-max-below-min", `<input data-cid="m" type="range" min="10" max="5" value="500" style="${at(20, 20)};margin:0;width:60px">`, [[3, 8], [7, 1], [7, 8], [7, 14], [20, 8], [30, 1], [45, 8], [58, 8]]],
    ["range-nan", `<input data-cid="m" type="range" min="NaN" max="Infinity" value="NaN" style="${at(20, 20)};margin:0;width:60px">`, [[5, 8], [20, 8], [30, 8], [30, 1], [30, 15], [45, 8], [58, 8]]],
    ["range-huge", `<input data-cid="m" type="range" min="-1e308" max="1e308" value="1e400" style="${at(20, 20)};margin:0;width:60px">`, [[5, 8], [20, 8], [30, 8], [30, 1], [30, 15], [45, 8], [58, 8]]],
    // The value snaps to min + n * step (default 1, the nearest; `step=any` does not snap),
    // and the max is lowered to the last step value (round 2 review #9).
    ["range-step-default", `<input data-cid="m" type="range" min="0" max="10" value="4.6" style="${at(20, 20)};margin:0;width:60px">`, [[5, 8], [20, 8], [26, 8], [30, 8], [30, 1], [30, 15], [34, 8], [45, 8], [58, 8]]],
    ["range-step", `<input data-cid="m" type="range" min="0" max="10" step="5" value="7" style="${at(20, 20)};margin:0;width:60px">`, [[5, 8], [20, 8], [30, 8], [30, 1], [30, 15], [38, 1], [45, 8], [58, 8]]],
    ["range-step-any", `<input data-cid="m" type="range" min="0" max="10" step="any" value="4.6" style="${at(20, 20)};margin:0;width:60px">`, [[5, 8], [20, 8], [24, 1], [28, 8], [28, 1], [28, 15], [34, 1], [45, 8], [58, 8]]],
    ["range-step-max", `<input data-cid="m" type="range" min="0" max="10" step="3" value="8" style="${at(20, 20)};margin:0;width:60px">`, [[5, 8], [30, 8], [45, 8], [46, 1], [52, 8], [52, 1], [58, 8], [53, 14]]],
    // A value that rounds to a step above the max goes one step back from a non-zero base:
    // 5 + 2n within 5..10 is 9, 80% along (mutation audit 0.7.0).
    ["range-step-base", `<input data-cid="m" type="range" min="5" max="10" step="2" value="10" style="${at(20, 20)};margin:0;width:60px">`, [[5, 8], [20, 8], [30, 8], [46, 8], [46, 1], [56, 8], [58, 8]]],
    ["color", `<input data-cid="m" type="color" style="${at(20, 20)};margin:0">`, [[0, 0], [25, 0], [25, 1], [2, 13], [4, 13], [25, 13], [10, 10], [40, 18], [25, 4], [25, 23], [25, 26], [49, 13], [48, 13]]],
    ["color-value", `<input data-cid="m" type="color" value="#ff0000" style="${at(20, 20)};margin:0">`, [[25, 13], [10, 10], [40, 18], [4, 13], [25, 4]]],
    ["text-appearance-none", `<input data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:30px;height:16px;appearance:none">`, [[0, 0], [15, 0], [15, 1], [15, 2], [0, 11], [1, 11], [2, 11]]],
    ["text-author-border", `<input data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:30px;height:16px;border:2px solid #ff0000">`, [[0, 0], [15, 0], [15, 1], [15, 2], [0, 11], [1, 11], [2, 11]]],
    ["button-author-background", `<button data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:42px;height:22px;background-color:#ffff00;border-style:solid"></button>`, [[0, 0], [20, 0], [20, 1], [20, 2], [1, 11], [2, 11], [20, 11]]],
    ["checkbox-appearance-none", `<input data-cid="m" type="checkbox" checked style="${at(20, 20)};margin:0;appearance:none">`, [[0, 0], [6, 0], [6, 6], [5, 8]]],
  ].map(([kind, html, probes]) => ({
    id: `form-native-${kind}`,
    html: DOC(html),
    probes: probes.map(([dx, dy]) => [20 + dx, 20 + dy]),
  })),
  // miaskiewicz/turbo-dom#283 (`author-border-*`): any author border declaration on a
  // button (or an <input> button type) turns its native appearance off, so the CSS
  // border paints, even one equal to the UA's (`border-width:2px`); with and without an
  // author background. 42x22 border-box controls at (20, 20), no text; probes are
  // box-relative (corners, edges, the 1px / 2px / 3px rings, the face).
  ...(() => {
    const BORDERS = [
      ["shorthand", "border:3px solid #ff0000"],
      ["width", "border-width:2px"],
      ["width-4", "border-width:4px"],
      ["style", "border-style:solid"],
      ["color", "border-color:#0000ff"],
      ["none", "border:none"],
      ["top-color", "border-top-color:#0000ff"],
    ];
    const probes = [[0, 0], [20, 0], [20, 1], [20, 2], [20, 3], [0, 11], [1, 11], [2, 11], [3, 11], [20, 11], [41, 11], [40, 11], [39, 11], [20, 21], [20, 20], [20, 19], [41, 21]];
    const control = (tag, style) =>
      tag === "button"
        ? `<button data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:42px;height:22px;${style}"></button>`
        : `<input data-cid="m" type="${tag}" value="" style="${at(20, 20)};font-family:'Fira Code';width:42px;height:22px;${style}">`;
    const cases = [];
    for (const [kind, border] of BORDERS) {
      for (const [bgKind, bg] of [["", ""], ["-bg", ";background-color:#ffff00"]]) {
        cases.push([`button-${kind}${bgKind}`, control("button", border + bg)]);
      }
    }
    for (const tag of ["button", "submit", "reset"]) {
      for (const [kind, border] of [BORDERS[0], BORDERS[1], BORDERS[3]]) {
        cases.push([`input-${tag}-${kind}`, control(tag, border)]);
      }
    }
    return cases.map(([kind, html]) => ({
      id: `author-border-${kind}`,
      html: DOC(html),
      probes: probes.map(([dx, dy]) => [20 + dx, 20 + dy]),
    }));
  })(),
  // miaskiewicz/turbo-dom#256 (`cascade-bg-*`): the `background` shorthand sets every
  // background longhand at its own cascade position, so it and the longhands cascade by
  // origin, `!important`, specificity and order like any declarations: an author shorthand
  // beats the UA button's `background-color`, the later of a shorthand and a longhand wins,
  // and a shorthand resets the image / size / position / repeat / origin / clip longhands
  // an earlier rule set. Each case is one 100x40 box at (20, 20); probes are box-relative.
  ...[
    ["ua-button", "", `<button data-cid="m" style="${at(20, 20)};width:100px;height:40px;padding:0;border:0;background:#ffff00"></button>`, [[10, 10], [50, 20], [90, 30]]],
    ["shorthand-then-longhand", ".x{background:#ff0000}.x{background-color:#00ff00}", "", [[10, 10], [90, 30]]],
    ["longhand-then-shorthand", ".x{background-color:#00ff00}.x{background:#0000ff}", "", [[10, 10], [90, 30]]],
    ["important-shorthand", ".x{background:#0000ff!important}.x{background-color:#ff0000}", "", [[10, 10], [90, 30]]],
    ["important-longhand-over-inline", ".x{background-color:#00ff00!important}", "background:#ff0000", [[10, 10], [90, 30]]],
    ["inline-shorthand-over-rule", ".x{background-color:#ff0000}", "background:#00ff00", [[10, 10], [90, 30]]],
    ["inline-longhand-then-shorthand", "", "background-color:#ff0000;background:#0000ff", [[10, 10], [90, 30]]],
    ["inline-shorthand-then-longhand", "", "background:#ff0000;background-color:#00ff00", [[10, 10], [90, 30]]],
    ["specificity", "#k.x{background-color:#00ff00}div.x{background:#ff0000}", "", [[10, 10], [90, 30]]],
    ["shorthand-resets-position-repeat", `.x{background-position:50px 0;background-repeat:no-repeat}.x{background:url(${SPRITE}) #00ff00}`, "", [[10, 10], [25, 20], [45, 30], [55, 10], [75, 20], [95, 30]]],
    ["longhand-size-after-shorthand", `.x{background:url(${SPRITE}) no-repeat #00ff00}.x{background-size:50px 20px}`, "", [[10, 5], [40, 15], [10, 30], [75, 10], [75, 30]]],
    ["shorthand-resets-clip", ".x{padding:10px;width:80px!important;height:20px!important;background-clip:content-box}.x{background:#ff0000}", "", [[3, 3], [50, 3], [50, 20]]],
    ["longhand-clip-after-shorthand", ".x{background:#ff0000}.x{padding:10px;width:80px!important;height:20px!important;background-clip:content-box}", "", [[3, 3], [50, 3], [50, 20]]],
    ["longhand-origin-after-shorthand", `.x{background:url(${SPRITE}) no-repeat 0 0 / 50px 20px #00ff00}.x{padding:10px;width:80px!important;height:20px!important;background-origin:content-box}`, "", [[5, 5], [20, 20], [45, 20], [70, 32]]],
  ].map(([kind, css, inline, probes]) => ({
    id: `cascade-bg-${kind}`,
    assets: css.includes(SPRITE) ? [SPRITE] : [],
    html: DOC(
      kind === "ua-button"
        ? inline
        : `<style>${css}</style><div data-cid="m" id="k" class="x" style="${at(20, 20)};width:100px;height:40px;${inline}"></div>`,
    ),
    probes: probes.map(([dx, dy]) => [20 + dx, 20 + dy]),
  })),
  // Round 1 review #9 (`bg-zero-area-*`): a background image whose positioning area (the
  // padding box) has no height paints nothing, even though the border box it is clipped
  // to does; the background colour still fills the border box.
  ...[
    ["round", "round"],
    ["repeat", "repeat;background-size:20px 10px"],
    ["no-repeat", "no-repeat;background-size:20px 10px"],
  ].map(([kind, tail]) => ({
    id: `bg-zero-area-${kind}`,
    assets: [SPRITE],
    html: DOC(`<div data-cid="m" style="${at(20, 20)};width:100px;height:0;border:20px solid transparent;background:url(${SPRITE}) #00ff00;background-repeat:${tail}"></div>`),
    probes: [[25, 25], [45, 30], [70, 40], [95, 50], [130, 55], [135, 25]],
  })),
  {
    id: "media-video-poster",
    html: DOC(`<video data-cid="m" poster="${SPRITE}" style="${at(20, 20)};width:100px;height:40px"></video>`),
    assets: [SPRITE],
    probes: [[25, 40], [65, 40], [75, 40], [115, 40], [125, 40]],
  },
  // miaskiewicz/turbo-dom#257 (`border-inset-*` / `border-outset-*`): an `inset` border
  // darkens its top and left sides and lightens its bottom and right (`outset` the other
  // way round); a colour too dark to darken is lightened once for the dark sides and
  // twice for the light ones. Each case is a 40x20 box with a 4px border at (20, 20);
  // probes are box-relative: top, left, bottom, right.
  ...["inset", "outset"].flatMap((style) =>
    [
      ["grey", "#767676"],
      ["black", "#000000"],
      ["white", "#ffffff"],
      ["red", "#ff0000"],
      ["dark", "#102030"],
      ["mid", "#808080"],
    ].map(([kind, colour]) => ({
      id: `border-${style}-${kind}`,
      html: DOC(`<div data-cid="m" style="${at(20, 20)};width:40px;height:20px;border:4px ${style} ${colour}"></div>`),
      probes: [[20, 1], [1, 14], [20, 26], [46, 14]].map(([dx, dy]) => [20 + dx, 20 + dy]),
    })),
  ),
  // The shading names physical sides in vertical writing modes too.
  ...[
    ["inset", "vertical-rl"],
    ["outset", "vertical-lr"],
  ].map(([style, mode]) => ({
    id: `border-${style}-${mode}`,
    html: DOC(`<div data-cid="m" style="${at(20, 20)};writing-mode:${mode};width:20px;height:40px;border:4px ${style} #767676"></div>`),
    probes: [[14, 1], [1, 24], [14, 46], [26, 24]].map(([dx, dy]) => [20 + dx, 20 + dy]),
  })),
  // The `appearance: none` text field's UA `2px inset` border, bottom and right; a button
  // with an author background keeps the UA `2px outset` border in black.
  {
    id: "border-inset-field",
    html: DOC(`<input data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:30px;height:16px;appearance:none">`),
    probes: [[15, 21], [37, 11], [15, 20], [36, 11]].map(([dx, dy]) => [20 + dx, 20 + dy]),
  },
  {
    id: "border-outset-button",
    html: DOC(`<button data-cid="m" style="${at(20, 20)};font-family:'Fira Code';width:42px;height:22px;background-color:#ffff00"></button>`),
    probes: [[20, 0], [0, 11], [20, 21], [41, 11], [20, 11]].map(([dx, dy]) => [20 + dx, 20 + dy]),
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

const browser = await chromium.launch({ executablePath: chromePath, headless: true, args: ["--use-mock-keychain", "--password-store=basic"] });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const cases = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  await page.addStyleTag({ content: FACE });
  await page.evaluate(() => document.fonts.ready);
  const boxes = await page.evaluate(async () => {
    for (const img of document.images) await img.decode().catch(() => {});
    const r = (n) => Math.round(n * 1000) / 1000;
    return [...document.querySelectorAll("[data-cid]")].map((el) => {
      const b = el.getBoundingClientRect();
      return { cid: el.dataset.cid, x: r(b.x), y: r(b.y), width: r(b.width), height: r(b.height) };
    });
  });
  const px = decodePng(await page.screenshot());
  const probes = testCase.probes.map(([x, y]) => ({ x, y, class: classOf(px(x, y)), rgb: px(x, y) }));
  cases.push({ id: testCase.id, html: testCase.html, assets: testCase.assets || [], boxes, probes });
}
await browser.close();

const fixture = {
  generator: "scripts/paint-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases,
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) {
  console.log(c.id.padEnd(22), c.probes.map((p) => `${p.x},${p.y}:${p.class}`).join(" "));
  console.log("".padEnd(22), c.boxes.map((b) => `${b.cid}:${b.x},${b.y} ${b.width}x${b.height}`).join("  "));
}

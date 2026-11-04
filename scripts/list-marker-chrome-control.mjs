// Chrome-156 control for turbo-dom-layout's list markers (`::marker`, miaskiewicz/turbo-dom#128):
// list-item counters (`<ol start reversed>`, `<li value>`, items outside a list), the
// `list-style-type` counter styles and symbols, `list-style-position`, and where the marker
// box sits (beside the first line box, its baseline on the line's baseline).
//
// Every case is one page (viewport 800x600, device scale factor 1) in the repo's fixture
// Liberation Serif (crates/turbo-dom-layout/assets/fonts, loaded through @font-face), so the
// Rust comparisons lay out with `test_support::fixture_fonts()` on every OS. Per case Chrome
// records:
//   boxes    the getBoundingClientRect() of every `[data-cid]` element;
//   markers  each `::marker` box (CDP `DOM.getBoxModel` border quad, as [x, y, width, height])
//            with its list item's `data-cid`, in document order;
//   texts    for each `[data-text]` list item, the x of its first text character (a Range);
//   ink      per marker, the bounding box [x0, y0, x1, y1] (exclusive) of the screenshot pixels
//            within 1px of the marker box (not past its end, where the item's text starts)
//            that differ from the white page by more than 96 in a channel, and their count
//            (`null` when none): the painted marker.
// Asserted by crates/turbo-dom-layout/src/live/list_marker_chrome_tests.rs (geometry,
// through the live snapshot) and crates/turbo-dom-raster/tests/list_marker_chrome.rs (ink,
// PNG and SVG).
//
// Usage: node scripts/list-marker-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156 headless shell;
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/list_markers_chrome156.json
//            (and then also writes the raster copy, crates/turbo-dom-raster/tests/fixtures/).
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
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/list_markers_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const FONT = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf")).toString("base64");
const FACE = `@font-face{font-family:"Liberation Serif";src:url(data:font/ttf;base64,${FONT})}`;
const DOC = (body, css = "") =>
  `<!doctype html><style>body{margin:8px;font-family:"Liberation Serif";font-size:16px;color:#ff0000}${css}</style>${body}`;
const OL = (attrs, items) => `<ol style="margin:0"${attrs}>${items}</ol>`;
const LI = (cid, text = "item", attrs = "") => `<li data-cid="${cid}"${attrs}>${text}</li>`;
const TYPE = (type, start, n = 2) =>
  `<ol style="margin:0;list-style-type:${type}" start="${start}">${Array.from({ length: n }, (_, i) => LI(`i${i}`)).join("")}</ol>`;

const CASES = [
  // Counters.
  { id: "decimal", html: DOC(OL("", LI("a") + LI("b") + LI("c"))) },
  { id: "ol-start", html: DOC(OL(' start="9"', LI("a") + LI("b"))) },
  { id: "ol-reversed", html: DOC(OL(" reversed", LI("a") + LI("b") + LI("c"))) },
  { id: "ol-reversed-start", html: DOC(OL(' reversed start="10"', LI("a") + LI("b"))) },
  { id: "li-value", html: DOC(OL("", LI("a") + LI("b", "item", ' value="20"') + LI("c"))) },
  { id: "ol-negative-start", html: DOC(OL(' start="-2"', LI("a") + LI("b"))) },
  // `start` / `value` are signed 32-bit integers: an out-of-range one is absent.
  { id: "ol-start-out-of-range", html: DOC(OL(' start="3000000000"', LI("a") + LI("b")) + OL(' start="-2147483649"', LI("c")) + OL(' start="2147483647"', LI("d"))) },
  { id: "li-value-out-of-range", html: DOC(OL("", LI("a") + LI("b", "item", ' value="2147483648"') + LI("c", "item", ' value="-2147483648"') + LI("d", "item", ' value="-99999999999"'))) },
  // Counters are 32-bit and clamp (Chrome's ClampAdd): past i32::MAX / i32::MIN the
  // ordinal stays put. lower-alpha makes the last letter, and so the marker width, differ.
  { id: "ol-start-i32-max-clamps", html: DOC(`<ol style="margin:0;list-style-type:lower-alpha" start="2147483646">${LI("a")}${LI("b")}${LI("c")}</ol>`) },
  { id: "ol-reversed-i32-min-clamps", html: DOC(OL(' reversed start="-2147483647"', LI("a") + LI("b") + LI("c"))) },
  { id: "li-outside-list", html: DOC(`<div>${LI("a")}${LI("b")}</div>`) },
  { id: "nested-ol", html: DOC(OL("", `<li data-cid="a">x<ol>${LI("b")}${LI("c")}</ol></li>${LI("d")}`)) },
  { id: "ol-type-attribute", html: DOC(`<ol style="margin:0" type="a">${LI("a")}</ol><ol style="margin:0" type="I">${LI("b")}</ol>`) },
  // Counter styles.
  { id: "decimal-leading-zero", html: DOC(TYPE("decimal-leading-zero", 9)) },
  { id: "lower-alpha", html: DOC(TYPE("lower-alpha", 26)) },
  { id: "upper-alpha", html: DOC(TYPE("upper-alpha", 1)) },
  { id: "lower-latin", html: DOC(TYPE("lower-latin", 702)) },
  { id: "lower-roman", html: DOC(TYPE("lower-roman", 3, 3)) },
  { id: "upper-roman", html: DOC(TYPE("upper-roman", 1999)) },
  { id: "roman-out-of-range", html: DOC(TYPE("upper-roman", 3999)) },
  { id: "alpha-zero", html: DOC(TYPE("lower-alpha", 0)) },
  { id: "type-none", html: DOC(TYPE("none", 1)) },
  { id: "type-string", html: DOC(`<ul style="margin:0;list-style-type:'-> '">${LI("a")}</ul>`) },
  // Symbols.
  { id: "ul-disc", html: DOC(`<ul style="margin:0">${LI("a")}${LI("b")}</ul>`) },
  { id: "ul-nested", html: DOC(`<ul style="margin:0"><li data-cid="a">x<ul><li data-cid="b">y<ul>${LI("c")}</ul></li></ul></li></ul>`) },
  { id: "symbol-circle", html: DOC(`<ul style="margin:0;list-style-type:circle">${LI("a")}</ul>`) },
  { id: "symbol-square", html: DOC(`<ul style="margin:0;list-style-type:square">${LI("a")}</ul>`) },
  { id: "symbol-large", html: DOC(`<ul style="margin:0;font-size:30px">${LI("a")}</ul><ul style="margin:0;font-size:30px;list-style-type:circle">${LI("b")}</ul><ul style="margin:0;font-size:30px;list-style-type:square">${LI("c")}</ul>`) },
  { id: "symbol-small", html: DOC(`<ul style="margin:0;font-size:10px">${LI("a")}</ul>`) },
  // list-style-position: inside.
  { id: "inside-decimal", html: DOC(`<ol style="margin:0;list-style-position:inside">${LI("a", "item", " data-text")}${LI("b", "   item", " data-text")}</ol>`) },
  { id: "inside-disc", html: DOC(`<ul style="margin:0;list-style:inside">${LI("a", "item", " data-text")}${LI("b", "item", ' data-text style="font-size:24px"')}</ul>`) },
  { id: "inside-square", html: DOC(`<ul style="margin:0;list-style:square inside">${LI("a", "item", " data-text")}</ul>`) },
  { id: "inside-block-child", html: DOC(`<ol style="margin:0;list-style-position:inside"><li data-cid="a"><div data-cid="d">item</div></li></ol>`) },
  // Where the outside marker box goes.
  { id: "first-line-in-paragraph", html: DOC(OL("", `<li data-cid="a"><p data-cid="p">item</p></li>`)) },
  { id: "first-line-after-block", html: DOC(OL("", `<li data-cid="a"><div style="height:30px"></div>item</li>`)) },
  { id: "first-line-baseline", html: DOC(OL("", `<li data-cid="a"><span style="font-size:30px">item</span></li>`)) },
  { id: "marker-font-size", html: DOC(OL("", `<li data-cid="a" style="font-size:30px">item</li>`)) },
  { id: "empty-item", html: DOC(OL("", LI("a", "") + LI("b"))) },
  { id: "block-only-item", html: DOC(OL("", `<li data-cid="a"><div data-cid="d" style="width:10px;height:10px;background:#0000ff"></div></li>${LI("b")}`)) },
  { id: "item-padding-border", html: DOC(OL("", LI("a", "item", ' style="padding-left:10px;border-left:3px solid #0000ff"'))) },
  { id: "item-wraps", html: DOC(`<ol style="margin:0;width:100px">${LI("a", "one two three four five six")}</ol>`) },
  { id: "marker-color", html: DOC(`<ol style="margin:0;color:#0000ff">${LI("a")}</ol><ul style="margin:0;color:#0000ff">${LI("b")}</ul>`) },
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

// The ink bounding box of `png` within `rect` grown by 1px, except at its end (where the
// item's own text starts).
function ink(png, [x, y, w, h]) {
  const x0 = Math.max(0, Math.floor(x) - 1);
  const y0 = Math.max(0, Math.floor(y) - 1);
  const x1 = Math.min(png.width, Math.ceil(x + w));
  const y1 = Math.min(png.height, Math.ceil(y + h) + 1);
  let box = null;
  let count = 0;
  for (let py = y0; py < y1; py++) {
    for (let px = x0; px < x1; px++) {
      const i = (py * png.width + px) * 4;
      const dark = [0, 1, 2].some((k) => 255 - png.rgba[i + k] > 96);
      if (!dark) continue;
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
const cdp = await page.context().newCDPSession(page);
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  await page.addStyleTag({ content: FACE });
  await page.evaluate(() => document.fonts.ready);
  const { boxes, texts } = await page.evaluate(() => {
    const rect = (r) => [r.x, r.y, r.width, r.height];
    const boxes = [...document.querySelectorAll("[data-cid]")].map((el) => ({ cid: el.dataset.cid, rect: rect(el.getBoundingClientRect()) }));
    const texts = [...document.querySelectorAll("[data-text]")].map((el) => {
      const text = [...el.childNodes].find((node) => node.nodeType === 3);
      const range = document.createRange();
      range.setStart(text, text.data.search(/\S/));
      range.setEnd(text, text.data.search(/\S/) + 1);
      return { cid: el.dataset.cid, x: range.getBoundingClientRect().x };
    });
    return { boxes, texts };
  });
  const { root } = await cdp.send("DOM.getDocument", { depth: -1, pierce: true });
  const markers = [];
  const walk = async (node) => {
    for (const pseudo of node.pseudoElements || []) {
      if (pseudo.pseudoType !== "marker") continue;
      const { model } = await cdp.send("DOM.getBoxModel", { nodeId: pseudo.nodeId });
      const q = model.border;
      const at = node.attributes || [];
      const cid = at[at.indexOf("data-cid") + 1];
      markers.push({ cid, rect: [q[0], q[1], q[2] - q[0], q[5] - q[1]] });
    }
    for (const child of node.children || []) await walk(child);
  };
  await walk(root);
  const png = decodePng(await page.screenshot());
  for (const marker of markers) marker.ink = ink(png, marker.rect);
  result.push({ ...testCase, boxes, markers, ...(texts.length ? { texts } : {}) });
}
await browser.close();

const fixture = {
  generator: "scripts/list-marker-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [800, 600],
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
// The raster screenshot test reads the same control from its own crate.
if (!values.out) writeFileSync(join(repo, "crates/turbo-dom-raster/tests/fixtures/list_markers_chrome156.json"), `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${result.length} cases -> ${out}`);

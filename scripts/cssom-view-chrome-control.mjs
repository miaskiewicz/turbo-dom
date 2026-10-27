// Chrome-156 control for turbo-dom-layout's CSSOM-View scroll / visibility / caret / Range
// modules (`cssom_view::{scroll, visibility, text, range}`), the reads turbo-surf answers
// from `turbo_dom_layout::live` snapshots:
//   - scroll: `getComputedStyle().overflowX/Y` (the computed overflow pair,
//     `scroll::overflow`), `scrollWidth`/`scrollHeight` and the maximum scroll offsets
//     (`scroll::scroll_metrics`, `scroll::viewport_metrics` for `document.scrollingElement`),
//     `getBoundingClientRect()` after scrolling (`scroll::scroll_translation`), and the
//     scroll offsets `scrollIntoView(options)` / `scrollIntoViewIfNeeded(center)` move a
//     scroller to (`scroll::alignment` + `scroll::expose`);
//   - visibility: `checkVisibility({opacityProperty, visibilityProperty,
//     contentVisibilityAuto})` for all eight flag combinations (`visibility::check`,
//     `visibility::near_viewport`);
//   - text: `document.caretPositionFromPoint(x, y)` (`text::caret`);
//   - range: `Range.getClientRects()` (`range::select` decides which nodes contribute).
//
// Every case runs in Chrome for Testing 156 (viewport 1280x800, device scale 1) with the
// crate's bundled "Fira Code" (assets/fonts/fira-code, injected as an @font-face so text
// metrics agree with the native layout's bundled face). Scrollers hide their scrollbars
// (`scrollbar-width:none`): the native layout reserves no scrollbar gutter.
// A case's inputs:
//   scroll    {cid | "viewport": [x, y]} offsets applied before any read;
//   metrics   cids whose overflow / scroll extents are read;
//   viewport  true to read document.scrollingElement's extents;
//   visibility cids whose checkVisibility() is read (index = flags: 1 opacity,
//             2 visibility, 4 content-visibility:auto);
//   carets    viewport points; each records the caret's parent cid, the text node's
//             child index (null when the caret is in an element), the offset and the
//             caret rect;
//   ranges    {start, end}: [cid, child index or null, offset]; records getClientRects();
//   intoView  {target, scroller, mode, block, inline, from}: mode 0 =
//             scrollIntoView({block, inline}), 1/2 = scrollIntoViewIfNeeded(true/false);
//             records the scroller's offsets afterwards (and that the viewport did not move).
// Asserted by crates/turbo-dom-layout/src/cssom_view/chrome_tests.rs.
//
// Usage: node scripts/cssom-view-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/cssom_view_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { readFileSync, writeFileSync } from "node:fs";
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/cssom_view_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;
const fira = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/fira-code/FiraCode-Regular.ttf")).toString("base64");
const FACE = `@font-face{font-family:"Fira Code";src:url(data:font/ttf;base64,${fira})}`;

// A 150x80 RGB PNG (the native layout sizes it from the same bytes).
const PNG =
  "iVBORw0KGgoAAAANSUhEUgAAAJYAAABQCAIAAAB00hscAAAAOUlEQVR42u3BAQ0AAADCoPdPbQ8HFAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADwaozwAAH3iqS6AAAAAElFTkSuQmCC";
const BASE = 'body{margin:0;font-family:"Fira Code";font-size:10px;line-height:1}.s{scrollbar-width:none}';
const DOC = (css, body) => `<!doctype html><style>${BASE}${css}</style>${body}`;
const BOX = (w, h, cid = "", style = "") =>
  `<div${cid ? ` data-cid="${cid}"` : ""} style="width:${w}px;height:${h}px;${style}"></div>`;

const CASES = [
  // scroll::overflow: longhands win over the shorthand, a one-value shorthand sets both
  // axes, a two-value one is x then y, and visible/clip pair with a scrollable axis as
  // auto/hidden.
  {
    id: "overflow-computed-pairs",
    html: DOC(
      "",
      [
        ["a", "overflow:hidden"],
        ["b", "overflow:hidden visible"],
        ["c", "overflow:visible scroll"],
        ["d", "overflow:clip visible"],
        ["e", "overflow:auto;overflow-y:clip"],
        ["f", "overflow-x:scroll"],
        ["g", "overflow:clip"],
        ["h", ""],
        ["i", "overflow:visible clip"],
      ]
        .map(([cid, style]) => BOX(10, 10, cid, style))
        .join("") +
        `<div data-cid="j" class="s" style="overflow:auto;overflow-y:clip;width:10px;height:10px">${BOX(30, 30)}</div>` +
        `<div data-cid="k" class="s" style="overflow-x:clip;overflow-y:scroll;width:10px;height:10px">${BOX(30, 30)}</div>`,
    ),
    metrics: ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"],
  },
  // scroll::scroll_metrics: in-flow content adds its margin and the scroller's end
  // padding; absolute boxes add only their edges; a clipping child owns its own overflow.
  {
    id: "scroll-extent-flow-and-padding",
    html: DOC(
      "",
      `<div data-cid="s" class="s" style="overflow:auto;width:100px;height:50px;padding:5px 7px;border:2px solid">${BOX(150, 80, "c", "margin:3px 4px")}</div>`,
    ),
    metrics: ["s", "c"],
  },
  {
    id: "scroll-extent-hidden-and-visible",
    html: DOC(
      "",
      `<div data-cid="h" style="overflow:hidden;width:60px;height:30px;padding:4px">${BOX(100, 70, "", "margin-right:6px;margin-bottom:9px")}</div>` +
        `<div data-cid="v" style="width:60px;height:30px;padding:4px">${BOX(100, 70, "", "margin-right:6px;margin-bottom:9px")}</div>`,
    ),
    metrics: ["h", "v"],
  },
  {
    id: "scroll-extent-positioned-children",
    html: DOC(
      "",
      `<div data-cid="s" class="s" style="position:relative;overflow:auto;width:100px;height:60px;padding:10px">` +
        `<div style="position:absolute;left:150px;top:120px;width:20px;height:20px;margin:5px"></div>` +
        `<div style="position:relative;left:30px;top:10px;width:110px;height:10px"></div></div>` +
        `<div data-cid="o" class="s" style="overflow:auto;width:100px;height:60px">` +
        `<div style="position:absolute;left:400px;top:500px;width:20px;height:20px"></div>${BOX(10, 10)}</div>` +
        `<div data-cid="f" class="s" style="position:relative;overflow:auto;width:100px;height:60px">` +
        `<div style="position:fixed;left:300px;top:5px;width:20px;height:20px"></div>` +
        `<div style="position:absolute;left:250px;top:5px;width:20px;height:20px"><div style="position:fixed;left:900px;width:5px;height:5px"></div></div></div>`,
    ),
    metrics: ["s", "o", "f"],
  },
  {
    id: "scroll-extent-nested-clip-and-transform",
    html: DOC(
      "",
      `<div data-cid="s" class="s" style="overflow:auto;width:100px;height:60px">` +
        `<div style="overflow:hidden;width:80px;height:40px">${BOX(500, 500)}</div>` +
        `<div style="display:contents"><div style="width:20px;height:20px;transform:translate(200px,100px) scale(2)"></div></div></div>` +
        `<div data-cid="t" class="s" style="overflow:auto;width:100px;height:60px;transform:rotate(90deg)">${BOX(130, 20)}</div>`,
    ),
    metrics: ["s", "t"],
  },
  {
    id: "scroll-extent-no-box",
    html: DOC(
      "",
      `<div data-cid="n" style="display:none;overflow:auto;width:10px">${BOX(50, 50)}</div>` +
        `<div data-cid="c" style="display:contents">${BOX(50, 50, "k")}</div>`,
    ),
    metrics: ["n", "c", "k"],
  },
  // An image's border box reaches the scroll extents; its paint fragment adds nothing.
  {
    id: "scroll-extent-image",
    html: DOC("img{display:block;margin:0 3px 4px 0}", `<div data-cid="s" class="s" style="overflow:auto;width:100px;height:50px"><img src="data:image/png;base64,${PNG}"></div><img src="data:image/png;base64,${PNG}" style="position:absolute;left:1300px;top:0">`),
    metrics: ["s"],
    viewport: true,
  },
  // scroll::viewport_metrics: the initial containing block plus the root's overflow,
  // without fixed boxes (or their descendants) and content clipped by a nested box.
  {
    id: "viewport-extent",
    html: DOC(
      "",
      `${BOX(1500, 100, "w", "margin-right:20px")}${BOX(100, 900, "t", "margin-bottom:30px")}` +
        `<div style="position:fixed;left:3000px;top:3000px;width:10px;height:10px"><div style="width:5000px;height:5000px"></div></div>` +
        `<div style="overflow:hidden;width:10px;height:10px">${BOX(4000, 4000)}</div>` +
        `<div style="display:contents"><div style="position:absolute;left:1700px;top:20px;width:30px;height:30px;margin:40px"></div></div>` +
        `<div style="width:10px;height:10px;transform:translate(2000px,0)"></div>`,
    ),
    viewport: true,
  },
  {
    id: "viewport-extent-fits",
    html: DOC("", BOX(10, 10, "a")),
    viewport: true,
  },
  {
    id: "viewport-extent-body-overflow-propagates",
    html: DOC("body{overflow:hidden}", `${BOX(1400, 900)}<div style="position:absolute;left:0;top:0;overflow:hidden;width:5px;height:5px">${BOX(3000, 3000)}</div>`),
    viewport: true,
  },
  // scroll::propagated_body / element_scroller: the body's overflow goes to the viewport
  // unless the root's own overflow is not visible.
  {
    id: "body-overflow-propagates",
    html: DOC("html{height:100px}body{overflow:auto;height:50px}", BOX(10, 3000)),
    metrics: ["body", "html"],
    viewport: true,
  },
  {
    id: "body-overflow-own-scroller",
    html: DOC("html{overflow:hidden;height:100px}body{overflow:auto;height:50px}", BOX(10, 3000)),
    metrics: ["body", "html"],
  },
  {
    id: "body-visible-not-a-scroller",
    html: DOC("html{overflow:hidden}", `<div data-cid="c" style="display:contents;overflow:auto">${BOX(10, 3000)}</div>`),
    metrics: ["body", "c"],
  },
  // scroll::scroll_translation: boxes move with every scrolled ancestor up to their
  // containing block; fixed boxes and their contents stay with the viewport.
  {
    id: "translation-nested-scrollers",
    html: DOC(
      "",
      `${BOX(10, 2000)}<div data-cid="o" class="s" style="position:absolute;left:20px;top:30px;overflow:auto;width:200px;height:100px">` +
        `<div data-cid="i" class="s" style="overflow:auto;width:150px;height:60px;margin-top:10px">${BOX(400, 400, "k")}</div>${BOX(400, 400)}` +
        `<div data-cid="z" class="s" style="overflow:auto;width:50px;height:50px">${BOX(100, 100, "zk")}</div></div>` +
        `<div data-cid="none" style="display:none"></div>` +
        `<div data-cid="fx" style="position:fixed;left:300px;top:10px;width:30px;height:30px">${BOX(10, 10, "fk")}</div>` +
        `<div style="position:relative;display:contents"><div data-cid="a" style="position:absolute;left:500px;top:50px;width:10px;height:10px"></div></div>`,
    ),
    scroll: { viewport: [0, 40], o: [15, 25], i: [7, 11], z: [0, 0] },
  },
  {
    id: "translation-absolute-skips-static-scroller",
    html: DOC(
      "",
      `<div data-cid="p" style="position:relative;width:300px;height:200px"><div data-cid="s" class="s" style="overflow:auto;width:100px;height:100px">` +
        `${BOX(300, 300, "in")}<div data-cid="ab" style="position:absolute;left:5px;top:150px;width:10px;height:10px"></div></div></div>`,
    ),
    scroll: { s: [20, 30] },
  },
  {
    id: "translation-transformed-scroller",
    html: DOC(
      "",
      `<div data-cid="s" class="s" style="overflow:auto;width:100px;height:100px;margin:50px;transform:scale(2)">${BOX(300, 300, "k")}</div>`,
    ),
    scroll: { s: [10, 20] },
  },
  // scroll::alignment + scroll::expose: the offsets scrollIntoView/IfNeeded leave.
  {
    id: "into-view",
    html: DOC(
      "",
      `<div data-cid="s" class="s" style="overflow:auto;width:100px;height:100px;padding:0">` +
        `<div style="width:400px;height:1000px;position:relative">` +
        `<div data-cid="t" style="position:absolute;left:150px;top:300px;width:30px;height:40px"></div>` +
        `<div data-cid="big" style="position:absolute;left:0;top:500px;width:300px;height:250px"></div>` +
        `<div data-cid="near" style="position:absolute;left:10px;top:50px;width:20px;height:20px"></div>` +
        `<div data-cid="part" style="position:absolute;left:90px;top:90px;width:30px;height:30px"></div>` +
        `<div data-cid="frac" style="position:absolute;left:0.3px;top:200.3px;width:10.2px;height:10.4px"></div>` +
        `</div></div>`,
    ),
    intoView: [
      ...[0, 1, 2, 3].flatMap((block) =>
        [0, 1, 2, 3].map((inline) => ({ target: "t", scroller: "s", mode: 0, block, inline, from: [0, 0] })),
      ),
      { target: "t", scroller: "s", mode: 0, block: 3, inline: 3, from: [200, 400] },
      { target: "big", scroller: "s", mode: 0, block: 3, inline: 3, from: [0, 0] },
      { target: "big", scroller: "s", mode: 0, block: 3, inline: 3, from: [100, 600] },
      { target: "big", scroller: "s", mode: 0, block: 3, inline: 3, from: [50, 520] },
      { target: "big", scroller: "s", mode: 0, block: 1, inline: 1, from: [50, 520] },
      { target: "big", scroller: "s", mode: 1, block: 3, inline: 3, from: [50, 520] },
      { target: "near", scroller: "s", mode: 1, block: 3, inline: 3, from: [0, 0] },
      { target: "near", scroller: "s", mode: 2, block: 3, inline: 3, from: [0, 0] },
      { target: "t", scroller: "s", mode: 1, block: 3, inline: 3, from: [0, 0] },
      { target: "t", scroller: "s", mode: 2, block: 3, inline: 3, from: [0, 0] },
      { target: "t", scroller: "s", mode: 2, block: 3, inline: 3, from: [300, 900] },
      { target: "part", scroller: "s", mode: 1, block: 3, inline: 3, from: [0, 0] },
      { target: "part", scroller: "s", mode: 2, block: 3, inline: 3, from: [0, 0] },
      { target: "part", scroller: "s", mode: 0, block: 3, inline: 3, from: [0, 0] },
      { target: "frac", scroller: "s", mode: 0, block: 0, inline: 0, from: [0, 0] },
      { target: "frac", scroller: "s", mode: 0, block: 2, inline: 2, from: [0, 0] },
    ],
  },
  // visibility::check: opacity / visibility on the element and its ancestors,
  // content-visibility:hidden ancestors and content-visibility:auto relevance.
  {
    id: "check-visibility",
    html: DOC(
      "",
      `<div data-cid="o" style="opacity:0">${BOX(10, 10, "oc")}</div>` +
        `<div data-cid="op" style="opacity:0%"></div><div data-cid="oh" style="opacity:0.5">${BOX(10, 10, "ohc")}</div>` +
        `<div data-cid="v" style="visibility:hidden">${BOX(10, 10, "vc")}${BOX(10, 10, "vv", "visibility:visible")}</div>` +
        `<div data-cid="cl" style="visibility:collapse"></div>` +
        `<div data-cid="ch" style="content-visibility:hidden;height:10px">${BOX(10, 10, "chc")}</div>` +
        `<div data-cid="ca" style="content-visibility:auto;height:10px">${BOX(10, 10, "cac")}</div>` +
        `<div style="height:3000px"></div>` +
        `<div data-cid="far" style="content-visibility:auto;height:10px">${BOX(10, 10, "farc")}</div>`,
    ),
    visibility: ["o", "oc", "op", "oh", "ohc", "v", "vc", "vv", "cl", "ch", "chc", "ca", "cac", "far", "farc"],
  },
  {
    id: "check-visibility-scrolled-relevance",
    html: DOC(
      "",
      `<div style="height:2000px"></div><div data-cid="ca" style="content-visibility:auto;height:10px">${BOX(10, 10, "cac")}</div><div style="height:2000px"></div>`,
    ),
    scroll: { viewport: [0, 1500] },
    visibility: ["ca", "cac"],
  },
  // text::caret: the closest line (then the closest run on it) holds the caret.
  {
    id: "caret-from-point",
    html: DOC(
      "p{margin:0 0 10px}",
      `<p data-cid="a">hello world</p><p data-cid="b" style="padding-left:30px">ab<span data-cid="sp">cd</span>ef</p>` +
        `<div data-cid="hid" style="width:200px;height:30px">vis<span style="visibility:hidden">hidden</span></div>` +
        `<div data-cid="sc" class="s" style="overflow:hidden;width:60px;height:10px"><div style="width:400px">abcdefghijklmnopqrstuvwxyz</div></div>` +
        `<div data-cid="tr" style="transform:translateX(100px)">moved</div>` +
        `<div data-cid="e" style="height:20px"></div>`,
    ),
    scroll: { sc: [30, 0] },
    carets: [
      [0, 0],
      [3, 5],
      [14, 5],
      [62, 5],
      [500, 5],
      [31, 25],
      [45, 25],
      [52, 25],
      [100, 25],
      [5, 42],
      [30, 42],
      [100, 42],
      [100, 65],
      [3, 85],
      [110, 85],
      [50, 85],
      [40, 105],
    ],
  },
  {
    id: "caret-wrap-and-markers",
    html: DOC(
      "",
      `<div data-cid="w" style="width:30px">abc def</div>` +
        `<ul style="margin:0;padding-left:30px"><li data-cid="li">xy</li></ul>`,
    ),
    carets: [
      [1, 15],
      [28, 5],
      [20, 5],
      [5, 25],
      [40, 25],
      [-5, -5],
    ],
  },
  // range::select: fully selected elements contribute their boxes, partially selected
  // text its own boxes (getClientRects()).
  {
    id: "range-client-rects",
    html: DOC(
      "",
      `<div data-cid="r"><div data-cid="a">abc</div><!--c--><div data-cid="b">de<span data-cid="s">fg</span>hi</div>${BOX(20, 5, "x")}</div>`,
    ),
    ranges: [
      { start: ["r", null, 0], end: ["r", null, 4] },
      { start: ["a", 0, 1], end: ["b", 0, 1] },
      { start: ["a", 0, 1], end: ["a", 0, 2] },
      { start: ["b", 0, 2], end: ["b", 2, 1] },
      { start: ["b", null, 1], end: ["b", null, 2] },
      { start: ["r", null, 1], end: ["r", null, 2] },
      { start: ["a", 0, 3], end: ["b", 0, 0] },
    ],
  },
];

const read = (cid) =>
  cid === "viewport"
    ? document.scrollingElement
    : cid === "body" || cid === "html"
      ? document.querySelector(cid)
      : document.querySelector(`[data-cid="${cid}"]`);
const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const frames = () => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  await page.addStyleTag({ content: FACE });
  await page.evaluate(() => document.fonts.ready.then(() => document.fonts.load('10px "Fira Code"')));
  await page.evaluate(
    ([scroll, readSrc]) => {
      const read = eval(readSrc);
      for (const [cid, [x, y]] of Object.entries(scroll)) read(cid).scrollTo(x, y);
    },
    [testCase.scroll || {}, read.toString()],
  );
  await frames();
  const reads = await page.evaluate(
    ([testCase, readSrc]) => {
      const read = eval(readSrc);
      const extents = (el) => {
        const saved = [el.scrollLeft, el.scrollTop];
        el.scrollTo(1e7, 1e7);
        const max = [el.scrollLeft, el.scrollTop];
        el.scrollTo(...saved);
        return { scrollWidth: el.scrollWidth, scrollHeight: el.scrollHeight, maxX: max[0], maxY: max[1] };
      };
      const boundary = ([cid, child, offset]) => {
        const el = read(cid);
        return [child === null ? el : el.childNodes[child], offset];
      };
      const rect = (b) => [b.x, b.y, b.width, b.height];
      const boxes = [...document.querySelectorAll("[data-cid]")].map((el) => {
        const b = el.getBoundingClientRect();
        return { cid: el.dataset.cid, x: b.x, y: b.y, width: b.width, height: b.height };
      });
      const metrics = (testCase.metrics || []).map((cid) => {
        const el = read(cid);
        const style = getComputedStyle(el);
        return { cid, overflowX: style.overflowX, overflowY: style.overflowY, ...extents(el) };
      });
      const viewport = testCase.viewport ? extents(document.scrollingElement) : undefined;
      const visibility = (testCase.visibility || []).map((cid) => ({
        cid,
        flags: [0, 1, 2, 3, 4, 5, 6, 7].map((f) =>
          read(cid).checkVisibility({ opacityProperty: !!(f & 1), visibilityProperty: !!(f & 2), contentVisibilityAuto: !!(f & 4) }),
        ),
      }));
      const carets = (testCase.carets || []).map(([x, y]) => {
        const caret = document.caretPositionFromPoint(x, y);
        if (!caret) return { x, y, caret: null };
        const node = caret.offsetNode;
        const text = node.nodeType === Node.TEXT_NODE;
        const owner = text ? node.parentElement : node;
        return {
          x,
          y,
          cid: owner.closest("[data-cid]")?.dataset.cid ?? null,
          owner: owner.localName,
          child: text ? [...owner.childNodes].indexOf(node) : null,
          offset: caret.offset,
          rect: rect(caret.getClientRect()),
        };
      });
      const ranges = (testCase.ranges || []).map(({ start, end }) => {
        const range = document.createRange();
        range.setStart(...boundary(start));
        range.setEnd(...boundary(end));
        return { start, end, rects: [...range.getClientRects()].map(rect) };
      });
      return { boxes, metrics, viewport, visibility, carets, ranges };
    },
    [testCase, read.toString()],
  );
  const intoView = [];
  for (const request of testCase.intoView || []) {
    intoView.push({
      ...request,
      to: await page.evaluate(
        ([{ target, scroller, mode, block, inline, from }, readSrc]) => {
          const read = eval(readSrc);
          const names = ["start", "center", "end", "nearest"];
          window.scrollTo(0, 0);
          read(scroller).scrollTo(...from);
          if (mode === 0) read(target).scrollIntoView({ block: names[block], inline: names[inline] });
          else read(target).scrollIntoViewIfNeeded(mode === 1);
          if (window.scrollX || window.scrollY) throw new Error("the viewport scrolled");
          return [read(scroller).scrollLeft, read(scroller).scrollTop];
        },
        [request, read.toString()],
      ),
    });
  }
  const { html, id, scroll } = testCase;
  const entry = { id, html, scroll: scroll || {}, boxes: reads.boxes };
  for (const key of ["metrics", "visibility", "carets", "ranges"]) if (reads[key].length) entry[key] = reads[key];
  if (reads.viewport) entry.viewport = reads.viewport;
  if (intoView.length) entry.intoView = intoView;
  result.push(entry);
}
await browser.close();

const fixture = {
  generator: "scripts/cssom-view-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${result.length} cases -> ${out}`);

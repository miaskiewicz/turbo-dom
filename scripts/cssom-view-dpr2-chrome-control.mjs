// Chrome-156 DPR-2 control for turbo-dom-layout's CSSOM-View inline + Range reads
// (`cssom_view::{inline, text, range}`). Captured at a REAL `--force-device-scale-factor=2`,
// not Playwright's emulated DSF, because emulation rounds font metrics in CSS pixels
// (CLAUDE.md "Text-geometry tests name a repo font"); the real device scale factor rounds
// them in device pixels, which is what the Rust live path does at `LiveOptions { layout_scale: 2.0 }`.
//
// The web font is the repo's bundled "Fira Code" (assets/fonts/fira-code), injected as an
// @font-face. Covers the live-layout inline y-position bug (miaskiewicz/turbo-dom#386,
// follow-up to #377): at DPR 2 the inline y of a 20px Fira Code line comes out 0.5 px
// below Chrome's.
//
// Usage: node scripts/cssom-view-dpr2-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156 under ~/.cache/turbo-consolidation/chrome156;
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/cssom_view_chrome156_dpr2.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
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
    ".cache/turbo-consolidation/chrome156/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(
  values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/cssom_view_chrome156_dpr2.json"),
);
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;
const fira = readFileSync(
  join(repo, "crates/turbo-dom-layout/assets/fonts/fira-code/FiraCode-Regular.ttf"),
).toString("base64");
const FACE = `@font-face{font-family:"Fira Code";src:url(data:font/ttf;base64,${fira})}`;
// line-height:25px matches the font's content height at 20px Fira Code (25 CSS px at
// DPR 2 per turbo-dom#377), so the line box and the font content box align and the
// isolated reading is the inline y itself — no leading overflow to drown the 0.5-px
// bug of miaskiewicz/turbo-dom#386.
const BASE = 'body{margin:0;font-family:"Fira Code";font-size:20px;line-height:25px}';
const DOC = (css, body) => `<!doctype html><style>${BASE}${css}</style>${body}`;

// A single case: an inline web-font Range in a block. 20px Fira Code has an odd content
// height at DPR 2 (`web_font_metrics_chrome`, #377 note), so the line's y position lands
// at 0.5 CSS px in the native layout; Chrome snaps it to a full CSS px (the next device
// pixel) in `getClientRects()` / `getBoundingClientRect()`.
const CASES = [
  {
    id: "webfont-range-dpr2",
    html: DOC(
      "",
      `<div data-cid="r"><div data-cid="a">abc</div><div data-cid="b">de<span data-cid="s">fg</span>hi</div></div>`,
    ),
    ranges: [
      { start: ["r", null, 0], end: ["r", null, 2] },
      { start: ["a", 0, 0], end: ["b", 2, 2] },
      { start: ["b", 0, 0], end: ["b", 2, 2] },
    ],
    inlines: ["s"],
  },
];

const server = createServer((_, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/`;

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: [
    "--use-mock-keychain",
    "--password-store=basic",
    "--force-device-scale-factor=2",
    "--window-size=1280,800",
  ],
});
const version = browser.version();
const context = await browser.newContext({ viewport: null });
const page = await context.newPage();
// Keep the forced device scale factor by setting deviceScaleFactor: 0.
const cdp = await context.newCDPSession(page);
await cdp.send("Emulation.setDeviceMetricsOverride", {
  width: 1280,
  height: 800,
  deviceScaleFactor: 0,
  mobile: false,
});
await page.goto(url);
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  await page.addStyleTag({ content: FACE });
  await page.evaluate(() => document.fonts.ready.then(() => document.fonts.load('20px "Fira Code"')));
  const reads = await page.evaluate((testCase) => {
    const read = (cid) => document.querySelector(`[data-cid="${cid}"]`);
    const rect = (b) => [b.x, b.y, b.width, b.height];
    const boundary = ([cid, child, offset]) => {
      const el = read(cid);
      return [child === null ? el : el.childNodes[child], offset];
    };
    const boxes = [...document.querySelectorAll("[data-cid]")].map((el) => {
      const b = el.getBoundingClientRect();
      return { cid: el.dataset.cid, x: b.x, y: b.y, width: b.width, height: b.height };
    });
    const ranges = (testCase.ranges || []).map(({ start, end }) => {
      const range = document.createRange();
      range.setStart(...boundary(start));
      range.setEnd(...boundary(end));
      return {
        start,
        end,
        rects: [...range.getClientRects()].map(rect),
        bounds: rect(range.getBoundingClientRect()),
      };
    });
    const inlines = (testCase.inlines || []).map((cid) => {
      const el = read(cid);
      const b = el.getBoundingClientRect();
      return {
        cid,
        bounds: rect(b),
        rects: [...el.getClientRects()].map(rect),
      };
    });
    return { boxes, ranges, inlines };
  }, testCase);
  const { html, id } = testCase;
  result.push({ id, html, boxes: reads.boxes, ranges: reads.ranges, inlines: reads.inlines });
}
const environment = await page.evaluate(() => ({
  userAgent: navigator.userAgent,
  viewport: [innerWidth, innerHeight],
  deviceScaleFactor: devicePixelRatio,
}));
await browser.close();
server.close();

const fixture = {
  generator: "scripts/cssom-view-dpr2-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  environment,
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version} (DPR ${environment.deviceScaleFactor}): ${result.length} cases -> ${out}`);

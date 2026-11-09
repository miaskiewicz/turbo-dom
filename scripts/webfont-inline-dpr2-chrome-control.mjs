// Chrome-156 DPR-2 control for the inline-box content-height of an IBM Plex Mono
// web face (turbo-dom-layout tests/web_font_metrics_chrome.rs::
// `an_inline_box_with_a_web_face_takes_chromes_content_height`, miaskiewicz/turbo-dom#386
// follow-up for 20px / 60px). Captured at a REAL `--force-device-scale-factor=2` — not
// Playwright's emulated DSF, which rounds metrics in CSS pixels (CLAUDE.md
// "Text-geometry tests name a repo font").
//
// Usage: node scripts/webfont-inline-dpr2-chrome-control.mjs [--chrome PATH]
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const plex = readFileSync(
  join(repo, "crates/turbo-dom-layout/assets/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf"),
).toString("base64");
const FACE = `@font-face{font-family:"wf";src:url(data:font/ttf;base64,${plex})}`;
const SIZES = [20, 60];
const BODY = (size) =>
  `<!doctype html><style>${FACE}body{margin:0}span{font-family:"wf";font-size:${size}px;white-space:nowrap}</style><div><span id="s">x</span></div>`;

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
const cdp = await context.newCDPSession(page);
await cdp.send("Emulation.setDeviceMetricsOverride", {
  width: 1280,
  height: 800,
  deviceScaleFactor: 0,
  mobile: false,
});
await page.goto(url);
const result = [];
for (const size of SIZES) {
  await page.setContent(BODY(size), { waitUntil: "load" });
  await page.evaluate(
    (s) => document.fonts.ready.then(() => document.fonts.load(`${s}px wf`)),
    size,
  );
  const reads = await page.evaluate(() => {
    const el = document.getElementById("s");
    const span = el.getBoundingClientRect();
    const range = document.createRange();
    range.selectNodeContents(el);
    const rects = [...range.getClientRects()].map((b) => [b.x, b.y, b.width, b.height]);
    const rb = range.getBoundingClientRect();
    return {
      spanBounds: [span.x, span.y, span.width, span.height],
      rangeBounds: [rb.x, rb.y, rb.width, rb.height],
      rangeRects: rects,
    };
  });
  result.push({ size, ...reads });
}
const environment = await page.evaluate(() => ({
  userAgent: navigator.userAgent,
  viewport: [innerWidth, innerHeight],
  deviceScaleFactor: devicePixelRatio,
}));
await browser.close();
server.close();

console.log(`Chrome ${version} DPR ${environment.deviceScaleFactor}`);
for (const r of result) {
  console.log(
    `size=${r.size}px: span.height=${r.spanBounds[3]} range.height=${r.rangeBounds[3]} rangeRects=${JSON.stringify(r.rangeRects)}`,
  );
}
console.log(JSON.stringify({ chrome: version, environment, cases: result }, null, 2));

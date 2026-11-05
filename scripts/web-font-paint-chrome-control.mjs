// Chrome-156 pixel control for web fonts in turbo-dom-raster's screenshots and canvas replay
// (turbo-dom#304): text in an @font-face face the raster does not bundle paints in that face
// when the caller passes it in a FontRegistry. Two fixture fonts loaded through @font-face:
// Go-Regular.ttf as "WebGo" (not bundled) and IBMPlexMono-Regular.ttf as "WebPlex" (its
// web-font metrics: 25px lines at 20px).
//   - canvas: fillText in 96px WebGo on a transparent canvas;
//   - screenshot cases: a page (viewport 640x240, DSF 1, white body, margin 0) of 64px WebGo
//     text, and four 20px WebPlex lines (`line-height: normal`, so each line is the face's
//     content height apart).
// The fixture keeps each page's body and style without the @font-face rules (the raster takes
// the faces from the caller's registry, not from the page). Each case records its ink: the [start, end) runs of ink columns and of ink rows (canvas:
// alpha >= 128; screenshot: a channel < 128). Glyph anti-aliasing differs between Skia and
// tiny-skia, so the run edges are compared to a pixel; a fallback face or a wrong line pitch
// moves them by many.
// Asserted by crates/turbo-dom-raster/tests/web_font_paint_chrome.rs.
//
// Usage: node scripts/web-font-paint-chrome-control.mjs [--chrome PATH]
//   --chrome defaults to the frozen Chrome 156.0.8078.47 headless shell (mock keychain).
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { decodePng } from "./lib/png-samples.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(homedir(), ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell");
const playwrightCore = process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const fonts = join(repo, "crates/turbo-dom-layout/assets/fonts");
const FACES = { WebGo: "Go-Regular.ttf", WebPlex: "ibm-plex-mono/IBMPlexMono-Regular.ttf" };
const css = Object.entries(FACES)
  .map(([family, file]) => {
    const b64 = readFileSync(join(fonts, file)).toString("base64");
    return `@font-face{font-family:"${family}";src:url(data:font/sfnt;base64,${b64})}`;
  })
  .join("");
const VIEWPORT = [640, 240];
const PAGE_STYLE = "body{margin:0;background:#fff;color:#000}";
const SCREENSHOTS = [
  { name: "go-64", body: `<div style="font:64px WebGo">Hamburg Waves</div>` },
  { name: "plex-lines", body: `<div style="font:20px WebPlex">X<br>X<br>X<br>X</div>` },
];
const CANVAS = { width: 1200, height: 160, baseline: [10, 120], font: "96px WebGo", text: "Hamburg Waves" };

function runs(count, inkAt) {
  const out = [];
  let start = -1;
  for (let i = 0; i <= count; i++) {
    const ink = i < count && inkAt(i);
    if (ink && start < 0) start = i;
    if (!ink && start >= 0) out.push([start, i]), (start = -1);
  }
  return out;
}

const pages = Object.fromEntries(
  SCREENSHOTS.map((c) => [`/${c.name}`, `<!doctype html><html><head><style>${css}${PAGE_STYLE}</style></head><body>${c.body}</body></html>`]),
);
const server = createServer((req, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end(pages[req.url] ?? `<!doctype html><html><head><style>${css}</style></head><body></body></html>`);
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const errors = [];
let version;
const screenshots = [];
let canvas;
try {
  const browser = await chromium.launch({
    executablePath: chromePath,
    headless: true,
    args: ["--use-mock-keychain", "--password-store=basic", "--force-device-scale-factor=1"],
  });
  version = browser.version();
  const page = await browser.newPage({ viewport: { width: VIEWPORT[0], height: VIEWPORT[1] } });
  page.on("pageerror", (e) => errors.push(String(e)));
  const base = `http://127.0.0.1:${server.address().port}`;
  for (const c of SCREENSHOTS) {
    await page.goto(`${base}/${c.name}`);
    await page.evaluate(async (families) => {
      for (const f of families) await document.fonts.load(`20px "${f}"`);
    }, Object.keys(FACES));
    const px = decodePng(await page.screenshot());
    const [w, h] = VIEWPORT;
    const dark = (x, y) => px(x, y).some((v) => v < 128);
    screenshots.push({
      name: c.name,
      body: c.body,
      columns: runs(w, (x) => {
        for (let y = 0; y < h; y++) if (dark(x, y)) return true;
        return false;
      }),
      rows: runs(h, (y) => {
        for (let x = 0; x < w; x++) if (dark(x, y)) return true;
        return false;
      }),
    });
  }
  await page.goto(`${base}/canvas`);
  canvas = await page.evaluate(async ({ width, height, baseline, font, text }) => {
    await document.fonts.load(font);
    const el = document.createElement("canvas");
    el.width = width;
    el.height = height;
    const ctx = el.getContext("2d");
    ctx.font = font;
    ctx.fillStyle = "#000";
    ctx.fillText(text, ...baseline);
    const { data } = ctx.getImageData(0, 0, width, height);
    const ink = (x, y) => data[(y * width + x) * 4 + 3] >= 128;
    const runs = (count, inkAt) => {
      const out = [];
      let start = -1;
      for (let i = 0; i <= count; i++) {
        const on = i < count && inkAt(i);
        if (on && start < 0) start = i;
        if (!on && start >= 0) out.push([start, i]), (start = -1);
      }
      return out;
    };
    return {
      width,
      height,
      baseline,
      font: ctx.font,
      text,
      measureText: ctx.measureText(text).width,
      columns: runs(width, (x) => {
        for (let y = 0; y < height; y++) if (ink(x, y)) return true;
        return false;
      }),
      rows: runs(height, (y) => {
        for (let x = 0; x < width; x++) if (ink(x, y)) return true;
        return false;
      }),
    };
  }, CANVAS);
  await browser.close();
} finally {
  server.close();
}
const control = {
  browser: `Chrome/${version}`,
  collectorSha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
  faces: Object.fromEntries(Object.entries(FACES).map(([family, file]) => [family, { file, sha256: sha256(readFileSync(join(fonts, file))) }])),
  viewport: VIEWPORT,
  pageStyle: PAGE_STYLE,
  errors,
  screenshots,
  canvas,
};
const out = join(repo, "crates/turbo-dom-raster/tests/fixtures/web_font_paint_chrome156.json");
writeFileSync(out, `${JSON.stringify(control, null, 1)}\n`);
console.log(out, control.browser, errors);
for (const s of screenshots) console.log(s.name, JSON.stringify(s.columns.slice(0, 6)), JSON.stringify(s.rows));
console.log("canvas", canvas.measureText, JSON.stringify(canvas.columns.slice(0, 6)), JSON.stringify(canvas.rows));

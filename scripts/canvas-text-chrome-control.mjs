// Chrome-156 pixel control for turbo-dom-raster's canvas fillText glyph pen (turbo-dom#236):
// where each glyph lands along the baseline. Chrome's canvas shapes text word by word (a
// U+0020 is a word of its own, so no kerning pair crosses a space) and advances by 16.16
// glyph positions, the same widths `measureText` reports.
//
// The fonts are the repo's fixture files (crates/turbo-dom-layout/assets/fonts) loaded through
// @font-face, which the raster resolves from its bundled set by family name, so the control is
// host-independent. Each case fills black text on a transparent canvas and records the ink
// columns: the [start, end) x ranges whose column has a pixel with alpha >= 128. Large sizes
// make a pen error of a fraction of a pixel at 16px move a run edge by whole pixels.
// Asserted by crates/turbo-dom-raster/tests/canvas_text_chrome.rs.
//
// Usage: node scripts/canvas-text-chrome-control.mjs [--chrome PATH]
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

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
  );
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

const fonts = join(repo, "crates/turbo-dom-layout/assets/fonts");
const FACES = [
  ["Liberation Serif", "liberation-serif/LiberationSerif-Regular.ttf", 400],
  ["Inter", "inter/Inter-Bold.otf", 700],
];
const css = FACES.map(([family, file, weight]) => {
  const type = file.endsWith(".otf") ? "font/otf" : "font/ttf";
  const b64 = readFileSync(join(fonts, file)).toString("base64");
  return `@font-face{font-family:"${family}";src:url(data:${type};base64,${b64});font-weight:${weight}}`;
}).join("");
const fontFiles = Object.fromEntries(FACES.map(([, file]) => [file, sha256(readFileSync(join(fonts, file)))]));

const CASES = [
  { font: "160px 'Liberation Serif'", text: "AVATAR Wave" },
  { font: "160px 'Liberation Serif'", text: "Hello, World!" },
  { font: "bold 120px Inter", text: "To Wave, AV" },
];
const [width, height] = [1700, 240];
const baseline = [10, 190];

const server = createServer((_, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end(`<!doctype html><html><head><style>${css}</style></head><body></body></html>`);
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
try {
  const browser = await chromium.launch({
    executablePath: chromePath,
    headless: true,
    args: ["--use-mock-keychain", "--password-store=basic"],
  });
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const result = await page.evaluate(
    async ({ cases, width, height, baseline }) => {
      for (const { font } of cases) await document.fonts.load(font);
      return cases.map(({ font, text }) => {
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext("2d");
        ctx.font = font;
        ctx.fillStyle = "#000";
        ctx.fillText(text, ...baseline);
        const { data } = ctx.getImageData(0, 0, width, height);
        const runs = [];
        let start = -1;
        for (let x = 0; x <= width; x++) {
          let ink = false;
          for (let y = 0; x < width && y < height && !ink; y++) ink = data[(y * width + x) * 4 + 3] >= 128;
          if (ink && start < 0) start = x;
          if (!ink && start >= 0) runs.push([start, x]), (start = -1);
        }
        return { font: ctx.font, text, width: ctx.measureText(text).width, runs };
      });
    },
    { cases: CASES, width, height, baseline },
  );
  const control = {
    browser: `Chrome/${browser.version()}`,
    collectorSha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
    fontFiles,
    canvas: [width, height],
    baseline,
    cases: result,
  };
  const out = join(repo, "crates/turbo-dom-raster/tests/fixtures/canvas_text_chrome156.json");
  writeFileSync(out, `${JSON.stringify(control, null, 1)}\n`);
  console.log(`${out}: ${control.browser}`);
  await browser.close();
} finally {
  server.close();
}

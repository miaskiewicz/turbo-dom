// Chrome 156 primary control for screenshot DPR rounding (#505).
// Usage: node scripts/screenshot-scale-chrome-control.mjs [--chrome PATH] [--out PATH]
// Local HTML only. Raw CDP and Playwright independently capture the same CSS
// viewport and positioned full-page extent. Fractional clip wrapper policy is
// intentionally outside this viewport/full-page dimension control.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const probePath = fileURLToPath(import.meta.url);
const repo = resolve(dirname(probePath), "..");
const { values } = parseArgs({
  options: { chrome: { type: "string" }, out: { type: "string" } },
});
const chromePath =
  values.chrome ||
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
  );
const out = resolve(
  values.out ||
    join(
      repo,
      "crates/turbo-dom-raster/tests/fixtures/screenshot-scale-chrome156.json",
    ),
);
const playwrightCore =
  process.env.PLAYWRIGHT_CORE ||
  resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const html =
  "<!doctype html><html><head><style>html,body{margin:0;background:white}#a{position:absolute;left:0;top:0;width:10px;height:10px;background:red}#b{position:absolute;left:0;top:220px;width:20px;height:20px;background:blue}</style></head><body><div id=a></div><div id=b></div></body></html>";
const matrix = [
  ["decimal-integral-1.1", 1.1, 100, 80],
  ["decimal-integral-1.2", 1.2, 100, 80],
  ["decimal-integral-1.3", 1.3, 100, 80],
  ["decimal-integral-1.4", 1.4, 100, 80],
  ["fraction-round-up", 1.2, 103, 83],
  ["fraction-round-down", 1.25, 101, 81],
  ["half-tie-up", 1.5, 101, 81],
  ["fraction-round-up-1.6", 1.6, 101, 81],
  ["half-tie-up-subunit", 0.5, 101, 81],
];

async function pixels(page, png, width, height) {
  return page.evaluate(
    async ({ data, width, height }) => {
      const bitmap = await createImageBitmap(
        await (await fetch(`data:image/png;base64,${data}`)).blob(),
      );
      const canvas = new OffscreenCanvas(width, height);
      const context = canvas.getContext("2d");
      context.drawImage(bitmap, 0, 0);
      return [
        [0, 0],
        [0, height - 1],
        [width - 1, height - 1],
      ].map(([x, y]) => ({
        x,
        y,
        rgba: [...context.getImageData(x, y, 1, 1).data],
      }));
    },
    { data: png.toString("base64"), width, height },
  );
}

async function record(page, png, fullPage) {
  const width = png.readUInt32BE(16);
  const height = png.readUInt32BE(20);
  const samples = await pixels(page, png, width, height);
  assert.deepEqual(
    samples.map((sample) => sample.rgba),
    [
      [255, 0, 0, 255],
      fullPage ? [0, 0, 255, 255] : [255, 255, 255, 255],
      [255, 255, 255, 255],
    ],
  );
  return { width, height, pngSha256: sha256(png) };
}

const args = ["--use-mock-keychain", "--password-store=basic"];
const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args,
});
const cases = [];
try {
  for (const [id, requestedDpr, width, height] of matrix) {
    const viewport = { width, height };
    const context = await browser.newContext({
      viewport,
      deviceScaleFactor: requestedDpr,
    });
    try {
      const page = await context.newPage();
      await page.route("**/*", (route) => route.abort());
      await page.setContent(html);
      const facts = await page.evaluate(() => ({
        reportedDpr: devicePixelRatio,
        viewport: { width: innerWidth, height: innerHeight },
        scrollWidth: document.documentElement.scrollWidth,
        scrollHeight: document.documentElement.scrollHeight,
        positionedBottom: document.querySelector("#b").getBoundingClientRect()
          .bottom,
      }));
      const cdp = await context.newCDPSession(page);
      // Device emulation is session-sensitive: a new unconfigured CDP session
      // captures at 1x even when page JS already reports the context's DPR.
      await cdp.send("Emulation.setDeviceMetricsOverride", {
        width,
        height,
        deviceScaleFactor: requestedDpr,
        mobile: false,
      });
      for (const fullPage of [false, true]) {
        const clip = {
          x: 0,
          y: 0,
          width: facts.scrollWidth,
          height: fullPage ? facts.scrollHeight : height,
          scale: 1,
        };
        const raw = await cdp.send("Page.captureScreenshot", {
          format: "png",
          fromSurface: true,
          captureBeyondViewport: fullPage,
          clip,
        });
        cases.push({
          id,
          requestedDpr,
          viewport,
          fullPage,
          facts,
          cdp: await record(page, Buffer.from(raw.data, "base64"), fullPage),
          playwright: await record(
            page,
            await page.screenshot({ fullPage }),
            fullPage,
          ),
        });
      }
    } finally {
      await context.close();
    }
  }
  const result = {
    browser: await browser.version(),
    playwrightVersion: require(join(playwrightCore, "package.json")).version,
    probeSha256: sha256(readFileSync(probePath)),
    args,
    note: "Offline viewport/full-page PNG control. Dimensions exhibit nearest rounding, half ties upward. CSS layout is unscaled. Fractional clip policy is excluded.",
    html,
    cases,
  };
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`);
} finally {
  await browser.close();
}

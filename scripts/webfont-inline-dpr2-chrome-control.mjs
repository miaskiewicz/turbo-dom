// Chrome-156 DPR-2 control for the inline-box content-height of a web face
// at the 11-19 px rounding boundary (miaskiewicz/turbo-dom#446), captured at
// a REAL `--force-device-scale-factor=2` — not Playwright's emulated DSF,
// which rounds metrics in CSS pixels (CLAUDE.md "Text-geometry tests name a
// repo font"). Also re-captures the #386 Plex sizes (20 / 60) for continuity
// (stdout only — the `web-font-metrics/chrome156.json` fixture pins those).
//
// Fixture fonts (Liberation Serif, Inter, Fira Code: the repo's
// test_support::fixture_fonts set) are loaded via `@font-face` and measured
// at 11/13/15/17/19 px. Output is a JSON object with `cases` keyed on
// `{font, size}`, reporting the span's `getBoundingClientRect().height` and
// the Range's content-box height.
//
// Usage: node scripts/webfont-inline-dpr2-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --out defaults to crates/turbo-dom-layout/tests/fixtures/web-font-metrics/dpr2_rounding_chrome156.json
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({
  options: { chrome: { type: "string" }, out: { type: "string" } },
});
const chromePath =
  values.chrome ||
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(
  values.out ||
    join(
      repo,
      "crates/turbo-dom-layout/tests/fixtures/web-font-metrics/dpr2_rounding_chrome156.json",
    ),
);
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);

const b64 = (p) => readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts", p)).toString("base64");

/** Repo-local fixture fonts loaded as data: web faces. */
const FONTS = [
  {
    name: "liberation-serif-regular",
    path: "liberation-serif/LiberationSerif-Regular.ttf",
    mime: "font/ttf",
  },
  {
    name: "inter-regular",
    path: "inter/Inter-Regular.otf",
    mime: "font/otf",
  },
  {
    name: "fira-code-regular",
    path: "fira-code/FiraCode-Regular.ttf",
    mime: "font/ttf",
  },
];

/** The rounding-boundary sizes from the issue. */
const SIZES = [11, 13, 15, 17, 19];

const pageFor = (font, size) => {
  const data = b64(font.path);
  const face = `@font-face{font-family:"wf";src:url(data:${font.mime};base64,${data})}`;
  return `<!doctype html><style>${face}body{margin:0}span{font-family:"wf";font-size:${size}px;white-space:nowrap}</style><div><span id="s">x</span></div>`;
};

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

const cases = [];
for (const font of FONTS) {
  for (const size of SIZES) {
    await page.setContent(pageFor(font, size), { waitUntil: "load" });
    await page.evaluate(
      (s) => document.fonts.ready.then(() => document.fonts.load(`${s}px wf`)),
      size,
    );
    const reads = await page.evaluate(() => {
      const el = document.getElementById("s");
      const span = el.getBoundingClientRect();
      const range = document.createRange();
      range.selectNodeContents(el);
      const rb = range.getBoundingClientRect();
      return {
        spanHeight: span.height,
        rangeHeight: rb.height,
      };
    });
    cases.push({ font: font.name, size, ...reads });
  }
}
const environment = await page.evaluate(() => ({
  userAgent: navigator.userAgent,
  viewport: [innerWidth, innerHeight],
  deviceScaleFactor: devicePixelRatio,
}));
await browser.close();
server.close();

const fixture = {
  generator: "scripts/webfont-inline-dpr2-chrome-control.mjs",
  chrome: version,
  environment,
  cases,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version} DPR ${environment.deviceScaleFactor}: ${cases.length} cases -> ${out}`);
for (const c of cases) {
  console.log(
    `${c.font.padEnd(28)} size=${c.size}px  span=${c.spanHeight}  range=${c.rangeHeight}`,
  );
}

// Chrome-156 control for the vertical metrics of @font-face web fonts (turbo-dom#304):
// every TrueType/OpenType fixture font under crates/turbo-dom-layout/assets/fonts, loaded as
// a web font from a data: URL, at font sizes 6..64px in quarter-pixel steps, at device scale
// factors 1 and 2. Per (face, size): the inline content box height (a <span>'s
// getBoundingClientRect height), its ascent (the span top to the baseline, read from a
// zero-height inline-block on the baseline), the line-height:normal line box height (the
// block's height) and canvas measureText's fontBoundingBoxAscent / fontBoundingBoxDescent.
// Asserted by crates/turbo-dom-layout/tests/web_font_metrics_chrome.rs.
//
// Usage: node scripts/web-font-metrics-chrome-control.mjs [--chrome PATH]
//   --chrome defaults to the frozen Chrome 156.0.8078.47 headless shell (mock keychain).
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

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
const fontsDir = join(repo, "crates/turbo-dom-layout/assets/fonts");

function walk(dir) {
  return readdirSync(dir)
    .sort()
    .flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return walk(path);
      return /\.(ttf|otf)$/.test(name) ? [path] : [];
    });
}
const FONTS = walk(fontsDir).map((path) => ({ path: relative(fontsDir, path), bytes: readFileSync(path) }));
const family = (i) => `wf${i}`;
const css = FONTS.map(
  (f, i) => `@font-face{font-family:"${family(i)}";src:url(data:font/sfnt;base64,${f.bytes.toString("base64")})}`,
).join("");
const SIZES = Array.from({ length: (64 - 6) * 4 + 1 }, (_, i) => 6 + i / 4);

const server = createServer((_, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end(`<!doctype html><html><head><style>${css}body{margin:0}</style></head><body></body></html>`);
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const result = {};
const errors = [];
let version;
try {
  for (const dsf of [1, 2]) {
    const browser = await chromium.launch({
      executablePath: chromePath,
      headless: true,
      args: ["--use-mock-keychain", "--password-store=basic", `--force-device-scale-factor=${dsf}`],
    });
    version = browser.version();
    const page = await browser.newPage();
    page.on("pageerror", (e) => errors.push(String(e)));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    result[`dsf${dsf}`] = await page.evaluate(
      async ({ count, sizes }) => {
        const out = [];
        const ctx = document.createElement("canvas").getContext("2d");
        for (let i = 0; i < count; i++) {
          const name = `wf${i}`;
          const loaded = await document.fonts.load(`16px "${name}"`);
          const rows = [];
          for (const size of sizes) {
            const div = document.createElement("div");
            div.setAttribute("style", `font-family:"${name}";font-size:${size}px;white-space:nowrap`);
            const span = document.createElement("span");
            span.textContent = "x";
            const mark = document.createElement("span");
            mark.setAttribute("style", "display:inline-block;width:0;height:0");
            div.append(span, mark);
            document.body.append(div);
            const r = span.getBoundingClientRect();
            const baseline = mark.getBoundingClientRect().top;
            ctx.font = `${size}px "${name}"`;
            const m = ctx.measureText("x");
            rows.push([size, r.height, baseline - r.top, div.getBoundingClientRect().height, m.fontBoundingBoxAscent, m.fontBoundingBoxDescent]);
            div.remove();
          }
          out.push({ loaded: loaded.length, rows });
        }
        return out;
      },
      { count: FONTS.length, sizes: SIZES },
    );
    await browser.close();
  }
} finally {
  server.close();
}
const control = {
  browser: `Chrome/${version}`,
  collectorSha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
  columns: ["size", "height", "ascent", "lineHeight", "canvasAscent", "canvasDescent"],
  errors,
  fonts: FONTS.map((f, i) => ({
    path: f.path,
    sha256: sha256(f.bytes),
    dsf1: result.dsf1[i],
    dsf2: result.dsf2[i],
  })),
};
const out = join(repo, "crates/turbo-dom-layout/tests/fixtures/web-font-metrics/chrome156.json");
writeFileSync(out, `${JSON.stringify(control)}\n`);
console.log(out);
for (const f of control.fonts) {
  const at20 = f.dsf1.rows.find((r) => r[0] === 20);
  console.log(f.path, f.dsf1.loaded, f.dsf2.loaded, JSON.stringify(at20));
}

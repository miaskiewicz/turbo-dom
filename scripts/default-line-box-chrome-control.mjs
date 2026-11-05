// Chrome-156 control for the `line-height: normal` line box at device scale factors 1 and 2
// (miaskiewicz/turbo-dom#319, turbo-surf#158). Blink rounds a font's ascent and descent
// (and, for the macOS Times / Helvetica / Courier, adds its 15% ascent hack) in DEVICE
// pixels, so the same 16px default `<p>` is 18px tall at DSF 1 and 18.5px at DSF 2. The
// layout's `layout_scale` is that device scale: a host must pass its devicePixelRatio.
//
// One page per DSF (a real --force-device-scale-factor, not Playwright's emulated one,
// which rounds font metrics in CSS pixels), 800x600, serving one `<p>` per case:
//   - the fixture faces of test_support::fixture_fonts() (Liberation Serif, Inter, Fira
//     Code) through @font-face (data: URLs), so those rows hold on every OS;
//   - `host`: no font-family (Chrome's default, Times on macOS) and `Times` itself, from
//     the capturing host's installed fonts (asserted on macOS only).
// Each row records the `<p>`'s getBoundingClientRect height.
// Asserted by crates/turbo-dom-layout/tests/default_line_box_chrome.rs.
//
// Usage: node scripts/default-line-box-chrome-control.mjs [--chrome PATH]
//   --chrome defaults to the frozen Chrome 156.0.8078.47 headless shell.
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir, platform, release } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" } } });
if (platform() !== "darwin") throw new Error("default line box control: capture on macOS (host rows)");
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
const out = join(repo, "crates/turbo-dom-layout/tests/fixtures/default_line_box_chrome156.json");
const FACES = [
  ["Liberation Serif", "liberation-serif/LiberationSerif-Regular.ttf"],
  ["Inter", "inter/Inter-Regular.otf"],
  ["Fira Code", "fira-code/FiraCode-Regular.ttf"],
];
const fontFiles = {};
const css = FACES.map(([family, file]) => {
  const bytes = readFileSync(join(fonts, file));
  fontFiles[file] = sha256(bytes);
  const type = file.endsWith(".otf") ? "font/otf" : "font/ttf";
  return `@font-face{font-family:"${family}";src:url(data:${type};base64,${bytes.toString("base64")})}`;
}).join("");
// [source, font-family ("" = the default)]; every family at every size.
const FAMILIES = [...FACES.map(([family]) => ["fixture", family]), ["host", ""], ["host", "Times"]];
const SIZES = [16, 13, 20];
const [width, height] = [800, 600];

const server = createServer((_, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/`;
const rows = [];
let browserVersion = "";
try {
  for (const dsf of [1, 2]) {
    const browser = await chromium.launch({
      executablePath: chromePath,
      headless: true,
      args: ["--use-mock-keychain", "--password-store=basic", `--force-device-scale-factor=${dsf}`, `--window-size=${width},${height}`],
    });
    browserVersion = browser.version();
    const context = await browser.newContext({ viewport: null });
    const page = await context.newPage();
    // Viewport size only; deviceScaleFactor 0 keeps the real (forced) scale factor.
    const cdp = await context.newCDPSession(page);
    await cdp.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 0, mobile: false });
    await page.goto(url);
    const measured = await page.evaluate(
      async ({ css, families, sizes }) => {
        const style = document.createElement("style");
        style.textContent = css;
        document.head.append(style);
        for (const [, family] of families) if (family && family !== "Times") await document.fonts.load(`16px "${family}"`);
        await document.fonts.ready;
        const out = [];
        for (const [source, family] of families)
          for (const size of sizes) {
            const p = document.createElement("p");
            p.style.cssText = `${family ? `font-family:"${family}";` : ""}font-size:${size}px`;
            p.textContent = "Hello world";
            document.body.append(p);
            out.push({ source, family, size, height: p.getBoundingClientRect().height });
          }
        return { out, dpr: devicePixelRatio, viewport: [innerWidth, innerHeight] };
      },
      { css, families: FAMILIES, sizes: SIZES },
    );
    if (measured.dpr !== dsf || measured.viewport[0] !== width) throw new Error(`environment ${JSON.stringify(measured)}`);
    for (const row of measured.out) rows.push({ dsf, ...row });
    await browser.close();
  }
} finally {
  server.close();
}
const control = {
  generator: "scripts/default-line-box-chrome-control.mjs",
  browser: `Chrome/${browserVersion}`,
  host: `darwin ${release()}`,
  viewport: [width, height],
  fontFiles,
  rows,
};
writeFileSync(out, `${JSON.stringify(control, null, 1)}\n`);
console.log(`${out}: ${control.browser}`);
for (const r of rows) console.log(r.dsf, r.source.padEnd(8), (r.family || "(default)").padEnd(18), r.size, r.height);

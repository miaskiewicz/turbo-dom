// Chrome-156 controls for turbo-dom-layout's text-geometry comparisons that lay out with the
// repo's font fixtures (`turbo_dom_layout::test_support::fixture_fonts()`): Liberation Serif,
// Inter and Fira Code from crates/turbo-dom-layout/assets/fonts. Chrome loads the same files
// through @font-face (data: URLs), so the controls never depend on the capturing host's
// installed fonts and the Rust comparisons run on every OS (miaskiewicz/turbo-dom#181).
//
// Each control is a probe script in crates/turbo-dom-layout/tests/fixtures/ that names the
// fixture families explicitly. Before a probe runs, the page defines
//   FIXTURE_FONTS_CSS            the @font-face rules (to copy into srcdoc frames), and
//   fixtureFontsReady(document)  resolves once every fixture face is loaded in `document`.
// The page is served from 127.0.0.1, headless, with --force-device-scale-factor=2 (the Rust
// tests' layout_scale). A real device scale factor, not Playwright's emulated one: emulation
// leaves font metrics rounded at CSS-pixel size, the real one rounds them in device pixels.
//
// Controls (probe -> output, viewport):
//   block-inline-probe.js          -> block-inline-chrome156.json          1280x800
//   native-iframe/probe.js         -> native-iframe/chrome156.json         1200x900
//   inline-borders/probe.js        -> inline-borders/chrome156.json        200x200
//
// Usage: node scripts/fixture-font-chrome-controls.mjs [--chrome PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
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
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const fonts = join(repo, "crates/turbo-dom-layout/assets/fonts");
const fixtures = join(repo, "crates/turbo-dom-layout/tests/fixtures");
// The faces of test_support::fixture_fonts(), with their CSS descriptors.
const FACES = [
  ["Liberation Serif", "liberation-serif/LiberationSerif-Regular.ttf", 400, "normal"],
  ["Liberation Serif", "liberation-serif/LiberationSerif-Bold.ttf", 700, "normal"],
  ["Liberation Serif", "liberation-serif/LiberationSerif-Italic.ttf", 400, "italic"],
  ["Liberation Serif", "liberation-serif/LiberationSerif-BoldItalic.ttf", 700, "italic"],
  ["Inter", "inter/Inter-Regular.otf", 400, "normal"],
  ["Inter", "inter/Inter-Bold.otf", 700, "normal"],
  ["Inter", "inter/Inter-Italic.otf", 400, "italic"],
  ["Inter", "inter/Inter-BoldItalic.otf", 700, "italic"],
  ["Fira Code", "fira-code/FiraCode-Regular.ttf", 400, "normal"],
  ["Fira Code", "fira-code/FiraCode-Bold.ttf", 700, "normal"],
];
const fontFiles = {};
const css = FACES.map(([family, file, weight, style]) => {
  const bytes = readFileSync(join(fonts, file));
  fontFiles[file] = sha256(bytes);
  const type = file.endsWith(".otf") ? "font/otf" : "font/ttf";
  return `@font-face{font-family:"${family}";src:url(data:${type};base64,${bytes.toString("base64")});font-weight:${weight};font-style:${style}}`;
}).join("");
const loads = FACES.map(([family, , weight, style]) => `${style} ${weight} 16px "${family}"`);
const setup = `window.FIXTURE_FONTS_CSS=${JSON.stringify(css)};
window.fixtureFontsReady=async(doc)=>{for(const f of ${JSON.stringify(loads)}){const got=await doc.fonts.load(f);if(!got.length)throw new Error("fixture font not loaded: "+f)}await doc.fonts.ready};
{const s=document.createElement("style");s.textContent=FIXTURE_FONTS_CSS;document.head.append(s)}`;

const CONTROLS = [
  ["block-inline-probe.js", "block-inline-chrome156.json", [1280, 800]],
  ["native-iframe/probe.js", "native-iframe/chrome156.json", [1200, 900]],
  ["inline-borders/probe.js", "inline-borders/chrome156.json", [200, 200]],
];

const server = createServer((_, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/`;
try {
  for (const [probe, out, [width, height]] of CONTROLS) {
    const source = readFileSync(join(fixtures, probe));
    const browser = await chromium.launch({
      executablePath: chromePath,
      headless: true,
      args: ["--force-device-scale-factor=2", `--window-size=${width},${height}`],
    });
    const context = await browser.newContext({ viewport: null });
    const page = await context.newPage();
    // Viewport size only; deviceScaleFactor 0 keeps the real (forced) scale factor.
    const cdp = await context.newCDPSession(page);
    await cdp.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 0, mobile: false });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error)));
    await page.goto(url);
    await page.evaluate(setup);
    await page.evaluate("fixtureFontsReady(document)");
    const result = await page.evaluate(source.toString());
    const environment = await page.evaluate(() => ({
      userAgent: navigator.userAgent,
      viewport: [innerWidth, innerHeight],
      deviceScaleFactor: devicePixelRatio,
    }));
    const control = {
      browser: `Chrome/${browser.version()}`,
      environment,
      probeSha256: sha256(source),
      collectorSha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
      fontFiles,
      errors,
      result: typeof result === "string" ? JSON.parse(result) : result,
    };
    const [w, h] = environment.viewport;
    if (w !== width || h !== height || environment.deviceScaleFactor !== 2)
      throw new Error(`${out}: environment ${JSON.stringify(environment)}`);
    writeFileSync(join(fixtures, out), `${JSON.stringify(control, null, 2)}\n`);
    console.log(`${out}: ${control.browser}, ${errors.length} errors`);
    await browser.close();
  }
} finally {
  server.close();
}

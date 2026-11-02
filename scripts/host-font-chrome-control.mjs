// Chrome-156 control for turbo-dom-layout's host-font text advances (turbo-dom#236): the
// macOS system faces (Times), which carry AAT `morx`/`trak` tables none of the fixture fonts
// has. Unlike scripts/fixture-font-chrome-controls.mjs no font is injected: Chrome resolves
// the families from the capturing host, so the control is only valid on macOS and the Rust
// test that reads it (tests/host_font_advances_chrome.rs) is `cfg(target_os = "macos")`.
//
//   host-font-advances/probe.js -> host-font-advances/chrome156.json   800x600, DSF 2
//
// Usage: node scripts/host-font-chrome-control.mjs [--chrome PATH]
//   --chrome defaults to the frozen Chrome 156.0.8078.47 headless shell (launched with a
//   mock keychain, so it never prompts for the macOS keychain).
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
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
if (platform() !== "darwin") throw new Error("host-font control: capture on macOS");
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
const fixtures = join(repo, "crates/turbo-dom-layout/tests/fixtures");
const probe = "host-font-advances/probe.js";
const out = "host-font-advances/chrome156.json";
const [width, height] = [800, 600];

const server = createServer((_, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/`;
try {
  const source = readFileSync(join(fixtures, probe));
  const browser = await chromium.launch({
    executablePath: chromePath,
    headless: true,
    args: ["--use-mock-keychain", "--password-store=basic", "--force-device-scale-factor=2", `--window-size=${width},${height}`],
  });
  const context = await browser.newContext({ viewport: null });
  const page = await context.newPage();
  // Viewport size only; deviceScaleFactor 0 keeps the real (forced) scale factor.
  const cdp = await context.newCDPSession(page);
  await cdp.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 0, mobile: false });
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.goto(url);
  const result = await page.evaluate(source.toString());
  const environment = await page.evaluate(() => ({
    userAgent: navigator.userAgent,
    viewport: [innerWidth, innerHeight],
    deviceScaleFactor: devicePixelRatio,
  }));
  const control = {
    browser: `Chrome/${browser.version()}`,
    host: `darwin ${release()}`,
    environment,
    probeSha256: sha256(source),
    collectorSha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
    errors,
    result,
  };
  if (environment.deviceScaleFactor !== 2) throw new Error(`${out}: environment ${JSON.stringify(environment)}`);
  writeFileSync(join(fixtures, out), `${JSON.stringify(control, null, 2)}\n`);
  console.log(`${out}: ${control.browser}, ${errors.length} errors`);
  await browser.close();
} finally {
  server.close();
}

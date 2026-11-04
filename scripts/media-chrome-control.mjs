// Chrome-156 control for a browser-env viewport probe (an expression, possibly a promise),
// run in the frozen headless shell at a 1024x768 viewport, device scale 1. Writes the
// control format viewport_cssom_tests.rs checks (browser, v8, viewport, probeSha256, errors,
// result).
// Usage: node scripts/media-chrome-control.mjs PROBE OUT [--chrome PATH]
// Needs playwright-core: PLAYWRIGHT_CORE (default: a sibling turbo-surf checkout's).
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values, positionals } = parseArgs({ allowPositionals: true, options: { chrome: { type: "string" } } });
const [probePath, out] = positionals;
const chromePath =
  values.chrome ||
  join(homedir(), ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell");
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core"));
const sha256 = (data) => createHash("sha256").update(data).digest("hex");
const probe = readFileSync(probePath, "utf8");
const flags = ["--use-mock-keychain", "--password-store=basic"];
const browser = await chromium.launch({ executablePath: chromePath, args: flags, headless: true });
const page = await browser.newPage({ viewport: { width: 1024, height: 768 }, deviceScaleFactor: 1 });
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
await page.goto("data:text/html,<!doctype html><html><head></head><body></body></html>");
const result = await page.evaluate(`Promise.resolve().then(()=>(${probe.trim().replace(/;$/, "")}))`);
const ua = await page.evaluate("navigator.userAgent");
const v8 = (await browser.newBrowserCDPSession().then((s) => s.send("Browser.getVersion"))).jsVersion;
await browser.close();
const control = {
  browser: /HeadlessChrome\/[\d.]+/.exec(ua)[0],
  v8,
  viewport: { width: 1024, height: 768 },
  probeSha256: sha256(probe),
  collectorSha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
  flags,
  errors,
  result,
};
writeFileSync(out, JSON.stringify(control, null, 2) + "\n");

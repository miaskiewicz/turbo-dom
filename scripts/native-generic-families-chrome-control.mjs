// Chrome-156 control for the macOS host-font policy of the CSS generic families
// (miaskiewicz/turbo-dom#259): which installed face `sans-serif`, `serif` and `monospace`
// resolve to, read as the `line-height: normal` line box height and the advance of
// "Hello world" at 16px, next to the physical families they may name. Host fonts by
// design (what is tested is the host lookup), so the fixture is only meaningful on macOS
// and is asserted by crates/turbo-dom-layout/tests/native_generic_families.rs under
// cfg(target_os = "macos").
//
// Usage: node scripts/native-generic-families-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156 headless shell;
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/native-generic/chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(homedir(), ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell");
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/native-generic/chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);

const TEXT = "Hello world";
const FAMILIES = ["sans-serif", "serif", "monospace", "Helvetica", "Helvetica Neue", "Arial", "Times", "Times New Roman", "Courier New", "Menlo"];
const css = (family) => (family.includes("-") || family === "serif" || family === "monospace" ? family : `"${family}"`);

const browser = await chromium.launch({ executablePath: chromePath, headless: true, args: ["--use-mock-keychain", "--password-store=basic"] });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
await page.setContent(
  `<!doctype html><body style="margin:0">${FAMILIES.map(
    (family) => `<div style='font-family:${css(family)};font-size:16px;line-height:normal'><span>${TEXT}</span></div>`,
  ).join("")}`,
);
const measured = await page.evaluate(() =>
  [...document.querySelectorAll("div")].map((div) => [div.getBoundingClientRect().height, div.firstChild.getBoundingClientRect().width]),
);
await browser.close();

const fixture = {
  generator: "scripts/native-generic-families-chrome-control.mjs",
  chrome: version,
  platform: process.platform,
  size: 16,
  text: TEXT,
  cases: FAMILIES.map((family, i) => ({ family, lineHeight: measured[i][0], width: measured[i][1] })),
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${FAMILIES.length} families -> ${out}`);

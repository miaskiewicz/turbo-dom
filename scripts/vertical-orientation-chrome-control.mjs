// Chrome-156 control for `text-orientation: mixed` per character (UAX #50
// Vertical_Orientation; R2 review #8): in a `writing-mode: vertical-rl` line of the
// repo's fixture Liberation Serif (loaded through @font-face), a character Chrome sets
// upright advances by the face's ascent + descent (it has no vmtx), one it turns
// sideways by its horizontal advance. Per case Chrome records the
// getBoundingClientRect() of the `[data-cid]` span ([x, y, w, h]); its height is that
// advance.
// Asserted by crates/turbo-dom-layout/tests/vertical_orientation_chrome.rs.
//
// Usage: node scripts/vertical-orientation-chrome-control.mjs [--chrome PATH] [--out PATH]
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { readFileSync, writeFileSync } from "node:fs";
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
const out = resolve(
  values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/vertical_orientation_chrome156.json"),
);
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const FONT = readFileSync(
  join(repo, "crates/turbo-dom-layout/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf"),
).toString("base64");
const FACE = `@font-face{font-family:"Liberation Serif";src:url(data:font/ttf;base64,${FONT})}`;

// Characters the face has a glyph for: Vo=U ones (©, ®, ±, ×, ÷, §, ■, ▲, ‰, №, ™) and
// Vo=R ones (a, -, «, €, ‐), each alone in a span.
// Then sequences: a variation selector or ZWJ takes the previous character's orientation
// (zero advance either way), the character after a ZWJ keeps its own: `a ZWJ ©` is a
// turned `a` then an upright `©`.
const CHARS = ["\u00a9", "\u00ae", "\u00b1", "\u00d7", "\u00f7", "\u00a7", "\u25a0", "\u25b2", "\u2030", "\u2116", "\u2122", "a", "-", "\u00ab", "\u20ac", "\u2010",
  "a\ufe0f", "a\u200d\u00a9", "\u00a9\ufe0e", "\u00a9\u200da"];
const spans = CHARS.map((c, i) => `<span data-cid="c${i}">${c}</span>`).join("<br>");
const HTML = `<!doctype html><style>${FACE}body{margin:8px;font-family:"Liberation Serif";font-size:16px;line-height:20px}</style><div style="writing-mode:vertical-rl;height:500px">${spans}</div>`;

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 });
await page.setContent(HTML, { waitUntil: "load" });
await page.evaluate(() => document.fonts.ready);
const boxes = await page.evaluate(() =>
  [...document.querySelectorAll("[data-cid]")].map((el) => {
    const r = el.getBoundingClientRect();
    return { cid: el.dataset.cid, text: el.textContent, rect: [r.x, r.y, r.width, r.height] };
  }),
);
await browser.close();
writeFileSync(
  out,
  `${JSON.stringify({ generator: "scripts/vertical-orientation-chrome-control.mjs", chrome: version, playwright_core: playwrightVersion, viewport: [800, 600], html: HTML, boxes }, null, 1)}\n`,
);
console.log(`Chrome ${version}: ${boxes.length} boxes -> ${out}`);
for (const b of boxes) console.log(b.text, b.rect.join(" "));

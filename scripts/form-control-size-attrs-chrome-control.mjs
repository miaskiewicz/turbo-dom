// Chrome-156 control for the form-control size attributes turbo-dom-layout reads
// (`textarea rows/cols`, `input size`, `select size`): a valid positive integer that does
// not fit a signed 32-bit int is the default, as in Chrome (R2 review #10).
//
// One page (viewport 1280x800, device scale factor 1) in the repo's fixture Liberation
// Serif (loaded through @font-face, so the Rust test lays out with
// `test_support::fixture_fonts()`); Chrome records getBoundingClientRect() of every
// `[data-cid]` element.
// Asserted by crates/turbo-dom-layout/tests/form_control_size_attrs_chrome.rs.
//
// Usage: node scripts/form-control-size-attrs-chrome-control.mjs [--chrome PATH] [--out PATH]
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
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
  );
const out = resolve(
  values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/form_control_size_attrs_chrome156.json"),
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

// The values: in range, i32::MAX + 1, beyond, u32::MAX, past u32, zero.
const VALUES = ["7", "2147483648", "3000000000", "4294967295", "4294967296", "0"];
// The selects go last: a huge list box saturates Chrome's layout (y at LayoutUnit max),
// which would corrupt every box after it. The controls use the fixture face (#263: the
// UA monospace was a host font, so the boxes were not comparable).
const rows = [];
for (const [i, v] of VALUES.entries()) {
  rows.push(`<div><textarea data-cid="rows-${i}" rows="${v}"></textarea></div>`);
  rows.push(`<div><textarea data-cid="cols-${i}" cols="${v}"></textarea></div>`);
  rows.push(`<div><input data-cid="size-${i}" size="${v}"></div>`);
}
for (const [i, v] of VALUES.entries()) {
  rows.push(`<div><select data-cid="select-${i}" size="${v}"><option>a</option><option>b</option></select></div>`);
}
const HTML = `<!doctype html><style>body{margin:8px;font-family:"Liberation Serif";font-size:16px}textarea,input,select{font-family:"Liberation Serif"}${FACE}</style>${rows.join("")}`;

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
await page.setContent(HTML, { waitUntil: "load" });
await page.evaluate(() => document.fonts.ready);
const boxes = await page.evaluate(() =>
  [...document.querySelectorAll("[data-cid]")].map((el) => {
    const r = el.getBoundingClientRect();
    return { cid: el.dataset.cid, x: r.x, y: r.y, width: r.width, height: r.height };
  }),
);
await browser.close();
const fixture = {
  generator: "scripts/form-control-size-attrs-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  values: VALUES,
  html: HTML,
  boxes,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${boxes.length} boxes -> ${out}`);

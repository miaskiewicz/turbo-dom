// Chrome-156 control for the default sizing of an SVG `<img>` (miaskiewicz/turbo-dom#320):
// an SVG image whose root has a `viewBox` but no `width`/`height` has a natural aspect
// ratio and no natural size, so CSS Images 3 default sizing makes an auto-sized `<img>` as
// wide as its containing block (Chrome 156; turbo-surf's parity page img-svg-viewbox-only).
//
// Every case is one `<img src="data:image/svg+xml,…">` (the `svg` document) with an inline
// `style`, alone in a wrapper `<div>` with its own `wrap` style, on a 1280x800 page at DSF 1.
// Chrome for Testing 156 records per case `natural` ([naturalWidth, naturalHeight]) and
// `size` (the img's getBoundingClientRect width and height).
// Asserted by crates/turbo-dom-layout/tests/svg_image_ratio_chrome.rs.
//
// Usage: node scripts/svg-image-ratio-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the frozen Chrome 156.0.8078.47 headless shell.
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { writeFileSync } from "node:fs";
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
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/svg_image_ratio_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);

const NS = 'xmlns="http://www.w3.org/2000/svg"';
const RECT = '<rect width="100%" height="100%" fill="teal"/>';
const svg = (attrs) => `<svg ${NS} ${attrs}>${RECT}</svg>`;
const VB = svg('viewBox="0 0 300 200"');
const VB73 = svg('viewBox="0 0 7 3"');
const WIDE = svg('viewBox="0 0 900 100"');
// [id, svg document, wrapper style, img style]
const CASES = [
  ["viewbox-only", VB, "width:400px", ""],
  ["viewbox-only-7x3", VB73, "width:400px", ""],
  ["viewbox-only-wide", WIDE, "width:400px", ""],
  ["viewbox-only-page", VB, "", ""],
  ["viewbox-only-block", VB, "width:400px", "display:block"],
  ["viewbox-only-block-frame", VB, "width:400px", "display:block;margin:0 10px;padding:4px;border:3px solid red"],
  ["viewbox-only-inline-frame", VB, "width:400px", "padding:4px;border:3px solid red"],
  ["viewbox-only-border-box", VB, "width:400px", "box-sizing:border-box;padding:4px;border:3px solid red"],
  ["viewbox-only-width", VB, "width:400px", "width:120px"],
  ["viewbox-only-width-pct", VB, "width:400px", "width:50%"],
  ["viewbox-only-height", VB, "width:400px", "height:60px"],
  ["viewbox-only-max-width", VB, "width:400px", "max-width:150px"],
  ["viewbox-only-max-height", VB, "width:400px", "max-height:90px"],
  ["viewbox-only-min-height", VB, "width:400px", "min-height:300px"],
  ["viewbox-only-float-wrap", VB, "float:left", ""],
  ["viewbox-only-inline-block-wrap", VB, "display:inline-block", ""],
  ["viewbox-only-abspos", VB, "position:relative;width:400px;height:0", "position:absolute"],
  ["viewbox-only-flex-item", VB, "display:flex;width:400px", ""],
  ["viewbox-only-table-cell", VB, "display:table", ""],
  ["width-only", svg('width="90" viewBox="0 0 300 200"'), "width:400px", ""],
  ["height-only", svg('height="40" viewBox="0 0 300 200"'), "width:400px", ""],
  ["both-lengths", svg('width="90" height="30" viewBox="0 0 300 200"'), "width:400px", ""],
  ["no-size-no-viewbox", svg(""), "width:400px", ""],
];

const page = (cases) =>
  `<!DOCTYPE html><html><head><style>body{margin:8px}div{margin:0 0 4px}</style></head><body>${cases
    .map(
      ([id, doc, wrap, style]) =>
        `<div style="${wrap}"><img data-cid="${id}" style="${style}" src="data:image/svg+xml,${encodeURIComponent(doc)}"></div>`,
    )
    .join("")}</body></html>`;
const html = page(CASES);

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const version = browser.version();
const tab = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
await tab.setContent(html, { waitUntil: "load" });
const measured = await tab.evaluate(() =>
  [...document.querySelectorAll("img")].map((img) => {
    const r = img.getBoundingClientRect();
    return { id: img.dataset.cid, natural: [img.naturalWidth, img.naturalHeight], size: [r.width, r.height] };
  }),
);
await browser.close();

const cases = CASES.map(([id, svgDoc, wrap, style]) => ({
  id,
  svg: svgDoc,
  wrap,
  style,
  ...measured.find((m) => m.id === id),
}));
const fixture = {
  generator: "scripts/svg-image-ratio-chrome-control.mjs",
  chrome: version,
  viewport: [1280, 800],
  deviceScaleFactor: 1,
  html,
  cases,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) console.log(c.id.padEnd(28), JSON.stringify(c.natural), JSON.stringify(c.size));

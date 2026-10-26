// Chrome-156 control for where Blink first reports an image's size (P5b-X7,
// miaskiewicz/turbo-dom#59): every byte prefix (0..=len) of each image in
// crates/turbo-dom-layout/tests/fixtures/images/truncation (scripts/make_truncation_fixtures.py)
// is loaded as a script-created `<img src="data:…;base64,…">`, plus `image/svg+xml` documents
// whose root element is or is not an SVG-namespace `<svg>`. Chrome for Testing 156 records per
// source: `event` (load / error, from the image's own events), `natural`
// ([naturalWidth, naturalHeight]) and `complete`. A prefix "loads" exactly when the decoder has
// read far enough to know the size (an all-data-received image whose decoder has a size is
// complete, partially painted); before that point the image is broken.
//
// The fixture stores, per image, `loads`: one character per prefix length ("L" load with the
// full natural size, "E" error, "?" anything else), so the asserting test
// (crates/turbo-dom-layout/tests/image_truncation_chrome.rs) can check the shared probe at
// every length.
//
// Usage: node scripts/image-truncation-chrome-control.mjs [--chrome PATH] [--out PATH]
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { gzipSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/image_truncation_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const DIR = join(repo, "crates/turbo-dom-layout/tests/fixtures/images/truncation");
const MIME = { jpg: "image/jpeg", gif: "image/gif", webp: "image/webp", png: "image/png" };
const files = readdirSync(DIR)
  .filter((f) => f !== "IMAGES.sha256")
  .sort();

const NS = 'xmlns="http://www.w3.org/2000/svg"';
// [id, document]: served as `data:image/svg+xml,<percent-encoded document>`.
const SVGS = [
  ["svg-namespace", `<svg ${NS} width="40" height="30"></svg>`],
  ["svg-no-namespace", `<svg width="40" height="30"></svg>`],
  ["svg-wrong-namespace", `<svg xmlns="http://example.com/svg" width="40" height="30"></svg>`],
  ["svg-prefixed", `<s:svg xmlns:s="http://www.w3.org/2000/svg" width="40" height="30"></s:svg>`],
  ["svg-prefix-unbound", `<s:svg width="40" height="30"></s:svg>`],
  ["svg-prolog", `<?xml version="1.0"?><!DOCTYPE svg><!-- c --><svg ${NS} width="40" height="30"></svg>`],
  ["svg-html-root", `<html xmlns="http://www.w3.org/1999/xhtml"><svg ${NS} width="40" height="30"></svg></html>`],
  ["svg-namespaced-child-only", `<svg width="40" height="30"><g ${NS}></g></svg>`],
  ["svg-malformed", `<svg ${NS} width="40" height="30"><g></svg>`],
  ["svg-uppercase", `<SVG ${NS} width="40" height="30"></SVG>`],
];

const sources = [];
for (const file of files) {
  const bytes = readFileSync(join(DIR, file));
  const mime = MIME[file.split(".").pop()];
  for (let n = 0; n <= bytes.length; n++)
    sources.push({ id: `${file}@${n}`, src: `data:${mime};base64,${bytes.subarray(0, n).toString("base64")}` });
}
for (const [id, doc] of SVGS) sources.push({ id, src: `data:image/svg+xml,${encodeURIComponent(doc)}` });
// A gzip-compressed (svgz) document: Blink decodes `image/svg+xml` bytes as XML only.
const SVGZ = gzipSync(Buffer.from(SVGS[0][1]));
sources.push({ id: "svgz", src: `data:image/svg+xml;base64,${SVGZ.toString("base64")}` });

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 });
await page.setContent("<!DOCTYPE html><body></body>", { waitUntil: "load" });
const measured = await page.evaluate(async (sources) => {
  const load = ({ id, src }) =>
    new Promise((done) => {
      const img = new Image();
      const finish = (event) =>
        done({ id, event, natural: [img.naturalWidth, img.naturalHeight], complete: img.complete });
      img.onload = () => finish("load");
      img.onerror = () => finish("error");
      img.src = src;
    });
  const out = [];
  for (let i = 0; i < sources.length; i += 64) out.push(...(await Promise.all(sources.slice(i, i + 64).map(load))));
  return out;
}, sources);
await browser.close();

const byId = new Map(measured.map((m) => [m.id, m]));
const images = files.map((file) => {
  const bytes = readFileSync(join(DIR, file));
  const full = byId.get(`${file}@${bytes.length}`);
  let loads = "";
  for (let n = 0; n <= bytes.length; n++) {
    const m = byId.get(`${file}@${n}`);
    const sameSize = m.natural[0] === full.natural[0] && m.natural[1] === full.natural[1];
    loads += m.event === "load" && m.complete && sameSize ? "L" : m.event === "error" && m.natural[0] === 0 ? "E" : "?";
  }
  return { file, length: bytes.length, natural: full.natural, loads };
});
const svgs = [...SVGS, ["svgz", `gzip(${SVGS[0][1]})`]].map(([id, doc]) => {
  const { event, natural, complete } = byId.get(id);
  return { id, document: doc, event, natural, complete };
});

const fixture = {
  generator: "scripts/image-truncation-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  images,
  svgs,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${sources.length} sources -> ${out}`);
for (const i of images) console.log(i.file.padEnd(18), JSON.stringify(i.natural), i.loads);
for (const s of svgs) console.log(s.id.padEnd(28), s.event, JSON.stringify(s.natural));

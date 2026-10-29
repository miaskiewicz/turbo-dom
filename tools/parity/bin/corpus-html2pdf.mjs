// Render the pinned html2pdf corpus through the napi addon and record outputs.
// Usage: node corpus-html2pdf.mjs <addon.node> <out-dir>
// Inputs are always read from the released-main checkout so every step renders
// identical sources; outputs + a sha256 manifest are written to <out-dir>.
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, existsSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

const [addonPath, outDir] = process.argv.slice(2);
// Pinned upstream inputs fetched by fetch-inputs.sh (git archive at pinned commits).
const CACHE = process.env.PARITY_CACHE || new URL("../.cache", import.meta.url).pathname;
const require = createRequire(import.meta.url);
const pdf = require(addonPath);
const MAIN = join(CACHE, "turbo-html2pdf/");
const FIXTURES = join(MAIN, "crates/turbo-html2pdf-core/tests/fixtures");
const CONFORMANCE = join(MAIN, "benches/conformance/fixtures");
const IMAGES = new URL("../inputs/image-matrix/pages/", import.meta.url).pathname;

mkdirSync(outDir, { recursive: true });
const manifest = {};
const sha = (buf) => createHash("sha256").update(buf).digest("hex");

function record(name, run) {
  let out;
  try {
    out = run();
  } catch (err) {
    out = Buffer.from(`ERROR ${err.code ?? ""} ${err.message}`);
  }
  const bytes = Buffer.isBuffer(out) ? out : Buffer.from(String(out));
  writeFileSync(join(outDir, name), bytes);
  manifest[name] = sha(bytes);
}

const variants = {
  plain: {},
  ua: { pdfUa: true, lang: "en-US" },
  cmyk: { cmyk: true },
  pdfa: { pdfA: true },
};

for (const dir of readdirSync(FIXTURES).sort()) {
  const template = join(FIXTURES, dir, "template.html");
  if (!existsSync(template)) continue;
  const html = readFileSync(template, "utf8");
  const cssPath = join(FIXTURES, dir, "style.css");
  const dataPath = join(FIXTURES, dir, "data.json");
  const css = existsSync(cssPath) ? readFileSync(cssPath, "utf8") : undefined;
  const data = existsSync(dataPath) ? JSON.parse(readFileSync(dataPath, "utf8")) : null;
  for (const [variant, extra] of Object.entries(variants)) {
    record(`template-${dir}-${variant}.pdf`, () => pdf.render(html, { data, css, now: 0, ...extra }).pdf);
  }
}

for (const file of readdirSync(CONFORMANCE).filter((f) => f.endsWith(".html")).sort()) {
  const html = readFileSync(join(CONFORMANCE, file), "utf8");
  const base = file.replace(/\.html$/, "");
  record(`conformance-${base}.boxes.json`, () => pdf.layoutBoxes(html, "", 1280, 800));
  record(`conformance-${base}.pdf`, () => pdf.render(html, { now: 0 }).pdf);
}

for (const file of readdirSync(IMAGES).filter((f) => f.endsWith(".html")).sort()) {
  const html = readFileSync(join(IMAGES, file), "utf8");
  const base = file.replace(/\.html$/, "");
  record(`image-${base}.boxes.json`, () => pdf.layoutBoxes(html, "", 1280, 800));
  record(`image-${base}.pdf`, () => pdf.render(html, { now: 0 }).pdf);
}

writeFileSync(join(outDir, "MANIFEST.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`${Object.keys(manifest).length} outputs -> ${outDir}`);

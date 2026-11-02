// Extra html2pdf corpus (P0 round-10 additions): xref documents, PDF/UA renders of the
// conformance + image pages, and broken/odd images. Kept separate from
// corpus-html2pdf.mjs so earlier comparisons stay stable.
// Usage: node corpus-html2pdf-extra.mjs <addon.node> <out-dir>
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

const [addonPath, outDir] = process.argv.slice(2);
// Pinned upstream inputs fetched by fetch-inputs.sh (git archive at pinned commits).
const CACHE = process.env.PARITY_CACHE || new URL("../.cache", import.meta.url).pathname;
const require = createRequire(import.meta.url);
const pdf = require(addonPath);
const MAIN = join(CACHE, "turbo-html2pdf/");
const CONFORMANCE = join(MAIN, "benches/conformance/fixtures");
const EXTRA = new URL("../inputs/extra/", import.meta.url).pathname;
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
const ua = { pdfUa: true, lang: "en-US", now: 0 };

for (const file of readdirSync(EXTRA).filter((f) => f.endsWith(".html")).sort()) {
  const html = readFileSync(join(EXTRA, file), "utf8");
  const base = file.replace(/\.html$/, "");
  record(`extra-${base}.pdf`, () => pdf.render(html, { now: 0 }).pdf);
  record(`extra-${base}.ua.pdf`, () => pdf.render(html, ua).pdf);
  record(`extra-${base}.boxes.json`, () => pdf.layoutBoxes(html, "", 1280, 800));
}
for (const [dir, prefix] of [[CONFORMANCE, "conformance"], [IMAGES, "image"]]) {
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".html")).sort()) {
    const html = readFileSync(join(dir, file), "utf8");
    record(`${prefix}-${file.replace(/\.html$/, "")}.ua.pdf`, () => pdf.render(html, ua).pdf);
  }
}
writeFileSync(join(outDir, "MANIFEST.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`${Object.keys(manifest).length} outputs -> ${outDir}`);

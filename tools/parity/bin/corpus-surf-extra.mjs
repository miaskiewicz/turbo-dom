// Extra surf static corpus: screenshots of inputs/extra (broken/odd images, xref pages).
// Usage: node corpus-surf-extra.mjs <addon.node> <out-dir>
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
const [addonPath, outDir] = process.argv.slice(2);
const surf = createRequire(import.meta.url)(addonPath);
const EXTRA = new URL("../inputs/extra/", import.meta.url).pathname;
mkdirSync(outDir, { recursive: true });
const manifest = {};
for (const file of readdirSync(EXTRA).filter((f) => f.endsWith(".html")).sort()) {
  const html = readFileSync(join(EXTRA, file), "utf8");
  for (const [ext, run] of [["png", () => surf.screenshot(html, 1280, 800)], ["svg", () => surf.screenshotSvg(html, 1280, 800)]]) {
    let out;
    try { out = run(); } catch (err) { out = `ERROR ${err.message}`; }
    const bytes = Buffer.isBuffer(out) ? out : Buffer.from(String(out));
    const name = `static-extra-${file.replace(/\.html$/, "")}.${ext}`;
    writeFileSync(join(outDir, name), bytes);
    manifest[name] = createHash("sha256").update(bytes).digest("hex");
  }
}
writeFileSync(join(outDir, "MANIFEST.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`${Object.keys(manifest).length} outputs -> ${outDir}`);

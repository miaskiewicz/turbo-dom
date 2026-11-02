// Render the pinned turbo-surf corpus through the napi addon and record outputs.
// Usage: node corpus-surf.mjs <addon.node> <out-dir>
// static: screenshot / screenshot_svg (HTML snapshot -> layout -> paint).
// live:   render(html, base, script) where the script dumps per-element geometry,
//         computed style and hit-testing into the hydrated HTML, so the capture,
//         scoped matcher and live-DOM styling of the live layout path are exercised.
// Inputs come from the released-main checkouts so every step renders identical
// sources.
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, relative } from "node:path";

const [addonPath, outDir] = process.argv.slice(2);
// Pinned upstream inputs fetched by fetch-inputs.sh (git archive at pinned commits).
const CACHE = process.env.PARITY_CACHE || new URL("../.cache", import.meta.url).pathname;
const require = createRequire(import.meta.url);
const surf = require(addonPath);
const MAIN = CACHE + "/";
const BASE = "https://corpus.test/";

function htmlFiles(dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    if (name === "node_modules" || name === "target" || name.startsWith(".")) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...htmlFiles(path));
    else if (name.endsWith(".html")) out.push(path);
  }
  return out;
}

const inputs = [
  ...htmlFiles(join(MAIN, "turbo-surf/harness")),
  ...htmlFiles(join(MAIN, "turbo-surf/rust/crates/turbo-surf-render")),
  ...htmlFiles(join(MAIN, "turbo-html2pdf/benches/conformance/fixtures")),
  ...htmlFiles(new URL("../inputs/image-matrix/pages/", import.meta.url).pathname),
];

const PROPS = ["display", "position", "color", "background-color", "font-size",
  "font-weight", "font-family", "margin-top", "padding-left", "width", "visibility"];
const DUMP = `(() => {
  const props = ${JSON.stringify(PROPS)};
  const rows = [];
  const attempt = (f) => { try { return f(); } catch (e) { return "THROW " + e.name + ": " + e.message; } };
  for (const el of document.querySelectorAll("*")) {
    const r = attempt(() => { const b = el.getBoundingClientRect(); return [b.x, b.y, b.width, b.height]; });
    const style = attempt(() => { const cs = getComputedStyle(el); return props.map((p) => cs.getPropertyValue(p)); });
    const hit = attempt(() => {
      if (!Array.isArray(r)) return null;
      const h = document.elementFromPoint(r[0] + r[2] / 2, r[1] + r[3] / 2);
      return h ? h.tagName + (h.id ? "#" + h.id : "") : null;
    });
    rows.push([el.tagName, r, style, hit]);
  }
  const out = document.createElement("script");
  out.type = "application/json";
  out.id = "__corpus";
  out.textContent = JSON.stringify(rows);
  document.documentElement.append(out);
})();`;

mkdirSync(outDir, { recursive: true });
const manifest = {};
const sha = (buf) => createHash("sha256").update(buf).digest("hex");
function record(name, run) {
  let out;
  try {
    out = run();
  } catch (err) {
    out = `ERROR ${err.message}`;
  }
  const bytes = Buffer.isBuffer(out) ? out : Buffer.from(String(out));
  writeFileSync(join(outDir, name), bytes);
  manifest[name] = sha(bytes);
}

for (const file of inputs) {
  const html = readFileSync(file, "utf8");
  const key = (file.startsWith(MAIN) ? relative(MAIN, file) : "image-matrix/" + file.split("/").pop()).replaceAll("/", "__").replace(/\.html$/, "");
  record(`static-${key}.png`, () => surf.screenshot(html, 1280, 800));
  record(`static-${key}.svg`, () => surf.screenshotSvg(html, 1280, 800));
  record(`live-${key}.html`, () => {
    const hydrated = surf.render(html, BASE, DUMP);
    const m = hydrated.match(/<script type="application\/json" id="__corpus">([\s\S]*?)<\/script>/);
    return m ? m[1] : `NO-DUMP ${hydrated.length}`;
  });
}

writeFileSync(join(outDir, "MANIFEST.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`${Object.keys(manifest).length} outputs from ${inputs.length} inputs -> ${outDir}`);

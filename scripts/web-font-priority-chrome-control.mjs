// Chrome-156 control for @font-face faces against installed faces of the same family name
// (turbo-dom#304). A page declares one @font-face face, Go-Regular.ttf (a fixture font, no
// installed copy), under the family name of an installed macOS font ("Helvetica", which has
// Regular and Bold faces) and again under a private name ("WebGo"). Per text and requested
// weight (400, 700): the HTML Range width of the text under "Helvetica" and under "WebGo",
// plus, on a second page without the @font-face rule, the width under the installed
// "Helvetica" (so the control shows the two faces differ). Chrome uses the @font-face face
// for the family whenever one is declared: the "Helvetica" widths equal the "WebGo" ones
// and differ from the installed ones, at 700 too although only an installed face is bold.
// Asserted by crates/turbo-dom-layout/tests/web_font_priority_chrome.rs.
//
// Usage: node scripts/web-font-priority-chrome-control.mjs [--chrome PATH]
//   --chrome defaults to the frozen Chrome 156.0.8078.47 headless shell (mock keychain).
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(homedir(), ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell");
const playwrightCore = process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const go = readFileSync(join(repo, "crates/turbo-dom-layout/assets/fonts/Go-Regular.ttf"));
const src = `url(data:font/sfnt;base64,${go.toString("base64")})`;
const WITH_FACE = `@font-face{font-family:"Helvetica";src:${src}}@font-face{font-family:"WebGo";src:${src}}`;
const TEXTS = ["Hamburgefonstiv", "Wikipedia 1234"];
const SIZE = 32;

const server = createServer((req, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  const css = req.url === "/web" ? WITH_FACE : "";
  res.end(`<!doctype html><html><head><style>${css}body{margin:0}</style></head><body></body></html>`);
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const measure = async (page, families) =>
  page.evaluate(
    async ({ families, texts, size }) => {
      for (const f of families) await document.fonts.load(`${size}px "${f}"`);
      const out = [];
      for (const family of families)
        for (const weight of [400, 700])
          for (const text of texts) {
            const div = document.createElement("div");
            div.setAttribute("style", `font-family:"${family}";font-size:${size}px;font-weight:${weight};width:2000px`);
            div.textContent = text;
            document.body.append(div);
            const r = document.createRange();
            r.selectNodeContents(div);
            out.push({ family, weight, text, width: r.getBoundingClientRect().width });
            div.remove();
          }
      return out;
    },
    { families, texts: TEXTS, size: SIZE },
  );
const errors = [];
let version;
let web;
let installed;
try {
  const browser = await chromium.launch({
    executablePath: chromePath,
    headless: true,
    args: ["--use-mock-keychain", "--password-store=basic", "--force-device-scale-factor=2"],
  });
  version = browser.version();
  const page = await browser.newPage();
  page.on("pageerror", (e) => errors.push(String(e)));
  const base = `http://127.0.0.1:${server.address().port}`;
  await page.goto(`${base}/web`);
  web = await measure(page, ["Helvetica", "WebGo"]);
  await page.goto(`${base}/installed`);
  installed = await measure(page, ["Helvetica"]);
  await browser.close();
} finally {
  server.close();
}
const control = {
  browser: `Chrome/${version}`,
  collectorSha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
  fontSha256: sha256(go),
  size: SIZE,
  errors,
  web,
  installed,
};
const out = join(repo, "crates/turbo-dom-layout/tests/fixtures/web-font-priority/chrome156.json");
writeFileSync(out, `${JSON.stringify(control, null, 1)}\n`);
console.log(out);
for (const r of [...web, ...installed.map((r) => ({ ...r, family: `installed ${r.family}` }))])
  console.log(r.family, r.weight, r.text, r.width);

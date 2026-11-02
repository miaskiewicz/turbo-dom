// Chrome 156 control for XML documents (turbo-dom#186): runs src/xml_documents_probe.js
// (DOMParser XML types, the parsed tree, <parsererror> documents, XMLSerializer) in the
// frozen headless Chrome and writes src/xml_documents_chrome156.json.
//
// Usage: PLAYWRIGHT_CORE=/path/to/playwright-core node scripts/xml-documents-chrome-control.mjs \
//          --chrome /path/to/chrome-headless-shell [--probe xml_inner_html] [--out PATH]
// --probe names src/<probe>_probe.js (default xml_documents) and its <probe>_chrome156.json.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" }, probe: { type: "string" } } });
if (!values.chrome) throw Error("Pass --chrome /absolute/chrome-headless-shell (Chrome 156.0.8078.47)");
const name = values.probe || "xml_documents";
const out = resolve(values.out || join(root, `src/${name}_chrome156.json`));
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_CORE || "playwright-core");
const probe = readFileSync(join(root, `src/${name}_probe.js`), "utf8");
const server = createServer((_request, response) => {
  response.setHeader("Content-Type", "text/html");
  response.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const browser = await chromium.launch({
  executablePath: resolve(values.chrome),
  headless: true,
  // Never touch the macOS keychain.
  args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const result = await page.evaluate(probe);
  const oracle = {
    browser: `Chrome/${browser.version()}`,
    recordedAt: new Date().toISOString(),
    fixture: `${name}_probe.js`,
    probeSha256: createHash("sha256").update(probe).digest("hex"),
    result,
  };
  writeFileSync(out, `${JSON.stringify(oracle, null, 2)}\n`);
  console.log(JSON.stringify({ browser: oracle.browser, out }));
} finally {
  await browser.close();
  await new Promise((done) => server.close(done));
}

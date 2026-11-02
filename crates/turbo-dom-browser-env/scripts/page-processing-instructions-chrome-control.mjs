// Chrome 156 control for HTML processing instructions in a LOADED page (turbo-dom#135): serve
// each page over http, let the frozen Chrome parse it as the main document, and record its
// node tree (document children, body children) and body serialization. Asserted by
// edge_probes_tests.rs `page_processing_instructions_match_chrome156`, which installs the
// same pages through install_with_html and both source-position installers.
//
// Usage: node crates/turbo-dom-browser-env/scripts/page-processing-instructions-chrome-control.mjs \
//          --chrome /path/to/frozen/Google\ Chrome\ for\ Testing [--out PATH]
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
if (!values.chrome) throw Error("Pass --chrome /path/to/frozen/chrome [--out control.json]");
const out = resolve(values.out || join(here, "../src/page_processing_instructions_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const { chromium } = createRequire(import.meta.url)(playwrightCore);

export const PAGES = [
  "<?pi x?><!doctype html><p><?q y?></p>",
  "<!doctype html><?pi x?><html><?a b?><head><?c?></head><body><?pi data?>t<?xml v?><?</body></html>",
  "<!doctype html><body><svg><?s z?></svg><table><?t u?></table><?a.b c?>",
  "<!doctype html><p>end<?pi cut",
];
export const DESCRIBE = `(() => {
  const describe = node => {
    switch (node.nodeType) {
      case 1: return [node.nodeName, Array.from(node.childNodes, describe)];
      case 3: return ['#text', node.data];
      case 7: return ['#pi', node.target, node.data];
      case 8: return ['#comment', node.data];
      case 10: return ['#doctype', node.name];
      default: return [node.nodeName];
    }
  };
  return [Array.from(document.childNodes, describe), document.body.innerHTML];
})()`;

const server = createServer((request, response) => {
  response.setHeader("Content-Type", "text/html; charset=utf-8");
  response.end(PAGES[Number(new URL(request.url, "http://x").searchParams.get("i"))]);
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const browser = await chromium.launch({
  executablePath: values.chrome,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const page = await browser.newPage();
const results = {};
for (let i = 0; i < PAGES.length; i++) {
  await page.goto(`http://127.0.0.1:${server.address().port}/?i=${i}`);
  results[PAGES[i]] = await page.evaluate(DESCRIBE);
}
const version = await browser.version();
await browser.close();
server.close();
writeFileSync(out, JSON.stringify({ browser: `Chrome/${version}`, describe: DESCRIBE, results }, null, 2) + "\n");
console.log(`wrote ${out}`);

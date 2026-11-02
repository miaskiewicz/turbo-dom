// Chrome 156 control for XPath contexts inside fragments / shadow roots and the namespace
// axis (turbo-dom#185). Runs src/xpath_contexts_probe.js in a local page of headless Chrome
// (`--dump-dom`, no network) and writes src/xpath_contexts_chrome156.json, asserted by
// src/xpath_contexts_tests.rs.
//
// Usage: node scripts/xpath-contexts-chrome-control.mjs --chrome /path/to/chrome-headless-shell [--out PATH]
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
if (!values.chrome) throw Error("Pass --chrome /path/to/chrome-headless-shell (Chrome 156)");
const out = resolve(values.out ?? join(root, "src/xpath_contexts_chrome156.json"));
const probePath = join(root, "src/xpath_contexts_probe.js");
const probe = readFileSync(probePath, "utf8");
const directory = mkdtempSync(join(tmpdir(), "turbo-xpath-contexts-"));
try {
  const page = join(directory, "page.html");
  writeFileSync(
    page,
    `<!doctype html><html><head></head><body><script>
const result = ${probe.replace(/^\/\/.*\n/gm, "")};
const pre = document.createElement("pre"); pre.id = "out";
pre.textContent = JSON.stringify({ result, version: navigator.userAgent });
document.documentElement.append(pre);
</script></body></html>`,
  );
  const dom = execFileSync(resolve(values.chrome), ["--headless", "--dump-dom", `file://${page}`], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  const text = dom.match(/<pre id="out">([\s\S]*?)<\/pre>/)[1];
  const decoded = text.replace(/&quot;/g, '"').replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
  const { result, version } = JSON.parse(decoded);
  const browser = version.match(/Chrome\/([\d.]+)/)[1];
  if (!browser.startsWith("156.")) throw Error(`expected Chrome 156, got ${browser}`);
  const oracle = {
    browser,
    fixture: "xpath_contexts_probe.js",
    probeSha256: createHash("sha256").update(probe).digest("hex"),
    result,
  };
  writeFileSync(out, `${JSON.stringify(oracle, null, 2)}\n`);
  console.log(JSON.stringify({ browser, out }));
} finally {
  rmSync(directory, { recursive: true, force: true });
}

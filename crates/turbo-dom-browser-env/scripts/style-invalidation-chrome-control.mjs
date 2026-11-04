// Chrome 156 control for subtree-scoped style invalidation (turbo-dom#279): runs
// src/style_invalidation_probe.js in a fresh page of the frozen chrome-headless-shell and
// writes its result to src/style_invalidation_chrome156.json (asserted by
// src/style_invalidation_tests.rs).
//
// Usage: node scripts/style-invalidation-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to ~/.cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chrome =
  values.chrome ||
  join(homedir(), ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell");
const out = resolve(values.out || join(root, "src/style_invalidation_chrome156.json"));
const probe = readFileSync(join(root, "src/style_invalidation_probe.js"), "utf8");
const directory = mkdtempSync(join(tmpdir(), "turbo-style-invalidation-"));
try {
  const page = join(directory, "probe.html");
  // The result replaces the document once the probe has run; --dump-dom prints it.
  writeFileSync(
    page,
    `<!DOCTYPE html><html><head></head><body><script>
const result = eval(${JSON.stringify(probe)});
document.documentElement.innerHTML = "<pre id=out></pre>";
document.getElementById("out").textContent = JSON.stringify(result);
</script></body></html>`,
  );
  const args = ["--headless", "--use-mock-keychain", "--password-store=basic", "--disable-gpu",
    "--no-first-run", `--user-data-dir=${join(directory, "profile")}`, "--dump-dom", `file://${page}`];
  const dom = execFileSync(chrome, args, { encoding: "utf8", maxBuffer: 1 << 26 });
  const version = execFileSync(chrome, ["--version"], { encoding: "utf8" }).trim();
  const text = /<pre id="out">([\s\S]*)<\/pre>/.exec(dom)[1]
    .replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&amp;/g, "&");
  writeFileSync(out, JSON.stringify({ browser: version, result: JSON.parse(text) }, null, 1) + "\n");
  console.log(`wrote ${out}`);
} finally {
  rmSync(directory, { recursive: true, force: true });
}

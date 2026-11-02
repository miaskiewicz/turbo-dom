// Capture the authored CSSOM fixture using the independently installed Chrome.
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
// root is crates/turbo-dom-browser-env; the probe and its Chrome control live in src/.
const envSrc = join(root, "src");
const { values } = parseArgs({
  options: {
    runner: { type: "string" },
    out: { type: "string" },
    chrome: { type: "string" },
  },
});
// The control is written next to its probe unless --out names another file.
values.out ??= join(root, "src/css_live_chrome155.json");
if (!values.runner)
  throw Error(
    "Pass --runner /absolute/chrome-cdp-trace.mjs [--out /absolute/oracle.json, default src/css_live_chrome155.json]",
  );
const directory = mkdtempSync(join(tmpdir(), "turbo-css-oracle-"));
const server = createServer((_request, response) => {
  response.setHeader("Content-Type", "text/html");
  response.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
let complete = false;
try {
  const config = join(directory, "config.json");
  writeFileSync(
    config,
    JSON.stringify({
      url: `http://127.0.0.1:${server.address().port}/`,
      waitMs: 25,
      disableExtensions: true,
      steps: [],
      probes: [join(envSrc, "css_live_probe.js")],
      ...(values.chrome ? { chromePath: resolve(values.chrome) } : {}),
    }),
  );
  const output = join(directory, "capture");
  const child = spawn(
    process.execPath,
    [resolve(values.runner), config, output],
    { stdio: "inherit" },
  );
  const status = await new Promise((done) => child.once("exit", done));
  if (status !== 0) throw Error(`Chrome capture exited ${status}`);
  const trace = JSON.parse(readFileSync(join(output, "trace.json"), "utf8"));
  if (trace.errors.length || trace.probes[0]?.error || !trace.probes[0]?.result)
    throw Error("Chrome control failed; inspect preserved diagnostics");
  const oracle = {
    browser: trace.browserVersion,
    v8: trace.browserV8Version,
    recordedAt: new Date().toISOString(),
    fixture: "css_live_probe.js",
    effectiveExtensionFlags: trace.effectiveExtensionFlags,
    result: trace.probes[0].result,
  };
  writeFileSync(resolve(values.out), `${JSON.stringify(oracle, null, 2)}\n`);
  console.log(
    JSON.stringify({ browser: oracle.browser, out: resolve(values.out) }),
  );
  complete = true;
} finally {
  await new Promise((done) => server.close(done));
  if (complete) rmSync(directory, { recursive: true, force: true });
  else console.error(`Failed capture preserved at ${directory}`);
}

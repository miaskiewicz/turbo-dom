// Capture a CSS probe fixture with independently installed Chrome.
//   node scripts/css-probe-chrome-control.mjs --runner /abs/chrome-cdp-trace.mjs \
//     --probe $ENV_SRC/css_selector_admission_probe.js --out $ENV_SRC/css_selector_admission_chrome156.json
// $ENV_SRC is turbo-dom-browser-env's source directory, where the binding's probes,
// cases and Chrome controls live (miaskiewicz/turbo-dom crates/turbo-dom-browser-env/src,
// e.g. ../turbo-dom/crates/turbo-dom-browser-env/src from this checkout).
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({
  options: {
    runner: { type: "string" },
    probe: { type: "string" },
    out: { type: "string" },
    chrome: { type: "string" },
  },
});
if (!values.runner || !values.probe || !values.out)
  throw Error(
    "Pass --runner /absolute/chrome-cdp-trace.mjs --probe $ENV_SRC/x_probe.js --out /absolute/oracle.json",
  );
const probe = resolve(root, values.probe);
const directory = mkdtempSync(join(tmpdir(), "turbo-css-oracle-"));
// The offline tests install this exact document before evaluating the probe.
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
      probes: [probe],
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
    fixture: basename(probe),
    probeSha256: createHash("sha256").update(readFileSync(probe)).digest("hex"),
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

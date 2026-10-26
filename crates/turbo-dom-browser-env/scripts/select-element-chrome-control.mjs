// Capture an authored probe in an independently installed Chrome over a raw CDP pipe.
// Usage: node scripts/select-element-chrome-control.mjs --probe $ENV_SRC/x_probe.js --cases $ENV_SRC/x_cases.json
//          --out $ENV_SRC/x_chrome156.json [--chrome /path/to/Chrome]
// $ENV_SRC is turbo-dom-browser-env's source directory, where the binding's probes,
// cases and Chrome controls live (miaskiewicz/turbo-dom crates/turbo-dom-browser-env/src,
// e.g. ../turbo-dom/crates/turbo-dom-browser-env/src from this checkout).
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    probe: { type: "string" },
    cases: { type: "string" },
    out: { type: "string" },
    chrome: { type: "string" },
  },
});
if (!values.probe || !values.cases || !values.out)
  throw Error("Pass --probe PROBE.js --cases CASES.json --out ORACLE.json");
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const probe = readFileSync(resolve(values.probe), "utf8");
const casesBytes = readFileSync(resolve(values.cases));
const cases = JSON.parse(casesBytes);
const executable =
  values.chrome || "/Applications/User Apps/Google Chrome.app/Contents/MacOS/Google Chrome";
const profile = mkdtempSync(join(tmpdir(), "turbo-select-oracle-"));
const flags = [
  "--headless=new",
  "--remote-debugging-pipe",
  "--user-data-dir=<fresh-profile>",
  "--no-first-run",
  "--no-default-browser-check",
  "--disable-extensions",
  "--disable-component-extensions-with-background-pages",
  "about:blank",
];
let html = "";
const server = createServer((_request, response) => {
  response.setHeader("Content-Type", "text/html");
  response.end(html);
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const chrome = spawn(
  executable,
  flags.map((flag) => (flag === "--user-data-dir=<fresh-profile>" ? `--user-data-dir=${profile}` : flag)),
  { stdio: ["ignore", "ignore", "ignore", "pipe", "pipe"] },
);
const pending = new Map();
let nextId = 1,
  buffer = Buffer.alloc(0);
chrome.stdio[4].on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  for (let end = buffer.indexOf(0); end >= 0; end = buffer.indexOf(0)) {
    const message = JSON.parse(buffer.subarray(0, end).toString("utf8"));
    buffer = buffer.subarray(end + 1);
    const waiter = message.id && pending.get(message.id);
    if (!waiter) continue;
    pending.delete(message.id);
    if (message.error) waiter.reject(Error(JSON.stringify(message.error)));
    else waiter.resolve(message.result);
  }
});
const call = (method, params = {}, sessionId) =>
  new Promise((done, fail) => {
    const id = nextId++;
    pending.set(id, { resolve: done, reject: fail });
    chrome.stdio[3].write(`${JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) })}\0`);
  });
const pause = (ms) => new Promise((done) => setTimeout(done, ms));
const errors = [],
  result = [];
let version;
try {
  version = await call("Browser.getVersion");
  for (const entry of cases) {
    html = entry.html;
    const { targetId } = await call("Target.createTarget", { url: "about:blank" });
    const { sessionId } = await call("Target.attachToTarget", { targetId, flatten: true });
    await call("Page.enable", {}, sessionId);
    await call("Page.navigate", { url: `http://127.0.0.1:${server.address().port}/` }, sessionId);
    for (let tries = 0; tries < 200; tries++) {
      const state = await call(
        "Runtime.evaluate",
        { expression: "document.readyState", returnByValue: true },
        sessionId,
      );
      if (state.result.value === "complete") break;
      await pause(10);
    }
    const evaluated = await call(
      "Runtime.evaluate",
      { expression: probe, returnByValue: true, awaitPromise: true },
      sessionId,
    );
    if (evaluated.exceptionDetails) errors.push({ id: entry.id, error: evaluated.exceptionDetails });
    result.push({ id: entry.id, result: evaluated.result.value });
    await call("Target.closeTarget", { targetId });
  }
} finally {
  const exited = new Promise((done) => chrome.once("exit", done));
  chrome.kill();
  await exited;
  await new Promise((done) => server.close(done));
  rmSync(profile, { recursive: true, force: true, maxRetries: 10 });
}
const oracle = {
  browser: version.product,
  v8: version.jsVersion,
  casesSha256: sha(casesBytes),
  probeSha256: sha(probe),
  collectorSha256: sha(readFileSync(fileURLToPath(import.meta.url))),
  flags,
  errors,
  result,
};
writeFileSync(resolve(values.out), `${JSON.stringify(oracle, null, 2)}\n`);
console.log(JSON.stringify({ browser: oracle.browser, errors: errors.length, out: resolve(values.out) }));

// Capture the grouping / condition rule CSSOM control (turbo-dom#254) in the frozen Chrome 156
// headless shell over a raw CDP pipe.
//   node scripts/css-grouping-rules-chrome-control.mjs --chrome /path/to/chrome-headless-shell \
//     [--probe src/css_grouping_rules_probe.js] [--out src/css_grouping_rules_chrome156.json]
// The grouping-rule cascade control (turbo-dom#273) uses the same collector:
//     --probe src/css_grouping_cascade_probe.js --out src/css_grouping_cascade_chrome156.json
// Run from crates/turbo-dom-browser-env. The offline test installs the same empty document
// before evaluating the probe and checks `browser` and `probeSha256`.
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    probe: { type: "string", default: "src/css_grouping_rules_probe.js" },
    out: { type: "string", default: "src/css_grouping_rules_chrome156.json" },
    chrome: { type: "string" },
  },
});
if (!values.chrome) throw Error("Pass --chrome /path/to/chrome-headless-shell (Chrome 156.0.8078.47)");
const probeBytes = readFileSync(resolve(values.probe));
const probe = probeBytes.toString("utf8");
const profile = mkdtempSync(join(tmpdir(), "turbo-grouping-rules-oracle-"));
const flags = [
  "--headless",
  "--remote-debugging-pipe",
  `--user-data-dir=${profile}`,
  "--no-first-run",
  "--no-default-browser-check",
  "--use-mock-keychain",
  "--password-store=basic",
  "--disable-extensions",
  "--disable-component-extensions-with-background-pages",
  "about:blank",
];
const server = createServer((_request, response) => {
  response.setHeader("Content-Type", "text/html");
  response.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const chrome = spawn(resolve(values.chrome), flags, {
  stdio: ["ignore", "ignore", "ignore", "pipe", "pipe"],
});
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
    chrome.stdio[3].write(
      `${JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) })}\0`,
    );
  });
const pause = (ms) => new Promise((done) => setTimeout(done, ms));
let version, evaluated;
try {
  version = await call("Browser.getVersion");
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
  evaluated = await call("Runtime.evaluate", { expression: probe, returnByValue: true }, sessionId);
} finally {
  const exited = new Promise((done) => chrome.once("exit", done));
  chrome.kill();
  await exited;
  await new Promise((done) => server.close(done));
  rmSync(profile, { recursive: true, force: true, maxRetries: 10 });
}
if (evaluated.exceptionDetails) throw Error(JSON.stringify(evaluated.exceptionDetails));
const oracle = {
  browser: version.product.replace("HeadlessChrome", "Chrome"),
  v8: version.jsVersion,
  fixture: basename(values.probe),
  probeSha256: createHash("sha256").update(probeBytes).digest("hex"),
  flags: flags.map((flag) => (flag.startsWith("--user-data-dir=") ? "--user-data-dir=<fresh-profile>" : flag)),
  result: evaluated.result.value,
};
writeFileSync(resolve(values.out), `${JSON.stringify(oracle, null, 2)}\n`);
console.log(JSON.stringify({ browser: oracle.browser, out: resolve(values.out) }));

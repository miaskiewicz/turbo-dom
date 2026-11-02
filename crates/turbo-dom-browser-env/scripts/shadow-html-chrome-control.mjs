// Capture a shadow-root/HTML-serialization probe in independently installed headless Chrome.
// Usage: node scripts/shadow-html-chrome-control.mjs --probe src/x_probe.js --out src/x_chrome156.json [--chrome PATH] [--page PAGE.html]
// --page serves that document (recorded with its SHA-256) instead of the empty default page.
// Run from crates/turbo-dom-browser-env: src/ holds the binding's probes, cases and Chrome
// controls (the files its tests and `test_support` read).
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    probe: { type: "string" },
    out: { type: "string" },
    chrome: { type: "string" },
    page: { type: "string" },
  },
});
if (!values.probe || !values.out) throw Error("Pass --probe PROBE.js --out ORACLE.json");
const executable = resolve(
  values.chrome ||
    process.env.TURBO_CHROME_PATH ||
    "/Applications/User Apps/Google Chrome.app/Contents/MacOS/Google Chrome",
);
if (!existsSync(executable)) throw Error(`Chrome not found at ${executable}`);
const source = readFileSync(resolve(values.probe), "utf8");
const page = values.page
  ? readFileSync(resolve(values.page), "utf8")
  : "<!doctype html><html><head></head><body></body></html>";
const server = createServer((_request, response) => {
  response.setHeader("Content-Type", "text/html");
  response.end(page);
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const profile = mkdtempSync(join(tmpdir(), "turbo-shadow-html-oracle-"));
const chrome = spawn(
  executable,
  [
    "--headless=new",
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    "about:blank",
  ],
  { stdio: ["ignore", "ignore", "pipe"] },
);
try {
  const endpoint = await new Promise((done, fail) => {
    let text = "";
    chrome.stderr.on("data", (chunk) => {
      text += chunk;
      const match = /DevTools listening on (ws:\/\/\S+)/.exec(text);
      if (match) done(match[1]);
    });
    chrome.once("exit", (code) => fail(Error(`Chrome exited ${code}`)));
  });
  const browser = new WebSocket(endpoint);
  await new Promise((done) => browser.addEventListener("open", done, { once: true }));
  let next = 0;
  const pending = new Map();
  browser.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { done, fail } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) fail(Error(JSON.stringify(message.error)));
      else done(message.result);
    }
  });
  const call = (method, params = {}, sessionId) =>
    new Promise((done, fail) => {
      const id = ++next;
      pending.set(id, { done, fail });
      browser.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
    });
  const version = await call("Browser.getVersion");
  const { targetId } = await call("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await call("Target.attachToTarget", { targetId, flatten: true });
  await call("Page.enable", {}, sessionId);
  await call("Page.navigate", { url: `http://127.0.0.1:${server.address().port}/` }, sessionId);
  for (let attempt = 0; attempt < 100; attempt++) {
    const state = await call(
      "Runtime.evaluate",
      { expression: "document.readyState", returnByValue: true },
      sessionId,
    );
    if (state.result.value === "complete") break;
    await new Promise((done) => setTimeout(done, 25));
  }
  const evaluated = await call(
    "Runtime.evaluate",
    {
      expression: `(async () => JSON.stringify(await (${source})))()`,
      returnByValue: true,
      awaitPromise: true,
    },
    sessionId,
  );
  if (evaluated.exceptionDetails)
    throw Error(evaluated.exceptionDetails.exception?.description || evaluated.exceptionDetails.text);
  const oracle = {
    browser: version.product,
    v8: version.jsVersion,
    recordedAt: new Date().toISOString(),
    fixture: basename(values.probe),
    probeSha256: createHash("sha256").update(source).digest("hex"),
    ...(values.page
      ? { page: basename(values.page), pageSha256: createHash("sha256").update(page).digest("hex") }
      : {}),
    flags: ["--headless=new", "--disable-extensions"],
    result: JSON.parse(evaluated.result.value),
  };
  writeFileSync(resolve(values.out), `${JSON.stringify(oracle, null, 2)}\n`);
  console.log(JSON.stringify({ browser: oracle.browser, out: resolve(values.out) }));
  browser.close();
} finally {
  const exited = new Promise((done) => chrome.once("exit", done));
  chrome.kill();
  await exited;
  await new Promise((done) => server.close(done));
  rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}

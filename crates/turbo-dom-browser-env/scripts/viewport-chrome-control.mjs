// Collect a viewport / CSSOM probe control (turbo-dom#285-#288) in the frozen Chrome
// headless shell at a fixed 1024x768 viewport (device scale factor 1):
//   node scripts/viewport-chrome-control.mjs src/x_probe.js src/x_chrome156.json
// Run from crates/turbo-dom-browser-env with TURBO_SURF_CHROME_PATH naming the Chrome 156
// binary. The page is the binding's frozen shell
// `<!doctype html><html><head></head><body></body></html>` served from loopback. A probe
// resizes the viewport through the `__resize` binding: it calls
// `__resize(JSON.stringify({ id, width, height }))`, the collector applies the size with
// `Emulation.setDeviceMetricsOverride` and calls `__resizeDone(id)` once the page has
// seen it (offline tests set innerWidth / innerHeight and dispatch `resize` instead).
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { cdpConnection } from "./trusted-types-control/cdp-connection.mjs";

const [probeArgument, outputArgument] = process.argv.slice(2);
if (!probeArgument || !outputArgument)
  throw new Error("Usage: node viewport-chrome-control.mjs probe.js output.json");
const probe = readFileSync(resolve(probeArgument), "utf8"),
  output = resolve(outputArgument);
const VIEWPORT = { width: 1024, height: 768 };
const profile = mkdtempSync(join(tmpdir(), "turbo-viewport-"));
const server = createServer((_request, response) => {
  response.writeHead(200, { "Content-Type": "text/html", "Cache-Control": "no-store" });
  response.end("<!doctype html><html><head></head><body></body></html>");
});
let chrome, socket, connection;
const pause = (ms) => new Promise((done) => setTimeout(done, ms));
const kill = (signal) => {
  if (!chrome?.pid || chrome.exitCode !== null || chrome.signalCode !== null) return;
  try {
    process.kill(-chrome.pid, signal);
  } catch (error) {
    if (error.code !== "ESRCH") throw error;
  }
};
try {
  await new Promise((done, fail) => {
    server.once("error", fail);
    server.listen(0, "127.0.0.1", done);
  });
  const url = `http://127.0.0.1:${server.address().port}/`;
  const flags = [
    "--remote-debugging-port=0",
    "--use-mock-keychain",
    "--password-store=basic",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    "--disable-component-extensions-with-background-pages",
    "about:blank",
  ];
  const executable = process.env.TURBO_SURF_CHROME_PATH;
  if (!executable) throw new Error("Set TURBO_SURF_CHROME_PATH to the Chrome 156 binary");
  chrome = spawn(executable, flags, { detached: true, stdio: ["ignore", "ignore", "pipe"] });
  let stderr = "",
    spawnError;
  chrome.once("error", (error) => {
    spawnError = error;
  });
  chrome.stderr.on("data", (chunk) => {
    if (stderr.length < 1048576) stderr += String(chunk);
  });
  const deadline = Date.now() + 30000;
  let endpoint;
  while (!(endpoint = /DevTools listening on (ws:\/\/[^\s]+)/.exec(stderr)?.[1])) {
    if (spawnError) throw spawnError;
    if (chrome.exitCode !== null || Date.now() > deadline) throw new Error("Chrome startup failed");
    await pause(50);
  }
  socket = new WebSocket(endpoint);
  await new Promise((done, fail) => {
    socket.onopen = done;
    socket.onerror = fail;
  });
  const errors = [];
  const resizes = [];
  connection = cdpConnection(
    socket,
    (event) => {
      if (event.method === "Runtime.exceptionThrown") errors.push(event.params.exceptionDetails);
      if (event.method === "Runtime.bindingCalled" && event.params.name === "__resize")
        resizes.push(event.params.payload);
    },
    30000,
  );
  const version = await connection.call("Browser.getVersion");
  const { targetId } = await connection.call("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await connection.call("Target.attachToTarget", { targetId, flatten: true });
  await connection.call("Runtime.enable", {}, sessionId);
  await connection.call("Page.enable", {}, sessionId);
  const metrics = (size) =>
    connection.call(
      "Emulation.setDeviceMetricsOverride",
      { ...size, deviceScaleFactor: 1, mobile: false, screenWidth: 1920, screenHeight: 1080 },
      sessionId,
    );
  await metrics(VIEWPORT);
  await connection.call("Runtime.addBinding", { name: "__resize" }, sessionId);
  const evaluate = async (expression, awaitPromise = true) => {
    const reply = await connection.call(
      "Runtime.evaluate",
      { expression, returnByValue: true, awaitPromise },
      sessionId,
    );
    if (reply.exceptionDetails)
      throw new Error(reply.exceptionDetails.exception?.description || reply.exceptionDetails.text);
    return reply.result.value;
  };
  await connection.call("Page.navigate", { url }, sessionId);
  const loaded = Date.now() + 10000;
  while (await evaluate(`document.readyState !== 'complete' || location.href !== ${JSON.stringify(url)}`)) {
    if (Date.now() > loaded) throw Error("The shell did not load");
    await pause(25);
  }
  await evaluate(
    `globalThis.__probeResult = Promise.resolve().then(() => (${probe.trim().replace(/;$/, "")})); 0`,
    false,
  );
  let result;
  for (const end = Date.now() + 20000; ; ) {
    while (resizes.length) {
      const { id, width, height } = JSON.parse(resizes.shift());
      await metrics({ width, height });
      // Two frames: Chrome evaluates media queries and fires `resize` in the frame update.
      await evaluate("new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)))");
      await evaluate(`__resizeDone(${id})`, false);
    }
    const state = await evaluate(
      "Promise.race([__probeResult.then(value => ({ done: true, value })), new Promise(done => setTimeout(() => done({ done: false }), 25))])",
    );
    if (state.done) {
      result = state.value;
      break;
    }
    if (Date.now() > end) throw new Error("The probe did not settle");
  }
  const control = {
    browser: version.product,
    v8: version.jsVersion,
    viewport: VIEWPORT,
    probeSha256: createHash("sha256").update(probe).digest("hex"),
    collectorSha256: createHash("sha256").update(readFileSync(new URL(import.meta.url))).digest("hex"),
    flags: flags.map((flag) => (flag.startsWith("--user-data-dir=") ? "--user-data-dir=<fresh-profile>" : flag)),
    errors,
    result,
  };
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, `${JSON.stringify(control, null, 2)}\n`);
  if (errors.length) throw new Error(`Chrome control reported ${errors.length} exception(s)`);
} finally {
  connection?.close();
  socket?.close();
  kill("SIGTERM");
  if (chrome && chrome.exitCode === null && chrome.signalCode === null)
    await Promise.race([new Promise((done) => chrome.once("exit", done)), pause(1000)]);
  if (chrome?.exitCode === null && chrome.signalCode === null) kill("SIGKILL");
  server.closeAllConnections();
  if (server.listening) await new Promise((done) => server.close(done));
  rmSync(profile, { recursive: true, force: true });
}

// Collect the source-pinned legacy Document control in a fresh, normal installed Chrome.
// Every case is served from loopback (plain HTTP, or HTTPS with a throwaway
// self-signed certificate); `host` names reach it through a host-resolver rule so
// effective-domain and secure-context cases need no network or public DNS.
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createServer as createTlsServer } from "node:https";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { cdpConnection } from "./trusted-types-control/cdp-connection.mjs";

const [casesArgument, probeArgument, outputArgument] = process.argv.slice(2);
if (!casesArgument || !probeArgument || !outputArgument)
  throw new Error("Usage: TZ=UTC node document-legacy-chrome-control.mjs cases.json probe.js output.json");
if (process.env.TZ !== "UTC") throw new Error("Run with TZ=UTC so lastModified is comparable");
const probe = readFileSync(resolve(probeArgument), "utf8"),
  output = resolve(outputArgument);
const casesText = readFileSync(resolve(casesArgument), "utf8"),
  cases = JSON.parse(casesText);
const profile = mkdtempSync(join(tmpdir(), "turbo-document-legacy-"));
const keys = mkdtempSync(join(tmpdir(), "turbo-document-legacy-tls-"));
const certificate = spawnSync("openssl", ["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1", "-subj", "/CN=localhost", "-keyout", join(keys, "key.pem"), "-out", join(keys, "cert.pem")], { stdio: "ignore" });
if (certificate.status !== 0) throw new Error("openssl could not create the loopback certificate");
const serve = (request, response) => {
  const id = Number(new URL(request.url, "http://localhost").searchParams.get("case") || 0);
  const item = cases[id];
  if (!item) {
    response.writeHead(404);
    response.end();
    return;
  }
  response.writeHead(
    200,
    [["Content-Type", "text/html"], ["Cache-Control", "no-store"], ...(item.headers || [])].flat(),
  );
  response.end(item.html);
};
const server = createServer(serve);
const tlsServer = createTlsServer({ key: readFileSync(join(keys, "key.pem")), cert: readFileSync(join(keys, "cert.pem")) }, serve);
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
  await new Promise((done, fail) => {
    tlsServer.once("error", fail);
    tlsServer.listen(0, "127.0.0.1", done);
  });
  const port = server.address().port,
    tlsPort = tlsServer.address().port;
  const flags = [
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    "--disable-component-extensions-with-background-pages",
    "--host-resolver-rules=MAP * 127.0.0.1",
    "--ignore-certificate-errors",
    "about:blank",
  ];
  chrome = spawn(
    process.env.TURBO_SURF_CHROME_PATH ||
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    flags,
    { detached: true, stdio: ["ignore", "ignore", "pipe"] },
  );
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
  connection = cdpConnection(
    socket,
    (event) => {
      if (event.method === "Runtime.exceptionThrown") errors.push(event.params.exceptionDetails);
    },
    30000,
  );
  const version = await connection.call("Browser.getVersion");
  const result = [];
  for (let index = 0; index < cases.length; index++) {
    const item = cases[index];
    const url =
      item.url ||
      `${item.https ? "https" : "http"}://${item.host || "127.0.0.1"}:${item.https ? tlsPort : port}/?case=${index}`;
    // A fresh top-level target per case keeps every case a new browsing context.
    const { targetId } = await connection.call("Target.createTarget", { url: "about:blank" });
    const { sessionId } = await connection.call("Target.attachToTarget", {
      targetId,
      flatten: true,
    });
    await connection.call("Runtime.enable", {}, sessionId);
    await connection.call("Page.enable", {}, sessionId);
    const evaluate = async (expression) => {
      const reply = await connection.call(
        "Runtime.evaluate",
        { expression, returnByValue: true, awaitPromise: true },
        sessionId,
      );
      if (reply.exceptionDetails)
        throw new Error(reply.exceptionDetails.exception?.description || reply.exceptionDetails.text);
      return reply.result.value;
    };
    if (url !== "about:blank") await connection.call("Page.navigate", { url }, sessionId);
    const loaded = Date.now() + 10000;
    while (
      await evaluate(
        `document.readyState !== 'complete' || location.href !== ${JSON.stringify(url)}`,
      )
    ) {
      if (Date.now() > loaded) throw Error(`Case ${item.id} did not load`);
      await pause(25);
    }
    result.push({ id: item.id, result: await evaluate(probe) });
    await connection.call("Target.closeTarget", { targetId });
  }
  const control = {
    browser: version.product,
    v8: version.jsVersion,
    timeZone: process.env.TZ,
    casesSha256: createHash("sha256").update(casesText).digest("hex"),
    probeSha256: createHash("sha256").update(probe).digest("hex"),
    collectorSha256: createHash("sha256")
      .update(readFileSync(new URL(import.meta.url)))
      .digest("hex"),
    flags: flags.map((flag) =>
      flag.startsWith("--user-data-dir=") ? "--user-data-dir=<fresh-profile>" : flag,
    ),
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
  for (const listener of [server, tlsServer]) {
    listener.closeAllConnections();
    if (listener.listening) await new Promise((done) => listener.close(done));
  }
  rmSync(profile, { recursive: true, force: true });
  rmSync(keys, { recursive: true, force: true });
}

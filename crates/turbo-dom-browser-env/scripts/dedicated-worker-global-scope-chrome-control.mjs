// Capture the DedicatedWorkerGlobalScope members probe
// (src/dedicated_worker_global_scope_probe.js) in independently installed headless
// Chrome. The probe runs inside a real dedicated Worker spawned from a Blob URL on a
// localhost page; this script attaches via CDP, builds the Worker from the probe
// source, forwards its first posted message, and writes the result (with the probe
// SHA-256) as crates/turbo-dom-browser-env/src/dedicated_worker_global_scope_chrome156.json.
// turbo-dom#385.
import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const { values } = parseArgs({
	options: {
		probe: { type: "string" },
		out: { type: "string" },
		chrome: { type: "string" },
	},
});
if (!values.probe || !values.chrome)
	throw Error(
		"Pass --chrome /path/to/chrome --probe committed-probe.js [--out control.json]",
	);
const hash = (path) =>
	createHash("sha256").update(readFileSync(path)).digest("hex");
const probe = resolve(values.probe),
	collector = fileURLToPath(import.meta.url);
const probeSource = readFileSync(probe, "utf8");
const page = `<!doctype html><html><body><script>
const src = ${JSON.stringify(probeSource)};
const blob = new Blob([src], { type: "application/javascript" });
const url = URL.createObjectURL(blob);
const worker = new Worker(url);
worker.onmessage = (event) => {
  window.__probe = event.data;
  document.title = "done";
};
worker.onerror = (error) => {
  window.__probe = { error: String(error && error.message || error) };
  document.title = "done";
};
</script></body></html>`;
const profile = mkdtempSync(join(tmpdir(), "dworker-chrome-"));
const server = createServer((_request, response) => {
	response.setHeader("Content-Type", "text/html");
	response.end(page);
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const flags = [
	"--headless=new",
	"--use-mock-keychain",
	"--password-store=basic",
	"--remote-debugging-pipe",
	`--user-data-dir=${profile}`,
	"--no-first-run",
	"--no-default-browser-check",
	"--disable-extensions",
	"--disable-component-extensions-with-background-pages",
	"about:blank",
];
const chrome = spawn(resolve(values.chrome), flags, {
	stdio: ["ignore", "ignore", "ignore", "pipe", "pipe"],
});
const [, , , input, output] = chrome.stdio;
let buffer = "",
	sequence = 0;
const pending = new Map();
output.on("data", (chunk) => {
	buffer += chunk;
	let end;
	while ((end = buffer.indexOf("\0")) >= 0) {
		const message = JSON.parse(buffer.slice(0, end));
		buffer = buffer.slice(end + 1);
		const waiter = message.id && pending.get(message.id);
		if (!waiter) continue;
		pending.delete(message.id);
		if (message.error) waiter.reject(Error(JSON.stringify(message.error)));
		else waiter.resolve(message.result);
	}
});
const call = (method, params = {}, sessionId) =>
	new Promise((resolveCall, reject) => {
		const id = ++sequence;
		pending.set(id, { resolve: resolveCall, reject });
		input.write(`${JSON.stringify({ id, method, params, sessionId })}\0`);
	});
let probeResult;
let chromeVersion;
try {
	const version = await call("Browser.getVersion");
	chromeVersion = version.product;
	const { targetId } = await call("Target.createTarget", {
		url: `http://127.0.0.1:${server.address().port}/`,
	});
	const { sessionId } = await call("Target.attachToTarget", {
		targetId,
		flatten: true,
	});
	await call("Runtime.enable", {}, sessionId);
	// Poll document.title until the worker's message was forwarded.
	const started = Date.now();
	while (Date.now() - started < 10_000) {
		const { result } = await call(
			"Runtime.evaluate",
			{ expression: "document.title", returnByValue: true },
			sessionId,
		);
		if (result.value === "done") break;
		await new Promise((r) => setTimeout(r, 50));
	}
	const { result } = await call(
		"Runtime.evaluate",
		{
			expression: "JSON.stringify(window.__probe)",
			returnByValue: true,
		},
		sessionId,
	);
	probeResult = JSON.parse(result.value);
} finally {
	chrome.kill("SIGKILL");
	server.close();
	rmSync(profile, { recursive: true, force: true });
}
const outPath =
	values.out ||
	fileURLToPath(
		new URL(
			"../src/dedicated_worker_global_scope_chrome156.json",
			import.meta.url,
		),
	);
const record = {
	chrome: chromeVersion,
	probe: `sha256:${hash(probe)}`,
	collector: `sha256:${hash(collector)}`,
	// Freeze member lists so a diff on this file is a real change to Chrome's scope.
	result: probeResult,
};
writeFileSync(outPath, `${JSON.stringify(record, null, 2)}\n`);
console.log("wrote", outPath);

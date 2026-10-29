// Capture the committed view-transition probes in independently installed headless Chrome.
//   node scripts/view-transitions-chrome-control.mjs --chrome /path/to/chrome \
//     --probe src/view_transitions_probe.js --hidden src/view_transitions_hidden_probe.js \
//     --out src/view_transitions_chrome156.json
// Self-contained CDP over --remote-debugging-pipe: no Playwright and no turbo-surf runner.
// The main probe runs in a foreground tab (visible document, real vsync-driven frames); the
// hidden probe runs in a background tab, whose document Chrome reports as hidden.
// Run from crates/turbo-dom-browser-env: src/ holds the binding's probes, cases and Chrome
// controls (the files its tests and `test_support` read).
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
		hidden: { type: "string" },
		out: { type: "string" },
		chrome: { type: "string" },
	},
});
if (!values.probe || !values.hidden || !values.chrome)
	throw Error(
		"Pass --chrome /path/to/chrome --probe probe.js --hidden hidden_probe.js [--out control.json]",
	);
const hash = (path) =>
	createHash("sha256").update(readFileSync(path)).digest("hex");
const probe = resolve(values.probe),
	hidden = resolve(values.hidden),
	collector = fileURLToPath(import.meta.url);
const profile = mkdtempSync(join(tmpdir(), "view-transitions-chrome-"));
// The offline tests install this exact document before evaluating the probe.
const server = createServer((_request, response) => {
	response.setHeader("Content-Type", "text/html");
	response.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const flags = [
	"--headless=new",
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
const open = async (background) => {
	const { targetId } = await call("Target.createTarget", {
		url: `http://127.0.0.1:${server.address().port}/`,
		background,
	});
	const { sessionId } = await call("Target.attachToTarget", {
		targetId,
		flatten: true,
	});
	await call("Runtime.enable", {}, sessionId);
	for (let attempt = 0; attempt < 100; attempt++) {
		const ready = await call(
			"Runtime.evaluate",
			{ expression: "document.readyState", returnByValue: true },
			sessionId,
		);
		if (ready.result.value === "complete") break;
		await new Promise((done) => setTimeout(done, 20));
	}
	return sessionId;
};
const evaluate = async (sessionId, path) => {
	const evaluated = await call(
		"Runtime.evaluate",
		{
			expression: `Promise.resolve(${readFileSync(path, "utf8").trim().replace(/;$/, "")}).then(JSON.stringify)`,
			awaitPromise: true,
			returnByValue: true,
		},
		sessionId,
	);
	if (evaluated.exceptionDetails)
		throw Error(JSON.stringify(evaluated.exceptionDetails));
	return JSON.parse(evaluated.result.value);
};
try {
	const version = await call("Browser.getVersion");
	const foreground = await open(false);
	const result = await evaluate(foreground, probe);
	const background = await open(true);
	const hiddenResult = await evaluate(background, hidden);
	const control = {
		browser: version.product,
		v8: version.jsVersion,
		probe: probe.split("/").pop(),
		probeSha256: hash(probe),
		hiddenProbe: hidden.split("/").pop(),
		hiddenProbeSha256: hash(hidden),
		collectorSha256: hash(collector),
		flags: flags.map((flag) =>
			flag.startsWith("--user-data-dir=")
				? "--user-data-dir=<fresh-profile>"
				: flag,
		),
		result,
		hidden: hiddenResult,
	};
	const text = `${JSON.stringify(control, null, 2)}\n`;
	if (values.out) writeFileSync(resolve(values.out), text);
	else process.stdout.write(text);
	await call("Browser.close").catch(() => {});
} finally {
	const exited =
		chrome.exitCode === null && chrome.signalCode === null
			? new Promise((done) => chrome.once("exit", done))
			: null;
	chrome.kill();
	await exited;
	await new Promise((done) => server.close(done));
	rmSync(profile, { recursive: true, force: true, maxRetries: 10 });
}

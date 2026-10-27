// Capture a committed probe that needs TRUSTED input (hover, arrow keys) in independently
// installed headless Chrome. Self-contained CDP over --remote-debugging-pipe. The probe calls the
// `__cdpInput(JSON)` binding with {id, steps}; each step is {type:"move", x, y} (Input mouseMoved)
// {type:"down"|"up", x, y} (Input mousePressed / mouseReleased, left button) or
// {type:"key", key, code, keyCode, modifiers?} (rawKeyDown + keyUp), then the collector calls
// `__cdpInputDone(id)`. Offline harnesses implement the same binding over their own trusted driver.
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
const profile = mkdtempSync(join(tmpdir(), "trusted-input-chrome-"));
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
	// Interest delays are timer-driven: keep the page's timers unthrottled and unaligned.
	"--disable-background-timer-throttling",
	"--disable-renderer-backgrounding",
	"--disable-backgrounding-occluded-windows",
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
		if (message.method === "Runtime.bindingCalled") {
			if (message.params.name === "__cdpInput")
				queued = queued
					.then(() => trusted(message.params.payload, message.sessionId))
					.catch((error) => process.stderr.write(`trusted input failed: ${error.stack}\n`));
			continue;
		}
		const waiter = message.id && pending.get(message.id);
		if (!waiter) continue;
		pending.delete(message.id);
		if (message.error) waiter.reject(Error(JSON.stringify(message.error)));
		else waiter.resolve(message.result);
	}
});
let queued = Promise.resolve();
async function trusted(payload, sessionId) {
	const { id, steps } = JSON.parse(payload);
	for (const step of steps) {
		if (step.type === "move")
			await call("Input.dispatchMouseEvent", { type: "mouseMoved", x: step.x, y: step.y }, sessionId);
		else if (step.type === "down" || step.type === "up") {
			await call("Input.dispatchMouseEvent", { type: "mouseMoved", x: step.x, y: step.y }, sessionId);
			await call(
				"Input.dispatchMouseEvent",
				{ type: step.type === "down" ? "mousePressed" : "mouseReleased", x: step.x, y: step.y, button: "left", buttons: step.type === "down" ? 1 : 0, clickCount: 1 },
				sessionId,
			);
		} else if (step.type === "key")
			for (const type of ["rawKeyDown", "keyUp"])
				await call(
					"Input.dispatchKeyEvent",
					{ type, key: step.key, code: step.code, windowsVirtualKeyCode: step.keyCode, nativeVirtualKeyCode: step.keyCode, modifiers: step.modifiers || 0 },
					sessionId,
				);
		else throw Error(`Unsupported trusted step ${JSON.stringify(step)}`);
	}
	await call("Runtime.evaluate", { expression: `__cdpInputDone(${Number(id)})` }, sessionId);
}
const call = (method, params = {}, sessionId) =>
	new Promise((resolveCall, reject) => {
		const id = ++sequence;
		pending.set(id, { resolve: resolveCall, reject });
		input.write(`${JSON.stringify({ id, method, params, sessionId })}\0`);
	});
try {
	const version = await call("Browser.getVersion");
	const { targetId } = await call("Target.createTarget", {
		url: `http://127.0.0.1:${server.address().port}/`,
	});
	const { sessionId } = await call("Target.attachToTarget", {
		targetId,
		flatten: true,
	});
	await call("Runtime.enable", {}, sessionId);
	await call("Runtime.addBinding", { name: "__cdpInput" }, sessionId);
	await call("Page.bringToFront", {}, sessionId);
	await call("Emulation.setFocusEmulationEnabled", { enabled: true }, sessionId);
	for (let attempt = 0; attempt < 100; attempt++) {
		const ready = await call(
			"Runtime.evaluate",
			{ expression: "document.readyState", returnByValue: true },
			sessionId,
		);
		if (ready.result.value === "complete") break;
		await new Promise((done) => setTimeout(done, 20));
	}
	const evaluated = await call(
		"Runtime.evaluate",
		{
			expression: `Promise.resolve(${readFileSync(probe, "utf8").trim().replace(/;$/, "")}).then(JSON.stringify)`,
			awaitPromise: true,
			returnByValue: true,
		},
		sessionId,
	);
	if (evaluated.exceptionDetails)
		throw Error(JSON.stringify(evaluated.exceptionDetails));
	const control = {
		browser: version.product,
		v8: version.jsVersion,
		probe: probe.split("/").pop(),
		probeSha256: hash(probe),
		collectorSha256: hash(collector),
		flags: flags.map((flag) =>
			flag.startsWith("--user-data-dir=")
				? "--user-data-dir=<fresh-profile>"
				: flag,
		),
		result: JSON.parse(evaluated.result.value),
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

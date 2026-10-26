// Capture committed trusted-input steps in independently installed headless Chrome: each step either
// evaluates script or drives real input through CDP (Input.insertText for committed text,
// Input.dispatchKeyEvent for Backspace/Delete). Self-contained CDP over --remote-debugging-pipe.
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
		steps: { type: "string" },
		out: { type: "string" },
		chrome: { type: "string" },
	},
});
if (!values.steps || !values.chrome)
	throw Error(
		"Pass --chrome /path/to/chrome --steps committed-steps.json [--out control.json]",
	);
const hash = (path) =>
	createHash("sha256").update(readFileSync(path)).digest("hex");
const probe = resolve(values.steps),
	collector = fileURLToPath(import.meta.url);
const profile = mkdtempSync(join(tmpdir(), "editing-input-chrome-"));
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
	for (let attempt = 0; attempt < 100; attempt++) {
		const ready = await call(
			"Runtime.evaluate",
			{ expression: "document.readyState", returnByValue: true },
			sessionId,
		);
		if (ready.result.value === "complete") break;
		await new Promise((done) => setTimeout(done, 20));
	}
	const keys = {
		Backspace: { code: "Backspace", windowsVirtualKeyCode: 8 },
		Delete: { code: "Delete", windowsVirtualKeyCode: 46 },
	};
	const results = [];
	for (const step of JSON.parse(readFileSync(probe, "utf8"))) {
		if (step.eval !== undefined) {
			const value = await call(
				"Runtime.evaluate",
				{ expression: step.eval, awaitPromise: true, returnByValue: true },
				sessionId,
			);
			if (value.exceptionDetails)
				throw Error(JSON.stringify(value.exceptionDetails));
			results.push(value.result.value);
		} else if (step.insertText !== undefined)
			await call("Input.insertText", { text: step.insertText }, sessionId);
		else
			for (const type of ["rawKeyDown", "keyUp"])
				await call(
					"Input.dispatchKeyEvent",
					{
						type,
						key: step.key,
						...keys[step.key],
						nativeVirtualKeyCode: keys[step.key].windowsVirtualKeyCode,
					},
					sessionId,
				);
	}
	const control = {
		browser: version.product,
		v8: version.jsVersion,
		steps: probe.split("/").pop(),
		stepsSha256: hash(probe),
		collectorSha256: hash(collector),
		flags: flags.map((flag) =>
			flag.startsWith("--user-data-dir=")
				? "--user-data-dir=<fresh-profile>"
				: flag,
		),
		result: results,
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

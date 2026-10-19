// Capture this sibling probe in normal installed Chrome with a fresh profile.
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import {
	existsSync,
	mkdtempSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const directory = dirname(fileURLToPath(import.meta.url));
const output = resolve(process.argv[2] || join(directory, "chrome155.json"));
if (existsSync(output))
	throw new Error(
		"Output exists; use a fresh path to preserve capture history",
	);
const probePath = join(directory, "probe.js");
const bytes = readFileSync(probePath);
const ownBytes = readFileSync(fileURLToPath(import.meta.url));
const sha = (value) => createHash("sha256").update(value).digest("hex");
const profile = mkdtempSync(join(tmpdir(), "attribute-case-chrome-"));
const pause = (milliseconds) =>
	new Promise((done) => setTimeout(done, milliseconds));
const server = createServer((_request, response) => {
	response.setHeader("Content-Type", "text/html; charset=utf-8");
	response.end("<!doctype html><html><head></head><body></body></html>");
});
let chrome, socket;
const pending = new Map();
let serial = 0;
const errors = [];
const kill = (signal) => {
	if (!chrome?.pid || chrome.exitCode !== null || chrome.signalCode !== null)
		return;
	try {
		process.kill(-chrome.pid, signal);
	} catch (error) {
		if (error.code !== "ESRCH") throw error;
	}
};
const rejectPending = (error) => {
	for (const item of pending.values()) {
		clearTimeout(item.timer);
		item.reject(error);
	}
	pending.clear();
};
try {
	await new Promise((done, reject) => {
		server.once("error", reject);
		server.listen(0, "127.0.0.1", done);
	});
	const flags = [
		"--remote-debugging-port=0",
		`--user-data-dir=${profile}`,
		"--no-first-run",
		"--no-default-browser-check",
		"--disable-extensions",
		"--disable-component-extensions-with-background-pages",
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
		stderr = (stderr + chunk).slice(-1048576);
	});
	let endpoint;
	const startupDeadline = Date.now() + 30000;
	while (
		!(endpoint = /DevTools listening on (ws:\/\/[^\s]+)/.exec(stderr)?.[1])
	) {
		if (spawnError) throw spawnError;
		if (chrome.exitCode !== null || Date.now() > startupDeadline)
			throw new Error("Chrome startup failed");
		await pause(25);
	}
	socket = new WebSocket(endpoint);
	await new Promise((done, reject) => {
		const timer = setTimeout(
			() => reject(new Error("CDP socket open timed out")),
			10000,
		);
		const finish = (callback, value) => {
			clearTimeout(timer);
			socket.onopen = socket.onerror = socket.onclose = null;
			callback(value);
		};
		socket.onopen = () => finish(done);
		socket.onerror = () =>
			finish(reject, new Error("CDP socket failed before open"));
		socket.onclose = () =>
			finish(reject, new Error("CDP socket closed before open"));
	});
	socket.onclose = () => rejectPending(new Error("CDP socket closed"));
	socket.onerror = () => rejectPending(new Error("CDP socket failed"));
	socket.onmessage = ({ data }) => {
		const row = JSON.parse(data);
		if (row.method === "Runtime.exceptionThrown")
			errors.push(row.params.exceptionDetails);
		if (!row.id) return;
		const item = pending.get(row.id);
		if (!item) return;
		pending.delete(row.id);
		clearTimeout(item.timer);
		row.error
			? item.reject(new Error(row.error.message))
			: item.resolve(row.result);
	};
	const call = (method, params = {}, sessionId) =>
		new Promise((resolveCall, reject) => {
			const id = ++serial;
			const timer = setTimeout(() => {
				pending.delete(id);
				reject(new Error(`CDP timeout: ${method}`));
			}, 15000);
			pending.set(id, { resolve: resolveCall, reject, timer });
			try {
				socket.send(
					JSON.stringify({
						id,
						method,
						params,
						...(sessionId ? { sessionId } : {}),
					}),
				);
			} catch (error) {
				pending.delete(id);
				clearTimeout(timer);
				reject(error);
			}
		});
	const version = await call("Browser.getVersion");
	const { targetId } = await call("Target.createTarget", {
		url: "about:blank",
	});
	const { sessionId } = await call("Target.attachToTarget", {
		targetId,
		flatten: true,
	});
	await call("Page.enable", {}, sessionId);
	await call("Runtime.enable", {}, sessionId);
	await call(
		"Page.navigate",
		{ url: `http://127.0.0.1:${server.address().port}/` },
		sessionId,
	);
	const evaluate = async (expression) => {
		const row = await call(
			"Runtime.evaluate",
			{ expression, returnByValue: true, awaitPromise: true },
			sessionId,
		);
		if (row.exceptionDetails)
			throw new Error(
				row.exceptionDetails.exception?.description ||
					row.exceptionDetails.text,
			);
		return row.result.value;
	};
	const readyDeadline = Date.now() + 10000;
	while ((await evaluate("document.readyState")) !== "complete") {
		if (Date.now() > readyDeadline) throw new Error("Page readiness timed out");
		await pause(25);
	}
	const result = await evaluate(bytes.toString("utf8"));
	for (const row of result.cases) {
		if (row.selector.isWellFormed()) continue;
		row.selectorEscapedUtf16 = row.selectorUnits
			.map((unit) => `\\u${unit.toString(16).padStart(4, "0")}`)
			.join("");
		row.selector = null;
	}
	if (errors.length) throw new Error(`Uncaught page errors: ${errors.length}`);
	if (
		!bytes.equals(readFileSync(probePath)) ||
		!ownBytes.equals(readFileSync(fileURLToPath(import.meta.url)))
	)
		throw new Error("Collector or probe changed during capture");
	const control = {
		schemaVersion: 2,
		selectorEncoding:
			"selectorUnits are authoritative; ill-formed scalar display is null with selectorEscapedUtf16",
		browser: version.product,
		v8: version.jsVersion,
		node: process.version,
		mode: "normal-headed-fresh-profile-direct-cdp",
		probeSha256: sha(bytes),
		collectorSha256: sha(ownBytes),
		fixtureHtmlSha256: sha(Buffer.from(result.fixtureHtml)),
		flags: flags.map((flag) =>
			flag.startsWith("--user-data-dir=")
				? "--user-data-dir=<fresh-profile>"
				: flag,
		),
		errors,
		result,
	};
	writeFileSync(output, `${JSON.stringify(control, null, 2)}\n`, {
		flag: "wx",
	});
	console.log(
		JSON.stringify({
			browser: control.browser,
			cases: result.cases.length,
			xmlCases: result.xmlCases.length,
			errors: errors.length,
			probeSha256: control.probeSha256,
			collectorSha256: control.collectorSha256,
		}),
	);
} finally {
	rejectPending(new Error("Collector stopped"));
	socket?.close();
	kill("SIGTERM");
	if (chrome && chrome.exitCode === null && chrome.signalCode === null)
		await Promise.race([
			new Promise((done) => chrome.once("exit", done)),
			pause(1000),
		]);
	kill("SIGKILL");
	server.closeAllConnections();
	await new Promise((done) => server.close(done));
	rmSync(profile, { recursive: true, force: true });
}

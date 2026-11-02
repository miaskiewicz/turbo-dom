// Capture a committed DOM lifecycle probe in independently installed Chrome.
import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({
	options: {
		runner: { type: "string" },
		probe: { type: "string" },
		out: { type: "string" },
		chrome: { type: "string" },
	},
});
if (!values.runner || !values.probe || !values.out)
	throw Error(
		"Pass --runner chrome-cdp-trace.mjs --probe committed-probe.js --out control.json",
	);
const hash = (path) =>
	createHash("sha256").update(readFileSync(path)).digest("hex");
const probe = resolve(values.probe),
	runner = resolve(values.runner);
const directory = mkdtempSync(join(tmpdir(), "dom-lifecycle-chrome-"));
const server = createServer((_request, response) => {
	response.setHeader("Content-Type", "text/html");
	response.end("<!doctype html><html><head></head><body></body></html>");
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
let complete = false;
try {
	const config = join(directory, "config.json"),
		output = join(directory, "capture");
	writeFileSync(
		config,
		JSON.stringify({
			url: `http://127.0.0.1:${server.address().port}/`,
			waitMs: 25,
			disableExtensions: true,
			steps: [],
			probes: [probe],
			...(values.chrome ? { chromePath: resolve(values.chrome) } : {}),
		}),
	);
	const child = spawn(process.execPath, [runner, config, output], {
		stdio: "inherit",
	});
	const status = await new Promise((done, reject) => {
		child.once("error", reject);
		child.once("exit", done);
	});
	if (status !== 0) throw Error(`Chrome capture exited ${status}`);
	const trace = JSON.parse(readFileSync(join(output, "trace.json"), "utf8"));
	if (trace.errors.length || trace.probes[0]?.error || !trace.probes[0]?.result)
		throw Error("Chrome control failed; diagnostics preserved");
	const control = {
		browser: trace.browserVersion,
		v8: trace.browserV8Version,
		probeSha256: hash(probe),
		collectorSha256: hash(runner),
		errors: trace.errors,
		result: trace.probes[0].result,
	};
	writeFileSync(resolve(values.out), `${JSON.stringify(control, null, 2)}\n`);
	complete = true;
} finally {
	await new Promise((done) => server.close(done));
	if (complete) rmSync(directory, { recursive: true, force: true });
	else console.error(`Failed capture preserved at ${directory}`);
}

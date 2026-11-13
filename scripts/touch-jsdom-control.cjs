// Usage: JSDOM_PATH=/path/to/jsdom node scripts/touch-jsdom-control.cjs
const { createHash } = require("node:crypto");
const { readFileSync, writeFileSync } = require("node:fs");
const { resolve } = require("node:path");
const jsdomPath = process.env.JSDOM_PATH || "jsdom";
const { JSDOM } = require(jsdomPath);
const version = require(require.resolve(`${jsdomPath}/package.json`)).version;
if (!version.startsWith("27."))
	throw new Error(`Expected jsdom 27, got ${version}`);
const repo = resolve(__dirname, "..");
const probe = readFileSync(
	resolve(repo, "crates/turbo-dom-browser-env/scripts/touch-jsdom-probe.js"),
	"utf8",
);
const window = new JSDOM("<canvas></canvas>", { runScripts: "outside-only" })
	.window;
try {
	const control = {
		jsdom: version,
		probeSha256: createHash("sha256").update(probe).digest("hex"),
		result: window.eval(probe),
	};
	writeFileSync(
		resolve(repo, "crates/turbo-dom-browser-env/src/touch_jsdom_jsdom27.json"),
		JSON.stringify(control, null, 2) + "\n",
	);
} finally {
	window.close();
}

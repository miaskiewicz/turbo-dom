// JSDOM_PATH=/path/to/jsdom27 node scripts/text-decoration-jsdom-control.cjs
const { createHash } = require("node:crypto");
const { readFileSync, writeFileSync } = require("node:fs");
const { resolve } = require("node:path");
const modulePath = process.env.JSDOM_PATH || "jsdom";
const { JSDOM } = require(modulePath);
const version = require(require.resolve(`${modulePath}/package.json`)).version;
if (!version.startsWith("27."))
	throw new Error(`Expected jsdom 27, got ${version}`);
const repo = resolve(__dirname, "..");
const probe = readFileSync(
	resolve(
		repo,
		"crates/turbo-dom-browser-env/scripts/text-decoration-jsdom-probe.js",
	),
	"utf8",
);
const window = new JSDOM("<html><head></head><body></body></html>", {
	runScripts: "outside-only",
}).window;
try {
	const control = {
		jsdom: version,
		probeSha256: createHash("sha256").update(probe).digest("hex"),
		result: window.eval(probe),
	};
	writeFileSync(
		resolve(
			repo,
			"crates/turbo-dom-browser-env/src/text_decoration_jsdom27.json",
		),
		JSON.stringify(control, null, 2) + "\n",
	);
} finally {
	window.close();
}

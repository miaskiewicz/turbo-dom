import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL("../", import.meta.url));
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(root, "../turbo-surf/node_modules/playwright-core"),
);
const source = readFileSync(
	resolve(
		root,
		"crates/turbo-dom-browser-env/src/window_getter_receivers_probe.js",
	),
	"utf8",
);
const executablePath = process.env.CHROME_EXECUTABLE;
if (!executablePath)
	throw new Error(
		"Set CHROME_EXECUTABLE to the reviewed Chrome 158 executable",
	);
const deadline = setTimeout(() => {
	console.error("Window getter oracle exceeded 25 seconds");
	process.exit(2);
}, 25000);
let browser;
try {
	browser = await chromium.launch({
		executablePath,
		headless: true,
		timeout: 10000,
	});
	if (!browser.version().startsWith("158.")) {
		throw new Error(`Expected Chrome 158, got ${browser.version()}`);
	}
	const page = await browser.newPage();
	const result = await page.evaluate(async (source) => {
		const frame = document.createElement("iframe");
		frame.srcdoc = "<!doctype html><body>child";
		const loaded = new Promise((resolve) => (frame.onload = resolve));
		document.body.append(frame);
		await loaded;
		const probe = (0, eval)(source)(frame.contentWindow);
		const live = probe.live;
		frame.remove();
		return { live, removed: probe.removed() };
	}, source);
	const report = {
		version: browser.version(),
		executablePath,
		probeSha256: createHash("sha256").update(source).digest("hex"),
		result,
	};
	writeFileSync(
		resolve(
			root,
			"crates/turbo-dom-browser-env/src/window_getter_receivers_chrome158.json",
		),
		JSON.stringify(report, null, 2) + "\n",
	);
	console.log(
		JSON.stringify({
			version: report.version,
			cases: result.live.length + result.removed.length,
		}),
	);
} finally {
	clearTimeout(deadline);
	if (browser) await browser.close();
}

// Offline Chromium control for #518/#519, using the unchanged component reduction.
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
const executablePath = process.env.CHROME_EXECUTABLE;
if (!executablePath)
	throw Error("Set CHROME_EXECUTABLE to reviewed Chromium 159");
const casesSource = readFileSync(
	resolve(root, "crates/turbo-dom-layout/tests/fixtures/flex_calc_cases.json"),
	"utf8",
);
const hash = (source) => createHash("sha256").update(source).digest("hex");
const browser = await chromium.launch({
	executablePath,
	headless: true,
	timeout: 30000,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const version = browser.version();
	if (version !== "159.0.8154.6")
		throw Error(`Expected Chromium 159.0.8154.6, got ${version}`);
	const rows = [];
	const fixtureHashes = {};
	for (const input of JSON.parse(casesSource)) {
		const context = await browser.newContext({
			viewport: { width: input.width, height: 1000 },
			deviceScaleFactor: 1,
		});
		context.setDefaultTimeout(5000);
		await context.setOffline(true);
		const page = await context.newPage();
		const fixture = readFileSync(
			resolve(root, `tools/component-parity/fixtures/${input.fixture}.html`),
			"utf8",
		);
		fixtureHashes[input.fixture] = hash(fixture);
		await page.setContent(fixture);
		if (input.css) await page.addStyleTag({ content: input.css });
		const boxes = await page.evaluate(() =>
			[".grid", ".item", ".card", ".badge", ".body"].flatMap((selector) =>
				Array.from(document.querySelectorAll(selector), (element, index) => {
					const r = element.getBoundingClientRect();
					return {
						key: `${selector}:${index}`,
						rect: [r.x, r.y, r.width, r.height],
					};
				}),
			),
		);
		rows.push({ ...input, boxes });
		await context.close();
	}
	const report = {
		browser: version,
		viewportHeight: 1000,
		deviceScale: 1,
		offline: true,
		fixtureHashes,
		casesSha256: hash(casesSource),
		collectorSha256: hash(readFileSync(fileURLToPath(import.meta.url))),
		rows,
	};
	writeFileSync(
		resolve(
			root,
			"crates/turbo-dom-layout/tests/fixtures/flex_calc_chrome159.json",
		),
		JSON.stringify(report, null, 2) + "\n",
	);
	console.log(JSON.stringify({ version, cases: rows.length }));
} finally {
	await browser.close();
}

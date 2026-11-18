// Offline, text-free Chromium control for signed flex column outer contributions (#526).
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
	resolve(
		root,
		"crates/turbo-dom-layout/tests/fixtures/flex_column_margin_cases.json",
	),
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
	for (const input of JSON.parse(casesSource)) {
		const context = await browser.newContext({
			viewport: { width: input.width, height: 1000 },
			deviceScaleFactor: 1,
		});
		context.setDefaultTimeout(5000);
		await context.setOffline(true);
		const page = await context.newPage();
		await page.setContent(input.html);
		if (input.css) await page.addStyleTag({ content: input.css });
		const boxes = await page.evaluate(() =>
			["[data-cid]"].flatMap((selector) =>
				Array.from(document.querySelectorAll(selector), (element) => {
					const r = element.getBoundingClientRect();
					return {
						key: element.getAttribute("data-cid"),
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
		casesSha256: hash(casesSource),
		collectorSha256: hash(readFileSync(fileURLToPath(import.meta.url))),
		rows,
	};
	writeFileSync(
		resolve(
			root,
			"crates/turbo-dom-layout/tests/fixtures/flex_column_margins_chrome159.json",
		),
		JSON.stringify(report, null, 2) + "\n",
	);
	console.log(JSON.stringify({ version, cases: rows.length }));
} finally {
	await browser.close();
}

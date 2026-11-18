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
		"crates/turbo-dom-layout/tests/fixtures/percentage_frame_cases.json",
	),
	"utf8",
);
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const browser = await chromium.launch({
	executablePath,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const version = browser.version();
	if (version !== "159.0.8154.6")
		throw Error(`Expected Chromium 159.0.8154.6, got ${version}`);
	const context = await browser.newContext({
		viewport: { width: 600, height: 1000 },
		deviceScaleFactor: 1,
	});
	await context.setOffline(true);
	const page = await context.newPage();
	const rows = [];
	for (const input of JSON.parse(casesSource)) {
		await page.setContent(input.html);
		const boxes = await page.evaluate(() =>
			[".grid", ".item", ".inner", ".next"].map((selector) => {
				const r = document.querySelector(selector).getBoundingClientRect();
				return { key: selector, rect: [r.x, r.y, r.width, r.height] };
			}),
		);
		rows.push({ id: input.id, boxes });
	}
	writeFileSync(
		resolve(
			root,
			"crates/turbo-dom-layout/tests/fixtures/percentage_frame_chrome159.json",
		),
		JSON.stringify(
			{
				browser: version,
				offline: true,
				casesSha256: hash(casesSource),
				collectorSha256: hash(readFileSync(fileURLToPath(import.meta.url))),
				rows,
			},
			null,
			2,
		) + "\n",
	);
	console.log(JSON.stringify({ version, cases: rows.length }));
} finally {
	await browser.close();
}

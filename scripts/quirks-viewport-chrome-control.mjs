// Offline Chrome 159 control for the physical HTML/body viewport-fill quirks.
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
	throw Error("Set CHROME_EXECUTABLE to the reviewed Chrome 159 executable");
const casesSource = readFileSync(
	resolve(
		root,
		"crates/turbo-dom-layout/tests/fixtures/quirks_viewport_cases.json",
	),
	"utf8",
);
const cases = JSON.parse(casesSource);
const probe = () => {
	const metrics = (element) => {
		const rect = element.getBoundingClientRect();
		const style = getComputedStyle(element);
		return {
			rect: [rect.x, rect.y, rect.width, rect.height],
			client: [element.clientWidth, element.clientHeight],
			offset: [element.offsetWidth, element.offsetHeight],
			writingMode: style.writingMode,
			display: style.display,
			position: style.position,
			float: style.float,
		};
	};
	return {
		compatMode: document.compatMode,
		inner: [innerWidth, innerHeight],
		html: metrics(document.documentElement),
		body: metrics(document.body),
		children: Array.from(document.querySelectorAll("p,div"), metrics),
	};
};
let browser;
let timer;
try {
	await Promise.race([
		(async () => {
			browser = await chromium.launch({
				executablePath,
				headless: true,
				timeout: 8000,
				args: [
					"--disable-background-networking",
					"--disable-component-update",
					"--disable-sync",
					"--no-first-run",
					"--no-default-browser-check",
				],
			});
			const version = browser.version();
			if (!version.startsWith("159."))
				throw Error(`Expected Chrome 159, got ${version}`);
			const context = await browser.newContext({
				viewport: { width: 640, height: 480 },
				deviceScaleFactor: 1,
				offline: true,
			});
			const page = await context.newPage();
			const rows = [];
			for (const input of cases) {
				await page.goto(
					"data:text/html;charset=utf-8," + encodeURIComponent(input.html),
					{ waitUntil: "load", timeout: 4000 },
				);
				rows.push({
					id: input.id,
					markup: input.html,
					...(await page.evaluate(probe)),
				});
			}
			const hitInput = {
				id: "quirks-root-last-hit",
				markup: "<body style='margin:0'></body>",
				viewport: [200, 200],
				point: [5, 150],
			};
			await page.setViewportSize({ width: 200, height: 200 });
			await page.goto(
				"data:text/html;charset=utf-8," + encodeURIComponent(hitInput.markup),
				{ waitUntil: "load", timeout: 4000 },
			);
			const hitReceipts = [
				{
					...hitInput,
					...(await page.evaluate(probe)),
					elements: await page.evaluate(
						([x, y]) =>
							document
								.elementsFromPoint(x, y)
								.map((element) => element.localName),
						hitInput.point,
					),
				},
			];
			const report = {
				browser: version,
				executablePath,
				viewport: [640, 480],
				deviceScaleFactor: 1,
				offline: true,
				casesSha256: createHash("sha256").update(casesSource).digest("hex"),
				collectorSha256: createHash("sha256")
					.update(readFileSync(fileURLToPath(import.meta.url)))
					.digest("hex"),
				probeSha256: createHash("sha256")
					.update(probe.toString())
					.digest("hex"),
				rows,
				hitReceipts,
			};
			writeFileSync(
				resolve(
					root,
					"crates/turbo-dom-layout/tests/fixtures/quirks_viewport_chrome159.json",
				),
				JSON.stringify(report, null, 2) + "\n",
			);
			console.log(JSON.stringify({ version, cases: rows.length }));
		})(),
		new Promise((_, reject) => {
			timer = setTimeout(
				() => reject(Error("Quirks viewport oracle exceeded 30 seconds")),
				30000,
			);
		}),
	]);
} finally {
	clearTimeout(timer);
	await browser?.close();
}

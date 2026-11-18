// Frozen, offline Chrome controls for translucent solid border geometry.
import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pngPixels } from "../tools/component-parity/png-pixels.mjs";
const require = createRequire(import.meta.url);
const root = fileURLToPath(new URL("../", import.meta.url));
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(root, "../turbo-surf/node_modules/playwright-core"),
);
if (!process.env.CHROME_EXECUTABLE)
	throw Error("Set CHROME_EXECUTABLE to reviewed Chrome159");
const browser = await chromium.launch({
	executablePath: process.env.CHROME_EXECUTABLE,
	headless: true,
});
try {
	if (browser.version() !== "159.0.8154.6") throw Error(browser.version());
	const page = await browser.newPage({
		viewport: { width: 16, height: 16 },
		deviceScaleFactor: 1,
	});
	await page.context().setOffline(true);
	const cases = [];
	for (const [id, css, points] of [
		[
			"equal-alpha",
			"width:4px;height:4px;border-width:2px;border-color:rgba(0,0,255,.5)",
			[
				[0, 0],
				[7, 0],
				[0, 7],
				[7, 7],
				[3, 0],
				[0, 3],
				[3, 3],
			],
		],
		[
			"unequal-alpha",
			"width:4px;height:4px;border-width:1px 3px 2px 2px;border-color:rgba(0,0,255,.5)",
			[
				[0, 0],
				[8, 0],
				[0, 6],
				[8, 6],
				[3, 0],
				[0, 3],
				[3, 3],
			],
		],

		[
			"zero-content",
			"width:0;height:0;border-width:2px;border-color:rgba(0,0,255,.5)",
			[
				[0, 0],
				[3, 0],
				[0, 3],
				[3, 3],
			],
		],
		[
			"rounded-alpha",
			"width:8px;height:8px;border-width:2px;border-color:rgba(0,0,255,.5);border-radius:4px",
			[
				[0, 0],
				[5, 0],
				[0, 5],
				[5, 5],
			],
		],
		[
			"translated-alpha",
			"width:4px;height:4px;border-width:2px;border-color:rgba(0,0,255,.5);transform:translate(3px,2px)",
			[
				[3, 2],
				[10, 2],
				[3, 9],
				[10, 9],
				[6, 5],
			],
		],
	]) {
		const html = `<!doctype html><style>body{margin:0;background:white}div{border-style:solid}</style><div style="${css}"></div>`;
		await page.setContent(html);
		const image = pngPixels(await page.screenshot());
		cases.push({
			id,
			html,
			pixels: points.map(([x, y]) => ({
				x,
				y,
				rgba: [...image.pixel(x, y)].slice(0, 3).concat(255),
			})),
		});
	}
	writeFileSync(
		new URL(
			"../crates/turbo-dom-raster/tests/fixtures/border_corners_chrome159.json",
			import.meta.url,
		),
		JSON.stringify({ chrome: browser.version(), cases }, null, 2) + "\n",
	);
} finally {
	await browser.close();
}

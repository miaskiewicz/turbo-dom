// Independent Chrome screenshot controls for multiple box-shadow layers.
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
		"crates/turbo-dom-raster/tests/fixtures/multiple_box_shadow_cases.json",
	),
	"utf8",
);
const input = JSON.parse(source);
const executablePath = process.env.CHROME_EXECUTABLE;
if (!executablePath)
	throw Error("Set CHROME_EXECUTABLE to the reviewed Chrome binary");
const browser = await chromium.launch({
	executablePath,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
const page = await browser.newPage({
	viewport: { width: input.viewport[0], height: input.viewport[1] },
	deviceScaleFactor: 1,
});
const cases = [];
for (const entry of input.cases) {
	await page.setContent(entry.html, { waitUntil: "load" });
	const computed = await page
		.locator("#subject")
		.evaluate((el) => getComputedStyle(el).boxShadow);
	const png = await page.screenshot();
	const probes = await page.evaluate(
		async ({ base64, points }) => {
			const image = new Image();
			image.src = `data:image/png;base64,${base64}`;
			await image.decode();
			const canvas = document.createElement("canvas");
			canvas.width = image.width;
			canvas.height = image.height;
			const context = canvas.getContext("2d");
			context.drawImage(image, 0, 0);
			return points.map(([x, y]) => ({
				x,
				y,
				rgba: [...context.getImageData(x, y, 1, 1).data],
			}));
		},
		{ base64: png.toString("base64"), points: entry.probes },
	);
	cases.push({ id: entry.id, html: entry.html, computed, probes });
}
const fixture = {
	generator: "scripts/multiple-box-shadow-chrome-control.mjs",
	chrome: browser.version(),
	source_sha256: createHash("sha256").update(source).digest("hex"),
	viewport: input.viewport,
	cases,
};
await browser.close();
writeFileSync(
	resolve(
		root,
		"crates/turbo-dom-raster/tests/fixtures/multiple_box_shadow_chrome159.json",
	),
	`${JSON.stringify(fixture, null, 2)}\n`,
);
console.log(
	`Chrome ${fixture.chrome}: ${cases.length} independent multi-shadow controls`,
);

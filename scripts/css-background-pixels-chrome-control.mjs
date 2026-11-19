// Independent Chrome controls for square CSS background pixel snapping.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pngPixels } from "../tools/component-parity/png-pixels.mjs";
const root = fileURLToPath(new URL("../", import.meta.url));
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(root, "../turbo-surf/node_modules/playwright-core"),
);
if (!process.env.CHROME_EXECUTABLE)
	throw Error("Set CHROME_EXECUTABLE to reviewed Chrome159");
const browser = await chromium.launch({
	executablePath: process.env.CHROME_EXECUTABLE,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	if (browser.version() !== "159.0.8154.6")
		throw Error("Expected Chrome159.0.8154.6");
	const context = await browser.newContext({
		viewport: { width: 8, height: 8 },
		deviceScaleFactor: 1,
	});
	await context.setOffline(true);
	const page = await context.newPage();
	const cases = [];
	for (const x of [-0.25, 0, 0.25, 0.5, 0.75]) {
		for (const width of [2.2, 2.8]) {
			for (const color of ["#123456", "rgba(18,52,86,0.5)"]) {
				const html = `<!doctype html><style>body{margin:0;background:white}div{position:absolute;left:${x}px;top:0.25px;width:${width}px;height:3.6px;background:${color}}</style><div></div>`;
				await page.setContent(html);
				const image = pngPixels(await page.screenshot());
				const rgba = [];
				for (let py = 0; py < 8; py++)
					for (let px = 0; px < 8; px++) {
						const p = image.pixel(px, py);
						rgba.push(...p);
						if (p.length === 3) rgba.push(255);
					}
				cases.push({ id: `${x}-${width}-${color}`, html, rgba });
			}
		}
	}
	writeFileSync(
		resolve(
			root,
			"crates/turbo-dom-raster/tests/fixtures/css_background_pixels_chrome159.json",
		),
		JSON.stringify(
			{
				browser: browser.version(),
				width: 8,
				height: 8,
				offline: true,
				probeSha256: createHash("sha256")
					.update(readFileSync(fileURLToPath(import.meta.url)))
					.digest("hex"),
				cases,
			},
			null,
			2,
		) + "\n",
	);
	console.log(JSON.stringify({ cases: cases.length }));
	await context.close();
} finally {
	await browser.close();
}

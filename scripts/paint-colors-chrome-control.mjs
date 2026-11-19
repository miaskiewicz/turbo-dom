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
	const inputs = [
		[
			"border-rgb-spaces",
			"<div style='width:4px;height:4px;color:#0b194d;border:1px solid rgb(185, 201, 255)'></div>",
			"",
		],
		[
			"border-rgb-compact",
			"<div style='width:4px;height:4px;color:#0b194d;border:1px solid rgb(185,201,255)'></div>",
			"",
		],
		[
			"border-alpha",
			"<div style='width:4px;height:4px;color:#0b194d;border:1px solid rgba(0, 0, 255, 0.5)'></div>",
			"",
		],
		[
			"svg-inherited",
			"<div style='color:#003af4'><svg width='6' height='6'><rect width='6' height='6' fill='currentColor'/></svg></div>",
			"",
		],
		[
			"svg-css-variable",
			"<style>:root{--ink:#003af4}.ink{color:var(--ink)}</style><div class='ink'><svg width='6' height='6'><rect width='6' height='6' fill='currentColor'/></svg></div>",
			"",
		],
		[
			"svg-linked-media",
			"<div class='ink'><svg width='6' height='6'><rect width='6' height='6' fill='currentColor'/></svg></div>",
			"@media(min-height:8px){.ink{color:#003af4}}",
		],
		[
			"svg-own-color",
			"<div style='color:red'><svg style='color:#003af4' width='6' height='6'><rect width='6' height='6' fill='currentColor'/></svg></div>",
			"",
		],
		[
			"svg-explicit-control",
			"<div style='color:red'><svg width='6' height='6'><rect width='6' height='6' fill='#003af4'/></svg></div>",
			"",
		],
		[
			"svg-own-presentation-color",
			"<div style='color:red'><svg width='6' height='6'><g color='#003af4'><rect width='6' height='6' fill='currentColor'/></g></svg></div>",
			"",
		],
		[
			"svg-descendant-sheet-vars",
			"<style>:root{--ink:#003af4}rect{fill:var(--ink);stroke:var(--ink);stroke-width:2px}</style><div><svg width='6' height='6'><rect x='1' y='1' width='4' height='4' fill='red' stroke='red'/></svg></div>",
			"",
		],
		[
			"svg-descendant-inline-important",
			"<style>rect{fill:red!important}</style><div><svg width='6' height='6'><rect width='6' height='6' fill='red' style='fill:#003af4!important'/></svg></div>",
			"",
		],
		[
			"svg-descendant-author-precedence",
			"<div><svg width='6' height='6' color='red'><rect width='6' height='6' color='green' fill='currentColor'/></svg></div>",
			"rect{color:#003af4}",
		],
	];
	for (const [id, body, css] of inputs) {
		const html = `<!doctype html><style>body{margin:0;background:white}svg{display:block}</style>${body}`;
		await page.setContent(
			html.replace("<!doctype html>", `<!doctype html><style>${css}</style>`),
		);
		const image = pngPixels(await page.screenshot());
		const rgba = [];
		for (let y = 0; y < 8; y++)
			for (let x = 0; x < 8; x++) {
				const p = image.pixel(x, y);
				rgba.push(...p);
				if (p.length === 3) rgba.push(255);
			}
		const rect = await page
			.locator("div")
			.first()
			.evaluate((e) => {
				const r = e.getBoundingClientRect();
				return [r.x, r.y, r.width, r.height];
			});
		cases.push({ id, html, css, rgba, rect });
	}
	writeFileSync(
		resolve(
			root,
			"crates/turbo-dom-raster/tests/fixtures/paint_colors_chrome159.json",
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

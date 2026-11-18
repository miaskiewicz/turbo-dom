// Independent Chrome159 HTML and emitted-SVG pixel controls for border opacity.
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { readFileSync, writeFileSync } from "node:fs";
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
	throw Error("Set reviewed Chrome159 CHROME_EXECUTABLE");
const browser = await chromium.launch({
	executablePath: process.env.CHROME_EXECUTABLE,
	headless: true,
});
const fixture = new URL(
	"../crates/turbo-dom-raster/tests/fixtures/svg_border_opacity_chrome159.json",
	import.meta.url,
);
function pixels(png) {
	const image = pngPixels(png);
	return Array.from({ length: 256 }, (_, i) =>
		image.pixel(i % 16, Math.floor(i / 16)).slice(0, 3),
	);
}
try {
	if (browser.version() !== "159.0.8154.6") throw Error(browser.version());
	const page = await browser.newPage({
		viewport: { width: 16, height: 16 },
		deviceScaleFactor: 1,
	});
	await page.context().setOffline(true);
	const same =
		"rgba(255,0,0,.5) rgba(0,128,0,.5) rgba(0,0,255,.5) rgba(255,128,0,.5)";
	const different =
		"rgba(255,0,0,.25) rgba(0,128,0,.75) rgba(0,0,255,.5) rgba(255,128,0,1)";
	const cases = [];
	for (const [id, colors, extra] of [
		["common-square", same, ""],
		["differing-square", different, ""],
		[
			"all-translucent",
			"rgba(255,0,0,.125) rgba(0,128,0,.5) rgba(0,0,255,.75) rgba(255,128,0,.25)",
			"",
		],
		[
			"opposite-translucent",
			"rgba(255,0,0,.5) transparent rgba(0,0,255,.5) transparent",
			"",
		],
		[
			"adjacent-translucent",
			"rgba(255,0,0,.5) rgba(0,128,0,.75) transparent transparent",
			"",
		],
		["common-translated", same, "transform:translate(.5px,.25px)"],
		[
			"differing-rotated",
			different,
			"transform:rotate(6deg);transform-origin:7px 7px",
		],
	]) {
		const html = `<!doctype html><style>body{margin:0;background:white}div{width:8px;height:8px;border:3px solid;border-color:${colors};${extra}}</style><div></div>`;
		await page.setContent(html);
		const rgb = pixels(await page.screenshot());
		await page.setContent(
			html.replace("background:white", "background:transparent"),
		);
		const alphaImage = pngPixels(
			await page.screenshot({ omitBackground: true }),
		);
		const alpha = Array.from(
			{ length: 256 },
			(_, i) => alphaImage.pixel(i % 16, Math.floor(i / 16))[3],
		);
		cases.push({ id, html, rgb, alpha });
	}
	const control = {
		chrome: browser.version(),
		probeSha256: createHash("sha256")
			.update(readFileSync(fileURLToPath(import.meta.url)))
			.digest("hex"),
		cases,
	};
	if (!process.env.SVG_BORDER_PROBE_DIR) {
		writeFileSync(fixture, JSON.stringify(control) + "\n");
	} else {
		const results = [];
		for (const c of cases) {
			const svg = readFileSync(
				resolve(process.env.SVG_BORDER_PROBE_DIR, `${c.id}.svg`),
			);
			const url = `data:image/svg+xml;base64,${svg.toString("base64")}`;
			if (process.env.SVG_BORDER_INLINE) {
				await page.setContent(`<style>body{margin:0}</style>${svg.toString()}`);
			} else {
				await page.setContent(
					`<style>body{margin:0}img{display:block}</style><img src="${url}">`,
				);
				await page.locator("img").evaluate((img) => img.decode());
			}
			const actual = pixels(await page.screenshot());
			let maxDelta = 0,
				differing = 0;
			actual.forEach((rgb, i) =>
				rgb.forEach((n, j) => {
					const delta = Math.abs(n - c.rgb[i][j]);
					maxDelta = Math.max(maxDelta, delta);
					if (delta > 1) differing++;
				}),
			);
			const worst = actual
				.map((rgb, i) => ({
					x: i % 16,
					y: Math.floor(i / 16),
					actual: rgb,
					expected: c.rgb[i],
					delta: Math.max(...rgb.map((n, j) => Math.abs(n - c.rgb[i][j]))),
				}))
				.filter((p) => p.delta === maxDelta)
				.slice(0, 5);
			results.push({
				id: c.id,
				maxDelta,
				channelsAboveOne: differing,
				worst,
			});
		}
		console.log(JSON.stringify({ chrome: browser.version(), results }));
		if (results.some((c) => c.maxDelta > 1)) process.exitCode = 1;
	}
} finally {
	await browser.close();
}

// Independent Chrome159 HTML and emitted-SVG pixel controls for border opacity.
import assert from "node:assert/strict";
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
	assert.equal(image.width, 16);
	assert.equal(image.height, 16);
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
		["common-rounded", same, "border-radius:5px"],
		["differing-rounded", different, "border-radius:5px"],
		[
			"common-rounded-translated",
			same,
			"border-radius:5px;transform:translate(.5px,.25px)",
		],
		[
			"differing-rounded-rotated",
			different,
			"border-radius:5px;transform:rotate(6deg);transform-origin:7px 7px",
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
		const rgba = Array.from({ length: 256 }, (_, i) =>
			alphaImage.pixel(i % 16, Math.floor(i / 16)),
		);
		cases.push({
			id,
			mode: id.includes("rounded") ? "rounded" : "square",
			html,
			rgb,
			alpha,
			rgba,
		});
	}
	const control = {
		chrome: browser.version(),
		width: 16,
		height: 16,
		provenance:
			"Chrome159.0.8154.6 offline HTML screenshot; grid row-major, CSS 1px/device pixel at DPR1",
		probeSha256: createHash("sha256")
			.update(readFileSync(fileURLToPath(import.meta.url)))
			.digest("hex"),
		cases,
	};
	assert.equal(cases.length, 11);
	assert.equal(new Set(cases.map((c) => c.id)).size, 11);

	if (!process.env.SVG_BORDER_PROBE_DIR) {
		writeFileSync(fixture, JSON.stringify(control) + "\n");
	} else {
		assert.deepEqual(
			JSON.parse(readFileSync(fixture, "utf8")),
			control,
			"Frozen Chrome grid/source hash changed",
		);
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
			const core = [
				[2, 2],
				[11, 2],
				[2, 11],
				[11, 11],
				[6, 1],
				[1, 6],
				[12, 6],
				[6, 12],
				[6, 6],
			].map(([x, y]) => ({
				x,
				y,
				actual: actual[y * 16 + x],
				expected: c.rgb[y * 16 + x],
				delta: Math.max(
					...actual[y * 16 + x].map((v, j) =>
						Math.abs(v - c.rgb[y * 16 + x][j]),
					),
				),
			}));

			const transparent = readFileSync(
				resolve(process.env.SVG_BORDER_PROBE_DIR, `${c.id}-transparent.svg`),
			);
			const transparentUrl = `data:image/svg+xml;base64,${transparent.toString("base64")}`;
			await page.setContent(
				`<style>body{margin:0}img{display:block}</style><img src="${transparentUrl}">`,
			);
			await page.locator("img").evaluate((img) => img.decode());
			const transparentImage = pngPixels(
				await page.screenshot({ omitBackground: true }),
			);
			const rgba = Array.from({ length: 256 }, (_, i) =>
				transparentImage.pixel(i % 16, Math.floor(i / 16)),
			);
			const ceiling = Math.max(...c.alpha);
			const alphaCeilingPass = rgba.every((pixel) => pixel[3] <= ceiling);
			let maxPremultipliedDelta = 0,
				maxScaleDelta = 0;
			rgba.forEach((pixel, i) => {
				pixel.forEach((v, j) => {
					const premul = (p, k) =>
						k === 3 ? p[k] : Math.round((p[k] * p[3]) / 255);
					maxPremultipliedDelta = Math.max(
						maxPremultipliedDelta,
						Math.abs(premul(pixel, j) - premul(c.rgba[i], j)),
					);
				});
				pixel.slice(0, 3).forEach((v, j) => {
					maxScaleDelta = Math.max(
						maxScaleDelta,
						Math.abs(
							actual[i][j] -
								Math.round((v * pixel[3] + 255 * (255 - pixel[3])) / 255),
						),
					);
				});
			});

			results.push({
				id: c.id,
				mode: c.mode,
				alphaCeilingPass,
				maxPremultipliedDelta,
				maxScaleDelta,
				maxDelta,
				channelsAboveOne: differing,
				worst,
				core,
			});
		}
		console.log(JSON.stringify({ chrome: browser.version(), results }));
		// Rounded fringes are recorded, not presented as exact CSS pixels: SVG's
		// even-odd path mask and Blink's clipRRect(Difference) use distinct AA.
		// Every rounded pixel still obeys alpha and color-scaling invariants,
		// and all nine geometrically covered/empty interior probes are checked.
		if (
			results.some(
				(c) =>
					!c.alphaCeilingPass ||
					c.maxScaleDelta > 1 ||
					(c.mode === "square"
						? c.maxDelta > 1 || c.maxPremultipliedDelta > 1
						: c.core.some((p) => p.delta > 1)),
			)
		)
			process.exitCode = 1;
	}
} finally {
	await browser.close();
}

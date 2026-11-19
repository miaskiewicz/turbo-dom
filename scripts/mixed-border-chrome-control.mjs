// Frozen, offline Chrome controls for translucent solid border geometry.
import { createRequire } from "node:module";
import { readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
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
	const inputs = [
		[
			"common-half",
			"rgba(255,0,0,.5) rgba(0,0,255,.5) rgba(0,128,0,.5) rgba(0,0,0,.5)",
		],
		[
			"common-quarter",
			"rgba(255,0,0,.25) rgba(0,0,255,.25) rgba(0,128,0,.25) rgba(0,0,0,.25)",
		],
		[
			"common-threequarter",
			"rgba(255,0,0,.75) rgba(0,0,255,.75) rgba(0,128,0,.75) rgba(0,0,0,.75)",
		],
		[
			"different-alpha",
			"rgba(255,0,0,1) rgba(0,0,255,.25) rgba(0,128,0,.5) rgba(0,0,0,.75)",
		],
		["opaque", "red blue green black"],
		[
			"unequal",
			"rgba(255,0,0,.5) rgba(0,0,255,.5) rgba(0,128,0,.5) rgba(0,0,0,.5)",
			"border-width:1px 3px 2px 2px",
		],
		[
			"zero-content",
			"rgba(255,0,0,.5) rgba(0,0,255,.5) rgba(0,128,0,.5) rgba(0,0,0,.5)",
			"width:0;height:0",
		],
		[
			"transparent-side",
			"rgba(255,0,0,.5) transparent rgba(0,128,0,.75) black",
		],
		[
			"translated",
			"rgba(255,0,0,.5) rgba(0,0,255,.5) rgba(0,128,0,.5) rgba(0,0,0,.5)",
			"transform:translate(3px,2px)",
		],
		[
			"uniform-rounded-backend-control",
			"rgba(0,0,255,.5)",
			"width:8px;height:8px;border-radius:4px",
		],
		[
			"rounded",
			"rgba(255,0,0,.5) rgba(0,0,255,.5) rgba(0,128,0,.5) rgba(0,0,0,.5)",
			"width:8px;height:8px;border-radius:4px",
		],
		[
			"offscreen",
			"rgba(255,0,0,.5) rgba(0,0,255,.5) rgba(0,128,0,.5) rgba(0,0,0,.5)",
			"transform:translate(-2px,-1px)",
		],
		[
			"fractional",
			"rgba(255,0,0,.5) rgba(0,0,255,.5) rgba(0,128,0,.5) rgba(0,0,0,.5)",
			"transform:translate(.25px,.5px)",
		],
	];
	for (const [id, colors, extra = ""] of inputs) {
		const css = `width:4px;height:4px;border-width:2px;border-color:${colors};${extra}`;
		const html = `<!doctype html><style>body{margin:0;background:white}div{border-style:solid}</style><div style="${css}"></div>`;
		await page.setContent(html);
		const image = pngPixels(await page.screenshot());
		cases.push({
			id,
			html,
			dimensions: [image.width, image.height],
			comparison: /border-radius|translate\(\.25/.test(css)
				? "analytic-coverage"
				: "integer-color",
			interiorPoints: /border-radius/.test(css)
				? [
						[5, 1],
						[10, 5],
						[5, 10],
						[1, 5],
					]
				: [
						[4, 1],
						[7, 4],
						[4, 7],
						[1, 4],
					],
			opaqueHtml: html.replace(
				/rgba\(([^,]+),([^,]+),([^,]+),[^)]+\)/g,
				"rgb($1,$2,$3)",
			),
			pixels: Array.from({ length: 256 }, (_, i) => [
				i % 16,
				Math.floor(i / 16),
			]).map(([x, y]) => ({
				x,
				y,
				rgba: [...image.pixel(x, y)].slice(0, 3).concat(255),
			})),
		});
	}
	const quadHtml =
		'<!doctype html><style>body{margin:0;background:white}svg{display:block}</style><svg width="16" height="16"><path d="M0 0 L2 2 L2 6 L0 8 Z" fill="black" transform="translate(.25 .5)"/></svg>';
	await page.setContent(quadHtml);
	const quadImage = pngPixels(await page.screenshot());
	const quadDiagnostic = {
		html: quadHtml,
		x: 2,
		y: 2,
		rgba: [...quadImage.pixel(2, 2)].slice(0, 3).concat(255),
		exactIntersectionArea: 0.15625,
	};
	writeFileSync(
		new URL(
			"../crates/turbo-dom-raster/tests/fixtures/mixed_border_chrome159.json",
			import.meta.url,
		),
		JSON.stringify(
			{
				chrome: browser.version(),
				viewport: [16, 16],
				collectorSha256: createHash("sha256")
					.update(readFileSync(fileURLToPath(import.meta.url)))
					.digest("hex"),
				cases,
				quadDiagnostic,
			},
			null,
			2,
		) + "\n",
	);
} finally {
	await browser.close();
}

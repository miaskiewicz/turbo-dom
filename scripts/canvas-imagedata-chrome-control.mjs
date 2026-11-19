// Chrome controls for device-space ImageData replacement (turbo-dom#523).
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const chromePath =
	values.chrome ||
	join(
		homedir(),
		"Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
	);
const out = resolve(
	values.out ||
		join(
			repo,
			"crates/turbo-dom-raster/tests/fixtures/canvas_image_data_chrome156.json",
		),
);
const playwrightCore =
	process.env.PLAYWRIGHT_CORE ||
	resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const image = {
	width: 3,
	height: 2,
	colorSpace: "srgb",
	pixelFormat: "rgba-unorm8",
	data: [
		255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 201, 77, 13, 77, 12, 99,
		233, 0, 50, 100, 150, 128,
	],
};
const put = (...args) => [
	"putImageData",
	[image, ...args],
	"red",
	"green",
	"10px serif",
	"top",
	"right",
	0,
	"destination-out",
];
const fill = [
	"fillRect",
	[0, 0, 5, 4],
	"#ffff00",
	"black",
	"10px serif",
	"top",
	"right",
	1,
	"source-over",
];
const CASES = [
	{
		id: "replace-alpha-ignore-state-transform",
		ops: [fill, ["translate", [100, 100]], put(1, 1, 0, 0, 3, 2)],
	},
	{ id: "default-dirty", ops: [fill, put(0, 0)] },
	{ id: "negative-dirty-size", ops: [fill, put(0, 0, 3, 2, -2, -1)] },
	{ id: "dirty-source-clipping", ops: [fill, put(1, 1, -2, -1, 4, 2)] },
	{ id: "negative-destination", ops: [fill, put(-1, -1, 0, 0, 3, 2)] },
	{ id: "empty-dirty", ops: [fill, put(0, 0, 0, 0, 0, 2)] },
	{ id: "outside-dirty", ops: [fill, put(0, 0, 99, 99, 3, 2)] },
	{
		id: "large-positive-offset",
		ops: [fill, put(2147483647, 2147483647, 0, 0, 3, 2)],
	},
	{
		id: "large-negative-offset",
		ops: [fill, put(-2147483648, -2147483648, 0, 0, 3, 2)],
	},
];
const browser = await chromium.launch({
	executablePath: chromePath,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
const chrome = browser.version();
if (!chrome.startsWith("156."))
	throw new Error(`Expected Chrome156, received ${chrome}`);
const page = await browser.newPage();
const cases = [];
for (const test of CASES) {
	const rgba = await page.evaluate((ops) => {
		const canvas = document.createElement("canvas");
		canvas.width = 5;
		canvas.height = 4;
		const ctx = canvas.getContext("2d", { willReadFrequently: true });
		for (const [
			name,
			args,
			fill,
			stroke,
			font,
			baseline,
			align,
			alpha,
			composite,
		] of ops) {
			if (fill !== undefined) {
				ctx.fillStyle = fill;
				ctx.strokeStyle = stroke;
				ctx.font = font;
				ctx.textBaseline = baseline;
				ctx.textAlign = align;
				ctx.globalAlpha = alpha;
				ctx.globalCompositeOperation = composite;
			}
			if (name === "putImageData") {
				const [payload, ...coordinates] = args;
				ctx.putImageData(
					new ImageData(
						new Uint8ClampedArray(payload.data),
						payload.width,
						payload.height,
					),
					...coordinates,
				);
			} else ctx[name](...args);
		}
		return [...ctx.getImageData(0, 0, 5, 4).data];
	}, test.ops);
	cases.push({ ...test, width: 5, height: 4, rgba });
}
await browser.close();
mkdirSync(dirname(out), { recursive: true });
writeFileSync(
	out,
	`${JSON.stringify({ generator: "scripts/canvas-imagedata-chrome-control.mjs", chrome, playwright_core: playwrightVersion, cases }, null, 2)}\n`,
);
console.log(`Chrome ${chrome}: ${cases.length} ImageData cases -> ${out}`);

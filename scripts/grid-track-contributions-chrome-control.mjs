// Issue #521: percentage grid items must contribute against their grid area,
// including when the grid is nested inside a centered column flex container.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
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
const chrome =
	values.chrome ||
	join(
		homedir(),
		"Library/Caches/ms-playwright/chromium_headless_shell-1255/chrome-headless-shell-mac-arm64/chrome-headless-shell",
	);
const out =
	values.out ||
	join(
		repo,
		"crates/turbo-dom-layout/tests/fixtures/grid_track_contributions_chrome159.json",
	);
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const fixtureFont = readFileSync(
	join(
		repo,
		"crates/turbo-dom-layout/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf",
	),
);
const fixtureFontSha256 = createHash("sha256")
	.update(fixtureFont)
	.digest("hex");
const variants = [
	["percent-max", "repeat(3,1fr)", "width:100%;max-width:374px"],
	["percent-half", "repeat(3,1fr)", "width:50%;max-width:374px"],
	[
		"percent-auto-min",
		"repeat(3,1fr)",
		"width:100%;min-width:auto;max-width:374px",
	],
	[
		"percent-explicit-min",
		"repeat(3,1fr)",
		"width:100%;min-width:80px;max-width:374px",
	],
	["auto-max", "repeat(3,1fr)", "width:auto;max-width:374px"],
	["fixed-tracks", "repeat(3,100px)", "width:100%;max-width:374px"],
	["zero-min-fr", "repeat(3,minmax(0,1fr))", "width:100%;max-width:374px"],
	["fixed-item", "repeat(3,1fr)", "width:140px;max-width:120px"],
	["overflow-percent", "repeat(3,1fr)", "width:150%;max-width:700px"],
	[
		"spanning",
		"repeat(4,1fr)",
		"width:100%;max-width:700px",
		"",
		"grid-column:span 2",
	],
	[
		"negative-implicit",
		"repeat(3,1fr)",
		"width:100%;max-width:700px",
		"",
		"grid-column:-5",
	],
	[
		"auto-fit",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:100%;max-width:374px",
	],
	[
		"auto-fill",
		"repeat(auto-fill,minmax(100px,1fr))",
		"width:100%;max-width:374px",
	],
	[
		"center",
		"repeat(3,100px)",
		"width:50%;max-width:374px",
		"justify-content:center",
	],
	[
		"space-between",
		"repeat(3,100px)",
		"width:50%;max-width:374px",
		"justify-content:space-between",
	],
	[
		"space-around",
		"repeat(3,100px)",
		"width:50%;max-width:374px",
		"justify-content:space-around",
	],
	[
		"space-evenly",
		"repeat(3,100px)",
		"width:50%;max-width:374px",
		"justify-content:space-evenly",
	],
	["percent-minmax", "repeat(3,1fr)", "width:100%;min-width:25%;max-width:75%"],
	["percent-overflow-min", "repeat(3,1fr)", "width:100%;min-width:200%"],
	["mixed-calc", "repeat(3,1fr)", "width:calc(100% - 8px);max-width:374px"],
	[
		"mixed-calc-minmax",
		"repeat(3,1fr)",
		"width:calc(100% - 8px);min-width:calc(50% + 4px);max-width:calc(100% - 4px)",
	],
	[
		"absolute-lines",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/4",
	],
	[
		"absolute-negative",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:-3/-1",
	],
	[
		"absolute-span",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/span 2",
	],
	["overflow-center", "repeat(3,200px)", "width:50%", "justify-content:center"],
	["overflow-end", "repeat(3,200px)", "width:50%", "justify-content:end"],
	[
		"gapless-fit",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:100%;max-width:374px",
		"column-gap:0",
	],
	[
		"absolute-auto",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute",
	],
	[
		"absolute-zero",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:0/0",
	],
	[
		"absolute-outside",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:9/11",
	],
	[
		"absolute-before",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:-9/-7",
	],
	[
		"absolute-fixed-inflow",
		"repeat(3,1fr)",
		"width:100px",
		"position:relative",
		"position:absolute;grid-column:9/11",
	],
	[
		"absolute-row-outside",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/3;grid-row:9/11",
	],
	[
		"absolute-row-before",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/3;grid-row:-9/-7",
	],
	[
		"absolute-row-lines",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/3;grid-row:1/2",
	],
	[
		"absolute-row-zero",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/3;grid-row:0/0",
	],
	[
		"absolute-large-span",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/span 65535",
	],
	[
		"absolute-large-lines",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:32767/span 65535",
	],
	[
		"absolute-minimum-line",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:-32768/-32767",
	],
	[
		"absolute-large-start-span",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:span 65535/-1",
	],
	[
		"absolute-reversed-lines",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:3/1",
	],
	[
		"absolute-same-line",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/2",
	],
	[
		"absolute-large-row-span",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:2/3;grid-row:1/span 65535",
	],
	[
		"absolute-negative-implicit-row",
		"repeat(3,1fr)",
		"width:100%",
		"position:relative;grid-template-rows:100px",
		"position:absolute;grid-column:2/3;grid-row:-3/-2",
		"grid-row:-3",
	],
	["intrinsic-word-wrap", "repeat(3,1fr)", "width:auto;max-width:374px"],
	["intrinsic-word-nowrap", "repeat(3,1fr)", "width:auto;max-width:374px"],
	[
		"intrinsic-word-wrap-zero",
		"repeat(3,1fr)",
		"width:auto;min-width:0;max-width:374px",
	],
	[
		"intrinsic-word-nowrap-zero",
		"repeat(3,1fr)",
		"width:auto;min-width:0;max-width:374px",
	],
];
// #525: retain the original #521 controls unchanged and extend the same inputs
// with independent repeat-count, collapsed-gutter and grid-line geometry cases.
variants.push(
	["fit-count-max", "repeat(auto-fit,minmax(100px,200px))", "width:100%"],
	[
		"fit-count-max-abs",
		"repeat(auto-fit,minmax(100px,200px))",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:5/6",
	],
	["fit-max-below-min", "repeat(auto-fit,minmax(200px,100px))", "width:100%"],
	[
		"fit-max-below-min-abs",
		"repeat(auto-fit,minmax(200px,100px))",
		"width:100%",
		"position:relative",
		"position:absolute;grid-column:5/6",
	],
	["fill-count-max", "repeat(auto-fill,minmax(100px,200px))", "width:100%"],
	[
		"fit-two-factors",
		"repeat(auto-fit,minmax(100px,1fr) minmax(150px,2fr))",
		"width:100%",
	],
	[
		"fill-two-factors",
		"repeat(auto-fill,minmax(100px,1fr) minmax(150px,2fr))",
		"width:100%",
	],
	[
		"fit-mixed-fixed",
		"80px repeat(auto-fit,minmax(100px,1fr)) 60px",
		"width:100%",
	],
	[
		"fill-mixed-fixed",
		"80px repeat(auto-fill,minmax(100px,1fr)) 60px",
		"width:100%",
	],
	["fit-larger-minimum", "repeat(auto-fit,minmax(250px,1fr))", "width:100%"],
	[
		"fit-zero-count",
		"repeat(auto-fit,minmax(0,1fr))",
		"width:100%",
		"width:120px;column-gap:0",
	],
	[
		"fill-zero-count",
		"repeat(auto-fill,minmax(0,1fr))",
		"width:100%",
		"width:120px;column-gap:0",
	],
	[
		"fit-zero-prefix-count",
		"0px repeat(auto-fit,minmax(0,1fr))",
		"width:100%",
		"width:120px;column-gap:0",
	],
	[
		"fit-negative-implicit",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:100%",
		"",
		"grid-column:-12",
	],
	[
		"fit-positive-implicit",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:100%",
		"",
		"grid-column:12",
	],
	[
		"fit-spanning",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:100%",
		"",
		"grid-column:span 2",
		"grid-column:span 2",
	],
	[
		"fit-row-span",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:100%",
		"width:600px;column-gap:20px;row-gap:20px",
		"grid-column:span 2;grid-row:span 2",
		"grid-column:span 2",
	],
	[
		"fit-invalid-template",
		"1fr repeat(auto-fit,minmax(100px,1fr))",
		"width:100%",
	],
	[
		"fit-invalid-two-repeats",
		"repeat(auto-fit,100px) repeat(auto-fill,100px)",
		"width:100%",
	],
	[
		"fit-abs-fixed-inflow",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:100px",
		"position:relative",
		"position:absolute;grid-column:3/4",
	],
	[
		"fit-item-minmax",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:100%;min-width:140px;max-width:180px",
	],
	[
		"fit-auto-sized-items",
		"repeat(auto-fit,minmax(100px,1fr))",
		"width:auto;max-width:374px",
	],
	[
		"fill-auto-sized-items",
		"repeat(auto-fill,minmax(100px,1fr))",
		"width:auto;max-width:374px",
	],
);

for (const alignment of [
	"start",
	"end",
	"center",
	"space-between",
	"space-around",
	"space-evenly",
	"stretch",
]) {
	const extra = `width:600px;column-gap:20px;row-gap:20px;position:relative;justify-content:${alignment}`;
	variants.push(
		[
			`fit-internal-${alignment}`,
			"repeat(auto-fit,100px)",
			"width:100%",
			extra,
			"position:absolute;grid-column:3/4",
			"grid-column:1",
			"grid-column:5",
		],
		[
			`fit-leading-${alignment}`,
			"repeat(auto-fit,100px)",
			"width:100%",
			extra,
			"position:absolute;grid-column:1/2",
			"grid-column:-3",
			"grid-column:-2",
		],
		[
			`fit-trailing-${alignment}`,
			"repeat(auto-fit,100px)",
			"width:100%",
			extra,
			"position:absolute;grid-column:5/6",
			"grid-column:1",
			"grid-column:3",
		],
		[
			`fit-empty-${alignment}`,
			"repeat(auto-fit,100px)",
			"position:absolute;width:100%",
			extra,
			"grid-column:1/2",
			"grid-column:-2/-1",
			"grid-column:3/4",
		],
	);
}

for (const alignment of ["start", "center", "end"]) {
	const extra = `width:600px;column-gap:20px;row-gap:20px;position:relative;justify-content:${alignment}`;
	variants.push(
		[
			`fit-first-end-${alignment}`,
			"repeat(auto-fit,100px)",
			"width:100%",
			extra,
			"position:absolute;grid-column:auto/1",
			"grid-column:-3",
			"grid-column:-2",
		],
		[
			`fit-last-start-${alignment}`,
			"repeat(auto-fit,100px)",
			"width:100%",
			extra,
			"position:absolute;grid-column:-1/auto",
			"grid-column:1",
			"grid-column:3",
		],
	);
}

const browser = await chromium.launch({
	executablePath: chrome,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const cases = [];
	for (const width of [1440, 768, 390]) {
		const page = await browser.newPage({
			viewport: { width, height: 1000 },
			deviceScaleFactor: 1,
		});
		for (const parent of ["flex", "block"]) {
			for (const [
				name,
				tracks,
				item,
				extra = "",
				first = "",
				second = "",
				third = "",
			] of variants) {
				const words = name.startsWith("intrinsic-word-");
				const textStyle = words
					? ` style="height:auto;font-family:'Liberation Serif';font-size:16px;line-height:20px;white-space:${name.includes("nowrap") ? "nowrap" : "normal"}"`
					: "";
				const text = words ? "aaaa aaaa aaaa" : "";
				const cards = [0, 1, 2]
					.map(
						(i) =>
							`<div class="card" data-cid="card${i}" style="${i === 0 ? first : i === 1 ? second : third}"><div class="image" data-cid="image${i}"></div><div class="content" data-cid="content${i}"><div class="text" data-cid="text${i}"${textStyle}>${text}</div></div></div>`,
					)
					.join("");
				const html = `<!doctype html><style>*{box-sizing:border-box;margin:0;padding:0}.stack{display:${parent};flex-direction:column;align-items:center;width:1152px;max-width:100%}.grid{display:grid;width:100%;gap:40px;grid-template-columns:${tracks};${extra}}.card{display:flex;flex-direction:column;gap:32px;padding:16px 16px 32px;${item}}.image{width:100%;height:195px}.content{display:flex;flex-direction:column;flex:1 1 0%;padding:0 12px;gap:24px}.text{width:100%;height:72px}</style><div class="stack" data-cid="stack"><div class="grid" data-cid="grid">${cards}</div></div>`;
				await page.setContent(html);
				if (words) {
					await page.evaluate(async (data) => {
						const font = new FontFace(
							"Liberation Serif",
							`url(data:font/ttf;base64,${data})`,
						);
						await font.load();
						document.fonts.add(font);
						await document.fonts.ready;
					}, fixtureFont.toString("base64"));
				}
				const boxes = await page.locator("[data-cid]").evaluateAll((nodes) =>
					nodes.map((node) => {
						const { x, y, width, height } = node.getBoundingClientRect();
						return { cid: node.getAttribute("data-cid"), x, y, width, height };
					}),
				);
				cases.push({
					id: `${parent}-${name}-${width}`,
					width,
					height: 1000,
					html,
					boxes,
				});
			}
		}
		await page.close();
	}
	writeFileSync(
		out,
		`${JSON.stringify(
			{
				browser: browser.version(),
				font_sha256: fixtureFontSha256,
				probe_sha256: createHash("sha256")
					.update(readFileSync(fileURLToPath(import.meta.url)))
					.digest("hex"),
				cases,
			},
			null,
			2,
		)}\n`,
	);
	console.log(`${cases.length} cases: ${out}`);
} finally {
	await browser.close();
}

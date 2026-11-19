// Issue #537: a baseline-aligned grid item lays out its text at the solved column width,
// not at its min-content width. Captures independent Chromium geometry for the label /
// value rows of the issue (1fr auto, auto 1fr, overflow-wrap: break-word) and nearby
// cases that must keep their geometry (start / center alignment, other track lists, a
// label long enough to really wrap).
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
		"crates/turbo-dom-layout/tests/fixtures/grid_baseline_chrome159.json",
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

const label = "Total percepciones";
const longLabel =
	"Total percepciones gravadas y exentas del periodo de pago quincenal";
// [id, grid-template-columns, align-items, first item style, first text, second text]
const variants = [
	["fr-auto-baseline", "1fr auto", "baseline", "", label, "$1,000.00"],
	[
		"fr-auto-baseline-break-word",
		"1fr auto",
		"baseline",
		"overflow-wrap:break-word",
		label,
		"$1,000.00",
	],
	[
		"auto-fr-baseline-break-word",
		"auto 1fr",
		"baseline",
		"overflow-wrap:break-word",
		"$1,000.00",
		label,
	],
	[
		"minmax-auto-baseline",
		"minmax(0,1fr) auto",
		"baseline",
		"",
		label,
		"$1,000.00",
	],
	["fr-auto-start", "1fr auto", "start", "", label, "$1,000.00"],
	["fr-auto-center", "1fr auto", "center", "", label, "$1,000.00"],
	["fr-fr-baseline", "1fr 1fr", "baseline", "", label, "$1,000.00"],
	[
		"fr-auto-baseline-wraps",
		"1fr auto",
		"baseline",
		"",
		longLabel,
		"$1,000.00",
	],
	[
		"fr-auto-baseline-wraps-break-word",
		"1fr auto",
		"baseline",
		"overflow-wrap:break-word",
		longLabel,
		"$1,000.00",
	],
	[
		"fr-auto-self-baseline",
		"1fr auto",
		"start",
		"align-self:baseline",
		label,
		"$1,000.00",
	],
	[
		"fr-auto-baseline-bigger-value",
		"1fr auto",
		"baseline",
		"",
		label,
		'<span style="font-size:16px">$1,000.00</span>',
	],
];

const browser = await chromium.launch({
	executablePath: chrome,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const cases = [];
	for (const width of [752, 390, 120]) {
		const page = await browser.newPage({
			viewport: { width, height: 992 },
			deviceScaleFactor: 1,
		});
		for (const [name, tracks, align, firstStyle, first, second] of variants) {
			const html = `<!doctype html><style>*{margin:0;padding:0}body{font-family:'Liberation Serif'}</style><div data-cid="grid" style="display:grid;grid-template-columns:${tracks};column-gap:8px;align-items:${align};font-size:8px;line-height:1.4"><span data-cid="first" style="${firstStyle}">${first}</span><span data-cid="second">${second}</span></div>`;
			await page.setContent(html);
			await page.evaluate(async (data) => {
				const font = new FontFace(
					"Liberation Serif",
					`url(data:font/ttf;base64,${data})`,
				);
				await font.load();
				document.fonts.add(font);
				await document.fonts.ready;
			}, fixtureFont.toString("base64"));
			const boxes = await page.evaluate(() =>
				[...document.querySelectorAll("[data-cid]")].map((el) => {
					const r = el.getBoundingClientRect();
					return {
						cid: el.dataset.cid,
						x: r.x,
						y: r.y,
						width: r.width,
						height: r.height,
					};
				}),
			);
			cases.push({ id: `${name}@${width}`, width, height: 992, html, boxes });
		}
		await page.close();
	}
	const probe = createHash("sha256")
		.update(readFileSync(fileURLToPath(import.meta.url)))
		.digest("hex");
	writeFileSync(
		out,
		`${JSON.stringify(
			{
				browser: browser.version(),
				probe_sha256: probe,
				font_sha256: fixtureFontSha256,
				cases,
			},
			null,
			1,
		)}\n`,
	);
	console.log(`${cases.length} cases -> ${out}`);
} finally {
	await browser.close();
}

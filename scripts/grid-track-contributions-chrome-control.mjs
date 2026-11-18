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
];
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
			for (const [name, tracks, item, extra = "", first = ""] of variants) {
				const cards = [0, 1, 2]
					.map(
						(i) =>
							`<div class="card" data-cid="card${i}" style="${i === 0 ? first : ""}"><div class="image" data-cid="image${i}"></div><div class="content" data-cid="content${i}"><div class="text" data-cid="text${i}"></div></div></div>`,
					)
					.join("");
				const html = `<!doctype html><style>*{box-sizing:border-box;margin:0;padding:0}.stack{display:${parent};flex-direction:column;align-items:center;width:1152px;max-width:100%}.grid{display:grid;width:100%;gap:40px;grid-template-columns:${tracks};${extra}}.card{display:flex;flex-direction:column;gap:32px;padding:16px 16px 32px;${item}}.image{width:100%;height:195px}.content{display:flex;flex-direction:column;flex:1 1 0%;padding:0 12px;gap:24px}.text{width:100%;height:72px}</style><div class="stack" data-cid="stack"><div class="grid" data-cid="grid">${cards}</div></div>`;
				await page.setContent(html);
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

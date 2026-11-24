import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const repo = fileURLToPath(new URL("../", import.meta.url));
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const chrome =
	values.chrome ||
	join(
		homedir(),
		".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
	);
const { chromium } = createRequire(import.meta.url)(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const variants = [
	["static-start", "width:80px;padding:9px", "width:12px;height:8px"],
	[
		"svg-checkbox",
		"width:24px;padding:9px;align-items:center;justify-content:center",
		"left:0;top:0;width:100%;height:100%",
		"input",
		"svg",
	],
	[
		"auto-height",
		"width:24px;padding:9px",
		"left:0;top:0;width:100%;height:100%",
	],
	[
		"fixed-height",
		"width:80px;height:50px;padding:9px",
		"left:0;top:0;width:100%;height:100%",
	],
	[
		"min-height",
		"width:80px;min-height:50px;padding:9px",
		"left:0;top:0;width:100%;height:100%",
	],
	[
		"border-box",
		"box-sizing:border-box;width:80px;height:60px;border:3px solid;padding:7px 11px 13px 5px",
		"left:0;top:0;width:100%;height:100%",
	],
	[
		"percentage-insets",
		"width:78px;height:62px;padding:9px",
		"left:25%;top:25%;width:50%;height:50%",
	],
	["stretch-insets", "width:80px;padding:9px", "inset:0", "div"],
	[
		"end-insets",
		"width:80px;padding:9px;border:3px solid",
		"right:0;bottom:0;width:12px;height:8px",
	],
	[
		"cross-axis-center",
		"width:80px;padding:9px;align-items:center;justify-content:center",
		"left:0;width:12px;height:8px",
	],
	[
		"percentage-padding",
		"width:80px;padding:5%",
		"left:0;top:0;width:100%;height:100%",
	],
	[
		"in-flow-percentage",
		"width:80px;padding:9px",
		"position:static;width:100%;height:12px",
		"div",
	],
];
const browser = await chromium.launch({
	executablePath: chrome,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
	const cases = [];
	for (const display of ["flex", "inline-flex"]) {
		for (const direction of ["row", "column"]) {
			for (const [
				name,
				parent,
				child,
				tag = "input",
				flow = "div",
			] of variants) {
				const content =
					flow === "svg"
						? '<svg data-cid="flow" width="24" height="24" style="flex:none;width:24px;height:24px" viewBox="0 0 24 24"><path d="M3 3h18v18H3z"/></svg>'
						: '<div data-cid="flow" style="flex:none;width:24px;height:24px"></div>';
				const html = `<!doctype html><body style="margin:0"><span data-cid="parent" style="position:relative;display:${display};flex-direction:${direction};${parent}"><${tag} data-cid="input" type="checkbox" style="position:absolute;padding:0;margin:0;border:0;opacity:0;${child}"></${tag}>${content}</span></body>`;
				await page.setContent(html);
				const boxes = await page.evaluate(() =>
					Array.from(document.querySelectorAll("[data-cid]"), (element) => {
						const { x, y, width, height } = element.getBoundingClientRect();
						return {
							cid: element.getAttribute("data-cid"),
							x,
							y,
							width,
							height,
						};
					}),
				);
				cases.push({ name: `${display}-${direction}-${name}`, html, boxes });
			}
		}
	}
	const out =
		values.out ||
		resolve(
			repo,
			"crates/turbo-dom-layout/tests/fixtures/absolute_padding_box_chrome159.json",
		);
	writeFileSync(
		out,
		JSON.stringify(
			{
				browser: await browser.version(),
				probe_sha256: createHash("sha256")
					.update(readFileSync(fileURLToPath(import.meta.url)))
					.digest("hex"),
				cases,
			},
			null,
			2,
		) + "\n",
	);
	console.log(out);
} finally {
	await browser.close();
}

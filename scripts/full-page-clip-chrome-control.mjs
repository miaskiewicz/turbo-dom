// Offline Chrome 156 control: CSS clip suppresses ink, not scrollable overflow.
// Usage: node scripts/full-page-clip-chrome-control.mjs [--chrome PATH] [--out PATH]
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
const probePath = fileURLToPath(import.meta.url);
const repo = resolve(dirname(probePath), "..");
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const args = ["--use-mock-keychain", "--password-store=basic"];
const browser = await chromium.launch({
	executablePath:
		values.chrome ||
		join(
			homedir(),
			".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
		),
	headless: true,
	args,
});
const inputs = [
	[
		"own-empty-clip",
		"<div style='position:absolute;top:220px;width:20px;height:20px;background:red;clip:rect(0,0,0,0)'></div>",
	],
	[
		"clipped-parent",
		"<div style='position:absolute;top:20px;width:20px;height:20px;clip:rect(0,0,0,0)'><div style='position:absolute;top:220px;width:20px;height:20px;background:red'></div></div>",
	],
	[
		"overflow-hidden",
		"<div style='height:20px;overflow:hidden'><div style='position:relative;top:220px;width:20px;height:20px;background:red'></div></div>",
	],
];
const cases = [];
try {
	for (const [id, body] of inputs) {
		const page = await browser.newPage({
			viewport: { width: 100, height: 80 },
		});
		try {
			await page.route("**/*", (route) => route.abort());
			const html =
				"<!doctype html><body style='margin:0;background:white'>" + body;
			await page.setContent(html);
			const scrollHeight = await page.evaluate(
				() => document.documentElement.scrollHeight,
			);
			const png = await page.screenshot({ fullPage: true });
			cases.push({
				id,
				html,
				scrollHeight,
				width: png.readUInt32BE(16),
				height: png.readUInt32BE(20),
			});
		} finally {
			await page.close();
		}
	}
	const result = {
		browser: await browser.version(),
		probeSha256: createHash("sha256")
			.update(readFileSync(probePath))
			.digest("hex"),
		args,
		viewport: { width: 100, height: 80 },
		cases,
	};
	writeFileSync(
		values.out ||
			join(
				repo,
				"crates/turbo-dom-raster/tests/fixtures/full-page-clip-chrome156.json",
			),
		JSON.stringify(result, null, 2) + "\n",
	);
} finally {
	await browser.close();
}

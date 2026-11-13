import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const html = readFileSync(
	join(
		repo,
		"crates/turbo-dom-browser-env/src/document_replacement_shadow.html",
	),
	"utf8",
);
const probe = readFileSync(
	join(
		repo,
		"crates/turbo-dom-browser-env/scripts/document-replacement-shadow-probe.js",
	),
	"utf8",
);
const browser = await chromium.launch({
	executablePath:
		values.chrome ||
		join(
			homedir(),
			".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
		),
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const page = await browser.newPage();
	await page.evaluate("globalThis.shadowOrder=[]; globalThis.handlerMark=0");
	await page.setContent(html);
	// The host handoff intentionally constructs custom elements after the complete
	// replacement is attached, so this control upgrades only after parsing too.
	await page.evaluate(probe);
	const result = await page.evaluate("inspectReplacement()");
	const hash = (source) => createHash("sha256").update(source).digest("hex");
	writeFileSync(
		resolve(
			values.out ||
				join(
					repo,
					"crates/turbo-dom-browser-env/src/document_replacement_shadow_chrome156.json",
				),
		),
		`${JSON.stringify({ browser: await browser.version(), htmlSha256: hash(html), probeSha256: hash(probe), result }, null, 2)}\n`,
	);
} finally {
	await browser.close();
}

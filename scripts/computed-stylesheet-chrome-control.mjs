// Offline custom-token oracle; no application services or live network.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { probe } from "../crates/turbo-dom/tests/fixtures/computed-stylesheet-probe.mjs";

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
const prefix = join(repo, "crates/turbo-dom/tests/fixtures");
const source = readFileSync(
	join(prefix, "computed-stylesheet-probe.mjs"),
	"utf8",
);
const browser = await chromium.launch({
	executablePath: chrome,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const version = browser.version();
	if (version !== "159.0.8154.6")
		throw new Error(
			`This frozen fixture requires Chromium159.0.8154.6; received ${version}. Pass the matching --chrome path.`,
		);
	const page = await browser.newPage();
	const result = await page.evaluate(probe);
	const sha = (text) => createHash("sha256").update(text).digest("hex");
	const out = values.out || join(prefix, "computed_stylesheet_chrome159.json");
	writeFileSync(
		out,
		JSON.stringify(
			{
				browser: version,
				probeSha256: sha(source),
				result,
			},
			null,
			2,
		) + "\n",
	);
	console.log(out);
} finally {
	await browser.close();
}

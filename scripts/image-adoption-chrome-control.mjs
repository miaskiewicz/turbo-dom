// Chrome 156 control for image state and decode method realms after adoption.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const executable =
	values.chrome ||
	join(
		homedir(),
		".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
	);
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const source = readFileSync(
	join(repo, "crates/turbo-dom-browser-env/src/html_image_adoption_probe.js"),
	"utf8",
);
const out =
	values.out ||
	join(
		repo,
		"crates/turbo-dom-browser-env/src/html_image_adoption_chrome156.json",
	);
const server = createServer((request, response) => {
	response.writeHead(200, { "Content-Type": "text/html", Connection: "close" });
	response.end(
		request.url === "/"
			? "<!doctype html><body><iframe src='/peer'></iframe>"
			: "<!doctype html><body>",
	);
});
let browser;
async function finite(work) {
	let timer;
	try {
		return await Promise.race([
			work,
			new Promise((_, reject) => {
				timer = setTimeout(
					() => reject(new Error("Image oracle deadline exceeded")),
					20000,
				);
			}),
		]);
	} finally {
		clearTimeout(timer);
	}
}
try {
	await finite(
		(async () => {
			await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
			browser = await chromium.launch({
				executablePath: executable,
				headless: true,
				args: ["--use-mock-keychain", "--password-store=basic"],
				timeout: 10000,
			});
			const context = await browser.newContext();
			const page = await context.newPage();
			const cdp = await context.newCDPSession(page);
			const version = await cdp.send("Browser.getVersion");
			await page.goto(`http://127.0.0.1:${server.address().port}/`, {
				waitUntil: "load",
				timeout: 5000,
			});
			const result = await page.evaluate(async (source) => {
				const peer = document.querySelector("iframe").contentWindow;
				const beginLoad = (image, good) =>
					new Promise((resolve) => {
						image.addEventListener("load", resolve, { once: true });
						image.addEventListener("error", resolve, { once: true });
						image.src = good
							? "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAMAAAACCAYAAACddGYaAAAAEUlEQVR4nGP4z8DwH4YZkDkAm34L9XKwuTwAAAAASUVORK5CYII="
							: "data:image/png;base64,AAAA";
					});
				return await (0, eval)(source)(peer, beginLoad);
			}, source);
			const report = {
				browser: version.product,
				version,
				probeSha256: createHash("sha256").update(source).digest("hex"),
				result,
			};
			writeFileSync(out, JSON.stringify(report, null, 2) + "\n");
			console.log(JSON.stringify(report));
		})(),
	);
} finally {
	if (browser) await browser.close();
	await new Promise((resolve) => server.close(resolve));
}

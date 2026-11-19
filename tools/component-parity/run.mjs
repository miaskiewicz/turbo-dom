import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve, extname } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs, promisify } from "node:util";
import { cases, measure, geometryDiff } from "./cases.mjs";
import { pixelDiff } from "./pixels.mjs";

const here = fileURLToPath(new URL("./", import.meta.url));
const fixtures = resolve(here, "fixtures");
const { values } = parseArgs({
	options: {
		native: { type: "string" },
		chrome: { type: "string" },
		out: { type: "string" },
		widths: { type: "string", default: "1440,768,390" },
		case: { type: "string" },
		assert: { type: "boolean" },
		"native-only": { type: "boolean" },
	},
});
if (!values.native)
	throw new Error("--native must name the built component_parity example");
const widths = values.widths.split(",").map(Number);
if (
	widths.some((width) => !Number.isInteger(width) || width < 1 || width > 16384)
)
	throw new Error("invalid viewport width");
const selected = cases.filter(
	(entry) => !values.case || entry.name === values.case,
);
if (!selected.length) throw new Error("unknown --case");
const out = resolve(values.out ?? "/tmp/turbo-dom-component-parity-results");
const native = resolve(values.native);
const reference = JSON.parse(
	await readFile(resolve(here, "chromium-reference.json")),
);
const sha256 = (data) => createHash("sha256").update(data).digest("hex");
const types = {
	".html": "text/html",
	".svg": "image/svg+xml",
	".png": "image/png",
};

async function serve(request, response) {
	const file = resolve(
		fixtures,
		`.${new URL(request.url, "http://localhost").pathname}`,
	);
	if (!file.startsWith(`${fixtures}/`)) {
		response.writeHead(403).end();
		return;
	}
	try {
		const body = await readFile(file);
		response.writeHead(200, {
			"content-type": types[extname(file)] ?? "application/octet-stream",
		});
		response.end(body);
	} catch {
		response.writeHead(404).end();
	}
}

async function chromeCapture(browser, origin, entry, width) {
	const page = await browser.newPage({
		viewport: { width, height: 1000 },
		deviceScaleFactor: 1,
	});
	const errors = [];
	page.on("pageerror", (error) => errors.push(error.message));
	await page.route("**/*", (route) =>
		new URL(route.request().url()).origin === origin
			? route.continue()
			: route.abort(),
	);
	try {
		await page.goto(`${origin}/${entry.name}.html`, {
			waitUntil: "networkidle",
		});
		await page.evaluate(() => document.fonts.ready);
		return {
			png: await page.screenshot(),
			boxes: await page.evaluate(measure, entry.selectors),
			errors,
		};
	} finally {
		await page.close();
	}
}

async function compare(browser, origin, entry, width) {
	const html = await readFile(resolve(fixtures, `${entry.name}.html`));
	const fixtureSha256 = sha256(html);
	const stem = resolve(out, `${entry.name}-${width}`);
	const recorded = reference.cases.find(
		(control) => control.name === entry.name && control.width === width,
	);
	if (values["native-only"] && recorded?.fixtureSha256 !== fixtureSha256)
		throw new Error(
			`missing/stale Chrome reference for ${entry.name}-${width}`,
		);
	const chrome = browser
		? await chromeCapture(browser, origin, entry, width)
		: { boxes: recorded.boxes, errors: [] };
	const request = `${stem}-request.json`;
	await writeFile(
		request,
		JSON.stringify({
			fixture: resolve(fixtures, `${entry.name}.html`),
			assets: resolve(fixtures, "assets"),
			output: `${stem}-native`,
			width,
			height: 1000,
			selectors: entry.selectors,
		}),
	);
	await promisify(execFile)(native, [request]);
	const boxes = JSON.parse(await readFile(`${stem}-native.json`));
	const differences = geometryDiff(chrome.boxes, boxes);
	const result = {
		name: entry.name,
		width,
		fixtureSha256,
		chromium: chrome.boxes,
		native: boxes,
		geometryDifferences: differences,
		chromiumErrors: chrome.errors,
	};
	if (chrome.png) {
		const pixels = pixelDiff(chrome.png, await readFile(`${stem}-native.png`));
		await writeFile(`${stem}-chromium.png`, chrome.png);
		await writeFile(`${stem}-diff.png`, pixels.png);
		result.changedFraction = pixels.changedFraction;
		result.meanChannelError = pixels.meanChannelError;
	}
	console.log(
		`${entry.name}-${width}: ${differences.length} geometry differences; changed pixels ${result.changedFraction ?? "not captured"}`,
	);
	return result;
}

await mkdir(out, { recursive: true });
const server = createServer(serve);
if (!values["native-only"]) {
	await new Promise((done, reject) => {
		server.once("error", reject);
		server.listen(0, "127.0.0.1", done);
	});
}
let browser;
try {
	if (!values["native-only"]) {
		const require = createRequire(import.meta.url);
		const { chromium } = require(
			process.env.PLAYWRIGHT_CORE ??
				resolve(here, "../../../turbo-surf/node_modules/playwright-core"),
		);
		browser = await chromium.launch({
			executablePath: values.chrome ?? process.env.TURBO_SURF_CHROME_PATH,
			headless: true,
		});
	}
	const results = [];
	for (const entry of selected) {
		for (const width of widths)
			results.push(
				await compare(
					browser,
					server.listening ? `http://127.0.0.1:${server.address().port}` : "",
					entry,
					width,
				),
			);
	}
	await writeFile(
		resolve(out, "results.json"),
		`${JSON.stringify({ browser: browser?.version() ?? reference.browser, nativeOnly: !!values["native-only"], nativeSha256: sha256(await readFile(native)), cargoLockSha256: sha256(await readFile(new URL("../../Cargo.lock", import.meta.url))), deviceScale: 1, viewportHeight: 1000, results }, null, 2)}\n`,
	);
	if (
		values.assert &&
		results.some(
			(result) =>
				result.geometryDifferences.length ||
				result.chromiumErrors.length ||
				(result.changedFraction ?? 0) > 0,
		)
	)
		process.exitCode = 1;
	console.log(`Evidence: ${out}`);
} finally {
	await browser?.close();
	if (server.listening) await new Promise((done) => server.close(done));
}

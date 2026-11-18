import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { cases, geometryDiff } from "./cases.mjs";
import { encodePng, pixelDiff } from "./pixels.mjs";

test("every frozen component has Chromium references tied to its exact fixture bytes", async () => {
	const reference = JSON.parse(
		await readFile(new URL("./chromium-reference.json", import.meta.url)),
	);
	assert.equal(reference.cases.length, cases.length * 3);
	for (const entry of cases) {
		const bytes = await readFile(
			new URL(`./fixtures/${entry.name}.html`, import.meta.url),
		);
		const hash = createHash("sha256").update(bytes).digest("hex");
		const controls = reference.cases.filter(
			(control) => control.name === entry.name,
		);
		assert.deepEqual(
			controls.map((control) => control.width),
			[1440, 768, 390],
		);
		for (const control of controls) {
			assert.equal(
				control.fixtureSha256,
				hash,
				`${entry.name}: regenerate stale Chromium evidence`,
			);
			assert.ok(control.boxes.length > 0);
		}
	}
});

test("geometry comparison catches wrapping, missing elements and extra elements", () => {
	const reference = [
		{ key: "a", rect: [0, 0, 100, 50] },
		{ key: "b", rect: [120, 0, 100, 50] },
	];
	assert.deepEqual(geometryDiff(reference, reference), []);
	const actual = [
		{ key: "a", rect: [0, 70, 100, 50] },
		{ key: "c", rect: [0, 0, 0, 0] },
	];
	assert.deepEqual(
		geometryDiff(reference, actual).map((entry) => entry.key),
		["a", "b", "c"],
	);
	assert.deepEqual(
		geometryDiff(
			reference,
			reference.map((entry) => ({
				...entry,
				rect: entry.rect.map((value) => value + 0.5),
			})),
		),
		[],
	);
});

test("pixel comparison records color changes and writes a decodable difference PNG", () => {
	const white = encodePng(2, 1, Buffer.from([0, 255, 255, 255, 255, 255, 255]));
	const oneBlack = encodePng(2, 1, Buffer.from([0, 0, 0, 0, 255, 255, 255]));
	assert.equal(pixelDiff(white, white).changedFraction, 0);
	const result = pixelDiff(white, oneBlack);
	assert.equal(result.changedPixels, 1);
	assert.equal(result.changedFraction, 0.5);
	assert.equal(result.meanChannelError, 127.5);
	assert.equal(pixelDiff(result.png, result.png).changedFraction, 0);
	assert.throws(
		() => pixelDiff(white, encodePng(1, 1, Buffer.from([0, 255, 255, 255]))),
		/identical viewport/,
	);
});

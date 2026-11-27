import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { createRealCssFixture } from "./css-shared-real.mjs";
import * as original from "../fixtures/css-shared-original18-159/fixture.mjs";
import * as api from "../fixtures/css-shared-api15-159/fixture.mjs";
const read = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url)));
const cases = read("../fixtures/css-shared-original18-159/cases.json");
const oracle = read("../fixtures/css-shared-original18-159/CHROME159.json");
const apiCases = read("../fixtures/css-shared-api15-159/cases.json");
const expectedApi = read("../fixtures/css-shared-api15-159/API-EXPECTED.json");
export function runOriginalControls(real) {
	assert.equal(oracle.browser, "159.0.8154.6");
	assert.equal(
		createHash("sha256")
			.update(
				readFileSync(
					new URL(
						"../fixtures/css-shared-original18-159/cases.json",
						import.meta.url,
					),
				),
			)
			.digest("hex"),
		oracle.casesSha256,
	);
	for (const scenario of cases) {
		const fixture = createRealCssFixture(real, original.caseHTML(scenario), {
			world: "original18:" + scenario.id,
		});
		try {
			const result = original.readCase(
				fixture.env.window,
				fixture.doc,
				scenario,
			);
			assert.deepEqual(
				result.values,
				oracle.result.find((row) => row.id === scenario.id).values,
				scenario.id,
			);
		} finally {
			fixture.dispose();
		}
	}
	for (const scenario of apiCases) {
		const fixture = createRealCssFixture(real, api.caseHTML(scenario), {
			world: "api15:" + scenario.id,
		});
		try {
			const result = api.readCase(fixture.env.window, fixture.doc, scenario);
			assert.deepEqual(
				result,
				expectedApi.results.find((row) => row.id === scenario.id),
				scenario.id,
			);
		} finally {
			fixture.dispose();
		}
	}
	if (real.mode !== "native") assert.deepEqual(real.nativeArtifacts(), []);
}

const boundaries = read("../fixtures/css-shared-boundaries-159/cases.json");
const boundaryOracle = read(
	"../fixtures/css-shared-boundaries-159/CHROME159.json",
);
export function runBoundaryControls(real) {
	assert.equal(boundaryOracle.browser, "159.0.8154.6");
	assert.equal(
		createHash("sha256")
			.update(
				readFileSync(
					new URL(
						"../fixtures/css-shared-boundaries-159/cases.json",
						import.meta.url,
					),
				),
			)
			.digest("hex"),
		boundaryOracle.casesSha256,
	);
	for (const scenario of boundaries) {
		const html =
			"<!doctype html><html><head><style>body{margin:0}" +
			scenario.css +
			"</style></head><body>" +
			scenario.html +
			"</body></html>";
		const fixture = createRealCssFixture(real, html, {
			world: "boundaries:" + scenario.id,
		});
		try {
			const target = fixture.doc.querySelector(".x"),
				authored = target.style.cssText,
				computed = fixture.adapter.get(target);
			const values = Object.fromEntries(
				scenario.properties.map((name) => [
					name,
					computed.getPropertyValue(name),
				]),
			);
			assert.deepEqual(
				values,
				boundaryOracle.results.find((row) => row.id === scenario.id).values,
				scenario.id,
			);
			assert.equal(target.style.cssText, authored);
		} finally {
			fixture.dispose();
		}
	}
}

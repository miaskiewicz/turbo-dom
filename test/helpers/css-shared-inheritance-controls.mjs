import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { createRealCssFixture } from "./css-shared-real.mjs";
import {
	caseHTML,
	readCase,
} from "../fixtures/css-shared-inheritance-159/fixture.mjs";
const cases = JSON.parse(
	readFileSync(
		new URL(
			"../fixtures/css-shared-inheritance-159/cases.json",
			import.meta.url,
		),
	),
);
const oracle = JSON.parse(
	readFileSync(
		new URL(
			"../fixtures/css-shared-inheritance-159/CHROME159.json",
			import.meta.url,
		),
	),
);
export function runInheritanceControls(real) {
	const mode = real.mode;
	assert.equal(oracle.browser, "159.0.8154.6");
	assert.equal(
		createHash("sha256")
			.update(
				readFileSync(
					new URL(
						"../fixtures/css-shared-inheritance-159/fixture.mjs",
						import.meta.url,
					),
				),
			)
			.digest("hex"),
		oracle.fixtureSha256,
	);

	for (const scenario of cases) {
		const fixture = createRealCssFixture(real, caseHTML(scenario), {
			world: scenario.id,
		});
		try {
			assert.equal(fixture.cssKernelMode, mode);
			assert.equal(
				fixture.htmlParserMode,
				mode === "native" ? "native" : "wasm",
			);
			const element = fixture.doc.querySelector(".x"),
				authored = element.style.cssText,
				style = fixture.adapter.get(element);
			style.getPropertyValue("color");
			assert.equal(fixture.counts().sceneCalls, 0);
			const result = readCase(fixture.env.window, fixture.doc, scenario);
			assert.deepEqual(
				result.values,
				oracle.results.find((row) => row.id === scenario.id).values,
			);
			assert.equal(fixture.counts().sceneCalls, 1);
			style.width;
			style.getPropertyValue("width");
			assert.equal(fixture.counts().sceneCalls, 1);
			assert.equal(element.style.cssText, authored);
		} finally {
			fixture.dispose();
		}
	}
	if (mode !== "native") assert.deepEqual(real.nativeArtifacts(), []);
}

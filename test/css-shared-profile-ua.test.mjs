import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import {
	loadRealCssBackend,
	createRealCssFixture,
} from "./helpers/css-shared-real.mjs";
import { html } from "../crates/turbo-dom-browser-env/tests/fixtures/default-profile-159/fixture.mjs";
const read = (name) =>
	readFileSync(
		new URL(
			`../crates/turbo-dom-browser-env/tests/fixtures/default-profile-159/${name}`,
			import.meta.url,
		),
	);
const cases = JSON.parse(read("cases.json")),
	oracle = JSON.parse(read("CHROME159.json"));
assert.equal(oracle.browser, "159.0.8154.6");
assert.equal(
	createHash("sha256").update(read("cases.json")).digest("hex"),
	oracle.casesSha256,
);
assert.equal(
	createHash("sha256").update(read("fixture.mjs")).digest("hex"),
	oracle.fixtureSha256,
);
const text = (units) => {
	let out = "";
	for (let i = 0; i < units.length; i++) out += String.fromCharCode(units[i]);
	return out;
};
const units = (value) => value.split("").map((char) => char.charCodeAt(0));
// Explicit test policy, not Node runtime defaults or a CSS measurement source.
const limits = {
	inputUnits: 1024,
	retainedUnits: 100_000,
	computation: { maxNesting: 32, maxOutputUnits: 1024, maxTotalUnits: 2048 },
};
function counted(real) {
	const counts = { legacyUa: 0, prefs: 0, compiled: 0 };
	const backend = {
		...real.backend,
		cssUserAgentSourceV1(...args) {
			counts.legacyUa++;
			return real.backend.cssUserAgentSourceV1(...args);
		},
		cssComputedPreferencesV1(...args) {
			counts.prefs++;
			return real.backend.cssComputedPreferencesV1(...args);
		},
		CompiledStylesV1: class {
			constructor(...args) {
				counts.compiled++;
				return new real.backend.CompiledStylesV1(...args);
			}
		},
	};
	return { real: { ...real, backend }, counts };
}
function profileFor(real) {
	let override = null;
	const cache = new WeakMap();
	const prefs = (backend) => {
		const key = JSON.stringify(override),
			capability = backend.cssComputedPreferencesV1,
			old = cache.get(backend);
		if (old && old.key === key && old.capability === capability)
			return old.response;
		assert.equal(
			typeof capability,
			"function",
			"Actual preferences capability is required",
		);
		const response = JSON.parse(
			capability.call(
				backend,
				JSON.stringify({
					abiVersion: 1,
					hostPlatform: "macos",
					fontOverride: override,
					limits,
				}),
			),
		);
		cache.set(backend, { key, capability, response });
		return response;
	};
	const profile = {
		userAgentSource: (backend) => prefs(backend).userAgentSource,
		targetContext(_row, { root }) {
			const initial = prefs(real.backend).initial;
			return {
				initialColor: text(initial.color),
				initialFontFamily: text(initial.fontFamily),
				initialFontSize: text(initial.fontSizeInput),
				unitBases: [],
				fontSizeUnitBases: [],
				isRoot: root,
			};
		},
	};
	return {
		profile,
		prefs,
		contextTransform: (context) => ({
			...context,
			environmentKey: text(prefs(real.backend).preferenceIdentity),
		}),
		setFamily(family) {
			override = { family: units(family), sizePx: 16 };
		},
	};
}
for (const mode of ["native", "node-wasm", "web-wasm"]) {
	test(`actual ${mode}: shared document UA and Core context match six Chrome159 initial/inherit font/color controls`, async () => {
		const actual = await loadRealCssBackend(mode);
		for (const scenario of cases.slice(0, 6)) {
			const metered = counted(actual),
				state = profileFor(metered.real),
				fixture = createRealCssFixture(metered.real, html(scenario), {
					profile: state.profile,
					contextTransform: state.contextTransform,
				});
			try {
				const style = fixture.adapter.get(fixture.doc.querySelector(".x"));
				const expected = oracle.results.find((row) => row.id === scenario.id)
					.initial.values;
				for (const name of scenario.properties)
					assert.equal(
						style.getPropertyValue(name),
						expected[name],
						`${mode}/${scenario.id}/${name}`,
					);
				assert.equal(metered.counts.legacyUa, 0);
				assert.equal(metered.counts.prefs, 1);
				assert.equal(fixture.counts().sceneCalls, 0);
			} finally {
				fixture.dispose();
			}
		}
	});
	test(`actual ${mode}: owned prefs UA avoids warm bridge calls and respects reset, backend and preference identity`, async () => {
		const actual = await loadRealCssBackend(mode),
			metered = counted(actual),
			state = profileFor(metered.real),
			fixture = createRealCssFixture(
				metered.real,
				"<style>.x{font-family:inherit}</style><div class=x>x</div>",
				{ profile: state.profile, contextTransform: state.contextTransform },
			);
		try {
			const element = fixture.doc.querySelector(".x"),
				old = fixture.adapter.get(element);
			assert.equal(old.fontFamily, "Times");
			assert.equal(fixture.adapter.get(element).fontFamily, "Times");
			assert.deepEqual(metered.counts, { legacyUa: 0, prefs: 1, compiled: 1 });
			fixture.adapter.reset(fixture.doc);
			assert.equal(fixture.adapter.get(element).fontFamily, "Times");
			assert.equal(metered.counts.prefs, 1);
			assert.equal(metered.counts.compiled, 2);
			assert.equal(old.fontFamily, "Times");
			state.setFamily("monospace");
			assert.equal(fixture.adapter.get(element).fontFamily, "monospace");
			assert.equal(metered.counts.prefs, 2);
			assert.equal(metered.counts.compiled, 2);
			const previousCapability = metered.real.backend.cssComputedPreferencesV1;
			metered.real.backend.cssComputedPreferencesV1 = (...args) =>
				previousCapability(...args);
			assert.equal(fixture.adapter.get(element).fontFamily, "monospace");
			assert.equal(metered.counts.prefs, 3);
			assert.equal(metered.counts.compiled, 2);
			const replacement = counted(actual);
			metered.real.backend = replacement.real.backend;
			assert.equal(fixture.adapter.get(element).fontFamily, "monospace");
			assert.equal(replacement.counts.prefs, 1);
			assert.equal(replacement.counts.compiled, 1);
			assert.equal(metered.counts.legacyUa, 0);
			assert.equal(replacement.counts.legacyUa, 0);
		} finally {
			fixture.dispose();
		}
	});
}
test("optional UA hook leaves absent legacy calls intact and does not freeze changed custom UA callbacks", async () => {
	const actual = await loadRealCssBackend("native"),
		old = counted(actual),
		legacy = createRealCssFixture(
			old.real,
			"<style>.x{font-family:inherit}</style><div class=x>x</div>",
		);
	try {
		const el = legacy.doc.querySelector(".x");
		legacy.adapter.get(el).fontFamily;
		legacy.adapter.get(el).fontFamily;
		assert.equal(old.counts.legacyUa, 2);
	} finally {
		legacy.dispose();
	}
	const metered = counted(actual),
		state = profileFor(metered.real),
		base = state.prefs(metered.real.backend).userAgentSource;
	let current = base;
	state.profile.userAgentSource = () => current;
	const fixture = createRealCssFixture(
		metered.real,
		"<style>.x{font-family:inherit}</style><div class=x>x</div>",
		{ profile: state.profile, contextTransform: state.contextTransform },
	);
	try {
		const el = fixture.doc.querySelector(".x");
		assert.equal(fixture.adapter.get(el).fontFamily, "Times");
		current = {
			...base,
			text: units(text(base.text) + "\nbody{font-family:monospace}"),
		};
		assert.equal(fixture.adapter.get(el).fontFamily, "monospace");
		assert.equal(metered.counts.compiled, 2);
		assert.equal(metered.counts.legacyUa, 0);
	} finally {
		fixture.dispose();
	}
});
test("optional UA provider errors and malformed ABI are visible without old-export fallback", async () => {
	const actual = await loadRealCssBackend("native"),
		metered = counted(actual),
		original = Error("host UA provider failed");
	for (const [provider, check] of [
		[
			() => {
				throw original;
			},
			(error) => error === original,
		],
		[() => ({ abiVersion: 2 }), /CSS user agent ABI/],
		[42, /profile.userAgentSource/],
	]) {
		const fixture = createRealCssFixture(metered.real, "<div class=x>x</div>", {
			profile: { userAgentSource: provider },
		});
		try {
			assert.throws(
				() => fixture.adapter.get(fixture.doc.querySelector(".x")),
				check,
			);
		} finally {
			fixture.dispose();
		}
	}
	assert.equal(metered.counts.legacyUa, 0);
});

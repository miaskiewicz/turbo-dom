import {
	runOriginalControls,
	runBoundaryControls,
} from "./helpers/css-shared-corpus-controls.mjs";
import { runInheritanceControls } from "./helpers/css-shared-inheritance-controls.mjs";
import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import {
	loadRealCssBackend,
	createRealCssFixture,
} from "./helpers/css-shared-real.mjs";
import {
	caseHTML,
	readCase,
} from "./fixtures/css-shared-inheritance-159/fixture.mjs";
const cases = JSON.parse(
	readFileSync(
		new URL(
			"./fixtures/css-shared-inheritance-159/cases.json",
			import.meta.url,
		),
	),
);
const oracle = JSON.parse(
	readFileSync(
		new URL(
			"./fixtures/css-shared-inheritance-159/CHROME159.json",
			import.meta.url,
		),
	),
);
assert.equal(oracle.browser, "159.0.8154.6");
assert.equal(
	createHash("sha256")
		.update(
			readFileSync(
				new URL(
					"./fixtures/css-shared-inheritance-159/fixture.mjs",
					import.meta.url,
				),
			),
		)
		.digest("hex"),
	oracle.fixtureSha256,
);
test("actual native: computed inheritance, Scene used values and read laziness", async () =>
	runInheritanceControls(await loadRealCssBackend("native")));

test("actual native: retained primitive presentation survives reset/dispose and later connected mutations", async () => {
	const real = await loadRealCssBackend("native");
	const fixture = createRealCssFixture(real, caseHTML(cases[0]));
	try {
		const element = fixture.doc.querySelector(".x"),
			old = fixture.adapter.get(element);
		fixture.adapter.reset(fixture.doc);
		element.style.width = "25%";
		assert.equal(old.width, "400px");
		assert.equal(fixture.adapter.get(element).width, "200px");
		const deferred = fixture.adapter.get(element);
		fixture.adapter.dispose();
		assert.throws(() => fixture.adapter.get(element), /disposed/);
		assert.equal(old.width, "400px");
		assert.equal(deferred.width, "200px");
	} finally {
		fixture.dispose();
	}
});
test("actual native: provider exceptions and caller computation quotas stay visible", async () => {
	const real = await loadRealCssBackend("native"),
		original = Error("actual provider failure");
	const failing = createRealCssFixture(real, caseHTML(cases[0]), {
		sceneFailure: original,
	});
	try {
		const style = failing.adapter.get(failing.doc.querySelector(".x"));
		assert.throws(
			() => style.width,
			(error) => error === original,
		);
	} finally {
		failing.dispose();
	}
	const bounded = createRealCssFixture(real, caseHTML(cases[0]), {
		policy: {
			compile: (sources) => ({
				sources: sources.length,
				sourceUnits: 0,
				rules: 0,
				sourceRulePreludes: 0,
				selectors: 0,
				declarations: 0,
				properties: 0,
				groupDepth: 0,
				tokenNesting: 0,
			}),
		},
	});
	try {
		assert.throws(
			() => bounded.adapter.get(bounded.doc.querySelector(".x")),
			(error) => error.message === "CSSBackendCompile:SourceUnits",
		);
	} finally {
		bounded.dispose();
	}
});

test("actual native: original18 Chrome and published API15 corpus", async () =>
	runOriginalControls(await loadRealCssBackend("native")));

test("actual native: measured scope limit, box absence, height and constraints", async () =>
	runBoundaryControls(await loadRealCssBackend("native")));

test("actual native: unavailable and independently unknown Scene metrics never become fabricated widths", async () => {
	const real = await loadRealCssBackend("native");
	for (const [change, message] of [
		[
			(rows) =>
				rows.map((row) => ({
					...row,
					result: {
						status: "unavailable",
						reason: "missing_box",
						detail: null,
					},
				})),
			/CanonicalUsedMeasurementUnavailable/,
		],
		[
			(rows) =>
				rows.map((row) => ({
					...row,
					result: {
						...row.result,
						geometry: {
							...row.result.geometry,
							contentBox: { ...row.result.geometry.contentBox, width: null },
						},
					},
				})),
			/MissingCanonicalUsedMeasurement/,
		],
	]) {
		const fixture = createRealCssFixture(real, caseHTML(cases[0]), {
			sceneTransform: change,
		});
		try {
			const style = fixture.adapter.get(fixture.doc.querySelector(".x"));
			assert.throws(() => style.width, message);
		} finally {
			fixture.dispose();
		}
	}
});

test("actual native: malformed projection endpoint metadata is rejected before Scene work", async () => {
	const actual = await loadRealCssBackend("native");
	const backend = {
		...actual.backend,
		projectCssomPropertyV1(request) {
			const response = JSON.parse(
				actual.backend.projectCssomPropertyV1(request),
			);
			response.result = {
				status: "needsMeasurement",
				axis: "unexpected",
				basis: "content",
			};
			return JSON.stringify(response);
		},
	};
	const fixture = createRealCssFixture(
		{ ...actual, backend },
		caseHTML(cases[0]),
	);
	try {
		const style = fixture.adapter.get(fixture.doc.querySelector(".x"));
		assert.throws(() => style.width, /CSS used projection axis/);
		assert.equal(fixture.counts().sceneCalls, 0);
	} finally {
		fixture.dispose();
	}
});

test("actual native: published array-backed stylesheet media and disabled state are recaptured without source changes", async () => {
	const real = await loadRealCssBackend("native");
	const fixture = createRealCssFixture(
		real,
		'<!doctype html><html><head><style>.x{color:red}</style><style id="conditional" media="print">.x{color:blue}</style></head><body><div class=x></div></body></html>',
	);
	try {
		const target = fixture.doc.querySelector(".x"),
			sheet = fixture.doc.getElementById("conditional");
		assert.equal(fixture.adapter.get(target).color, "rgb(255, 0, 0)");
		sheet.setAttribute("media", "screen");
		assert.equal(fixture.adapter.get(target).color, "rgb(0, 0, 255)");
		sheet.sheet.disabled = true;
		assert.equal(fixture.adapter.get(target).color, "rgb(255, 0, 0)");
		sheet.sheet.disabled = false;
		sheet.sheet.media.push("print");
		assert.equal(fixture.adapter.get(target).color, "rgb(255, 0, 0)");
		sheet.sheet.media.splice(0, 1, "screen");
		assert.equal(fixture.adapter.get(target).color, "rgb(0, 0, 255)");
		assert.equal(fixture.counts().sceneCalls, 0);
	} finally {
		fixture.dispose();
	}
});

test("actual native: optional UA document source and requested pseudo context stay explicit", async () => {
	const actual = await loadRealCssBackend("native");
	const backend = {
		...actual.backend,
		cssUserAgentSourceV1() {
			const ua = JSON.parse(actual.backend.cssUserAgentSourceV1());
			delete ua.documentText;
			return JSON.stringify(ua);
		},
	};
	const fixture = createRealCssFixture(
		{ ...actual, backend },
		'<!doctype html><html><head><style>.x{color:red}.x::before{width:10px;content:"x"}</style></head><body><div class=x></div></body></html>',
	);
	try {
		const target = fixture.doc.querySelector(".x");
		assert.equal(fixture.adapter.get(target).color, "rgb(255, 0, 0)");
		const pseudo = fixture.adapter.get(target, "::before");
		assert.throws(() => pseudo.width, /UnsupportedPseudo/);
		assert.equal(fixture.counts().sceneCalls, 0);
	} finally {
		fixture.dispose();
	}
});

test("actual native: changed backend identity and invalid result identities cannot reuse retained views", async () => {
	const real = await loadRealCssBackend("native"),
		fixture = createRealCssFixture(real, caseHTML(cases[0]));
	try {
		const view = fixture.adapter.get(fixture.doc.querySelector(".x"));
		real.artifact += ":new-generation";
		assert.throws(() => view.width, /CSS property backend changed/);
	} finally {
		fixture.dispose();
	}
	const actual = await loadRealCssBackend("native");
	for (const field of ["abiVersion", "id", "property"]) {
		const backend = {
			...actual.backend,
			projectCssomPropertyV1(request) {
				const response = JSON.parse(
					actual.backend.projectCssomPropertyV1(request),
				);
				response[field] = field === "property" ? "different" : 999;
				return JSON.stringify(response);
			},
		};
		const fixture = createRealCssFixture(
			{ ...actual, backend },
			caseHTML(cases[0]),
		);
		try {
			const view = fixture.adapter.get(fixture.doc.querySelector(".x"));
			assert.throws(() => view.width, /CSS used projection identity/);
		} finally {
			fixture.dispose();
		}
	}
});

test("actual native: serialization and UA ABI mismatches preserve the API boundary", async () => {
	const actual = await loadRealCssBackend("native");
	for (const kind of ["abiVersion", "targets", "id", "ua"]) {
		const backend = { ...actual.backend };
		if (kind === "ua")
			backend.cssUserAgentSourceV1 = () => {
				const row = JSON.parse(actual.backend.cssUserAgentSourceV1());
				row.abiVersion = 2;
				return JSON.stringify(row);
			};
		else
			backend.serializeCssomV1 = (request) => {
				const response = JSON.parse(actual.backend.serializeCssomV1(request));
				if (kind === "abiVersion") response.abiVersion = 2;
				else if (kind === "targets") response.targets = [];
				else response.targets[0].id = 999;
				return JSON.stringify(response);
			};
		const fixture = createRealCssFixture(
			{ ...actual, backend },
			caseHTML(cases[0]),
		);
		try {
			assert.throws(
				() => fixture.adapter.get(fixture.doc.querySelector(".x")),
				kind === "ua" ? /CSS user agent ABI/ : /CSS serialization identity/,
			);
		} finally {
			fixture.dispose();
		}
	}
});

test("actual native: inconsistent caller snapshot ownership and unsupported shadow world are not inferred", async () => {
	const real = await loadRealCssBackend("native");
	for (const [kind, message] of [
		["root", /CSS original document element missing/],
		["subject", /CSS original subject missing/],
		["pseudo", /CSS original pseudo subject missing/],
	]) {
		const fixture = createRealCssFixture(real, caseHTML(cases[0]), {
			snapshotTransform: (snapshot) => ({
				...snapshot,
				records: snapshot.records.filter((row) =>
					kind === "root"
						? !(row.kind === "element" && row.parent === snapshot.documentId)
						: row.element.className !== "x",
				),
			}),
		});
		try {
			assert.throws(
				() =>
					fixture.adapter.get(
						fixture.doc.querySelector(".x"),
						kind === "pseudo" ? "before" : null,
					),
				message,
			);
		} finally {
			fixture.dispose();
		}
	}
	const fixture = createRealCssFixture(real, caseHTML(cases[0]));
	try {
		fixture.doc.querySelector(".x").attachShadow({ mode: "open" });
		assert.throws(
			() => fixture.adapter.get(fixture.doc.querySelector(".x")),
			/shadow matching context unsupported/,
		);
	} finally {
		fixture.dispose();
	}
});

test("actual native: explicit scope reference is routed through the compiled metadata, not inferred ancestry", async () => {
	const real = await loadRealCssBackend("native");
	const oracle = JSON.parse(
		readFileSync(
			new URL(
				"./fixtures/css-shared-boundaries-159/SCOPE-LAYER-ORACLE.json",
				import.meta.url,
			),
		),
	);
	assert.equal(oracle.browser, "159.0.8154.6");
	for (const id of ["scope-anchor", "scope-functional-branch"]) {
		const scenario = oracle.cases.find((row) => row.id === id),
			fixture = createRealCssFixture(
				real,
				"<!doctype html><html><head><style>" +
					scenario.css +
					"</style></head><body>" +
					scenario.html +
					"</body></html>",
			);
		try {
			assert.equal(
				fixture.adapter.get(fixture.doc.querySelector(".target")).color,
				oracle.result.find((row) => row.id === id).color,
			);
		} finally {
			fixture.dispose();
		}
	}
});

test("an explicit missing CSS payload fails instead of falling back to packaged/native parser artifacts", async () => {
	const previous = process.env.TURBO_DOM_CSS_TEST_NATIVE;
	process.env.TURBO_DOM_CSS_TEST_NATIVE =
		"/__turbo_dom_missing_css_payload__/must-not-fallback.node";
	try {
		await assert.rejects(
			loadRealCssBackend("native"),
			/Required actual CSS test artifact missing: TURBO_DOM_CSS_TEST_NATIVE/,
		);
	} finally {
		if (previous === undefined) delete process.env.TURBO_DOM_CSS_TEST_NATIVE;
		else process.env.TURBO_DOM_CSS_TEST_NATIVE = previous;
	}
});

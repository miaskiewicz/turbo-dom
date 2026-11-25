import {
	nativeCssSuffix,
	detectNativeMusl,
	normalizeCssArtifact,
} from "../../src/runtime/css-artifact.mjs";
import { getParserSelection } from "../../src/runtime/parser.mjs";
// Explicit fixture policy, not runtime defaults or authored-value geometry.
import { readFileSync, existsSync } from "node:fs";
import { createRequire } from "node:module";
import { pathToFileURL, fileURLToPath } from "node:url";
import { createSharedCssAdapter } from "../../src/runtime/css-shared-adapter.mjs";
import { createSharedSceneProvider } from "../../src/runtime/css-shared-scene.mjs";
import { createEnvironment, setParserMode } from "../../src/runtime/index.mjs";
import {
	cssStateSnapshot,
	ensureCssState,
} from "../../src/runtime/css-invalidation.mjs";
import { createTestPublicCssFacts } from "./css-shared-public-facts.mjs";
const require = createRequire(import.meta.url);
export const fixtureViewport = { width: 1024, height: 768 };
const computation = {
	maxNesting: 32,
	maxOutputUnits: 8192,
	maxTotalUnits: 16384,
};
const snapshotLimits = {
	maxNodes: 256,
	maxDepth: 64,
	maxPathUnits: 16384,
	maxTextUnits: 262144,
	maxAttributes: 1024,
	maxAttributeUnits: 65536,
};
const environment = {
	mediaType: "screen",
	features: {
		width: { kind: "lengthPx", value: 1024 },
		height: { kind: "lengthPx", value: 768 },
		orientation: { kind: "ident", value: "landscape" },
	},
	unitBases: [
		["em", 16],
		["rem", 16],
		["vw", 10.24],
		["vh", 7.68],
		["vmin", 7.68],
		["vmax", 10.24],
	],
	selectorSupport: {},
	stylePhase: "steady",
};
const bases = [
	["vw", 10.24],
	["vh", 7.68],
	["vmin", 7.68],
	["vmax", 10.24],
];
function compile(sources) {
	const units = sources.reduce((sum, row) => sum + row.text.length, 0);
	return {
		sources: sources.length,
		sourceUnits: units,
		rules: units,
		sourceRulePreludes: units,
		selectors: units,
		declarations: units,
		properties: units,
		groupDepth: 32,
		tokenNesting: 32,
	};
}
function resolve({ snapshot, targets, mechanical, request, pathNodes }) {
	if (request) return { ...request.limits, pathNodes };
	return {
		targets: targets.length,
		routingContexts: targets.length * snapshot.roots.length,
		answerRows:
			mechanical.domains.length +
			mechanical.selectorAnswers.length +
			mechanical.scopeAnswers.length,
		answerIds:
			mechanical.domains.reduce((sum, row) => sum + row.nodeIds.length, 0) +
			[...mechanical.selectorAnswers, ...mechanical.scopeAnswers].reduce(
				(sum, row) => sum + row.matches.length,
				0,
			),
		pathNodes: targets.reduce((sum, row) => sum + row.path.length, 0),
		inputUnits: 8192,
		retainedResultUnits: 16384,
		matchingDecisions: 16384,
		inlineDeclarations: 128,
		inlineProperties: 1024,
		customRollbacks: 128,
		valueRetainedUnits: 8192,
		computation,
	};
}
function serialize(target) {
	return {
		targets: 1,
		inputUnits:
			target.values.reduce(
				(sum, [name, value]) => sum + name.length + value.length,
				0,
			) +
			target.publicKeys.reduce((sum, name) => sum + name.length, 0) +
			(target.declaredTextDecoration
				? target.declaredTextDecoration.value.length +
					target.declaredTextDecoration.origin.length
				: 0),
		retainedUnits: 16384,
		computation,
	};
}
function property({ target, property, measurement }) {
	return {
		inputUnits:
			property.length +
			target.world.length +
			target.namespace.length +
			(target.pseudo?.length || 0) +
			target.ordinary.reduce(
				(sum, [name, value]) => sum + name.length + value.length,
				0,
			) +
			target.publicKeys.reduce((sum, name) => sum + name.length, 0) +
			(measurement?.world.length || 0),
		retainedUnits: 16384,
		computation,
	};
}
function quotas() {
	return {
		compile,
		prepare: ({ sources, uses, ua }) => ({
			uses: uses.length,
			rules:
				ua.text.length +
				(ua.documentText?.length || 0) +
				sources.sources.reduce((sum, row) => sum + row.text.length, 0),
			layers: 16384,
			computation,
		}),
		matching: ({ snapshot, selectorQueries, scopeQueries }) => ({
			maxNodes: snapshot.matchNodes.length,
			maxAnswers:
				(selectorQueries.length + scopeQueries.length) *
					(snapshot.matchNodes.length + 1) ** 2 +
				snapshot.matchNodes.length,
			maxAncestorDepth: snapshot.used.maxNodes,
		}),
		resolve,
		serialize,
		property,
		providerSnapshot: () => ({
			maxDepth: snapshotLimits.maxDepth,
			maxEntries:
				snapshotLimits.maxPathUnits +
				snapshotLimits.maxTextUnits +
				snapshotLimits.maxAttributeUnits,
		}),
	};
}
function packagedArtifact(mode) {
	if (mode === "node-wasm")
		return fileURLToPath(
			new URL("../../pkg-css/turbo_dom_css.js", import.meta.url),
		);
	if (mode === "web-wasm")
		return fileURLToPath(
			new URL("../../pkg-css-web/turbo_dom_css.js", import.meta.url),
		);
	const suffix = nativeCssSuffix(
		process.platform,
		process.arch,
		process.platform === "linux" && detectNativeMusl(require),
	);
	return suffix
		? fileURLToPath(
				new URL(
					`../../native-css/turbo-dom-css.${suffix}.node`,
					import.meta.url,
				),
			)
		: null;
}
export async function loadRealCssBackend(mode) {
	const variable = {
		native: "TURBO_DOM_CSS_TEST_NATIVE",
		"node-wasm": "TURBO_DOM_CSS_TEST_WASM_NODE",
		"web-wasm": "TURBO_DOM_CSS_TEST_WASM_WEB",
	}[mode];
	const artifact = process.env[variable] || packagedArtifact(mode);
	if (!artifact || !existsSync(artifact))
		throw new Error(
			`Required actual CSS test artifact missing: ${variable} or packaged path ${artifact}`,
		);
	let backend;
	if (mode === "web-wasm") {
		backend = await import(pathToFileURL(artifact));
		backend.initSync({
			module: readFileSync(
				new URL("./turbo_dom_css_bg.wasm", pathToFileURL(artifact)),
			),
		});
	} else backend = require(artifact);
	if (normalizeCssArtifact(backend) === null)
		throw new Error("Required actual CSS artifact metadata missing");
	for (const name of [
		"CompiledStylesV1",
		"serializeCssomV1",
		"projectCssomPropertyV1",
		"measureSceneV1",
		"cssUserAgentSourceV1",
	])
		if (typeof backend[name] !== "function")
			throw new Error(`Required actual CSS capability missing: ${name}`);
	return {
		backend,
		mode,
		artifact,
		nativeArtifacts: () =>
			Object.keys(require.cache).filter((path) => path.endsWith(".node")),
	};
}
function selectHtmlParser(mode) {
	const selected = mode === "native" ? "native" : "wasm";
	setParserMode(selected);
	if (getParserSelection().mode !== selected)
		throw new Error("Explicit HTML parser mode was not selected");
	return selected;
}
export function createRealCssFixture(
	real,
	html,
	{
		world = "real-css-fixture",
		sceneFailure = null,
		sceneTransform = null,
		snapshotTransform = null,
		policy = {},
	} = {},
) {
	const htmlParserMode = selectHtmlParser(real.mode);
	const env = createEnvironment(html),
		doc = env.document,
		reader = createTestPublicCssFacts(),
		environmentKey = {};
	let sceneCalls = 0,
		snapshotCalls = 0;
	const measure = createSharedSceneProvider({
		selectBackend: () => real.backend,
		viewport: fixtureViewport,
		fonts: real.mode === "native" ? "native_html_system" : "bundled_html",
		limits: ({ snapshot, nodes, requested, world, counts }) => ({
			maxNodes: nodes.length,
			maxDepth: snapshotLimits.maxDepth,
			maxPathUnits: snapshotLimits.maxPathUnits,
			maxTextUnits: snapshot.used.maxTextUnits,
			maxAttributes: snapshot.used.maxAttributes,
			maxAttributeUnits: snapshot.used.maxAttributeUnits,
			maxProperties: counts.properties,
			maxStyleUnits: counts.styleUnits,
			maxWorldUnits: world.length,
			maxRequests: requested.length,
		}),
	});
	const limits = { ...quotas(), ...policy };
	const adapter = createSharedCssAdapter({
		selectBackend: () => real.backend,
		readContext(_element, pseudo) {
			ensureCssState(doc);
			return {
				document: doc,
				version: doc.__version,
				renderToken: cssStateSnapshot(doc),
				environmentKey,
				resultKey: { pseudo },
			};
		},
		captureSnapshot(document, context) {
			snapshotCalls++;
			const snapshot = reader.capture(document, snapshotLimits, context);
			return snapshotTransform ? snapshotTransform(snapshot) : snapshot;
		},
		profile: {
			textDecoration: "declared",
			environment: () => environment,
			scopeDepth: () => 0,
			targetContext: (_row, { root }) => ({
				initialColor: "rgb(0, 0, 0)",
				initialFontFamily: "serif",
				initialFontSize: "16px",
				unitBases: bases,
				fontSizeUnitBases: [],
				isRoot: root,
			}),
			world: () => world,
			backendIdentity: () => real.artifact,
		},
		limits,
		measureScene(request) {
			sceneCalls++;
			if (sceneFailure) throw sceneFailure;
			const rows = measure(request);
			return sceneTransform ? sceneTransform(rows) : rows;
		},
	});
	env.window.getComputedStyle = (element, pseudo) =>
		adapter.get(element, pseudo);
	return {
		env,
		doc,
		cssKernelMode: real.mode,
		htmlParserMode,
		adapter,
		counts: () => ({ sceneCalls, snapshotCalls }),
		dispose: () => adapter.dispose(),
	};
}

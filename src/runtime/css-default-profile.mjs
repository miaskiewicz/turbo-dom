import { ensureCssState } from "./css-invalidation.mjs";
// Default host facts are supplied by the trusted Window owner. All CSS preference,
// initial-value, media-feature and physical-unit policy remains in shared Rust.
import { nativeDocument } from "./css-default-capture.mjs";
import { createSharedSceneProvider } from "./css-shared-scene.mjs";
import {
	sharedEncode,
	sharedDecode,
	sharedInvoke,
	sharedText,
} from "./css-shared-codec.mjs";
const NativeWeakMap = WeakMap,
	weakGet = WeakMap.prototype.get,
	weakSet = WeakMap.prototype.set,
	finite = Number.isFinite,
	maximumFrame = Number.MAX_SAFE_INTEGER;
function weakRead(map, key) {
	return sharedInvoke(weakGet, map, [key]);
}
function weakWrite(map, key, value) {
	sharedInvoke(weakSet, map, [key, value]);
}
function required(value, label) {
	if (typeof value !== "function")
		throw new TypeError(`CSS default profile missing ${label}`);
	return value;
}
function invoke(backend, name, input, decodeResult) {
	const result = decodeResult(
		sharedInvoke(required(backend[name], name), backend, [sharedEncode(input)]),
        name,
	);
	if (result.abiVersion !== 1)
		throw new TypeError(`CSS default profile ${name} ABI`);
	return result;
}
function axis(value) {
	return value === undefined
		? { status: "unknown" }
		: { status: "known", value };
}
function hostInput(facts, limits) {
	if (facts.hostPlatform !== "macos" && facts.hostPlatform !== "other")
		throw new TypeError("CSS default profile host platform unavailable");
	return {
		abiVersion: 1,
		hostPlatform: facts.hostPlatform,
		fontOverride: facts.fontOverride ?? null,
		limits,
	};
}
function contextInitial(preferences, environment, root) {
	return {
		initialColor: sharedText(preferences.initial.color),
		initialFontFamily: sharedText(preferences.initial.fontFamily),
		initialFontSize: sharedText(preferences.initial.fontSizeInput),
		unitBases: environment.valueUnitBases,
		fontSizeUnitBases: [],
		isRoot: root,
	};
}
/** Internal policy is explicit and Rust-owned by the caller. No test-fixture
 * budgets, font defaults or fallback CSS backend are installed here. */
export function createDefaultCssProfile({
	backend,
	capture,
	readWindowContext,
	resourcePolicy,
  decodeResult = sharedDecode,
}) {
	required(readWindowContext, "readWindowContext");
	required(capture?.capture, "capture.capture");
	required(capture?.sources, "capture.sources");
	required(capture?.selectorFacts, "capture.selectorFacts");
	if (!resourcePolicy)
		throw new TypeError("CSS default resource policy unavailable");
	const limits = resourcePolicy.adapter;
	required(resourcePolicy.preferences, "resourcePolicy.preferences");
	required(resourcePolicy.environment, "resourcePolicy.environment");
	required(resourcePolicy.snapshot, "resourcePolicy.snapshot");
	required(resourcePolicy.selectorFacts, "resourcePolicy.selectorFacts");
	required(resourcePolicy.scene, "resourcePolicy.scene");
	required(resourcePolicy.composition, "resourcePolicy.composition");
	const contexts = new NativeWeakMap(),
		snapshots = new NativeWeakMap();
	let preferenceKey = null,
		preferences = null,
		environmentKeyForBackend = null,
		cachedEnvironment = null,
		frame = 0;
	function readEnvironment(facts) {
		const input = {
			...hostInput(facts, resourcePolicy.environment(facts)),
			viewport: { width: axis(facts.width), height: axis(facts.height) },
		};
		const key = sharedEncode([input, preferences.preferenceIdentity]);
		if (key === environmentKeyForBackend) return cachedEnvironment;
		const environment = invoke(backend, "cssComputedEnvironmentV1", input, decodeResult);
		if (
			sharedEncode(preferences.preferenceIdentity) !==
			sharedEncode(environment.preferenceIdentity)
		)
			throw new Error("CSS default preference identity changed");
		environmentKeyForBackend = key;
		cachedEnvironment = environment;
		return environment;
	}
	function readContext(element, pseudo) {
		const document = nativeDocument(element);
		ensureCssState(document);
		const facts = readWindowContext(document);
		const input = hostInput(facts, resourcePolicy.preferences(facts));
		const key = sharedEncode(input);
		if (key !== preferenceKey) {
			preferences = invoke(backend, "cssComputedPreferencesV1", input, decodeResult);
			preferenceKey = key;
		}
		const environment = readEnvironment(facts);
		if (frame === maximumFrame)
			throw new RangeError("CSS default frame identity exhausted");
		const environmentKey = sharedEncode({
			identity: preferences.preferenceIdentity,
			viewport: [axis(facts.width), axis(facts.height)],
			devicePixelRatio: facts.devicePixelRatio,
			mediaType: facts.mediaType,
		});
		const context = {
			document,
			version: ++frame,
			renderToken: null,
			environmentKey,
			resultKey: { frame, pseudo: pseudo || null },
			facts,
			preferences,
			environment,
		};
		return context;
	}
	function captureSnapshot(document, context) {
		if (document !== context.document)
			throw new Error("CSS default capture owner mismatch");
		const snapshot = capture.capture(
			document,
			resourcePolicy.snapshot(context),
		);
		weakWrite(contexts, snapshot, context);
		weakWrite(snapshots, snapshot, context.version);
		return snapshot;
	}
	function snapshotContext(snapshot) {
		const context = weakRead(contexts, snapshot);
		if (!context) throw new Error("CSS default unknown snapshot");
		return context;
	}
	const profile = {
		compactCascade: true,
		textDecoration: "declared",
		userAgentSource(selected) {
			if (selected !== backend) throw new Error("CSS default backend changed");
			if (!preferences) throw new Error("CSS default preferences unavailable");
			return preferences.userAgentSource;
		},
		environment(context) {
			return {
				mediaType: context.facts.mediaType,
				features: context.environment.features,
				unitBases: context.environment.mediaUnitBases,
				selectorSupport: {},
				stylePhase: "steady",
			};
		},
		targetContext(row, { root, snapshot }) {
			const context = snapshotContext(snapshot);
			return contextInitial(context.preferences, context.environment, root);
		},
		scopeDepth() {
			throw new Error("CSS default requires contextual routing");
		},
		backendIdentity(selected) {
			if (selected !== backend) throw new Error("CSS default backend changed");
			return 1;
		},
		world(context, snapshot) {
			if (snapshotContext(snapshot) !== context)
				throw new Error("CSS default Scene context mismatch");
			return sharedEncode({
				frame: weakRead(snapshots, snapshot),
				environment: context.environmentKey,
				viewport: {
					width: axis(context.facts.width),
					height: axis(context.facts.height),
				},
			});
		},
	};
	function measureScene(request) {
		const world = sharedDecode(request.world),
			viewport = {
				width: world.viewport.width.value,
				height: world.viewport.height.value,
			};
		if (!finite(viewport.width) || !finite(viewport.height))
			throw new TypeError("CSS default Scene viewport unavailable");
		return createSharedSceneProvider({
			selectBackend: () => backend,
			viewport,
			fonts: resourcePolicy.fonts,
			limits: resourcePolicy.scene,
			compositionLimits: resourcePolicy.composition,
		})(request);
	}
	return {
		selectBackend: () => backend,
		readContext,
		captureSnapshot,
		captureSourceFacts(snapshot) {
			return capture.sources(snapshot, resourcePolicy.adapter.compile([]));
		},
		captureSelectorFacts(snapshot) {
			return capture.selectorFacts(
				snapshot,
				resourcePolicy.selectorFacts(snapshotContext(snapshot)),
			);
		},
		profile,
		limits,
		measureScene,
	};
}

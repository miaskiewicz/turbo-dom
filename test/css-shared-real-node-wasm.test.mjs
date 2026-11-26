import {
	runOriginalControls,
	runBoundaryControls,
} from "./helpers/css-shared-corpus-controls.mjs";
import test from "node:test";
import { loadRealCssBackend } from "./helpers/css-shared-real.mjs";
import { runInheritanceControls } from "./helpers/css-shared-inheritance-controls.mjs";
test("actual node-wasm: computed inheritance, Scene used values and read laziness", async () =>
	runInheritanceControls(await loadRealCssBackend("node-wasm")));

test("actual node-wasm: original18 Chrome and published API15 corpus", async () =>
	runOriginalControls(await loadRealCssBackend("node-wasm")));

test("actual node-wasm: measured scope limit, box absence, height and constraints", async () =>
	runBoundaryControls(await loadRealCssBackend("node-wasm")));

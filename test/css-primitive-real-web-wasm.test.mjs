import test from 'node:test';
import {loadRealCssBackend} from './helpers/css-shared-real.mjs';
import {runOriginalControls} from './helpers/css-shared-corpus-controls.mjs';
import {runInheritanceControls} from './helpers/css-shared-inheritance-controls.mjs';
import {configurePrimitiveReal} from './helpers/css-primitive-real-controls.mjs';
test('actual web-wasm: primitive facts preserve original18 and published API15', async()=>runOriginalControls(configurePrimitiveReal(await loadRealCssBackend('web-wasm'))));
test('actual web-wasm: primitive facts preserve computed inheritance and lazy Scene4', async()=>runInheritanceControls(configurePrimitiveReal(await loadRealCssBackend('web-wasm'))));

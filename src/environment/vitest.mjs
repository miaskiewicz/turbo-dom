// Vitest environment adapter (vitest 1–4). Use in vitest config:
//
//   import TurboDom from '@miaskiewicz/turbo-dom/environment/vitest';
//   export default defineConfig({ test: { environment: TurboDom } });
//
// or point at the file path:
//
//   test: { environment: './node_modules/@miaskiewicz/turbo-dom/src/environment/vitest.mjs' }
//
// (vitest's bare-name `environment: 'name'` only works for a package literally
// named `vitest-environment-<name>`; use the object or path form for a scoped pkg.)
//
// Per-file options via environmentOptions:
//   test: { environmentOptions: { turboDom: { html: '<!doctype html>...', url: 'http://localhost/' } } }

import { installGlobals } from './install.mjs';

const environment = {
  name: 'turbo-dom',
  // vitest 3/4 read `viteEnvironment`; older versions read `transformMode`.
  viteEnvironment: 'client',
  transformMode: 'web',

  setup(global, options) {
    const opts = (options && options.turboDom) || {};
    const { env, teardown } = installGlobals(global, opts);
    return {
      teardown(g) {
        teardown();
        env.reset();
      },
    };
  },
};

export default environment;

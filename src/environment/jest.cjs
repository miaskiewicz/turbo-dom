// Jest environment adapter. Use in jest config:
//
//   // jest.config.js
//   module.exports = { testEnvironment: 'turbodom/jest' }
//
// or point directly at this file:
//
//   testEnvironment: './node_modules/turbodom/dist/environment/jest.cjs'
//
// Per-file / project options:
//   testEnvironmentOptions: { html: '<!doctype html>...', url: 'http://localhost/' }
//
// Requires `jest-environment-node` (a jest dependency) to be resolvable.

const nodeEnv = require('jest-environment-node');
const NodeEnvironment = nodeEnv.TestEnvironment || nodeEnv.default || nodeEnv;

class TurbodomEnvironment extends NodeEnvironment {
  constructor(config, context) {
    super(config, context);
    const projectConfig = config.projectConfig || config;
    this.__opts = projectConfig.testEnvironmentOptions || {};
  }

  async setup() {
    await super.setup();
    // runtime is ESM; load it dynamically from this CJS environment
    const { installGlobals } = await import('./install.mjs');
    installGlobals(this.global, this.__opts.turbodom || this.__opts);
  }

  async teardown() {
    if (this.global && this.global.__turbodom) this.global.__turbodom.reset();
    await super.teardown();
  }
}

module.exports = TurbodomEnvironment;
module.exports.default = TurbodomEnvironment;
module.exports.TestEnvironment = TurbodomEnvironment;

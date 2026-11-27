export * from '../../src/runtime/index.mjs';
import {createEnvironment as actualCreateEnvironment} from '../../src/runtime/index.mjs';
export function createEnvironment(html, options = {}) { return actualCreateEnvironment(html, {...options, cssBackend:'rust'}); }

export * from '../../src/environment/install.mjs';
import {installGlobals as actualInstallGlobals} from '../../src/environment/install.mjs';
export function installGlobals(target, options = {}) { return actualInstallGlobals(target, {...options, cssBackend:'rust'}); }

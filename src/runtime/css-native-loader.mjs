// Fixed built-in native artifact selection; no parser provenance is granted here.
export function nativeCssSuffix(platform, arch, musl = false) {
  if (platform === 'darwin') return `darwin-${arch}`;
  if (platform === 'win32') return `win32-${arch}-msvc`;
  if (platform === 'linux') return `linux-${arch}-${musl ? 'musl' : 'gnu'}`;
  if (platform === 'android') return `android-${arch === 'arm' ? 'arm-eabi' : arch}`;
  if (platform === 'freebsd') return `freebsd-${arch}`;
  return null;
}

export function loadCssModule(require, spec) {
  let path;
  try {
    path = require.resolve(spec);
  } catch (error) {
    if (error?.code === 'MODULE_NOT_FOUND' && error.message?.startsWith(`Cannot find module '${spec}'`)) return null;
    throw error;
  }
  return require(path);
}

export function detectNativeMusl(require, process = globalThis.process) {
  if (process?.report && typeof process.report.getReport === 'function') {
    return !process.report.getReport().header.glibcVersionRuntime;
  }
  try {
    const lddPath = require('child_process').execSync('which ldd').toString().trim();
    return require('fs').readFileSync(lddPath, 'utf8').includes('musl');
  } catch {
    return true;
  }
}

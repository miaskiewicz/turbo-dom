// Independent, localhost-only Chrome oracle for auxiliary browsing contexts.
// From crates/turbo-dom-browser-env:
// node scripts/auxiliary-contexts-chrome-control.mjs --out src/auxiliary_contexts_chrome156.json
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { accessSync, constants, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { homedir, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: {
  probe: { type: 'string', default: 'src/auxiliary_contexts_probe.js' },
  cases: { type: 'string', default: 'src/auxiliary_contexts_cases.json' },
  out: { type: 'string' },
  chrome: { type: 'string' },
} });
if (!values.out) throw new Error('Pass --out ORACLE.json');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const probe = readFileSync(resolve(values.probe), 'utf8');
const casesBytes = readFileSync(resolve(values.cases));
const cases = JSON.parse(casesBytes);
const executable = values.chrome || join(homedir(), '.cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell');
accessSync(executable, constants.X_OK);
const profile = mkdtempSync(join(tmpdir(), 'turbo-popup-oracle-'));
const requests = [];
function server() {
  return createServer((request, response) => {
    requests.push({ path: request.url, referrer: request.headers.referer || '' });
    response.setHeader('Content-Type', 'text/html; charset=utf-8');
    response.end('<!doctype html><title>Auxiliary context oracle</title><body>real popup document</body>');
  });
}
const primary = server(), secondary = server();
await Promise.all([primary, secondary].map(server => new Promise(resolve => server.listen(0, '127.0.0.1', resolve))));
const origin = `http://127.0.0.1:${primary.address().port}`;
const crossOrigin = `http://127.0.0.1:${secondary.address().port}`;
const flags = [
  '--headless=new', '--use-mock-keychain', '--password-store=basic',
  '--remote-debugging-pipe', '--user-data-dir=<fresh-profile>',
  '--no-first-run', '--no-default-browser-check', '--disable-extensions',
  '--disable-background-networking',
  '--disable-component-extensions-with-background-pages',
  // Positive creation is measured independently of the browser's UI blocker.
  // The sandbox cases retain real browser-enforced popup denial.
  '--disable-popup-blocking', 'about:blank',
];
const chrome = spawn(executable, flags.map(flag => flag === '--user-data-dir=<fresh-profile>' ? `--user-data-dir=${profile}` : flag), {
  stdio: ['ignore', 'ignore', 'ignore', 'pipe', 'pipe'],
});
const pending = new Map(), created = [], destroyed = new Set();
let sequence = 1, buffer = Buffer.alloc(0);
chrome.stdio[4].on('data', chunk => {
  buffer = Buffer.concat([buffer, chunk]);
  for (let end = buffer.indexOf(0); end >= 0; end = buffer.indexOf(0)) {
    const message = JSON.parse(buffer.subarray(0, end).toString('utf8'));
    buffer = buffer.subarray(end + 1);
    if (message.method === 'Target.targetCreated') created.push(message.params.targetInfo);
    if (message.method === 'Target.targetDestroyed') destroyed.add(message.params.targetId);
    const waiter = message.id && pending.get(message.id);
    if (!waiter) continue;
    pending.delete(message.id);
    clearTimeout(waiter.timer);
    if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
    else waiter.resolve(message.result);
  }
});
function call(method, params = {}, sessionId) {
  return new Promise((resolve, reject) => {
    const id = sequence++;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15000);
    pending.set(id, { resolve, reject, timer });
    chrome.stdio[3].write(`${JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) })}\0`);
  });
}
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function evaluate(sessionId, expression) {
  const result = await call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId);
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function waitFor(sessionId, expression) {
  for (let attempt = 0; attempt < 400; attempt++) {
    if (await evaluate(sessionId, expression)) return;
    await pause(5);
  }
  throw new Error(`Page fixture did not settle: ${expression}`);
}
const result = [], errors = [];
let version;
try {
  version = await call('Browser.getVersion');
  if (!/^(?:Headless)?Chrome\/156\./.test(version.product)) {
    throw new Error(`Expected the frozen Chrome 156 oracle, received ${version.product}`);
  }
  await call('Target.setDiscoverTargets', { discover: true });
  for (const entry of cases) {
    const start = created.length, requestStart = requests.length;
    const { targetId } = await call('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true });
    const mainUrl = `${origin}/case?id=${encodeURIComponent(entry.id)}`;
    const config = { ...entry,
      sameOriginUrl: `${origin}/popup?id=${encodeURIComponent(entry.id)}`,
      crossOriginUrl: `${crossOrigin}/popup?id=${encodeURIComponent(entry.id)}`,
    };
    try {
      await call('Page.enable', {}, sessionId);
      await call('Page.navigate', { url: mainUrl }, sessionId);
      await waitFor(sessionId, `location.href===${JSON.stringify(mainUrl)}&&document.readyState==='complete'`);
      await evaluate(sessionId, `globalThis.__popupProbeCase=${JSON.stringify(config)}`);
      const observed = await evaluate(sessionId, probe);
      // Navigation and target discovery are independently observed after the
      // script returns; null alone never proves a noopener popup was created.
      await pause(100);
      const popups = created.slice(start).filter(target => target.type === 'page' && target.targetId !== targetId);
      const live = [];
      for (const popup of popups) {
        if (destroyed.has(popup.targetId)) continue;
        const { sessionId: popupSession } = await call('Target.attachToTarget', { targetId: popup.targetId, flatten: true });
        const priorUnnavigated = entry.scenario === 'named-noopener' && entry.existing && popups.length > 1 && popup === popups[0];
        if (['features', 'hyperlink', 'cross-origin', 'named-noopener'].includes(entry.scenario) && !priorUnnavigated) {
          await waitFor(popupSession, `location.pathname==='/popup'&&document.readyState==='complete'`);
        }
        live.push(await evaluate(popupSession, `({name:name,openerNull:opener===null,closed:closed,referrerEmpty:document.referrer==='',urlKind:location.href==='about:blank'?'about:blank':location.pathname,originKind:location.origin===${JSON.stringify(origin)}?'same':location.origin===${JSON.stringify(crossOrigin)}?'cross':'opaque'})`));
      }
      const network = requests.slice(requestStart).filter(request => request.path.startsWith('/popup?')).map(request => ({ referrerEmpty: request.referrer === '' }));
      result.push({ id: entry.id, result: observed, host: { created: popups.length, live, network } });
    } catch (error) {
      errors.push({ id: entry.id, error: String(error.stack || error) });
    } finally {
      for (const target of created.slice(start)) {
        if (target.type === 'page' && !destroyed.has(target.targetId)) {
          await call('Target.closeTarget', { targetId: target.targetId }).catch(() => {});
        }
      }
    }
  }
} finally {
  if (chrome.exitCode === null && chrome.signalCode === null) {
    const exited = new Promise(resolve => chrome.once('exit', resolve));
    const timeout = setTimeout(() => chrome.kill('SIGKILL'), 5000);
    chrome.kill();
    await exited;
    clearTimeout(timeout);
  }
  await Promise.all([primary, secondary].map(server => new Promise(resolve => server.close(resolve))));
  rmSync(profile, { recursive: true, force: true, maxRetries: 10 });
}
const oracle = {
  browser: version.product, v8: version.jsVersion,
  casesSha256: sha(casesBytes), probeSha256: sha(probe),
  collectorSha256: sha(readFileSync(fileURLToPath(import.meta.url))),
  flags, errors, result,
};
writeFileSync(resolve(values.out), `${JSON.stringify(oracle, null, 2)}\n`);
console.log(JSON.stringify({ browser: oracle.browser, errors: errors.length, cases: result.length, out: resolve(values.out) }));
if (errors.length) process.exitCode = 1;

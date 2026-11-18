// Localhost-only cross-origin child WindowProxy index oracle (Chrome 156).
// From crates/turbo-dom-browser-env:
// node scripts/auxiliary-window-indexes-chrome-control.mjs --out src/auxiliary_window_indexes_chrome156.json
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { accessSync, constants, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { homedir, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: {
  probe: { type: 'string', default: 'src/auxiliary_window_indexes_probe.js' },
  cases: { type: 'string', default: 'src/auxiliary_window_indexes_cases.json' },
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
const profile = mkdtempSync(join(tmpdir(), 'turbo-window-index-oracle-'));
function server() {
  return createServer((request, response) => {
    response.setHeader('Content-Type', 'text/html; charset=utf-8');
    const url = new URL(request.url, 'http://localhost');
    let html = '<!doctype html><title>Child WindowProxy index oracle</title><body>';
    if (url.pathname === '/container') {
      const count = Number(url.searchParams.get('count'));
      if (!Number.isInteger(count) || count < 0 || count > 2) {
        response.writeHead(400).end();
        return;
      }
      for (let index = 0; index < count; index++) {
        html += `<iframe name="NamedChild${index}" src="/nested"></iframe>`;
      }
    }
    response.end(html);
  });
}
const primary = server(), secondary = server();
await Promise.all([primary, secondary].map(server => new Promise(resolve => server.listen(0, '127.0.0.1', resolve))));
const origin = `http://127.0.0.1:${primary.address().port}`;
const crossOrigin = `http://127.0.0.1:${secondary.address().port}`;
const flags = ['--headless=new', '--remote-debugging-pipe', '--user-data-dir=<fresh-profile>', '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--disable-background-networking', 'about:blank'];
const chrome = spawn(executable, flags.map(flag => flag === '--user-data-dir=<fresh-profile>' ? `--user-data-dir=${profile}` : flag), { stdio: ['ignore', 'ignore', 'ignore', 'pipe', 'pipe'] });
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
const rows = [], errors = [];
let version;
try {
  version = await call('Browser.getVersion');
  if (version.product !== 'HeadlessChrome/156.0.8078.47') throw new Error(`Expected pinned Chrome 156, received ${version.product}`);
  for (const entry of cases) {
    const { targetId } = await call('Target.createTarget', { url: `${origin}/root` });
    const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true });
    try {
      await call('Page.enable', {}, sessionId);
      await waitFor(sessionId, `location.origin===${JSON.stringify(origin)}&&document.readyState==='complete'`);
      await evaluate(sessionId, `new Promise(resolve=>{const frame=document.createElement('iframe');frame.src=${JSON.stringify(`${crossOrigin}/container?count=${entry.count}`)};frame.onload=()=>{globalThis.__windowIndexTarget=frame.contentWindow;resolve(true)};document.body.append(frame)})`);
      rows.push({ count: entry.count, result: await evaluate(sessionId, probe) });
    } catch (error) {
      errors.push({ id: entry.id, error: String(error.stack || error) });
    } finally {
      await call('Target.closeTarget', { targetId }).catch(() => {});
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
  flags, errors, rows,
};
writeFileSync(resolve(values.out), `${JSON.stringify(oracle, null, 2)}\n`);
console.log(JSON.stringify({ browser: oracle.browser, errors: errors.length, cases: rows.length, out: resolve(values.out) }));
if (errors.length) process.exitCode = 1;

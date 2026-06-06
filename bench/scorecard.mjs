// Hot-path microbench scorecard — the DETERMINISTIC cross-version speed signal.
// Run on any version (`node bench/scorecard.mjs`) and compare ops/s to know whether
// a change actually sped up the paths the perf loop targets. Unlike the real-suite
// wall-clock (noisy ±10-40s), these are tight, warmed, best-of-N microbenches with
// dead-code-elimination sinks — stable enough for version-to-version comparison.
import { createEnvironment } from '../src/runtime/index.mjs';

const REPS = 6;
function best(fn, iters) {
  for (let i = 0; i < Math.min(iters, 50000); i++) fn(i);          // warm
  let b = Infinity;
  for (let r = 0; r < REPS; r++) {
    const t = process.hrtime.bigint();
    for (let i = 0; i < iters; i++) fn(i);
    const ms = Number(process.hrtime.bigint() - t) / 1e6;
    if (ms < b) b = ms;
  }
  return iters / (b / 1000); // ops/s
}
const fmt = (n) => Math.round(n).toLocaleString().padStart(14);
let SINK = 0;

// --- fixtures ---
const SHELL = '<!doctype html><html><head></head><body></body></html>';
const BIG = '<!doctype html><body>' +
  Array.from({ length: 300 }, (_, i) => `<div class="r${i}" data-testid="row-${i}"><span>a</span><b>b</b><i>c</i></div>`).join('') +
  '</body>';

const env = createEnvironment(BIG);
const leaf = (() => { const e = env.document.createElement('span'); e.textContent = 'hello world'; env.document.body.appendChild(e); return e; })();
const W = env.window;

// dispatch fixtures
const dEnv = createEnvironment('<!doctype html><body><div id=a><div><div><span id=leaf>x</span></div></div></div></body>');
const dLeaf = dEnv.document.getElementById('leaf');
dEnv.document.getElementById('a').addEventListener('ping', () => { SINK++; }); // single delegated listener
const Ev = dEnv.window.Event;

// mutation fixture
const mEl = env.document.createElement('div'); env.document.body.appendChild(mEl);

const rows = [
  ['createEnvironment (empty shell)', () => { SINK += createEnvironment(SHELL) ? 1 : 0; }, 300_000],
  ['inflate+traverse (fresh env, ~1200 els)', () => { const e = createEnvironment(BIG); SINK += e.document.querySelectorAll('*').length; }, 8_000],
  ['dispatch listener-less (bubbles)', () => { const e = new Ev('none', { bubbles: true }); SINK += dLeaf.dispatchEvent(e) ? 1 : 0; }, 1_000_000],
  ['dispatch single-listener (bubbles)', () => { dLeaf.dispatchEvent(new Ev('ping', { bubbles: true })); }, 1_000_000],
  ['mutation append+setAttr+remove (no obs)', () => { const c = env.document.createElement('span'); mEl.appendChild(c); mEl.setAttribute('data-x', '1'); mEl.removeChild(c); }, 800_000],
  ['textContent read (single text child)', () => { SINK += leaf.textContent.length; }, 3_000_000],
  ['addEventListener (3 / fresh elem)', () => { const el = env.document.createElement('div'); el.addEventListener('click', onC); el.addEventListener('input', onI); el.addEventListener('keydown', onK); SINK += el.__listeners.size; }, 1_500_000],
];
function onC() {} function onI() {} function onK() {}

console.log(`turbo-dom scorecard (ops/s, higher=faster, best of ${REPS})\n`);
for (const [label, fn, iters] of rows) {
  console.log(`${fmt(best(fn, iters))}  ${label}`);
}
console.log(`\nsink=${SINK} (ignore — dead-code-elimination guard)`);

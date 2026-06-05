// Benchmark fixtures: representative shapes the spec calls out.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));

const small = `<div class="card"><h2>Title</h2><p>Some body text with a <a href="/x">link</a>.</p><button type="button">Go</button></div>`;

// Large SSR-style output: a wide data table.
const rows = Array.from({ length: 400 }, (_, i) =>
  `<tr><td>${i}</td><td>Row ${i}</td><td><span class="badge">ok</span></td><td>2026-01-${(i % 28) + 1}</td><td><a href="/r/${i}">open</a></td><td>${i * 7}</td></tr>`
).join('');
const ssrLarge = `<!doctype html><html><body><table><thead><tr><th>#</th><th>name</th><th>status</th><th>date</th><th>link</th><th>n</th></tr></thead><tbody>${rows}</tbody></table></body></html>`;

// Deeply nested.
const deep = '<div>'.repeat(400) + 'leaf' + '</div>'.repeat(400);

// Deliberately malformed: stray table content, unclosed tags, adoption-agency bait.
const malformed = `<table>foo<tr><td>cell<a><p>x</a></p><b>bold<i>both</b>italic</i></table><ul><li>a<li>b<li>c`;

// Real rendered HTML from the Flux UI Storybook build.
const real = readFileSync(join(here, '..', 'vendor', 'iframe-fixture.html'), 'utf8');

export const fixtures = [
  { name: 'small', html: small },
  { name: 'ssr-large', html: ssrLarge },
  { name: 'deep-nested', html: deep },
  { name: 'malformed', html: malformed },
  { name: 'real-storybook', html: real },
];

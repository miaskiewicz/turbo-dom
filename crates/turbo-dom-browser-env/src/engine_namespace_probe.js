// The engine namespace is unreachable from page script by every reflective route
// (miaskiewicz/turbo-dom#222); run as a function body.
const seen = new Set(), found = [];
const objectish = v => v !== null && (typeof v === 'object' || typeof v === 'function');
const visit = (o, path) => {
  if (!objectish(o) || seen.has(o)) return;
  seen.add(o);
  for (const s of Object.getOwnPropertySymbols(o)) {
    if (Symbol.keyFor(s) !== undefined || s.description === 'ts') found.push(path + '[' + String(s) + ']');
  }
  for (const k of Reflect.ownKeys(o)) if (typeof k === 'symbol' && Symbol.keyFor(k) !== undefined) found.push(path + ' ownKeys ' + String(k));
};
const chain = [];
for (let o = globalThis; o; o = Object.getPrototypeOf(o)) { visit(o, 'chain' + chain.length); chain.push(o); }
for (const k of Reflect.ownKeys(globalThis)) {
  let v;
  try { v = globalThis[k]; } catch (e) { continue; }
  visit(v, String(k));
  if (typeof v === 'function') { try { visit(v.prototype, String(k) + '.prototype'); } catch (e) {} }
}
visit(document, 'document');
for (let o = Object.getPrototypeOf(document); o; o = Object.getPrototypeOf(o)) visit(o, 'document.proto');
return JSON.stringify({
  symbolFor: [globalThis[Symbol.for('ts')], window[Symbol.for('ts')], self[Symbol.for('ts')], Reflect.get(globalThis, Symbol.for('ts'))].map(v => v === undefined),
  inOperator: Symbol.for('ts') in globalThis,
  ownSymbols: Object.getOwnPropertySymbols(globalThis).map(String),
  windowOwnSymbols: Object.getOwnPropertySymbols(window).map(String),
  reflectSymbols: Reflect.ownKeys(window).filter(k => typeof k === 'symbol').map(String),
  descriptor: Object.getOwnPropertyDescriptor(globalThis, Symbol.for('ts')) === undefined,
  found,
});

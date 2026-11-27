// Only complete primitive DTOs privately constructed by native capture enter here.
// This comparison never admits arbitrary caller facts or skips fresh validation.
const same = Object.is;
const names = Object.getOwnPropertyNames;
const descriptor = Object.getOwnPropertyDescriptor;
const owns = Object.hasOwn;
function objectPair(left, right) {
  return left !== null && right !== null && typeof left === 'object' && typeof right === 'object';
}
export function sameOwnedSelectorInput(left, right) {
  if (same(left, right)) return true;
  if (!objectPair(left, right)) return false;
  const a = names(left), b = names(right);
  if (a.length !== b.length) return false;
  for (let index = 0; index < a.length; index++) {
    if (a[index] !== b[index]) return false;
    const first = descriptor(left, a[index]), second = descriptor(right, b[index]);
    if (!owns(first, 'value') || !owns(second, 'value') || !sameOwnedSelectorInput(first.value, second.value)) return false;
  }
  return true;
}

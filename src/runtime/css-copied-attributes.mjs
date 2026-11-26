// Both native attribute readers create dense, private rows with these three
// own data fields. A changed iterator can substitute a row on the slow path.
const iteratorPrototype = Object.getPrototypeOf([][Symbol.iterator]());
const descriptor = Object.getOwnPropertyDescriptor;
const originalNext = descriptor(iteratorPrototype, 'next');
const hasOwn = Object.hasOwn;

function append(result, state, hooks, name, value, prefix) {
  if (typeof name !== 'string' || typeof value !== 'string' || typeof prefix !== 'string')
    hooks.unsupported('unknown attribute');
  hooks.charge(state, 'maxAttributeUnits', name.length + value.length + prefix.length);
  result.push({ name, value, prefix, nameUnits: hooks.units(name),
    valueUnits: hooks.units(value), prefixUnits: hooks.units(prefix) });
}

export function collectCopiedCssAttributes(rows, state, hooks) {
  const result = [];
  const next = descriptor(iteratorPrototype, 'next');
  if (next && hasOwn(next, 'value') && next.value === originalNext?.value) {
    for (let index = 0; index < rows.length; index++) {
      const row = rows[index];
      hooks.charge(state, 'maxAttributes', 1);
      append(result, state, hooks, row.name, row.value, row.prefix);
    }
  } else {
    for (const row of rows) {
      hooks.charge(state, 'maxAttributes', 1);
      append(result, state, hooks, hooks.data(row, 'name'), hooks.data(row, 'value'),
        hooks.data(row, 'prefix', true) ?? '');
    }
  }
  return result;
}

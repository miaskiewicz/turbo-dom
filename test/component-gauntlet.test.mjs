// Phase 3, test strategy #2 (the headline): real component libraries, UNMODIFIED,
// rendered with React + @testing-library/react against the turbodom environment.
// This is the test happy-dom fails on real libraries — passing it is the point.

import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import vitestEnv from '../src/environment/vitest.mjs';

const require = createRequire(import.meta.url);

// install a turbodom DOM environment on globalThis (what the vitest adapter does)
globalThis.IS_REACT_ACT_ENVIRONMENT = true;
vitestEnv.setup(globalThis, { turbodom: { html: '<!doctype html><html><body></body></html>' } });

// Each real test FILE gets a fresh realm; here all tests share one Element
// prototype, so libraries that patch focus/blur (user-event, headlessui) would
// cross-contaminate. Restore the pristine focus/blur descriptors + clear any
// library patch markers before each test to mimic per-file isolation.
const HTMLElementCtor = globalThis.HTMLElement;
const ORIG_FOCUS = Object.getOwnPropertyDescriptor(HTMLElementCtor.prototype, 'focus');
const ORIG_BLUR = Object.getOwnPropertyDescriptor(HTMLElementCtor.prototype, 'blur');
beforeEach(() => {
  Object.defineProperty(HTMLElementCtor.prototype, 'focus', ORIG_FOCUS);
  Object.defineProperty(HTMLElementCtor.prototype, 'blur', ORIG_BLUR);
  for (const s of Object.getOwnPropertySymbols(HTMLElementCtor.prototype)) delete HTMLElementCtor.prototype[s];
});

const React = require('react');
const { render, screen, cleanup, waitFor } = require('@testing-library/react');
const userEvent = require('@testing-library/user-event').default;
const h = React.createElement;

// ---------------------------------------------------------------- React core
test('React: useState + effects + event re-render', async () => {
  function Counter() {
    const [n, setN] = React.useState(0);
    React.useEffect(() => { document.title = `count ${n}`; }, [n]);
    return h('button', { onClick: () => setN((x) => x + 1) }, `count: ${n}`);
  }
  render(h(Counter));
  const btn = screen.getByText('count: 0');
  await userEvent.setup().click(btn);
  assert.ok(await screen.findByText('count: 1'));
  cleanup();
});

// ---------------------------------------------------------------- downshift
test('downshift useCombobox: type filters items (aria wiring intact)', async () => {
  const { useCombobox } = require('downshift');
  function Combo() {
    const all = ['apple', 'banana', 'cherry'];
    const [input, setInput] = React.useState('');
    const items = all.filter((i) => i.includes(input.toLowerCase()));
    const cb = useCombobox({ items, inputValue: input, onInputValueChange: ({ inputValue }) => setInput(inputValue || '') });
    return h('div', null,
      h('label', cb.getLabelProps(), 'Fruit'),
      h('input', cb.getInputProps({ placeholder: 'fruit' })),
      h('ul', cb.getMenuProps(), items.map((it, i) => h('li', { ...cb.getItemProps({ item: it, index: i }), key: it }, it))),
    );
  }
  render(h(Combo));
  const input = screen.getByPlaceholderText('fruit');
  await userEvent.setup().type(input, 'an');
  await waitFor(() => {
    assert.ok(screen.getByText('banana'));
    assert.equal(screen.queryByText('apple'), null);
  });
  cleanup();
});

// ---------------------------------------------------------------- Radix UI
test('@radix-ui/react-tabs: click switches panel', async () => {
  const Tabs = require('@radix-ui/react-tabs');
  function Demo() {
    return h(Tabs.Root, { defaultValue: 'a' },
      h(Tabs.List, null,
        h(Tabs.Trigger, { value: 'a' }, 'Tab A'),
        h(Tabs.Trigger, { value: 'b' }, 'Tab B')),
      h(Tabs.Content, { value: 'a' }, 'Content A'),
      h(Tabs.Content, { value: 'b' }, 'Content B'));
  }
  render(h(Demo));
  assert.ok(screen.getByText('Content A'));
  await userEvent.setup().click(screen.getByText('Tab B'));
  assert.ok(await screen.findByText('Content B'));
  cleanup();
});

// ---------------------------------------------------------------- Headless UI
test('@headlessui/react Disclosure: toggle reveals panel', async () => {
  const { Disclosure } = require('@headlessui/react');
  function Demo() {
    return h(Disclosure, null,
      h(Disclosure.Button, null, 'Toggle'),
      h(Disclosure.Panel, null, 'Panel body'));
  }
  render(h(Demo));
  assert.equal(screen.queryByText('Panel body'), null);
  await userEvent.setup().click(screen.getByText('Toggle'));
  assert.ok(await screen.findByText('Panel body'));
  cleanup();
});

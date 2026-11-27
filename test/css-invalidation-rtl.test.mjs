import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { installGlobals } from '../src/environment/install.mjs';
import { cssStateSnapshot } from '../src/runtime/css-invalidation.mjs';

const require = createRequire(import.meta.url);

test('React and RTL rendering keep CSS tracking and computed-style globals lazy', () => {
  const { env, teardown } = installGlobals(globalThis);
  const previousAct = globalThis.IS_REACT_ACT_ENVIRONMENT;
  globalThis.IS_REACT_ACT_ENVIRONMENT = true;
  const React = require('react');
  const { render, screen, cleanup } = require('@testing-library/react');
  try {
    function Counter() {
      const [count, setCount] = React.useState(0);
      return React.createElement('button', { onClick: () => setCount(count + 1) }, `count ${count}`);
    }
    render(React.createElement(Counter));
    const target = screen.getByText('count 0');
    assert.equal(cssStateSnapshot(env.document, target), null);
    React.act(() => target.click());
    assert.equal(screen.getByText('count 1'), target);
    assert.equal(cssStateSnapshot(env.document, target), null);
    assert.equal(env.touched().includes('getComputedStyle'), false);
  } finally {
    cleanup();
    teardown();
    if (previousAct === undefined) delete globalThis.IS_REACT_ACT_ENVIRONMENT;
    else globalThis.IS_REACT_ACT_ENVIRONMENT = previousAct;
  }
});

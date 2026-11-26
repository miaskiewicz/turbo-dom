import { test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { useRef, useCallback } from 'react';

test('programmatic .click() re-entrancy does not recurse (click-in-progress)', () => {
  function Dropzone() {
    const ref = useRef(null);
    const handleClick = useCallback(() => { ref.current?.click(); }, []);
    return (
      <div role="button" tabIndex={0} aria-label="dropzone" onClick={handleClick}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleClick(); } }}>
        <input ref={ref} type="file" aria-label="file" style={{ display: 'none' }} />
      </div>
    );
  }
  render(<Dropzone />);
  const input = document.querySelector('input[type="file"]');
  const clickSpy = vi.spyOn(input, 'click'); // calls through — would infinite-loop without the guard
  const dz = screen.getByRole('button', { name: /dropzone/i });
  fireEvent.keyDown(dz, { key: 'Enter' });
  expect(clickSpy).toHaveBeenCalled();
});

test('getByLabelText: wrapping <label> labels only its FIRST control', () => {
  render(
    <label>
      Street
      <input aria-label="Street" />
      <button type="button" aria-label="Street-select-address">pick</button>
    </label>
  );
  expect(screen.getByLabelText('Street').localName).toBe('input');
  expect(screen.getByRole('button', { name: 'Street-select-address' })).toBeTruthy();
});

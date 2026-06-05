import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  test: {
    include: ['test-vitest/**/*.test.{js,jsx}'],
    // turbo-dom environment (file-path form; works on vitest 1–4)
    environment: fileURLToPath(new URL('../src/environment/vitest.mjs', import.meta.url)),
    environmentOptions: { turboDom: { url: 'http://localhost/' } },
  },
});

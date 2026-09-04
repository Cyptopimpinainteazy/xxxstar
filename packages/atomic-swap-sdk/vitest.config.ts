import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    // Run only authored source tests. Never pick up stale compiled copies under
    // dist/ (which would otherwise double-run and error on import paths).
    include: ['src/**/*.test.ts'],
    exclude: ['node_modules/**', 'dist/**'],
  },
});

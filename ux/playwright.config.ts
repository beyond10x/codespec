import { defineConfig } from '@playwright/test';
import { specPattern } from './core/specs.ts';

const round = process.env.UX_MODE === 'round';

export default defineConfig({
  testDir: './tests',
  testMatch: specPattern(round ? 'round' : 'check'),
  globalSetup: './tests/global-setup.ts',
  fullyParallel: true,
  workers: process.env.UX_WORKERS ? Number(process.env.UX_WORKERS) : 4,
  retries: 0,
  timeout: 60_000,
  reporter: [['list'], ['json', { outputFile: (process.env.UX_OUT ?? '.out/check') + '/playwright.json' }]],
  outputDir: (process.env.UX_OUT ?? '.out/check') + '/test-results',
  use: {
    browserName: 'chromium',
    headless: true,
    reducedMotion: 'reduce',
    serviceWorkers: 'block',
  },
});

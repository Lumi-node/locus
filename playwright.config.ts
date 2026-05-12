import { defineConfig, devices } from '@playwright/test';

// Playwright is used here only for the Solscan recording in scripts/demo-solscan.spec.ts.
// Records the browser session to MP4 automatically — narrate over the resulting clip.

export default defineConfig({
  testDir: './scripts',
  testMatch: /demo-.*\.spec\.ts/,
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  timeout: 180_000,

  use: {
    viewport: { width: 1920, height: 1080 },
    video: {
      mode: 'on',
      size: { width: 1920, height: 1080 },
    },
    screenshot: 'off',
    trace: 'off',
    actionTimeout: 30_000,
    navigationTimeout: 30_000,
  },

  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1920, height: 1080 },
      },
    },
  ],

  outputDir: 'demo-recordings',
});

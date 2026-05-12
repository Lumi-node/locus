// Playwright recording for the Locus hackathon pitch.
//
// Opens the local demo/demo.html (auto-advancing 6-section presentation that
// matches the §13 narration cadence) and records the whole thing to a video.
//
// Run:
//   npx playwright test scripts/demo-solscan.spec.ts --headed
//
// Output:
//   demo-recordings/.../video.webm  (rename + remux to MP4 if you like)

import { test } from '@playwright/test';
import path from 'path';

const DEMO_HTML = `file://${path.resolve(__dirname, '..', 'demo', 'demo.html')}`;

const TOTAL_MS = 183_000; // 3:00 presentation + 3s tail

test('Locus pitch — 6-section walkthrough (3:00)', async ({ page }) => {
  test.setTimeout(TOTAL_MS + 30_000);

  // Make the presentation fill the entire recording viewport.
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto(DEMO_HTML);

  // Wait for first stage to be visible before timing starts.
  await page.waitForSelector('#s1.live', { timeout: 5_000 });

  // Let the auto-paced presentation run to completion.
  await page.waitForTimeout(TOTAL_MS);
});

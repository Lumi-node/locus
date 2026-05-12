// Records the ≤2-minute pitch video (demo-pitch.html, no live solscan).
//
// Run:
//   npx playwright test scripts/demo-pitch.spec.ts

import { test } from '@playwright/test';
import path from 'path';

const DEMO = `file://${path.resolve(__dirname, '..', 'demo', 'demo-pitch.html')}`;
const TOTAL_MS = 122_000; // 2:00 + 2s tail

test('Locus pitch (≤2:00)', async ({ page }) => {
  test.setTimeout(TOTAL_MS + 30_000);
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto(DEMO);
  await page.waitForSelector('#p1.live', { timeout: 5_000 });
  await page.waitForTimeout(TOTAL_MS);
});

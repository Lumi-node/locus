// Playwright recording for the Locus hackathon pitch.
//
// Multi-page flow:
//   0:00 - 1:45  → file://demo/demo.html  (sections 1-4, ARMS-Insight-Problem stack)
//   1:45 - 2:10  → still demo.html  (section 5: animated terminal w/ real CLI output)
//   2:10 - 2:30  → REAL Solscan tx + RetrievalAttestation PDA (live devnet)
//   2:30 - 3:00  → demo.html?seek=150  (section 6, vision + ask)
//
// Run:
//   npx playwright test scripts/demo-solscan.spec.ts

import { test } from '@playwright/test';
import path from 'path';

const DEMO = `file://${path.resolve(__dirname, '..', 'demo', 'demo.html')}`;
const CLUSTER = 'devnet';
const ATTEST_TX  = '4UxWaB1TXvJdxwEfchLd7cKnAS17MhXDJ8zcfhNGzg5CLfbzYdSDghvjbQkL59zp5aoGdFGnsTJ2koKYxHxLj8iX';
const ATTEST_PDA = '9z8SwfDQRu2Nt1wbtQooLitpb3WQXge8PzxdyYebQLVt';

const beat = (ms: number) => new Promise(r => setTimeout(r, ms));

test('Locus pitch — hybrid slides + live devnet (3:00)', async ({ page }) => {
  test.setTimeout(220_000);
  await page.setViewportSize({ width: 1920, height: 1080 });

  // Phase A — sections 1-5 from demo.html (animated slides + animated terminal)
  await page.goto(DEMO);
  await page.waitForSelector('#s1.live', { timeout: 5_000 });
  await beat(130_000); // wall-clock 0:00 → 2:10

  // Phase B — real Solscan walkthrough (2:10 → 2:30)
  // Tx page first (shows decoded instructions, fee, logs)
  await page.goto(`https://solscan.io/tx/${ATTEST_TX}?cluster=${CLUSTER}`, { waitUntil: 'domcontentloaded', timeout: 30_000 }).catch(() => {});
  await beat(2_500);
  await page.mouse.wheel(0, 350);
  await beat(2_500);
  await page.mouse.wheel(0, 350);
  await beat(2_500);

  // Then the RetrievalAttestation PDA itself
  await page.goto(`https://solscan.io/account/${ATTEST_PDA}?cluster=${CLUSTER}`, { waitUntil: 'domcontentloaded', timeout: 30_000 }).catch(() => {});
  await beat(3_000);
  await page.mouse.wheel(0, 350);
  await beat(2_500);
  await page.mouse.wheel(0, 350);
  await beat(2_500);

  // Phase C — resume demo.html at section 6 (2:30 → 3:00)
  await page.goto(`${DEMO}?seek=150`);
  await page.waitForSelector('#s6.live', { timeout: 5_000 });
  await beat(31_000);
});

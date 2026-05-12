// Playwright recording for the Locus hackathon pitch.
//
// Multi-page flow:
//   0:00 - 1:45  → file://demo/demo.html  (sections 1-4)
//   1:45 - 2:10  → still demo.html  (section 5: animated terminal w/ real CLI output)
//   2:10 - 2:30  → REAL explorer.solana.com  (devnet tx + RetrievalAttestation account)
//   2:30 - 3:00  → demo.html?seek=150  (section 6, vision + ask)
//
// We use explorer.solana.com (the official Solana Foundation explorer) instead of
// solscan.io because Solscan presents a Cloudflare challenge to headless chromium.

import { test } from '@playwright/test';
import path from 'path';

const DEMO = `file://${path.resolve(__dirname, '..', 'demo', 'demo.html')}`;
const CLUSTER = 'devnet';
const ATTEST_TX  = '4UxWaB1TXvJdxwEfchLd7cKnAS17MhXDJ8zcfhNGzg5CLfbzYdSDghvjbQkL59zp5aoGdFGnsTJ2koKYxHxLj8iX';
const ATTEST_PDA = '9z8SwfDQRu2Nt1wbtQooLitpb3WQXge8PzxdyYebQLVt';
const PROGRAM_ID = 'C6AJ43ZpzPLtmcwDS1FQP7cQXtWHNwsLty5ijdLTxzmK';

const beat = (ms: number) => new Promise(r => setTimeout(r, ms));

test('Locus pitch — hybrid slides + live devnet (3:00)', async ({ page }) => {
  test.setTimeout(260_000);
  await page.setViewportSize({ width: 1920, height: 1080 });

  // Phase A — sections 1-5 from demo.html (animated slides + animated terminal)
  await page.goto(DEMO);
  await page.waitForSelector('#s1.live', { timeout: 5_000 });
  await beat(130_000); // 0:00 → 2:10

  // Phase B — real explorer.solana.com walkthrough (2:10 → 2:30)
  // The official explorer is bot-friendly (no Cloudflare). Pages render fast
  // and the on-chain decoded data is visible without any login.

  // 1. The attest_retrieval transaction — shows decoded instructions + fee
  await page.goto(`https://explorer.solana.com/tx/${ATTEST_TX}?cluster=${CLUSTER}`, {
    waitUntil: 'networkidle',
    timeout: 25_000,
  }).catch(() => {});
  await beat(1_500);
  // give the React app time to render rows beyond skeleton
  await page.waitForSelector('text=Transaction', { timeout: 8_000 }).catch(() => {});
  await beat(3_500);
  // scroll to show the instruction details + program logs
  await page.evaluate(() => window.scrollBy({ top: 600, behavior: 'smooth' }));
  await beat(3_000);
  await page.evaluate(() => window.scrollBy({ top: 500, behavior: 'smooth' }));
  await beat(2_000);

  // 2. The RetrievalAttestation PDA account — shows owner = locus program + data
  await page.goto(`https://explorer.solana.com/address/${ATTEST_PDA}?cluster=${CLUSTER}`, {
    waitUntil: 'networkidle',
    timeout: 25_000,
  }).catch(() => {});
  await beat(2_000);
  await page.waitForSelector('text=Account', { timeout: 8_000 }).catch(() => {});
  await beat(2_500);
  await page.evaluate(() => window.scrollBy({ top: 500, behavior: 'smooth' }));
  await beat(3_000);

  // Phase C — resume demo.html at section 6 (2:30 → 3:00)
  await page.goto(`${DEMO}?seek=150`);
  await page.waitForSelector('#s6.live', { timeout: 5_000 });
  await beat(31_000);
});

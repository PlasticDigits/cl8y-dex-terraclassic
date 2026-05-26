import { test, expect, type Page } from '@playwright/test'

import { isLocalTerraOptional } from './chain'
import { firstDualCwPair, type LcdPairInfo } from './lcd'

const DEFAULT_DUAL_PAIR_MSG =
  'No dual-CW20 pair on factory first page; run bash scripts/deploy-dex-local.sh with LocalTerra up (see docs/testing.md § E2E).'

/** Strict default: fail when the deployed factory has no dual-CW20 pair. Optional chain skips instead. */
export function requireDualCwPair(
  pairs: LcdPairInfo[],
  detail = DEFAULT_DUAL_PAIR_MSG
): { pair: LcdPairInfo; index: number } {
  const hit = firstDualCwPair(pairs)
  if (hit) return hit
  if (isLocalTerraOptional()) {
    test.skip(true, detail)
  }
  expect(hit, detail).toBeTruthy()
  return hit as { pair: LcdPairInfo; index: number }
}

const HYBRID_CONTROLS_MSG =
  'Direct swap hybrid controls hidden; pick a dual-CW20 pair with a direct route (see scripts/e2e-seed-hybrid-book.sh and docs/testing.md).'

/** Settings panel must expose the limit-book leg controls for direct CW20 swaps. */
export async function requireHybridControlsVisible(page: Page, detail = HYBRID_CONTROLS_MSG): Promise<void> {
  const hybridHeading = page.getByText('Direct swap: limit book leg')
  if ((await hybridHeading.count()) > 0) return
  if (isLocalTerraOptional()) {
    test.skip(true, detail)
  }
  expect(await hybridHeading.count(), detail).toBeGreaterThan(0)
}

const PAUSED_PAIR_MSG =
  'Selected pair is paused by governance; unpause or redeploy LocalTerra for hybrid E2E (invariant L6).'

export async function skipOrFailIfPairPaused(page: Page, detail = PAUSED_PAIR_MSG): Promise<void> {
  const paused = page.getByRole('status').filter({ hasText: /paused by governance/i })
  if (!(await paused.isVisible().catch(() => false))) return
  if (isLocalTerraOptional()) {
    test.skip(true, detail)
  }
  expect(await paused.isVisible(), detail).toBe(false)
}

const SWAP_CTA_MSG =
  'Hybrid swap CTA blocked after E2E provisioning; verify scripts/e2e-provision-dev-wallet.sh and scripts/e2e-seed-hybrid-book.sh (GitLab #193).'

/** Primary swap submit control must not show Insufficient Balance / No Route after global setup. */
export function assertHybridSwapCtaNotBlocked(label: string | null, detail = SWAP_CTA_MSG): void {
  expect(label, detail).not.toMatch(/Insufficient Balance|No Route|Connect/i)
}

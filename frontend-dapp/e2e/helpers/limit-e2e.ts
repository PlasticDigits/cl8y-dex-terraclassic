import { test, expect, type Page } from '@playwright/test'
import type { APIRequestContext } from '@playwright/test'

import { isChainOptional } from './chain'
import { firstUnpausedDualCwPair, type LcdPairInfo } from './lcd'
import { skipOrFailIfPairPaused } from './hybrid-e2e'

const DEFAULT_LIMIT_PAIR_MSG =
  'No unpaused dual-CW20 pair on factory; run bash scripts/deploy-dex-local.sh with LocalTerra up (see docs/testing.md § E2E, GitLab #195).'

/** Strict default: fail when every dual-CW20 pair is paused or missing. Optional chain skips instead. */
export async function requireLimitTxPair(
  request: APIRequestContext,
  pairs: LcdPairInfo[],
  detail = DEFAULT_LIMIT_PAIR_MSG
): Promise<{ pair: LcdPairInfo; index: number }> {
  const hit = await firstUnpausedDualCwPair(request, pairs)
  if (hit) return hit
  if (isChainOptional()) {
    test.skip(true, detail)
  }
  expect(hit, detail).toBeTruthy()
  return hit as { pair: LcdPairInfo; index: number }
}

/**
 * Select a factory pair in the Limits page MenuSelect.
 * `factoryIndex` is the index in the factory `pairs` array; option `nth(factoryIndex + 1)` skips the placeholder row.
 */
export async function selectLimitPairByFactoryIndex(page: Page, factoryIndex: number): Promise<void> {
  const pairTrigger = page.locator('#limit-pair')
  await expect(pairTrigger).toBeVisible({ timeout: 60_000 })
  await expect(pairTrigger).toBeEnabled({ timeout: 60_000 })
  await pairTrigger.click()
  await page
    .getByRole('option')
    .nth(factoryIndex + 1)
    .click()
  await expect(pairTrigger).toContainText(/\//, { timeout: 30_000 })
  await skipOrFailIfPairPaused(page)
}

const PLACE_CTA_MSG =
  'Place limit CTA blocked after E2E provisioning; verify scripts/e2e-provision-dev-wallet.sh (GitLab #195).'

/** Place limit submit must not show Insufficient Balance / Connect after global setup. */
export function assertLimitPlaceCtaNotBlocked(label: string | null, detail = PLACE_CTA_MSG): void {
  expect(label, detail).not.toMatch(/Insufficient Balance|Connect Wallet|Connect/i)
}

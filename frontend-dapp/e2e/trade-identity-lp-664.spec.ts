import { test, expect } from '@playwright/test'
import { e2eTradePairFromDeploy } from './helpers/deploy-pair'

const TRADE_PAIR = e2eTradePairFromDeploy()

async function skipIfNoIdentityRow(page: import('@playwright/test').Page, timeoutMs = 45_000) {
  const row = page.getByTestId('pair-token-links').first()
  try {
    await expect(row).toBeVisible({ timeout: timeoutMs })
  } catch {
    test.skip(true, 'No factory/indexer pair with a checksummed address on this stack')
  }
}

/**
 * Playwright smoke for Trade / Charts v2 LP identity chip (GitLab #664).
 * Chip is present iff the indexer stamped `liquidity_usd`; otherwise quiet omit.
 * PLAYWRIGHT_SKIP_CHAIN=1 skips chain seed (5 workers, no e2e-tx).
 */
test.describe('Trade / Charts v2 LP identity (GitLab #664)', () => {
  test('E1: /trade identity is quiet (v2 LP $ or omit — not garbage HTML)', async ({ page }) => {
    await page.goto(`/trade/${TRADE_PAIR}`, { waitUntil: 'domcontentloaded' })
    await expect(page.getByTestId('trade-pair-select-panel')).toBeVisible({ timeout: 30_000 })
    await skipIfNoIdentityRow(page)
    const chip = page.getByTestId('token-identity-v2-lp-usd')
    const n = await chip.count()
    if (n > 0) {
      await expect(chip.first()).toContainText('v2 LP')
      await expect(chip.first()).toContainText('$')
      expect(await chip.first().locator('a').count()).toBe(0)
    }
    await expect(page.getByTestId('trade-pair-select-panel').locator('.card-glass')).toHaveCount(0)
  })

  test('E2: /charts identity chip matches Trade semantics (present iff stamped)', async ({ page }) => {
    await page.goto(`/charts/${TRADE_PAIR}`, { waitUntil: 'domcontentloaded' })
    await expect(page.getByRole('heading', { name: /charts & analytics/i })).toBeVisible({
      timeout: 30_000,
    })
    await skipIfNoIdentityRow(page)
    const chip = page.getByTestId('token-identity-v2-lp-usd')
    const n = await chip.count()
    if (n > 0) {
      await expect(chip.first()).toContainText('v2 LP')
      await expect(chip.first()).toContainText('$')
    }
    const vol = page.getByTestId('charts-pair-volume-usd')
    if ((await vol.count()) > 0) {
      await expect(vol.first()).toBeVisible()
    }
  })

  test('E3: invalid /trade deep link has no LP chip', async ({ page }) => {
    await page.goto('/trade/lilwayne%20babyyy', { waitUntil: 'domcontentloaded' })
    await expect(page.getByTestId('trade-invalid-pair-link-notice')).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('token-identity-v2-lp-usd')).toHaveCount(0)
    await expect(page.getByTestId('pair-token-links')).toHaveCount(0)
  })
})

import { test, expect } from '@playwright/test'
import { selectLimitPairByFactoryIndex } from './helpers/limit-e2e'

/**
 * CI: job frontend-e2e-indexer-outage. Local: make test-e2e-indexer-outage (GitLab #218, #219).
 */
test.describe('Limits page market-data outage banner (GitLab #218)', () => {
  test.skip(process.env.E2E_INDEXER_OUTAGE !== '1', 'Set E2E_INDEXER_OUTAGE=1 with indexer stopped')

  test('shows retail banner without env URLs', async ({ page }) => {
    await page.goto('/limits', { waitUntil: 'networkidle' })
    await selectLimitPairByFactoryIndex(page, 0)

    const banner = page.getByTestId('limits-market-data-outage-banner')
    await expect(banner).toBeVisible({ timeout: 30_000 })
    await expect(banner).toContainText(/market data service unavailable/i)
    await expect(banner).not.toContainText(/VITE_INDEXER_URL|127\.0\.0\.1/i)
  })
})

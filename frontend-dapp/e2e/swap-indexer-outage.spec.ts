import { test, expect } from '@playwright/test'

/**
 * Indexer stopped + E2E_INDEXER_OUTAGE=1 (GitLab #241, #219).
 * CI: job frontend-e2e-indexer-outage. Local: make test-e2e-indexer-outage
 */
test.describe('Swap page market-data outage banner (GitLab #241)', () => {
  test.skip(process.env.E2E_INDEXER_OUTAGE !== '1', 'Set E2E_INDEXER_OUTAGE=1 with indexer stopped')

  test('shows retail banner without env URLs after quote attempt', async ({ page }) => {
    await page.goto('/', { waitUntil: 'networkidle' })
    await page.getByPlaceholder('0.00').fill('1')

    const banner = page.getByTestId('swap-market-data-outage-banner')
    await expect(banner).toBeVisible({ timeout: 30_000 })
    await expect(banner).toContainText(/market data service unavailable/i)
    await expect(banner).not.toContainText(/VITE_INDEXER_URL|127\.0\.0\.1/i)
  })
})

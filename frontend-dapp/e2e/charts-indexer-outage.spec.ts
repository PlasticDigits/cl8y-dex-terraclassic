import { test, expect } from '@playwright/test'

/**
 * Manual QA: run with indexer stopped (GitLab #215).
 * `E2E_INDEXER_OUTAGE=1 npx playwright test e2e/charts-indexer-outage.spec.ts`
 */
test.describe('Charts page market-data outage banner (GitLab #215)', () => {
  test.skip(process.env.E2E_INDEXER_OUTAGE !== '1', 'Set E2E_INDEXER_OUTAGE=1 with indexer stopped')

  test('shows retail banner without env URLs', async ({ page }) => {
    await page.goto('/charts', { waitUntil: 'networkidle' })

    const banner = page.getByTestId('charts-market-data-outage-banner')
    await expect(banner).toBeVisible({ timeout: 30_000 })
    await expect(banner).toContainText(/market data service unavailable/i)
    await expect(banner).not.toContainText(/VITE_INDEXER_URL|127\.0\.0\.1/i)
  })
})

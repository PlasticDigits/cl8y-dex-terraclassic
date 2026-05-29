import { test, expect } from '@playwright/test'

/**
 * Manual QA: run with indexer stopped (GitLab #218).
 * `E2E_INDEXER_OUTAGE=1 npx playwright test e2e/limits-indexer-outage.spec.ts`
 */
test.describe('Limits page market-data outage banner (GitLab #218)', () => {
  test.skip(process.env.E2E_INDEXER_OUTAGE !== '1', 'Set E2E_INDEXER_OUTAGE=1 with indexer stopped')

  test('shows retail banner without env URLs', async ({ page }) => {
    await page.goto('/limits', { waitUntil: 'networkidle' })
    const pairControl = page.locator('#limit-pair')
    await expect(pairControl).toBeVisible({ timeout: 30_000 })
    await pairControl.click()
    await page.getByRole('option').first().click()

    const banner = page.getByTestId('limits-market-data-outage-banner')
    await expect(banner).toBeVisible({ timeout: 30_000 })
    await expect(banner).toContainText(/market data service unavailable/i)
    await expect(banner).not.toContainText(/VITE_INDEXER_URL|127\.0\.0\.1/i)
  })
})

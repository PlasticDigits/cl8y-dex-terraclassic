import { test, expect } from '@playwright/test'

const PAIR = process.env.E2E_TRADE_PAIR ?? 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'

/**
 * Manual QA: run with indexer stopped (GitLab #165).
 * `E2E_INDEXER_OUTAGE=1 npx playwright test e2e/trade-indexer-outage.spec.ts`
 */
test.describe('Trade page indexer outage panels (GitLab #165)', () => {
  test.skip(process.env.E2E_INDEXER_OUTAGE !== '1', 'Set E2E_INDEXER_OUTAGE=1 with indexer stopped')

  test('book, tape, and chart show market-data-unavailable copy', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto(`/trade/${PAIR}`, { waitUntil: 'networkidle' })

    await expect(page.getByTestId('trade-indexer-outage-banner')).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('trade-indexer-outage-banner')).toContainText(/market data service unavailable/i)

    await expect(page.getByTestId('trade-tape-unavailable')).toBeVisible()
    await expect(page.getByTestId('trade-tape-unavailable')).toContainText(/recent trades are unavailable/i)

    await expect(page.getByTestId('trade-chart-unavailable')).toBeVisible()
    await expect(page.getByTestId('trade-chart-unavailable')).toContainText(/price chart is unavailable/i)

    await expect(page.getByTestId('trade-book-unavailable-bid')).toBeVisible()
    await expect(page.getByTestId('trade-book-unavailable-ask')).toBeVisible()
  })
})

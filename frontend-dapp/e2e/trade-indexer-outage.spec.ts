import { test, expect } from '@playwright/test'

import { e2eTradePairFromDeploy } from './helpers/deploy-pair'

const PAIR = e2eTradePairFromDeploy()

/**
 * Indexer stopped + E2E_INDEXER_OUTAGE=1 (GitLab #165, #219).
 * CI: job frontend-e2e-indexer-outage. Local: make test-e2e-indexer-outage
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

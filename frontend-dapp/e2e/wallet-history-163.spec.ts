import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import { requireLimitTxPair, selectLimitPairByFactoryIndex } from './helpers/limit-e2e'
import { gotoAndCaptureFactoryPairsPage, reloadAndCaptureFactoryPairsPage } from './helpers/lcd'

test.describe('Wallet indexer history (#163)', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('cl8y-dex-risk-ack', JSON.stringify({ v: 1 }))
    })
  })

  test('limits page shows history panel with swaps section', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet

    await page.goto('/limits', { waitUntil: 'networkidle' })
    let pairs = await reloadAndCaptureFactoryPairsPage(page).catch(
      () => [] as Awaited<ReturnType<typeof reloadAndCaptureFactoryPairsPage>>
    )
    if (pairs.length === 0) {
      pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
    }
    const { pair } = await requireLimitTxPair(request, pairs)
    await selectLimitPairByFactoryIndex(page, pair.contract_addr)

    const history = page.getByTestId('wallet-indexer-history')
    await expect(history).toBeVisible({ timeout: 30_000 })
    await expect(history.getByRole('heading', { name: /Your history/i })).toBeVisible()
    await expect(history.getByText('Swaps (AMM)')).toBeVisible()
    await expect(history.getByText('Limit fills (maker)')).toBeVisible()
    await expect(history.getByText('Limit cancellations')).toBeVisible()
    await expect(history.getByTestId('wallet-history-download-csv').first()).toBeVisible()
  })

  test('limits page CSV download hits format=csv and shows amounts when rows exist (#479)', async ({
    page,
    connectWallet,
    request,
  }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet

    await page.goto('/limits', { waitUntil: 'networkidle' })
    let pairs = await reloadAndCaptureFactoryPairsPage(page).catch(
      () => [] as Awaited<ReturnType<typeof reloadAndCaptureFactoryPairsPage>>
    )
    if (pairs.length === 0) {
      pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
    }
    const { pair } = await requireLimitTxPair(request, pairs)
    await selectLimitPairByFactoryIndex(page, pair.contract_addr)

    const history = page.getByTestId('wallet-indexer-history')
    await expect(history).toBeVisible({ timeout: 30_000 })

    const scroll = history.getByTestId('wallet-history-table-scroll').first()
    if (await scroll.isVisible().catch(() => false)) {
      await expect(history.getByRole('columnheader', { name: 'Amount in' }).first()).toBeVisible()
      await expect(history.getByRole('columnheader', { name: 'Amount out' }).first()).toBeVisible()
    }

    const csvResponsePromise = page.waitForResponse(
      (res) =>
        res.url().includes('/api/v1/traders/') && res.url().includes('format=csv') && res.request().method() === 'GET',
      { timeout: 30_000 }
    )
    await history.getByTestId('wallet-history-download-csv').first().click()
    const csvRes = await csvResponsePromise
    expect(csvRes.ok()).toBeTruthy()
    await expect(history.getByTestId('wallet-history-csv-error')).toHaveCount(0)
  })

  test('trade page shows swaps-only history', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/trade')
    const { pair } = await requireLimitTxPair(request, pairs)
    await page.goto(`/trade/${pair.contract_addr}`, { waitUntil: 'networkidle' })

    const history = page.getByTestId('wallet-indexer-history')
    await expect(history).toBeVisible({ timeout: 60_000 })
    await expect(history.getByText('Swaps (AMM)')).toBeVisible()
    await expect(history.getByText('Limit fills (maker)')).toHaveCount(0)
    await expect(history.getByText('Limit cancellations')).toHaveCount(0)
  })
})

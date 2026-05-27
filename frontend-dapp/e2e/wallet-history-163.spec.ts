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
    const { index } = await requireLimitTxPair(request, pairs)
    await selectLimitPairByFactoryIndex(page, index)

    const history = page.getByTestId('wallet-indexer-history')
    await expect(history).toBeVisible({ timeout: 30_000 })
    await expect(history.getByRole('heading', { name: /Your history/i })).toBeVisible()
    await expect(history.getByText('Swaps (AMM)')).toBeVisible()
    await expect(history.getByText('Limit fills (maker)')).toBeVisible()
    await expect(history.getByText('Limit cancellations')).toBeVisible()
    await expect(history.getByRole('button', { name: 'Download CSV' }).first()).toBeVisible()
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

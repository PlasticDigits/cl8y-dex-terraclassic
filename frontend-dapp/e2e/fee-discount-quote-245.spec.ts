import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import {
  E2E_DEV_WALLET,
  captureHybridSimulationQuote,
  captureRouterSimulateQuote,
  submitTxButtonWithRetry,
} from './helpers/fee-discount-quote-e2e'
import { requireDualCwPair } from './helpers/hybrid-e2e'
import {
  assetInfoLabel,
  fetchTxJson,
  gotoAndCaptureFactoryPairsPage,
  readTxHashFromAlertLink,
  txJsonWasmAttrForAction,
} from './helpers/lcd'

/** Small pay amount — enough fee-discount delta without draining E2E balances. */
const POOL_ONLY_AMOUNT_HUMAN = '0.01'

async function openTradeMarketPoolOnly(page: import('@playwright/test').Page, pairAddr: string) {
  await page.goto(`/trade/${pairAddr}`)
  await page.waitForLoadState('networkidle')
  await page.getByTestId('trade-order-tab-market').click()
  const hybridBook = page.getByRole('checkbox', { name: /Use hybrid book/i })
  if (await hybridBook.isChecked()) {
    await hybridBook.uncheck()
  }
}

test.describe.configure({ mode: 'serial' })

test.describe('CL8Y fee-discount quote parity (GitLab #245)', () => {
  test('Trade market: hybrid_simulation sends trader; executed return matches quote', async ({
    page,
    connectWallet,
    request,
  }) => {
    test.setTimeout(300_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/')
    const { pair } = requireDualCwPair(pairs)
    await openTradeMarketPoolOnly(page, pair.contract_addr)

    const quotePromise = captureHybridSimulationQuote(page, { requireTrader: E2E_DEV_WALLET })
    await page.getByTestId('limit-order-escrow-amount-input').fill(POOL_ONLY_AMOUNT_HUMAN)
    const quoted = await quotePromise

    await expect(page.getByTestId('trade-market-quote')).toBeVisible({ timeout: 60_000 })
    await expect(page.getByTestId('trade-market-quote')).toContainText(/\d/)

    const submit = page.getByTestId('trade-market-submit')
    await expect(submit).toBeEnabled({ timeout: 60_000 })
    const successAlert = await submitTxButtonWithRetry(page, submit)
    const txHash = await readTxHashFromAlertLink(page, successAlert)
    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      expect(json).toBeTruthy()
      const executed = txJsonWasmAttrForAction(json, 'swap', 'return_amount')
      expect(executed, 'swap wasm return_amount').toBeTruthy()
      expect(executed).toBe(quoted.returnAmount)
    }).toPass({ timeout: 180_000 })
  })

  test('Swap page: indexer route + router sim send trader (wallet LCD quote)', async ({
    page,
    connectWallet,
    request,
  }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/')
    const { pair } = requireDualCwPair(pairs)
    const t0 = assetInfoLabel(pair.asset_infos[0])
    const t1 = assetInfoLabel(pair.asset_infos[1])

    await page.getByLabel('Select token you pay').click()
    await page.getByTestId(`token-option-${t0}`).click()
    await page.getByLabel('Select token you receive').click()
    await page.getByTestId(`token-option-${t1}`).click()

    const routeSolvePromise = page.waitForResponse(
      (r) =>
        r.request().method() === 'GET' &&
        r.ok() &&
        r.url().includes('/api/v1/route/solve') &&
        r.url().includes('trader='),
      { timeout: 120_000 }
    )
    const routerSimPromise = captureRouterSimulateQuote(page, { requireTrader: E2E_DEV_WALLET })
    await page.getByPlaceholder('0.00').first().fill('0.001')

    const routeResp = await routeSolvePromise
    expect(routeResp.url()).toContain(E2E_DEV_WALLET)
    const quoted = await routerSimPromise
    expect(quoted.trader).toBe(E2E_DEV_WALLET)

    await expect(page.getByTestId('swap-route-summary')).toBeVisible({ timeout: 60_000 })
    const receive = page.locator('.swap-io-card-receive').getByText(/\d/)
    await expect(receive.first()).toBeVisible({ timeout: 30_000 })
  })
})

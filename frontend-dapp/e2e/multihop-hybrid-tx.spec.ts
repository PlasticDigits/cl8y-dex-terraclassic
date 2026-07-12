import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import { E2E_DEV_WALLET } from './helpers/fee-discount-quote-e2e'
import { assertHybridSwapCtaNotBlocked } from './helpers/hybrid-e2e'
import {
  assertReturnWithinQuoteTolerance,
  captureMultihopHybridQuote,
  MULTIHOP_HYBRID_RECEIVE_SYMBOL,
  resolveMultihopHybridReceiveToken,
} from './helpers/multihop-hybrid-e2e'
import {
  assetInfoLabel,
  fetchTxJson,
  firstDualCwPair,
  readTxHashFromAlertLink,
  reloadAndCaptureFactoryPairsPage,
  txJsonHasWasmAction,
  txJsonLastSwapReturnAmount,
} from './helpers/lcd'
import { clickSwapSubmit, openSwapSettingsAndSetSlippage, swapActionPanel } from './helpers/swap-ui'
import { ARIA_SELECT_TOKEN_RECEIVE } from './helpers/token-select'

/** Minimum CORAL pay that attaches hybrid on hop 0 (CORAL→EMBER ask book). */
const PAY_AMOUNT = '600'
const SLIPPAGE_PERCENT = 15

async function enableExpertModeForSwap(page: import('@playwright/test').Page) {
  const enableExpert = page.getByTestId('swap-enable-expert-mode')
  if (await enableExpert.isVisible().catch(() => false)) {
    await enableExpert.click()
    await page.getByTestId('expert-mode-confirm-input').fill('ENABLE EXPERT MODE')
    await page.getByTestId('expert-mode-confirm-enable').click()
    return
  }
  const expert = page.getByTestId('swap-expert-mode-toggle')
  if (!(await expert.isChecked())) {
    await expert.click({ force: true })
    const confirm = page.getByTestId('expert-mode-confirm-input')
    if (await confirm.isVisible().catch(() => false)) {
      await confirm.fill('ENABLE EXPERT MODE')
      await page.getByTestId('expert-mode-confirm-enable').click()
    }
  }
}

async function dismissGettingStartedIfPresent(page: import('@playwright/test').Page) {
  const dismiss = page.getByRole('button', { name: 'Dismiss getting started tips' })
  if (await dismiss.isVisible().catch(() => false)) {
    await dismiss.click()
  }
}

async function selectTokenByContract(
  page: import('@playwright/test').Page,
  label: 'Select token you pay' | 'Select token you receive',
  contract: string
) {
  await page.getByLabel(label).click()
  const opt = page.getByTestId(`token-option-${contract}`)
  await expect(opt, `token option ${contract} missing from ${label}`).toBeVisible({ timeout: 60_000 })
  await opt.click()
}

test.describe.configure({ mode: 'serial' })

test.describe('Multihop hybrid router swap (LocalTerra, GitLab #422)', () => {
  test('CORAL→IRON multihop hybrid tx matches quote within slippage', async ({
    page,
    connectWallet,
    request,
  }, testInfo) => {
    test.setTimeout(420_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await reloadAndCaptureFactoryPairsPage(page)
    const { pair } = firstDualCwPair(pairs)
    expect(pair, 'factory must expose dual-CW20 EMBER/CORAL pair for multihop hybrid seed').toBeTruthy()
    const coral = assetInfoLabel(pair!.asset_infos[1])
    const receive = await resolveMultihopHybridReceiveToken(request, pairs, coral)

    await page.waitForLoadState('networkidle')
    await dismissGettingStartedIfPresent(page)
    await expect(page.getByText('Loading pairs…')).toHaveCount(0, { timeout: 60_000 })
    await expect(page.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue(/ember/i, {
      timeout: 60_000,
    })

    // Default pair load is EMBER→CORAL; flip to CORAL pay then set IRON (or fallback) receive.
    await page.getByRole('button', { name: 'Swap pay and receive tokens' }).click()
    await selectTokenByContract(page, ARIA_SELECT_TOKEN_RECEIVE, receive)
    await expect(page.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue(
      new RegExp(MULTIHOP_HYBRID_RECEIVE_SYMBOL, 'i'),
      { timeout: 60_000 }
    )

    await openSwapSettingsAndSetSlippage(page, SLIPPAGE_PERCENT)
    await enableExpertModeForSwap(page)
    if (
      await page
        .locator('#swap-slippage-settings')
        .isVisible()
        .catch(() => false)
    ) {
      await page.getByRole('button', { name: 'Settings' }).click()
    }

    const routeSummary = page.getByTestId('swap-route-summary')
    const quote = await captureMultihopHybridQuote(page, PAY_AMOUNT, { requireTrader: E2E_DEV_WALLET })
    await expect(routeSummary).toBeVisible({ timeout: 60_000 })
    const routeText = (await routeSummary.textContent()) ?? ''
    expect(routeText.match(/→/g)?.length ?? 0, 'route summary should show multihop path').toBeGreaterThanOrEqual(2)
    expect(routeText.toUpperCase()).toContain(MULTIHOP_HYBRID_RECEIVE_SYMBOL)

    const swapPanel = swapActionPanel(page)
    const swapAction = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
    await expect(swapAction).toBeVisible({ timeout: 60_000 })
    assertHybridSwapCtaNotBlocked(await swapAction.textContent())

    await clickSwapSubmit(page, swapPanel)

    const successAlert = swapPanel.locator('.alert-success').first()
    await expect(successAlert).toBeVisible({ timeout: 120_000 })

    const screenshot = await page.screenshot({ fullPage: false })
    await testInfo.attach('multihop-hybrid-success', { body: screenshot, contentType: 'image/png' })

    const txHash = await readTxHashFromAlertLink(page, successAlert)
    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      expect(json, 'LCD tx not indexed yet').toBeTruthy()
      expect(txJsonHasWasmAction(json, 'limit_order_fill'), 'hybrid leg should emit limit_order_fill').toBe(true)
      const swapReturns = txJsonLastSwapReturnAmount(json)
      expect(swapReturns, 'final hop swap return_amount').toBeTruthy()
      assertReturnWithinQuoteTolerance(quote.simulateReturnAmount, swapReturns!, SLIPPAGE_PERCENT)
    }).toPass({ timeout: 180_000 })

    expect(quote.routeSolve.hops.length).toBeGreaterThanOrEqual(2)
  })
})

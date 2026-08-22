import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import { assertHybridSwapCtaNotBlocked, requireDualCwPair } from './helpers/hybrid-e2e'
import {
  MULTIHOP_HYBRID_RECEIVE_SYMBOL,
  captureMultihopHybridQuote,
  resolveMultihopHybridReceiveToken,
} from './helpers/multihop-hybrid-e2e'
import {
  assetInfoLabel,
  fetchTxJson,
  firstDualCwPair,
  readTxHashFromAlertLink,
  reloadAndCaptureFactoryPairsPage,
  txJsonWasmSwapHops,
} from './helpers/lcd'
import { assertDisplayedRouteMatchesTxHops, parseDisplayedRouteSymbols } from './helpers/route-alignment-e2e'
import { clickSwapSubmit, openSwapSettingsAndSetSlippage, swapActionPanel } from './helpers/swap-ui'
import { ARIA_SELECT_TOKEN_RECEIVE } from './helpers/token-select'

const DIRECT_AMOUNT = '0.001'
const MULTIHOP_PAY = '600'
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

async function readDisplayedRouteSymbols(page: import('@playwright/test').Page): Promise<string[]> {
  const routeSummary = page.getByTestId('swap-route-summary')
  await expect(routeSummary).toBeVisible({ timeout: 120_000 })
  const routeLine = (await routeSummary.locator('.font-mono').textContent()) ?? ''
  const symbols = parseDisplayedRouteSymbols(routeLine)
  expect(symbols.length, 'route line should list at least pay and receive').toBeGreaterThanOrEqual(2)
  return symbols
}

async function submitSwapAndCaptureHops(
  page: import('@playwright/test').Page,
  request: import('@playwright/test').APIRequestContext
) {
  const swapPanel = swapActionPanel(page)
  await clickSwapSubmit(page, swapPanel)
  const successAlert = swapPanel.locator('.alert-success').first()
  await expect(successAlert).toBeVisible({ timeout: 120_000 })
  const txHash = await readTxHashFromAlertLink(page, successAlert)
  let hops: ReturnType<typeof txJsonWasmSwapHops> = []
  await expect(async () => {
    const json = await fetchTxJson(request, txHash)
    expect(json, 'LCD tx not indexed yet').toBeTruthy()
    hops = txJsonWasmSwapHops(json)
    expect(hops.length, 'tx should emit wasm swap hops').toBeGreaterThan(0)
  }).toPass({ timeout: 180_000 })
  return hops
}

async function forceClientDirectPoolQuote(page: import('@playwright/test').Page): Promise<void> {
  await page.route('**/api/v1/route/solve**', (route) => route.abort('failed'))
  const settings = page.locator('#swap-slippage-settings')
  if (await settings.isVisible().catch(() => false)) {
    await swapActionPanel(page).getByRole('button', { name: 'Settings' }).click()
  }
}

test.describe.configure({ mode: 'serial' })

test.describe('Swap route display vs on-chain ops (SEC-E07 / GitLab #428)', () => {
  test('direct pair: displayed route matches single wasm swap hop', async ({ page, connectWallet, request }) => {
    test.setTimeout(300_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await reloadAndCaptureFactoryPairsPage(page)
    const { pair } = requireDualCwPair(pairs)
    const t0 = assetInfoLabel(pair.asset_infos[0])
    const t1 = assetInfoLabel(pair.asset_infos[1])

    await page.waitForLoadState('networkidle')
    await dismissGettingStartedIfPresent(page)

    await page.getByLabel('Select token you pay').click()
    await page.getByTestId(`token-option-${t0}`).click()
    await page.getByLabel('Select token you receive').click()
    await page.getByTestId(`token-option-${t1}`).click()

    await forceClientDirectPoolQuote(page)
    await openSwapSettingsAndSetSlippage(page, SLIPPAGE_PERCENT)
    await page.getByPlaceholder('0.00').first().fill(DIRECT_AMOUNT)

    const displayed = await readDisplayedRouteSymbols(page)
    expect(displayed, 'direct pair route should be exactly two tokens').toHaveLength(2)
    expect(displayed.join(' → ')).toMatch(/→/)

    const hops = await submitSwapAndCaptureHops(page, request)
    expect(hops, 'direct swap should emit exactly one wasm swap hop').toHaveLength(1)
    await assertDisplayedRouteMatchesTxHops(request, displayed, hops, 'direct pair')
    await page.unroute('**/api/v1/route/solve**')
  })

  test('multihop: displayed route matches wasm swap hop sequence', async ({ page, connectWallet, request }) => {
    test.setTimeout(420_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await reloadAndCaptureFactoryPairsPage(page)
    const { pair } = firstDualCwPair(pairs)
    expect(pair, 'factory must expose dual-CW20 pair for multihop route alignment').toBeTruthy()
    const coral = assetInfoLabel(pair!.asset_infos[1])
    const receive = await resolveMultihopHybridReceiveToken(request, pairs, coral)

    await page.waitForLoadState('networkidle')
    await dismissGettingStartedIfPresent(page)
    await expect(page.getByText('Loading pairs…')).toHaveCount(0, { timeout: 60_000 })

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
    await captureMultihopHybridQuote(page, MULTIHOP_PAY)

    const displayed = await readDisplayedRouteSymbols(page)
    expect(displayed.length, 'multihop route should show ≥3 symbols').toBeGreaterThanOrEqual(3)
    expect(
      (await page.getByTestId('swap-route-summary').locator('.font-mono').textContent())?.match(/→/g)?.length ?? 0
    ).toBeGreaterThanOrEqual(2)

    const swapPanel = swapActionPanel(page)
    const swapAction = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
    await expect(swapAction).toBeVisible({ timeout: 60_000 })
    assertHybridSwapCtaNotBlocked(await swapAction.textContent())

    const hops = await submitSwapAndCaptureHops(page, request)
    expect(hops.length, 'multihop swap should emit ≥2 wasm swap hops').toBeGreaterThanOrEqual(2)
    await assertDisplayedRouteMatchesTxHops(request, displayed, hops, 'multihop')
  })
})

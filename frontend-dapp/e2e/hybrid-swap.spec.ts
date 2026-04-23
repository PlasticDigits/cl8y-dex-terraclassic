import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, skipIfNoTxAlert } from './helpers/chain'
import {
  assetInfoLabel,
  fetchTxJson,
  firstDualCwPair,
  gotoAndCaptureFactoryPairsPage,
  readTxHashFromAlertLink,
  reloadAndCaptureFactoryPairsPage,
  txJsonHasWasmAction,
} from './helpers/lcd'

test.describe('Hybrid swap UI (LocalTerra)', () => {
  test('shows hybrid book disclosure and documentation link when book leg is set', async ({
    page,
    connectWallet,
    request,
  }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.waitForLoadState('networkidle')

    const pairs = await reloadAndCaptureFactoryPairsPage(page)
    const hit = firstDualCwPair(pairs)
    if (!hit) {
      test.skip(true, 'No dual-CW20 pair on first factory page.')
    }

    const { pair } = hit
    const t0 = assetInfoLabel(pair.asset_infos[0])
    const t1 = assetInfoLabel(pair.asset_infos[1])

    await page.getByLabel('Select token you pay').click()
    await page.getByTestId(`token-option-${t0}`).click()
    await page.getByLabel('Select token you receive').click()
    await page.getByTestId(`token-option-${t1}`).click()

    await page.getByRole('button', { name: 'Settings' }).click()

    const hybridHeading = page.getByText('Direct swap: limit book leg')
    if ((await hybridHeading.count()) === 0) {
      test.skip(true, 'No direct CW20 route for selected pair; hybrid controls hidden.')
    }

    await page.getByRole('checkbox', { name: /Route part of input through the limit book/i }).check()
    await page.locator('.card-neo').filter({ hasText: 'Book leg amount' }).getByPlaceholder('0.0').fill('0.01')
    await page.getByPlaceholder('0.00').first().fill('1')

    const alert = page.getByRole('alert')
    await expect(alert).toBeVisible({ timeout: 15_000 })
    await expect(alert).toContainText(/limit book/i)
    const doc = alert.getByRole('link', { name: /docs\/limit-orders\.md/i })
    await expect(doc).toHaveAttribute('href', /limit-orders\.md/)

    const execution = page.getByTestId('swap-execution-summary')
    await expect(execution).toBeVisible({ timeout: 15_000 })
    await expect(execution).toContainText(/Indexer hybrid/i)
    await expect(execution).toContainText(/pool \+ limit book/i)
  })

  test('shows quote source disclosure after amount for dual-CW20 route', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.waitForLoadState('networkidle')

    const pairs = await reloadAndCaptureFactoryPairsPage(page)
    const hit = firstDualCwPair(pairs)
    if (!hit) {
      test.skip(true, 'No dual-CW20 pair on first factory page.')
    }

    const { pair } = hit
    const t0 = assetInfoLabel(pair.asset_infos[0])
    const t1 = assetInfoLabel(pair.asset_infos[1])

    await page.getByLabel('Select token you pay').click()
    await page.getByTestId(`token-option-${t0}`).click()
    await page.getByLabel('Select token you receive').click()
    await page.getByTestId(`token-option-${t1}`).click()

    await page.getByPlaceholder('0.00').first().fill('0.001')

    await expect(page.getByText(/^Quote source:/i)).toBeVisible({ timeout: 120_000 })
  })
})

test.describe('Hybrid on-chain limit book fill (LocalTerra)', () => {
  test.describe.configure({ mode: 'serial' })

  test('hybrid swap emits wasm limit_order_fill (LCD)', async ({ page, connectWallet, request }) => {
    test.setTimeout(240_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
    const hit = firstDualCwPair(pairs)
    if (!hit) {
      test.skip(true, 'No dual-CW20 pair on first factory page.')
    }

    const { pair, index } = hit
    const t0 = assetInfoLabel(pair.asset_infos[0])
    const t1 = assetInfoLabel(pair.asset_infos[1])

    await expect(page.locator('#limit-pair')).toBeVisible({ timeout: 60_000 })

    const paused = page.getByRole('status').filter({ hasText: /paused by governance/i })
    if (await paused.isVisible().catch(() => false)) {
      test.skip(true, 'Selected pair is paused.')
    }

    await page.locator('#limit-pair').click()
    await page
      .getByRole('option')
      .nth(index + 1)
      .click()

    const placeCard = page.locator('.card-neo').filter({ hasText: 'Place limit' })
    await placeCard.getByPlaceholder('0.0').fill('50')
    await placeCard.getByRole('button', { name: /^Place limit$/i }).click()
    await skipIfNoTxAlert(page)

    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(page.getByLabel('Select token you pay')).toBeVisible({ timeout: 60_000 })

    await page.getByLabel('Select token you pay').click()
    await page.getByTestId(`token-option-${t0}`).click()
    await page.getByLabel('Select token you receive').click()
    await page.getByTestId(`token-option-${t1}`).click()

    await page.getByRole('button', { name: 'Settings' }).click()
    const hybridHeading = page.getByText('Direct swap: limit book leg')
    if ((await hybridHeading.count()) === 0) {
      test.skip(true, 'No direct CW20 route for selected pair; hybrid controls hidden.')
    }

    await page.getByRole('checkbox', { name: /Route part of input through the limit book/i }).check()
    await page.locator('.card-neo').filter({ hasText: 'Book leg amount' }).getByPlaceholder('0.0').fill('0.01')
    await page.getByPlaceholder('0.00').first().fill('1')

    const swapPanel = page.locator('main .shell-panel-strong').first()

    await expect(async () => {
      const calculating = swapPanel.getByRole('button', { name: /^Calculating/ })
      expect(await calculating.count()).toBe(0)
    }).toPass({ timeout: 120_000 })

    if (
      await swapPanel
        .getByRole('button', { name: /^Insufficient Balance$/ })
        .isVisible()
        .catch(() => false)
    ) {
      test.skip(true, 'Dev wallet has no spendable balance for hybrid swap.')
    }
    if (
      await swapPanel
        .getByRole('button', { name: /^No Route$/ })
        .isVisible()
        .catch(() => false)
    ) {
      test.skip(true, 'No swap route for the current selection.')
    }

    const swapAction = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
    await expect(swapAction).toBeVisible({ timeout: 60_000 })
    await expect(swapAction).toBeEnabled({ timeout: 30_000 })
    await swapAction.click()
    await page.waitForTimeout(500)
    const confirmSwap = swapPanel.getByRole('button').filter({ hasText: /^Confirm Swap/ })
    if (await confirmSwap.isVisible().catch(() => false)) {
      await confirmSwap.click()
    }

    const successAlert = swapPanel.locator('.alert-success')
    await skipIfNoTxAlert(page)
    const txHash = await readTxHashFromAlertLink(page, successAlert)

    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'limit_order_fill')).toBe(true)
    }).toPass({ timeout: 180_000 })
  })
})

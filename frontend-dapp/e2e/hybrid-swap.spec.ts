import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import { assertHybridSwapCtaNotBlocked, requireDualCwPair, requireHybridControlsVisible } from './helpers/hybrid-e2e'
import { openSwapAdvancedSettings } from './helpers/swap-ui'
import {
  assetInfoLabel,
  fetchTxJson,
  gotoAndCaptureFactoryPairsPage,
  readTxHashFromAlertLink,
  reloadAndCaptureFactoryPairsPage,
  txJsonHasWasmAction,
  txJsonWasmAttrForAction,
} from './helpers/lcd'

async function selectDualCwPairTokens(page: import('@playwright/test').Page, t0: string, t1: string): Promise<void> {
  await page.getByLabel('Select token you pay').click()
  await page.getByTestId(`token-option-${t0}`).click()
  await page.getByLabel('Select token you receive').click()
  await page.getByTestId(`token-option-${t1}`).click()
}

async function enableHybridBookLeg(page: import('@playwright/test').Page): Promise<void> {
  await openSwapAdvancedSettings(page)
  await requireHybridControlsVisible(page)
  await page.getByRole('checkbox', { name: /Route part of input through the limit book/i }).check()
  await page.locator('.card-glass').filter({ hasText: 'Book leg amount' }).getByPlaceholder('0.0').fill('0.01')
}

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
    const { pair } = requireDualCwPair(pairs)
    const t0 = assetInfoLabel(pair.asset_infos[0])
    const t1 = assetInfoLabel(pair.asset_infos[1])

    await selectDualCwPairTokens(page, t0, t1)
    await enableHybridBookLeg(page)
    await page.getByPlaceholder('0.00').first().fill('1')

    const alert = page.getByRole('alert')
    await expect(alert).toBeVisible({ timeout: 15_000 })
    await expect(alert).toContainText(/limit book/i)
    const doc = alert.getByRole('link', { name: /docs\/limit-orders\.md/i })
    await expect(doc).toHaveAttribute('href', /limit-orders\.md/)

    const execution = page.getByTestId('swap-execution-summary')
    await expect(execution).toBeVisible({ timeout: 15_000 })
    await expect(execution).toContainText(/pool \+ limit book/i)
    // Indexer hybrid line appears when route/solve succeeds; LCD-only path still shows hybrid split copy.
    const execText = await execution.textContent()
    expect(execText).toMatch(/Indexer hybrid|Hybrid \(pool \+ limit book\)/i)
  })

  test('shows single execution-aligned route row for dual-CW20 quote (GitLab #158)', async ({
    page,
    connectWallet,
    request,
  }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.waitForLoadState('networkidle')

    const pairs = await reloadAndCaptureFactoryPairsPage(page)
    const { pair } = requireDualCwPair(pairs)
    const t0 = assetInfoLabel(pair.asset_infos[0])
    const t1 = assetInfoLabel(pair.asset_infos[1])

    await selectDualCwPairTokens(page, t0, t1)
    await page.getByPlaceholder('0.00').first().fill('0.001')

    const routeSummary = page.getByTestId('swap-route-summary')
    await expect(routeSummary).toBeVisible({ timeout: 120_000 })
    await expect(routeSummary).toContainText(/→/)
    await expect(page.getByTestId('swap-route-summary')).toHaveCount(1)
    await expect(page.getByText(/^Quote source:/i)).toHaveCount(0)
    await expect(page.getByText(/Route \(indexer\)/i)).toHaveCount(0)
  })
})

test.describe('Hybrid on-chain limit book fill (LocalTerra)', () => {
  test.describe.configure({ mode: 'serial' })

  test('hybrid swap emits wasm limit_order_fill and book_return_amount (LCD)', async ({
    page,
    connectWallet,
    request,
  }) => {
    test.setTimeout(420_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/')
    const { pair } = requireDualCwPair(pairs)
    const t0 = assetInfoLabel(pair.asset_infos[0])
    const t1 = assetInfoLabel(pair.asset_infos[1])

    await page.waitForLoadState('networkidle')
    await expect(page.getByLabel('Select token you pay')).toBeVisible({ timeout: 60_000 })

    await selectDualCwPairTokens(page, t0, t1)
    await enableHybridBookLeg(page)
    await page.getByPlaceholder('0.00').first().fill('1')

    const swapPanel = page.locator('main .shell-panel-strong').first()

    await expect(async () => {
      const calculating = swapPanel.getByRole('button', { name: /^Calculating/ })
      expect(await calculating.count()).toBe(0)
    }).toPass({ timeout: 120_000 })

    const swapAction = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
    await expect(swapAction).toBeVisible({ timeout: 60_000 })
    assertHybridSwapCtaNotBlocked(await swapAction.textContent())

    await expect(swapAction).toBeEnabled({ timeout: 30_000 })
    await swapAction.click()
    await page.waitForTimeout(500)
    const confirmSwap = swapPanel.getByRole('button').filter({ hasText: /^Confirm Swap/ })
    if (await confirmSwap.isVisible().catch(() => false)) {
      await confirmSwap.click()
    }

    const successAlert = swapPanel.locator('.alert-success').first()
    const gasHint = swapPanel.getByText(/more gas than estimated/i)
    for (let gasRetry = 0; gasRetry < 10; gasRetry++) {
      if (await successAlert.isVisible().catch(() => false)) break
      if (await gasHint.isVisible().catch(() => false)) {
        await page.waitForTimeout(5_000)
        const retryBtn = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
        await expect(retryBtn).toBeEnabled({ timeout: 30_000 })
        await retryBtn.click()
        await page.waitForTimeout(500)
        if (await confirmSwap.isVisible().catch(() => false)) {
          await confirmSwap.click()
        }
        continue
      }
      await page.waitForTimeout(3_000)
    }

    await expect(swapPanel.locator('.alert-success, .alert-error').first()).toBeVisible({ timeout: 90_000 })
    const errorAlert = swapPanel.locator('.alert-error').first()
    if (await errorAlert.isVisible().catch(() => false)) {
      const msg = await errorAlert.textContent()
      expect(msg, 'hybrid swap should succeed after globalSetup book seed').not.toMatch(
        /sequence mismatch|insufficient/i
      )
    }
    await expect(successAlert).toBeVisible({ timeout: 30_000 })
    const txHash = await readTxHashFromAlertLink(page, successAlert)

    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'limit_order_fill')).toBe(true)
      const bookReturn = txJsonWasmAttrForAction(json, 'swap', 'book_return_amount')
      expect(bookReturn, 'hybrid swap wasm swap event should include book_return_amount').toBeTruthy()
      expect(BigInt(bookReturn!)).toBeGreaterThan(0n)
    }).toPass({ timeout: 180_000 })
  })
})

import { test, expect } from './fixtures/dev-wallet'
import {
  routeUst1EffectiveSwap,
  ust1WindowAddressFromEnv,
  type Ust1EffectiveSwapMock,
} from './helpers/ust1-window-lcd-mock'

/**
 * /ust1 oracle window CTA gates (GitLab #506).
 * Uses LCD mocks for effective_swap — does not require LocalTerra ust1-window deploy.
 * Requires Vite to bake VITE_UST1_* (playwright.config webServer.env defaults to columbus-5).
 */
async function openUst1(
  page: Parameters<typeof routeUst1EffectiveSwap>[0],
  connectWallet: Promise<void>,
  body: Ust1EffectiveSwapMock = {}
) {
  const window = ust1WindowAddressFromEnv()
  await routeUst1EffectiveSwap(page, window, body)
  await connectWallet
  await page.goto('/ust1')
  const unavailable = page.getByTestId('ust1-unavailable')
  if (await unavailable.isVisible().catch(() => false)) {
    test.skip(true, 'UST1 env not baked into Vite — restart Playwright webServer (ensure VITE_UST1_* are set)')
  }
}

test.describe('UST1 window UI gates (GitLab #506)', () => {
  test('loads Deposit/Withdraw and shows token symbols', async ({ page, connectWallet }) => {
    await openUst1(page, connectWallet)
    await expect(page.getByTestId('ust1-mode-tabs')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByTestId('ust1-tab-deposit')).toBeVisible()
    await expect(page.getByTestId('ust1-tab-withdraw')).toBeVisible()
    await expect(page.getByTestId('ust1-pay-symbol')).toContainText('vFDUSD')
    await expect(page.getByTestId('ust1-receive-symbol')).toContainText('UST1')
    await expect(page.getByTestId('ust1-oracle-status')).toHaveText('Fresh')
  })

  test('disables CTA when window paused', async ({ page, connectWallet }) => {
    await openUst1(page, connectWallet, { paused: true })
    const submit = page.getByTestId('ust1-submit')
    await expect(submit).toBeVisible({ timeout: 20_000 })
    await expect(submit).toBeDisabled()
    await expect(submit).toContainText(/paused/i)
  })

  test('disables CTA when oracle stale', async ({ page, connectWallet }) => {
    await openUst1(page, connectWallet, {
      oracle: { last_update_sec: 1, paused: false },
    })
    const submit = page.getByTestId('ust1-submit')
    await expect(submit).toBeVisible({ timeout: 20_000 })
    await expect(submit).toBeDisabled()
    await expect(submit).toContainText(/stale/i)
  })

  test('disables CTA when amount exceeds per-tx limit', async ({ page, connectWallet }) => {
    await openUst1(page, connectWallet, { per_tx_ust1_limit: '1000000' })
    await expect(page.getByTestId('ust1-pay-amount')).toBeVisible({ timeout: 20_000 })
    await page.getByTestId('ust1-pay-amount').fill('10')
    const submit = page.getByTestId('ust1-submit')
    await expect(submit).toBeDisabled()
    await expect(submit).toContainText(/per-tx/i)
    await expect(page.getByTestId('ust1-block-reason')).toContainText(/per-transaction/i)
  })

  test('disables CTA when amount exceeds remaining 24h capacity', async ({ page, connectWallet }) => {
    await openUst1(page, connectWallet, {
      rolling_24h_ust1_limit: '5000000',
      rolling_volume_ust1: '4500000',
    })
    await expect(page.getByTestId('ust1-pay-amount')).toBeVisible({ timeout: 20_000 })
    await page.getByTestId('ust1-pay-amount').fill('2')
    const submit = page.getByTestId('ust1-submit')
    await expect(submit).toBeDisabled()
    await expect(submit).toContainText(/24h/i)
    await expect(page.getByTestId('ust1-block-reason')).toContainText(/24h/i)
  })

  test('enables Deposit CTA with healthy quote and shows withdraw slippage note', async ({ page, connectWallet }) => {
    await openUst1(page, connectWallet)
    await expect(page.getByTestId('ust1-pay-amount')).toBeVisible({ timeout: 20_000 })
    await page.getByTestId('ust1-pay-amount').fill('1')
    const submit = page.getByTestId('ust1-submit')
    await expect(submit).toBeEnabled({ timeout: 15_000 })
    await expect(submit).toHaveText(/Deposit/i)
    await expect(page.getByTestId('ust1-receive-amount')).not.toHaveText('—')

    await page.getByTestId('ust1-tab-withdraw').click()
    await expect(page.getByTestId('ust1-withdraw-slippage-note')).toContainText(/1% minimum output/i)
  })
})

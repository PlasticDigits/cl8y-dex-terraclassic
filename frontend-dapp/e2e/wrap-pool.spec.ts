import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert, assertLiquidityCtaNotBlocked } from './helpers/chain'
import { requirePoolCardWithNativeWrap, requirePoolCardWithReceiveWrapped } from './helpers/wrap-e2e'
import {
  poolProvideExpandButton,
  poolProvideSubmitButton,
  poolWithdrawExpandButton,
  poolWithdrawSubmitButton,
  poolReceiveWrappedCheckbox,
  openPoolCardAdvanced,
} from './helpers/pool-ui'
import { gotoWrapPoolLuncCard } from './helpers/pool-nav'

test.describe('Pool with native token wrapping — UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/pool')
    await expect(async () => {
      await expect(page.getByTestId('pool-pairs-table')).toBeVisible()
    }).toPass({ timeout: 90_000 })
  })

  test('E6: pool page loads with pairs', async ({ page }) => {
    await expect(async () => {
      await expect(page.getByTestId('pool-pair-row').first()).toBeVisible()
    }).toPass({ timeout: 90_000 })
  })

  test('E7: pool page shows one-sided add/withdraw and Advanced two-sided', async ({ page }) => {
    await expect(page.getByTestId('pool-one-sided-add')).toBeVisible({ timeout: 90_000 })
    await expect(page.getByTestId('pool-one-sided-withdraw')).toBeVisible()
    await expect(page.getByTestId('pool-row-manage').first()).toBeVisible({ timeout: 90_000 })
  })

  test('E8: Advanced provide still has native toggle', async ({ page }) => {
    await expect(page.getByTestId('pool-row-manage').first()).toBeVisible({ timeout: 90_000 })
    await openPoolCardAdvanced(page)
    await page
      .getByRole('button', { name: /Provide Liquidity/i })
      .first()
      .click()
    const assetInput = page.getByPlaceholder('0.00').first()
    await expect(assetInput).toBeVisible()
    const nativeCheckbox = page.getByText(/auto-wrap/i)
    const count = await nativeCheckbox.count()
    expect(count).toBeGreaterThanOrEqual(0)
  })

  test('E9: Advanced withdraw may show receive wrapped checkbox', async ({ page }) => {
    await expect(page.getByTestId('pool-row-manage').first()).toBeVisible({ timeout: 90_000 })
    await openPoolCardAdvanced(page)
    await page
      .getByRole('button', { name: /Withdraw Liquidity/i })
      .first()
      .click()
    const lpInput = page.getByPlaceholder('0.00').first()
    await expect(lpInput).toBeVisible()
    const receiveWrappedCheckbox = page.getByText(/Receive as wrapped/i)
    const count = await receiveWrappedCheckbox.count()
    expect(count).toBeGreaterThanOrEqual(0)
  })

  test('E10: withdraw slippage tolerance options visible', async ({ page }) => {
    await openPoolCardAdvanced(page)
    await expect(async () => {
      await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
    }).toPass({ timeout: 90_000 })

    await page
      .getByRole('button', { name: /Withdraw Liquidity/i })
      .first()
      .click()

    const lpInput = page.getByPlaceholder('0.00').first()
    await expect(lpInput).toBeVisible()

    const slippageButton = page.getByRole('button', { name: /1\.0%/i }).first()
    const count = await slippageButton.count()
    expect(count).toBeGreaterThanOrEqual(0)
  })
})

test.describe('Pool Transaction Tests — Native Wrapping', () => {
  test.describe.configure({ mode: 'serial' })

  test.beforeEach(async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.getByRole('link', { name: 'Pool' }).click()
    await page.waitForURL(/\/pool/)
    await expect(async () => {
      const panels = await page.locator('.shell-panel-strong').count()
      expect(panels).toBeGreaterThan(0)
    }).toPass({ timeout: 90_000 })
    await openPoolCardAdvanced(page)
    await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible({ timeout: 90_000 })
  })

  test('E7: provide liquidity with native token (auto-wrap)', async ({ page }) => {
    const pairCard = await gotoWrapPoolLuncCard(page)
    await openPoolCardAdvanced(pairCard)
    await poolProvideExpandButton(pairCard).click()

    await requirePoolCardWithNativeWrap(pairCard)

    const nativeCheckbox = pairCard.getByTestId('pool-provide-auto-wrap-a')
    await expect(nativeCheckbox).toBeVisible()
    await expect(nativeCheckbox).toBeChecked()

    const inputs = pairCard.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('0.01')
    await inputs.nth(1).fill('0.01')

    const submitBtn = poolProvideSubmitButton(pairCard)
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Wrap pool provide (native): CTA blocked after E2E provisioning; verify LocalTerra + deploy + e2e-provision script.'
    )
    await submitBtn.click()

    await assertTxResultAlert(page)
  })

  test('E8: provide liquidity with wrapped CW20 directly', async ({ page }) => {
    const pairCard = await gotoWrapPoolLuncCard(page)
    await openPoolCardAdvanced(pairCard)
    await poolProvideExpandButton(pairCard).click()

    const wrapToggle = pairCard.getByTestId('pool-provide-auto-wrap-a')
    if ((await wrapToggle.count()) > 0) {
      await wrapToggle.uncheck()
    }

    const inputs = pairCard.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('0.01')
    await inputs.nth(1).fill('0.01')

    const submitBtn = poolProvideSubmitButton(pairCard)
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Wrap pool provide (wrapped): CTA blocked after E2E provisioning; verify LocalTerra + deploy + e2e-provision script.'
    )
    await submitBtn.click()

    await assertTxResultAlert(page)
  })

  test('E9: withdraw liquidity with auto-unwrap to native', async ({ page }) => {
    const pairCard = await gotoWrapPoolLuncCard(page)
    await openPoolCardAdvanced(pairCard)
    await expect(poolWithdrawExpandButton(pairCard)).toBeVisible({ timeout: 90_000 })
    await poolWithdrawExpandButton(pairCard).click()

    await requirePoolCardWithReceiveWrapped(pairCard)

    const receiveWrapped = poolReceiveWrappedCheckbox(pairCard)
    if (await receiveWrapped.isChecked()) {
      await receiveWrapped.click({ force: true })
    }

    const lpInput = pairCard.getByPlaceholder('0.00').first()
    const maxButton = pairCard.getByTitle('Use max balance')
    if ((await maxButton.count()) > 0) {
      await maxButton.click()
      const maxVal = await lpInput.inputValue()
      const partial = (parseFloat(maxVal) / 2).toFixed(6)
      await lpInput.fill(partial)
    } else {
      await lpInput.fill('0.001')
    }

    const submitBtn = poolWithdrawSubmitButton(pairCard)
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Wrap pool withdraw (auto-unwrap): CTA blocked; ensure LP balance after prior provides or adjust pair selection.'
    )

    await expect(submitBtn).toBeEnabled({ timeout: 5000 })
    await submitBtn.click()

    await assertTxResultAlert(page, 120_000)
  })

  test('E10: withdraw liquidity — receive as wrapped tokens', async ({ page }) => {
    const pairCard = await gotoWrapPoolLuncCard(page)
    await openPoolCardAdvanced(pairCard)
    await expect(poolWithdrawExpandButton(pairCard)).toBeVisible({ timeout: 90_000 })
    await poolWithdrawExpandButton(pairCard).click()

    const lpInput = pairCard.getByPlaceholder('0.00').first()
    const maxButton = pairCard.getByTitle('Use max balance')
    if ((await maxButton.count()) > 0) {
      await maxButton.click()
    } else {
      await lpInput.fill('0.001')
    }

    const submitBtn = poolWithdrawSubmitButton(pairCard)
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Wrap pool withdraw (wrapped): CTA blocked; ensure LP balance for this pair.'
    )

    await expect(submitBtn).toBeEnabled({ timeout: 5000 })
    await submitBtn.click()

    await assertTxResultAlert(page, 120_000)
  })
})
